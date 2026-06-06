//! JSON/REST API handlers.

use crate::error::ApiError;
use crate::state::AppState;
use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHasher};
use axum::extract::{Path, Query, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::Local;
use serde::Deserialize;
use serde_json::{json, Value};
use snap_core::models::{NewPhoto, Photo};
use snap_core::{ids, settings};

/// Liveness probe.
pub async fn health() -> Json<Value> {
    Json(json!({ "status": "ok", "version": snap_core::VERSION }))
}

/// Aggregate status for the admin dashboard: camera, setup state, counts.
pub async fn status(State(st): State<AppState>) -> Json<Value> {
    let camera_state = st.camera.subscribe_state().borrow().clone();
    let photo_count = Photo::count(&st.db).await.unwrap_or(0);
    let setup_complete = settings::is_setup_complete(&st.db).await.unwrap_or(false);
    let event_name = settings::get_or(&st.db, settings::keys::EVENT_NAME, "")
        .await
        .unwrap_or_default();

    Json(json!({
        "version": snap_core::VERSION,
        "setup_complete": setup_complete,
        "event_name": event_name,
        "camera": {
            "info": st.camera.info(),
            "state": camera_state,
        },
        "photo_count": photo_count,
        // TODO(Phase 3): include detected CUPS printers + their status here.
        "printers": Value::Array(vec![]),
    }))
}

#[derive(Deserialize)]
pub struct Page {
    limit: Option<i64>,
    offset: Option<i64>,
}

/// Most recent photos first, paginated.
pub async fn list_photos(
    State(st): State<AppState>,
    Query(p): Query<Page>,
) -> Result<Json<Vec<Photo>>, ApiError> {
    let limit = p.limit.unwrap_or(50).clamp(1, 200);
    let offset = p.offset.unwrap_or(0).max(0);
    Ok(Json(Photo::list(&st.db, limit, offset).await?))
}

/// Trigger the shutter, store the original + thumbnail, index it in the DB.
/// When chroma-key is enabled and a background is selected, the stored image is
/// the composited result.
pub async fn capture(State(st): State<AppState>) -> Result<Json<Photo>, ApiError> {
    let shot = st.camera.capture().await?;

    let jpeg = render_capture(&st, shot.jpeg).await?;

    // Organize files on disk by year/month.
    let rel_dir = Local::now().format("%Y/%m").to_string();
    let dir = st.config.photos_dir.join(&rel_dir);
    tokio::fs::create_dir_all(&dir).await?;

    let fid = ids::new_ulid();
    let orig_rel = format!("{rel_dir}/{fid}.jpg");
    let thumb_rel = format!("{rel_dir}/{fid}_thumb.jpg");

    tokio::fs::write(st.config.photos_dir.join(&orig_rel), &jpeg).await?;

    // Thumbnailing is CPU-bound -> run off the async runtime.
    let thumb_src = jpeg.clone();
    let thumb =
        tokio::task::spawn_blocking(move || snap_imaging::thumbnail_jpeg(&thumb_src, 480, 80))
            .await
            .map_err(|e| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))??;
    tokio::fs::write(st.config.photos_dir.join(&thumb_rel), &thumb).await?;

    let photo = Photo::insert(
        &st.db,
        NewPhoto {
            session_id: None,
            original_path: orig_rel,
            thumb_path: Some(thumb_rel),
            width: Some(shot.width as i64),
            height: Some(shot.height as i64),
            kind: "single".to_string(),
        },
    )
    .await?;

    // Auto-print if enabled (best-effort; does not block the capture response
    // on a slow/faulty printer beyond the submit).
    crate::print::maybe_autoprint(&st, &photo).await;

    Ok(Json(photo))
}

