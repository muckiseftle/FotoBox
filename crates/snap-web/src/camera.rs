//! Camera / capture settings.
//!
//! App-level capture options (countdown, beep, selfie mirror), live preview mode
//! and which webcam to use. `GET /api/camera` is public (the kiosk needs these);
//! changes, re-detect and switching require an admin.

use crate::auth::AdminAuth;
use crate::error::ApiError;
use crate::state::AppState;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use snap_core::settings::{self, keys};

#[derive(Serialize, Deserialize)]
pub struct CameraSettings {
    pub countdown_seconds: i64,
    pub countdown_sound: bool,
    pub mirror_preview: bool,
    #[serde(default)]
    pub camera_index: i64,
    /// `live` | `on_demand` | `off`
    #[serde(default = "default_preview_mode")]
    pub preview_mode: String,
}

fn default_preview_mode() -> String {
    "live".to_string()
}

async fn load(st: &AppState) -> Result<CameraSettings, ApiError> {
    Ok(CameraSettings {
        countdown_seconds: settings::get_or(&st.db, keys::COUNTDOWN_SECONDS, "3")
            .await?
            .parse()
            .unwrap_or(3),
        countdown_sound: settings::get_or(&st.db, keys::COUNTDOWN_SOUND, "true").await? == "true",
        mirror_preview: settings::get_or(&st.db, keys::MIRROR_PREVIEW, "false").await? == "true",
        camera_index: settings::get_or(&st.db, keys::CAMERA_INDEX, "0")
            .await?
            .parse()
            .unwrap_or(0),
        preview_mode: settings::get_or(&st.db, keys::PREVIEW_MODE, "live").await?,
    })
}

/// Public: live status, capture settings, and the list of available cameras.
pub async fn get_camera(State(st): State<AppState>) -> Result<Json<Value>, ApiError> {
    let state = st.camera.subscribe_state().borrow().clone();
    let cams = tokio::task::spawn_blocking(snap_camera::webcam::list_cameras)
        .await
        .unwrap_or_default();
    let available: Vec<Value> = cams
        .into_iter()
        .map(|(index, name)| json!({ "index": index, "name": name }))
        .collect();
    Ok(Json(json!({
        "info": st.camera.info(),
        "state": state,
        "settings": load(&st).await?,
        "available": available,
    })))
}

/// Admin: save capture settings; switches the webcam live if the index changed.
pub async fn set_camera(
    _admin: AdminAuth,
    State(st): State<AppState>,
    Json(s): Json<CameraSettings>,
) -> Result<Json<Value>, ApiError> {
    settings::set(
        &st.db,
        keys::COUNTDOWN_SECONDS,
        &s.countdown_seconds.clamp(0, 10).to_string(),
    )
    .await?;
    settings::set(
        &st.db,
        keys::COUNTDOWN_SOUND,
        if s.countdown_sound { "true" } else { "false" },
    )
    .await?;
    settings::set(
        &st.db,
        keys::MIRROR_PREVIEW,
        if s.mirror_preview { "true" } else { "false" },
    )
    .await?;
    let mode = match s.preview_mode.as_str() {
        "on_demand" | "off" => s.preview_mode.as_str(),
        _ => "live",
    };
    settings::set(&st.db, keys::PREVIEW_MODE, mode).await?;

    let prev = settings::get_or(&st.db, keys::CAMERA_INDEX, "0")
        .await?
        .parse::<i64>()
        .unwrap_or(0);
    let idx = s.camera_index.max(0);
    settings::set(&st.db, keys::CAMERA_INDEX, &idx.to_string()).await?;
    if idx != prev {
        st.camera.use_webcam(idx as u32).await.map_err(|e| {
            ApiError::new(
                StatusCode::SERVICE_UNAVAILABLE,
                format!("Kamera-Wechsel fehlgeschlagen: {e}"),
            )
        })?;
    }

    Ok(Json(json!({ "ok": true })))
}

/// Admin: re-probe the camera and return its info.
pub async fn detect_camera(
    _admin: AdminAuth,
    State(st): State<AppState>,
) -> Result<Json<Value>, ApiError> {
    let info = st
        .camera
        .detect()
        .await
        .map_err(|e| ApiError::new(StatusCode::SERVICE_UNAVAILABLE, e.to_string()))?;
    Ok(Json(json!({ "info": info })))
}
