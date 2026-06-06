//! Chroma-key configuration, calibration, WYSIWYG preview and background
//! management — the API behind the "just works" keying wizard.

use crate::auth::AdminAuth;
use crate::error::ApiError;
use crate::state::AppState;
use axum::body::Bytes;
use axum::extract::{Path, Query, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};
use snap_core::models::Background;
use snap_core::settings;
use snap_imaging::ChromaKey;
use std::time::Duration;

// ---- Config ----------------------------------------------------------------

pub async fn get_chroma(State(st): State<AppState>) -> Result<Json<ChromaKey>, ApiError> {
    Ok(Json(load_chroma(&st).await?))
}

pub async fn put_chroma(
    _admin: AdminAuth,
    State(st): State<AppState>,
    Json(key): Json<ChromaKey>,
) -> Result<Json<Value>, ApiError> {
    let serialized = serde_json::to_string(&key)
        .map_err(|e| ApiError::new(StatusCode::BAD_REQUEST, e.to_string()))?;
    settings::set(&st.db, settings::keys::CHROMA_CALIBRATION, &serialized).await?;
    Ok(Json(json!({ "ok": true })))
}

// ---- Auto-calibration ------------------------------------------------------

#[derive(Deserialize)]
pub struct Region {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

/// Sample a normalized background region of the current live frame and suggest
/// a key colour, tolerances and an honest lighting-evenness assessment.
pub async fn calibrate(
    _admin: AdminAuth,
    State(st): State<AppState>,
    Json(region): Json<Region>,
) -> Result<Json<snap_imaging::Calibration>, ApiError> {
    let jpeg = current_frame(&st).await?;
    let cal = tokio::task::spawn_blocking(move || -> Result<_, snap_imaging::ImagingError> {
        let img = snap_imaging::decode(&jpeg)?.to_rgb8();
        Ok(snap_imaging::analyze(
            &img, region.x, region.y, region.w, region.h,
        ))
    })
    .await
    .map_err(|e| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))??;
    Ok(Json(cal))
}

// ---- WYSIWYG preview -------------------------------------------------------

#[derive(Deserialize)]
pub struct PreviewReq {
    key: ChromaKey,
    /// `composite` (default) or `matte`.
    #[serde(default)]
    view: Option<String>,
}

/// Render the current live frame keyed with the given config, as JPEG. Used by
/// the wizard for an instant, what-you-see-is-what-you-get preview.
pub async fn preview(
    _admin: AdminAuth,
    State(st): State<AppState>,
    Json(req): Json<PreviewReq>,
) -> Result<Response, ApiError> {
    let jpeg = current_frame(&st).await?;
    let background = load_selected_background(&st).await?;
    let matte_view = req.view.as_deref() == Some("matte");

    let out =
        tokio::task::spawn_blocking(move || -> Result<Vec<u8>, snap_imaging::ImagingError> {
            let fg = snap_imaging::decode(&jpeg)?.to_rgb8();
            let rendered = if matte_view {
                image::DynamicImage::ImageLuma8(snap_imaging::matte(&fg, &req.key))
            } else {
                let bg = background.unwrap_or_else(|| default_background(fg.width(), fg.height()));
                image::DynamicImage::ImageRgb8(snap_imaging::composite(&fg, &bg, &req.key))
            };
            snap_imaging::encode_jpeg(&rendered, 75)
        })
        .await
        .map_err(|e| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))??;

    Ok(jpeg_response(out))
}

// ---- Backgrounds -----------------------------------------------------------

#[derive(Deserialize)]
pub struct UploadQuery {
    name: Option<String>,
}