/// Apply chroma-key compositing to a freshly captured frame when enabled.
async fn render_capture(st: &AppState, jpeg: Vec<u8>) -> Result<Vec<u8>, ApiError> {
    let key = crate::chroma::load_chroma(st).await?;
    if !key.enabled {
        return Ok(jpeg);
    }
    let Some(background) = crate::chroma::load_selected_background(st).await? else {
        return Ok(jpeg);
    };
    let out =
        tokio::task::spawn_blocking(move || -> Result<Vec<u8>, snap_imaging::ImagingError> {
            let fg = snap_imaging::decode(&jpeg)?.to_rgb8();
            let composed = snap_imaging::composite(&fg, &background, &key);
            snap_imaging::encode_jpeg(&image::DynamicImage::ImageRgb8(composed), 90)
        })
        .await
        .map_err(|e| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))??;
    Ok(out)
}

/// Serve a photo's full-resolution original by its public share token.
pub async fn photo_file(
    State(st): State<AppState>,
    Path(token): Path<String>,
) -> Result<Response, ApiError> {
    let photo = Photo::by_token(&st.db, &token).await?;
    serve_image(st.config.photos_dir.join(&photo.original_path)).await
}

/// Serve a photo's thumbnail (falls back to the original).
pub async fn thumb_file(
    State(st): State<AppState>,
    Path(token): Path<String>,
) -> Result<Response, ApiError> {
    let photo = Photo::by_token(&st.db, &token).await?;
    let rel = photo.thumb_path.unwrap_or(photo.original_path);
    serve_image(st.config.photos_dir.join(&rel)).await
}

pub(crate) async fn serve_image(path: std::path::PathBuf) -> Result<Response, ApiError> {
    let bytes = tokio::fs::read(&path)
        .await
        .map_err(|_| ApiError::new(StatusCode::NOT_FOUND, "image file missing"))?;
    let mime = mime_guess::from_path(&path).first_or_octet_stream();
    Ok((
        [
            (header::CONTENT_TYPE, mime.as_ref()),
            (header::CACHE_CONTROL, "public, max-age=31536000, immutable"),
        ],
        bytes,
    )
        .into_response())
}

// ---- First-run setup wizard ------------------------------------------------

/// Setup status: whether the wizard has been completed and current basics.
pub async fn setup_status(State(st): State<AppState>) -> Result<Json<Value>, ApiError> {
    Ok(Json(json!({
        "complete": settings::is_setup_complete(&st.db).await?,
        "event_name": settings::get_or(&st.db, settings::keys::EVENT_NAME, "").await?,
        "language": settings::get_or(&st.db, settings::keys::LANGUAGE, "de").await?,
    })))
}

#[derive(Deserialize)]
pub struct SetupForm {
    admin_password: String,
    event_name: Option<String>,
    language: Option<String>,
}

/// Complete first-run setup: set the admin password and event basics.
pub async fn setup_submit(
    State(st): State<AppState>,
    headers: HeaderMap,
    Json(form): Json<SetupForm>,
) -> Result<Json<Value>, ApiError> {
    // First-run setup is open; afterwards only an authenticated admin may change
    // the credentials (so nobody on the LAN can reset the admin password).
    if settings::is_setup_complete(&st.db).await? && !crate::auth::is_authed(&st, &headers) {
        return Err(ApiError::new(
            StatusCode::FORBIDDEN,
            "Bereits eingerichtet — bitte als Admin anmelden.",
        ));
    }
    if form.admin_password.len() < 6 {
        return Err(ApiError::bad_request(
            "Admin-Passwort muss mindestens 6 Zeichen haben.",
        ));
    }

    let hash = hash_password(&form.admin_password)
        .map_err(|e| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    snap_core::models::admin::set_password_hash(&st.db, &hash).await?;

    if let Some(name) = form.event_name {
        settings::set(&st.db, settings::keys::EVENT_NAME, &name).await?;
    }
    settings::set(
        &st.db,
        settings::keys::LANGUAGE,
        form.language.as_deref().unwrap_or("de"),
    )
    .await?;
    settings::set(&st.db, settings::keys::SETUP_COMPLETE, "true").await?;

    Ok(Json(json!({ "ok": true })))
}

fn hash_password(password: &str) -> Result<String, String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| e.to_string())
}
