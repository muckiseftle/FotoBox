//! Branding: a theme (primary colour, logo, event title/subtitle) editable live
//! from the admin, applied client-side via a CSS variable.

use crate::auth::AdminAuth;
use crate::error::ApiError;
use crate::state::AppState;
use axum::body::Bytes;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Response;
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use snap_core::settings::{self, keys};

#[derive(Serialize, Deserialize)]
pub struct Theme {
    #[serde(default = "default_primary")]
    pub primary: String,
    #[serde(default)]
    pub logo_url: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub subtitle: String,
}

fn default_primary() -> String {
    "#0ea5e9".to_string()
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            primary: default_primary(),
            logo_url: String::new(),
            title: String::new(),
            subtitle: String::new(),
        }
    }
}

async fn load_theme(st: &AppState) -> Result<Theme, ApiError> {
    Ok(match settings::get(&st.db, keys::THEME).await? {
        Some(s) => serde_json::from_str(&s).unwrap_or_default(),
        None => Theme::default(),
    })
}

pub async fn get_theme(State(st): State<AppState>) -> Result<Json<Theme>, ApiError> {
    Ok(Json(load_theme(&st).await?))
}

pub async fn put_theme(
    _admin: AdminAuth,
    State(st): State<AppState>,
    Json(theme): Json<Theme>,
) -> Result<Json<Value>, ApiError> {
    let serialized = serde_json::to_string(&theme)
        .map_err(|e| ApiError::new(StatusCode::BAD_REQUEST, e.to_string()))?;
    settings::set(&st.db, keys::THEME, &serialized).await?;
    Ok(Json(json!({ "ok": true })))
}

pub async fn upload_logo(
    _admin: AdminAuth,
    State(st): State<AppState>,
    body: Bytes,
) -> Result<Json<Value>, ApiError> {
    let raw = body.to_vec();
    let png =
        tokio::task::spawn_blocking(move || -> Result<Vec<u8>, snap_imaging::ImagingError> {
            let img = snap_imaging::decode(&raw)?;
            let mut buf = std::io::Cursor::new(Vec::new());
            img.write_to(&mut buf, image::ImageFormat::Png)
                .map_err(|e| snap_imaging::ImagingError::Encode(e.to_string()))?;
            Ok(buf.into_inner())
        })
        .await
        .map_err(|e| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .map_err(|e| ApiError::bad_request(format!("kein gültiges Bild: {e}")))?;

    let dir = st.config.data_dir.join("branding");
    tokio::fs::create_dir_all(&dir).await?;
    tokio::fs::write(dir.join("logo.png"), &png).await?;

    let mut theme = load_theme(&st).await?;
    theme.logo_url = "/branding/logo".to_string();
    settings::set(
        &st.db,
        keys::THEME,
        &serde_json::to_string(&theme).unwrap_or_default(),
    )
    .await?;
    Ok(Json(json!({ "ok": true, "logo_url": theme.logo_url })))
}

pub async fn logo_file(State(st): State<AppState>) -> Result<Response, ApiError> {
    crate::api::serve_image(st.config.data_dir.join("branding/logo.png")).await
}
