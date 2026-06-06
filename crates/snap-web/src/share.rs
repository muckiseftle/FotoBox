//! Sharing: QR codes, a mobile download page reachable by scanning, and
//! e-mail delivery (with an offline retry queue).

use crate::auth::AdminAuth;
use crate::error::ApiError;
use crate::state::AppState;
use axum::extract::{Path, Query, State};
use axum::http::{header, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use snap_core::models::{email_queue, Photo};
use snap_core::settings::{self, keys};

// ---- QR --------------------------------------------------------------------

#[derive(Deserialize)]
pub struct QrQuery {
    /// The URL (or text) to encode — typically `${origin}/d/${token}`.
    data: String,
}

pub async fn qr(Query(q): Query<QrQuery>) -> Result<Response, ApiError> {
    if q.data.is_empty() || q.data.len() > 1024 {
        return Err(ApiError::bad_request("ungültige QR-Daten"));
    }
    let png = tokio::task::spawn_blocking(move || render_qr(&q.data))
        .await
        .map_err(|e| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))??;
    Ok((
        [
            (header::CONTENT_TYPE, "image/png"),
            (header::CACHE_CONTROL, "no-store"),
        ],
        png,
    )
        .into_response())
}

fn render_qr(data: &str) -> Result<Vec<u8>, ApiError> {
    use image::Luma;
    use qrcode::QrCode;
    let code = QrCode::new(data.as_bytes())
        .map_err(|e| ApiError::new(StatusCode::BAD_REQUEST, e.to_string()))?;
    let img = code
        .render::<Luma<u8>>()
        .min_dimensions(320, 320)
        .quiet_zone(true)
        .build();
    let mut buf = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageLuma8(img)
        .write_to(&mut buf, image::ImageFormat::Png)
        .map_err(|e| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(buf.into_inner())
}

// ---- Mobile download page (server-rendered, no SPA needed) -----------------

pub async fn download_page(
    State(st): State<AppState>,
    Path(token): Path<String>,
) -> Result<Response, ApiError> {
    // Verify the token exists (and is therefore a real share link).
    let _photo = Photo::by_token(&st.db, &token).await?;
    let event = settings::get_or(&st.db, keys::EVENT_NAME, "SnapStation").await?;
    let email_enabled = settings::get_or(&st.db, keys::EMAIL_ENABLED, "false").await? == "true";
    let lang = settings::get_or(&st.db, keys::LANGUAGE, "de").await?;
    Ok(Html(render_download_html(&token, &event, email_enabled, &lang)).into_response())
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn render_download_html(token: &str, event: &str, email_enabled: bool, lang: &str) -> String {
    let en = lang == "en";
    let event = html_escape(event);
    let l_download = if en {
        "Download photo"
    } else {
        "Foto herunterladen"
    };
    let l_email_ph = if en {
        "E-mail address"
    } else {
        "E-Mail-Adresse"
    };
    let l_email_btn = if en {
        "Send by e-mail"
    } else {
        "Per E-Mail senden"
    };
    let l_sending = if en { "Sending …" } else { "Sende …" };
    let l_sent = if en { "Sent ✓" } else { "Gesendet ✓" };
    let l_error = if en { "Error" } else { "Fehler" };
    let email_form = if email_enabled {
        format!(
            r#"<form id="ef" onsubmit="return sendMail(event)">
                 <input id="to" type="email" required placeholder="{l_email_ph}" />
                 <button type="submit">{l_email_btn}</button>
                 <p id="msg"></p>
               </form>
               <script>
                 async function sendMail(e){{e.preventDefault();const m=document.getElementById('msg');m.textContent='{l_sending}';
                   try{{const r=await fetch('/api/share/email',{{method:'POST',headers:{{'Content-Type':'application/json'}},body:JSON.stringify({{token:'{token}',to:document.getElementById('to').value}})}});
                     const d=await r.json().catch(()=>({{}}));m.textContent=r.ok?'{l_sent}':(d.error||'{l_error}');m.style.color=r.ok?'#34d399':'#f87171';}}
                   catch(err){{m.textContent='{l_error}';m.style.color='#f87171';}}return false;}}
               </script>"#
        )
    } else {
        String::new()
    };

    format!(
        r#"<!doctype html><html lang="de"><head>
<meta charset="utf-8"/><meta name="viewport" content="width=device-width, initial-scale=1"/>
<title>{event} – Foto</title>
<style>
  :root{{color-scheme:dark}}
  body{{margin:0;background:#0b0f17;color:#e5e7eb;font-family:system-ui,sans-serif;text-align:center}}
  .wrap{{max-width:560px;margin:0 auto;padding:20px}}
  h1{{font-size:1.25rem}}
  img.photo{{width:100%;border-radius:16px;box-shadow:0 10px 40px rgba(0,0,0,.5)}}
  a.btn,button{{display:inline-block;margin-top:14px;padding:14px 22px;border:0;border-radius:12px;
    background:#0ea5e9;color:#04121c;font-weight:700;font-size:1rem;text-decoration:none;cursor:pointer}}
  input{{width:100%;box-sizing:border-box;margin-top:14px;padding:12px;border-radius:12px;border:1px solid #334155;
    background:#0f172a;color:#e5e7eb;font-size:1rem}}
  form{{margin-top:8px}} p#msg{{min-height:1.2rem}}
</style></head><body><div class="wrap">
  <h1>{event}</h1>
  <img class="photo" src="/p/{token}" alt="Foto"/>
  <div><a class="btn" href="/p/{token}" download="{token}.jpg">{l_download}</a></div>
  {email_form}
</div></body></html>"#
    )
}

// ---- E-mail settings -------------------------------------------------------

#[derive(Serialize)]
pub struct EmailSettingsOut {
    enabled: bool,
    host: String,
    port: u16,
    user: String,
    from: String,
    has_password: bool,
}

#[derive(Deserialize)]
pub struct EmailSettingsIn {
    enabled: bool,
    host: String,
    port: u16,
    user: String,
    from: String,
    /// Only updated when present and non-empty.
    password: Option<String>,
}

pub async fn get_email_settings(
    _admin: AdminAuth,
    State(st): State<AppState>,
) -> Result<Json<EmailSettingsOut>, ApiError> {
    Ok(Json(EmailSettingsOut {
        enabled: settings::get_or(&st.db, keys::EMAIL_ENABLED, "false").await? == "true",
        host: settings::get_or(&st.db, keys::SMTP_HOST, "").await?,
        port: settings::get_or(&st.db, keys::SMTP_PORT, "587")
            .await?
            .parse()
            .unwrap_or(587),
        user: settings::get_or(&st.db, keys::SMTP_USER, "").await?,
        from: settings::get_or(&st.db, keys::SMTP_FROM, "").await?,
        has_password: !settings::get_or(&st.db, keys::SMTP_PASS, "")
            .await?
            .is_empty(),
    }))
}

pub async fn set_email_settings(
    _admin: AdminAuth,
    State(st): State<AppState>,
    Json(s): Json<EmailSettingsIn>,
) -> Result<Json<Value>, ApiError> {
    settings::set(
        &st.db,
        keys::EMAIL_ENABLED,
        if s.enabled { "true" } else { "false" },
    )
    .await?;
    settings::set(&st.db, keys::SMTP_HOST, &s.host).await?;
    settings::set(&st.db, keys::SMTP_PORT, &s.port.to_string()).await?;
    settings::set(&st.db, keys::SMTP_USER, &s.user).await?;
    settings::set(&st.db, keys::SMTP_FROM, &s.from).await?;
    if let Some(pw) = s.password.filter(|p| !p.is_empty()) {
        settings::set(&st.db, keys::SMTP_PASS, &pw).await?;
    }
    Ok(Json(json!({ "ok": true })))
}

// ---- Sending ---------------------------------------------------------------

#[derive(Deserialize)]
pub struct EmailReq {
    token: String,
    to: String,
}

pub async fn share_email(
    State(st): State<AppState>,
    Json(req): Json<EmailReq>,
) -> Result<Json<Value>, ApiError> {
    if !req.to.contains('@') || req.to.len() > 254 {
        return Err(ApiError::bad_request("ungültige E-Mail-Adresse"));
    }
    Photo::by_token(&st.db, &req.token).await?; // ensure the photo exists

    if !email_configured(&st).await? {
        return Err(ApiError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "E-Mail ist nicht konfiguriert — bitte SMTP im Admin einrichten.",
        ));
    }

    match send_photo_email(&st, &req.token, &req.to).await {
        Ok(()) => Ok(Json(json!({ "ok": true, "queued": false }))),
        Err(e) => {
            // Keep it for retry (offline-friendly).
            let _ = email_queue::enqueue(&st.db, &req.token, &req.to, &e).await;
            Err(ApiError::new(
                StatusCode::BAD_GATEWAY,
                format!("Versand fehlgeschlagen (in Warteschlange): {e}"),
            ))
        }
    }
}

async fn email_configured(st: &AppState) -> Result<bool, ApiError> {
    let enabled = settings::get_or(&st.db, keys::EMAIL_ENABLED, "false").await? == "true";
    let host = settings::get_or(&st.db, keys::SMTP_HOST, "").await?;
    let from = settings::get_or(&st.db, keys::SMTP_FROM, "").await?;
    Ok(enabled && !host.is_empty() && !from.is_empty())
}

async fn send_photo_email(st: &AppState, token: &str, to: &str) -> Result<(), String> {
    use lettre::message::header::ContentType;
    use lettre::message::{Attachment, MultiPart, SinglePart};
    use lettre::transport::smtp::authentication::Credentials;
    use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};

    let photo = Photo::by_token(&st.db, token)
        .await
        .map_err(|_| "Foto nicht gefunden".to_string())?;
    let host = settings::get_or(&st.db, keys::SMTP_HOST, "")
        .await
        .map_err(|e| e.to_string())?;
    let port: u16 = settings::get_or(&st.db, keys::SMTP_PORT, "587")
        .await
        .map_err(|e| e.to_string())?
        .parse()
        .unwrap_or(587);
    let user = settings::get_or(&st.db, keys::SMTP_USER, "")
        .await
        .map_err(|e| e.to_string())?;
    let pass = settings::get_or(&st.db, keys::SMTP_PASS, "")
        .await
        .map_err(|e| e.to_string())?;
    let from = settings::get_or(&st.db, keys::SMTP_FROM, "")
        .await
        .map_err(|e| e.to_string())?;
    let event = settings::get_or(&st.db, keys::EVENT_NAME, "SnapStation")
        .await
        .map_err(|e| e.to_string())?;

    let bytes = tokio::fs::read(st.config.photos_dir.join(&photo.original_path))
        .await
        .map_err(|e| format!("Datei: {e}"))?;

    let attachment = Attachment::new(format!("{token}.jpg"))
        .body(bytes, ContentType::parse("image/jpeg").unwrap());
    let email = Message::builder()
        .from(from.parse().map_err(|e| format!("Absender: {e}"))?)
        .to(to.parse().map_err(|e| format!("Empfänger: {e}"))?)
        .subject(format!("Dein Foto – {event}"))
        .multipart(
            MultiPart::mixed()
                .singlepart(SinglePart::plain(
                    "Dein Foto im Anhang. Viel Spaß!".to_string(),
                ))
                .singlepart(attachment),
        )
        .map_err(|e| e.to_string())?;

    let mut builder = AsyncSmtpTransport::<Tokio1Executor>::relay(&host)
        .map_err(|e| e.to_string())?
        .port(port);
    if !user.is_empty() {
        builder = builder.credentials(Credentials::new(user, pass));
    }
    let mailer = builder.build();
    mailer.send(email).await.map_err(|e| e.to_string())?;
    Ok(())
}

/// Background retry of queued e-mails. Spawned once at startup.
pub async fn process_queue(st: AppState) {
    use std::time::Duration;
    loop {
        tokio::time::sleep(Duration::from_secs(60)).await;
        if !email_configured(&st).await.unwrap_or(false) {
            continue;
        }
        let pending = email_queue::pending(&st.db, 10).await.unwrap_or_default();
        for (id, token, to) in pending {
            match send_photo_email(&st, &token, &to).await {
                Ok(()) => {
                    let _ = email_queue::mark_sent(&st.db, id).await;
                    tracing::info!("queued e-mail {id} sent");
                }
                Err(e) => {
                    let _ = email_queue::mark_attempt(&st.db, id, &e).await;
                }
            }
        }
    }
}
