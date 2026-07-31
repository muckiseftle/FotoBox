//! Collage / multi-shot: settings plus an endpoint that composes several
//! already-captured photos into one collage image (and removes the sources).

use crate::auth::AdminAuth;
use crate::error::ApiError;
use crate::state::AppState;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use chrono::Local;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use snap_core::ids;
use snap_core::models::{NewPhoto, Photo};
use snap_core::settings::{self, keys};

#[derive(Serialize, Deserialize)]
pub struct CollageSettings {
    pub enabled: bool,
    /// `grid2x2` | `strip1x3` | `strip1x4` | `duo1x2` | `grid2x3`
    pub layout: String,
    /// Countdown seconds before each shot.
    pub countdown_seconds: i64,
    /// Seconds each shot is shown before the next countdown.
    pub review_seconds: i64,
}

/// Map a layout id to (cols, rows, shot count).
pub fn layout_dims(layout: &str) -> (u32, u32, usize) {
    match layout {
        "strip1x3" => (1, 3, 3),
        "strip1x4" => (1, 4, 4),
        "duo1x2" => (1, 2, 2),
        "grid2x3" => (2, 3, 6),
        _ => (2, 2, 4), // grid2x2 default
    }
}

fn flag(v: bool) -> &'static str {
    if v {
        "true"
    } else {
        "false"
    }
}

pub async fn get_collage(State(st): State<AppState>) -> Result<Json<CollageSettings>, ApiError> {
    Ok(Json(CollageSettings {
        enabled: settings::get_or(&st.db, keys::COLLAGE_ENABLED, "false").await? == "true",
        layout: settings::get_or(&st.db, keys::COLLAGE_LAYOUT, "grid2x2").await?,
        countdown_seconds: settings::get_or(&st.db, keys::COLLAGE_COUNTDOWN, "3")
            .await?
            .parse()
            .unwrap_or(3),
        review_seconds: settings::get_or(&st.db, keys::COLLAGE_REVIEW, "2")
            .await?
            .parse()
            .unwrap_or(2),
    }))
}

pub async fn set_collage(
    _admin: AdminAuth,
    State(st): State<AppState>,
    Json(s): Json<CollageSettings>,
) -> Result<Json<Value>, ApiError> {
    settings::set(&st.db, keys::COLLAGE_ENABLED, flag(s.enabled)).await?;
    let layout = match s.layout.as_str() {
        "strip1x3" | "strip1x4" | "duo1x2" | "grid2x3" | "grid2x2" => s.layout.as_str(),
        _ => "grid2x2",
    };
    settings::set(&st.db, keys::COLLAGE_LAYOUT, layout).await?;
    settings::set(
        &st.db,
        keys::COLLAGE_COUNTDOWN,
        &s.countdown_seconds.clamp(0, 10).to_string(),
    )
    .await?;
    settings::set(
        &st.db,
        keys::COLLAGE_REVIEW,
        &s.review_seconds.clamp(0, 10).to_string(),
    )
    .await?;
    Ok(Json(json!({ "ok": true })))
}

#[derive(Deserialize)]
pub struct ComposeReq {
    /// Share tokens of the already-captured shots, in order.
    tokens: Vec<String>,
    layout: String,
}

/// Compose the given shots into a single collage photo, delete the sources,
/// and return the new photo. The shots are produced by the normal capture
/// endpoint so chroma-key/auto-print of the individual frames already applied.
pub async fn compose(
    State(st): State<AppState>,
    Json(req): Json<ComposeReq>,
) -> Result<Json<Photo>, ApiError> {
    let (cols, rows, _count) = layout_dims(&req.layout);
    if req.tokens.is_empty() || req.tokens.len() > 12 {
        return Err(ApiError::bad_request("ungültige Anzahl Bilder"));
    }

    // Load the source images from disk.
    let mut sources = Vec::new();
    for tok in &req.tokens {
        let photo = Photo::by_token(&st.db, tok).await?;
        let bytes = tokio::fs::read(st.config.photos_dir.join(&photo.original_path))
            .await
            .map_err(|_| ApiError::new(StatusCode::NOT_FOUND, "Quellbild fehlt"))?;
        sources.push(bytes);
    }

    // Compose off the async runtime (CPU-bound).
    let jpeg =
        tokio::task::spawn_blocking(move || -> Result<Vec<u8>, snap_imaging::ImagingError> {
            let imgs = sources
                .iter()
                .map(|b| snap_imaging::decode(b))
                .collect::<Result<Vec<_>, _>>()?;
            let collage = snap_imaging::compose_collage(&imgs, cols, rows);
            snap_imaging::encode_jpeg(&collage, 90)
        })
        .await
        .map_err(|e| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))??;

    // Save the collage as a new photo.
    let rel_dir = Local::now().format("%Y/%m").to_string();
    let dir = st.config.photos_dir.join(&rel_dir);
    tokio::fs::create_dir_all(&dir).await?;
    let fid = ids::new_ulid();
    let orig_rel = format!("{rel_dir}/{fid}.jpg");
    let thumb_rel = format!("{rel_dir}/{fid}_thumb.jpg");
    tokio::fs::write(st.config.photos_dir.join(&orig_rel), &jpeg).await?;

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
            width: None,
            height: None,
            kind: "collage".to_string(),
        },
    )
    .await?;

    // Remove the individual source shots so only the collage remains.
    for tok in &req.tokens {
        if let Ok(src) = Photo::delete_by_token(&st.db, tok).await {
            let _ = tokio::fs::remove_file(st.config.photos_dir.join(&src.original_path)).await;
            if let Some(t) = src.thumb_path {
                let _ = tokio::fs::remove_file(st.config.photos_dir.join(&t)).await;
            }
        }
    }

    crate::print::maybe_autoprint(&st, &photo).await;
    Ok(Json(photo))
}