/// Upload a background image (raw bytes). It is decoded for validation and
/// re-encoded to JPEG before being stored.
pub async fn upload_background(
    _admin: AdminAuth,
    State(st): State<AppState>,
    Query(q): Query<UploadQuery>,
    body: Bytes,
) -> Result<Json<Value>, ApiError> {
    let raw = body.to_vec();
    let jpeg =
        tokio::task::spawn_blocking(move || -> Result<Vec<u8>, snap_imaging::ImagingError> {
            let img = snap_imaging::decode(&raw)?;
            snap_imaging::encode_jpeg(&img, 90)
        })
        .await
        .map_err(|e| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .map_err(|e| ApiError::bad_request(format!("kein gültiges Bild: {e}")))?;

    let dir = st.config.data_dir.join("backgrounds");
    tokio::fs::create_dir_all(&dir).await?;
    let ulid = snap_core::ids::new_ulid();
    let rel = format!("backgrounds/{ulid}.jpg");
    tokio::fs::write(st.config.data_dir.join(&rel), &jpeg).await?;

    let name = q.name.unwrap_or_else(|| "Hintergrund".to_string());
    let bg = Background::insert(&st.db, &name, &rel).await?;
    Ok(Json(
        json!({ "ulid": bg.ulid, "name": bg.name, "url": format!("/bg/{}", bg.ulid) }),
    ))
}

pub async fn list_backgrounds(
    _admin: AdminAuth,
    State(st): State<AppState>,
) -> Result<Json<Value>, ApiError> {
    let backgrounds = Background::list(&st.db).await?;
    let selected = settings::get_or(&st.db, settings::keys::CHROMA_BACKGROUND, "").await?;
    let items: Vec<Value> = backgrounds
        .iter()
        .map(|b| {
            json!({
                "ulid": b.ulid,
                "name": b.name,
                "url": format!("/bg/{}", b.ulid),
                "selected": b.ulid == selected,
            })
        })
        .collect();
    Ok(Json(json!({ "backgrounds": items, "selected": selected })))
}

pub async fn background_file(
    State(st): State<AppState>,
    Path(ulid): Path<String>,
) -> Result<Response, ApiError> {
    let bg = Background::by_ulid(&st.db, &ulid).await?;
    crate::api::serve_image(st.config.data_dir.join(&bg.path)).await
}

#[derive(Deserialize)]
pub struct SelectReq {
    /// ULID to select, or empty string to clear.
    ulid: String,
}

pub async fn select_background(
    _admin: AdminAuth,
    State(st): State<AppState>,
    Json(req): Json<SelectReq>,
) -> Result<Json<Value>, ApiError> {
    if !req.ulid.is_empty() {
        Background::by_ulid(&st.db, &req.ulid).await?; // validate it exists
    }
    settings::set(&st.db, settings::keys::CHROMA_BACKGROUND, &req.ulid).await?;
    Ok(Json(json!({ "ok": true })))
}

pub async fn delete_background(
    _admin: AdminAuth,
    State(st): State<AppState>,
    Path(ulid): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let path = Background::delete(&st.db, &ulid).await?;
    let _ = tokio::fs::remove_file(st.config.data_dir.join(&path)).await;
    if settings::get_or(&st.db, settings::keys::CHROMA_BACKGROUND, "").await? == ulid {
        settings::set(&st.db, settings::keys::CHROMA_BACKGROUND, "").await?;
    }
    Ok(Json(json!({ "ok": true })))
}

// ---- Shared helpers (used by the capture path too) -------------------------

pub(crate) async fn load_chroma(st: &AppState) -> Result<ChromaKey, ApiError> {
    match settings::get(&st.db, settings::keys::CHROMA_CALIBRATION).await? {
        Some(s) => Ok(serde_json::from_str(&s).unwrap_or_default()),
        None => Ok(ChromaKey::default()),
    }
}

pub(crate) async fn load_selected_background(
    st: &AppState,
) -> Result<Option<image::RgbImage>, ApiError> {
    let selected = settings::get_or(&st.db, settings::keys::CHROMA_BACKGROUND, "").await?;
    if selected.is_empty() {
        return Ok(None);
    }
    let bg = match Background::by_ulid(&st.db, &selected).await {
        Ok(b) => b,
        Err(_) => return Ok(None),
    };
    let Ok(bytes) = tokio::fs::read(st.config.data_dir.join(&bg.path)).await else {
        return Ok(None);
    };
    let img =
        tokio::task::spawn_blocking(move || snap_imaging::decode(&bytes).map(|d| d.to_rgb8()))
            .await
            .map_err(|e| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
            .ok();
    Ok(img)
}

/// Grab the most recent live-view frame, registering a temporary viewer and
/// waiting briefly if preview is not already running.
async fn current_frame(st: &AppState) -> Result<Vec<u8>, ApiError> {
    if let Some(frame) = st.camera.subscribe_preview().borrow().clone() {
        return Ok((*frame).clone());
    }
    let _guard = st.camera.preview_guard().await;
    let mut rx = st.camera.subscribe_preview();
    tokio::time::timeout(Duration::from_secs(3), rx.changed())
        .await
        .map_err(|_| {
            ApiError::new(
                StatusCode::SERVICE_UNAVAILABLE,
                "keine Live-Vorschau verfügbar",
            )
        })?
        .map_err(|_| ApiError::new(StatusCode::SERVICE_UNAVAILABLE, "Kamera nicht verfügbar"))?;
    let frame = rx.borrow_and_update().clone();
    frame
        .map(|a| (*a).clone())
        .ok_or_else(|| ApiError::new(StatusCode::SERVICE_UNAVAILABLE, "keine Live-Vorschau"))
}

/// A neutral studio gradient shown when no background has been chosen yet.
fn default_background(w: u32, h: u32) -> image::RgbImage {
    image::RgbImage::from_fn(w, h, |x, y| {
        let t = (x + y) as f32 / (w + h).max(1) as f32;
        let v = (40.0 + 90.0 * t) as u8;
        image::Rgb([v, v, v.saturating_add(25)])
    })
}

fn jpeg_response(bytes: Vec<u8>) -> Response {
    (
        [
            (header::CONTENT_TYPE, "image/jpeg"),
            (header::CACHE_CONTROL, "no-store"),
        ],
        bytes,
    )
        .into_response()
}
