//! Comfort features: language, slideshow/screensaver settings, and backup
//! (export of all photos to a folder / USB drive).

use crate::auth::AdminAuth;
use crate::error::ApiError;
use crate::state::AppState;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use snap_core::models::Photo;
use snap_core::settings::{self, keys};

#[derive(Serialize, Deserialize)]
pub struct Comfort {
    pub language: String,
    pub slideshow_enabled: bool,
    /// Idle seconds before the slideshow starts (0 = disabled).
    pub slideshow_idle: i64,
    pub slideshow_interval: i64,
}

pub async fn get_comfort(State(st): State<AppState>) -> Result<Json<Comfort>, ApiError> {
    Ok(Json(Comfort {
        language: settings::get_or(&st.db, keys::LANGUAGE, "de").await?,
        slideshow_enabled: settings::get_or(&st.db, keys::SLIDESHOW_ENABLED, "true").await?
            == "true",
        slideshow_idle: settings::get_or(&st.db, keys::SLIDESHOW_IDLE, "120")
            .await?
            .parse()
            .unwrap_or(120),
        slideshow_interval: settings::get_or(&st.db, keys::SLIDESHOW_INTERVAL, "5")
            .await?
            .parse()
            .unwrap_or(5),
    }))
}

pub async fn set_comfort(
    _admin: AdminAuth,
    State(st): State<AppState>,
    Json(c): Json<Comfort>,
) -> Result<Json<Value>, ApiError> {
    let lang = if c.language == "en" { "en" } else { "de" };
    settings::set(&st.db, keys::LANGUAGE, lang).await?;
    settings::set(
        &st.db,
        keys::SLIDESHOW_ENABLED,
        if c.slideshow_enabled { "true" } else { "false" },
    )
    .await?;
    settings::set(
        &st.db,
        keys::SLIDESHOW_IDLE,
        &c.slideshow_idle.max(0).to_string(),
    )
    .await?;
    settings::set(
        &st.db,
        keys::SLIDESHOW_INTERVAL,
        &c.slideshow_interval.clamp(1, 60).to_string(),
    )
    .await?;
    Ok(Json(json!({ "ok": true })))
}

/// Candidate backup destinations (removable drives / mount points).
pub async fn backup_targets(_admin: AdminAuth) -> Json<Value> {
    Json(json!({ "targets": detect_targets() }))
}

#[cfg(target_os = "windows")]
fn detect_targets() -> Vec<String> {
    // Drive roots that currently exist (e.g. "D:\\", "E:\\").
    ('A'..='Z')
        .map(|c| format!("{c}:\\"))
        .filter(|p| std::path::Path::new(p).exists())
        .collect()
}

#[cfg(not(target_os = "windows"))]
fn detect_targets() -> Vec<String> {
    // Typical removable-media mount points on Linux.
    let mut out = Vec::new();
    for base in ["/media", "/mnt", "/run/media"] {
        if let Ok(entries) = std::fs::read_dir(base) {
            for e in entries.flatten() {
                let p = e.path();
                if p.is_dir() {
                    // /run/media/<user>/<label> is one level deeper.
                    if base == "/run/media" {
                        if let Ok(sub) = std::fs::read_dir(&p) {
                            out.extend(
                                sub.flatten()
                                    .map(|s| s.path().to_string_lossy().to_string()),
                            );
                        }
                    } else {
                        out.push(p.to_string_lossy().to_string());
                    }
                }
            }
        }
    }
    out
}

#[derive(Deserialize)]
pub struct BackupReq {
    path: String,
}

/// Copy all photo originals into `<path>/snapstation-export/`.
pub async fn backup(
    _admin: AdminAuth,
    State(st): State<AppState>,
    Json(req): Json<BackupReq>,
) -> Result<Json<Value>, ApiError> {
    // Confine backups to a detected removable target so a stray/compromised
    // admin request cannot scatter files into arbitrary system directories.
    if !detect_targets().iter().any(|t| t == &req.path) {
        return Err(ApiError::bad_request(
            "Ungültiges Ziel — bitte einen erkannten Datenträger wählen",
        ));
    }
    let base = std::path::PathBuf::from(&req.path);
    if !base.is_dir() {
        return Err(ApiError::bad_request("Zielverzeichnis existiert nicht"));
    }
    let photos = Photo::list(&st.db, 1_000_000, 0).await?;
    let total = photos.len();
    let photos_dir = st.config.photos_dir.clone();
    let dest = base.join("snapstation-export");

    let (copied, dest_str) =
        tokio::task::spawn_blocking(move || -> std::io::Result<(usize, String)> {
            std::fs::create_dir_all(&dest)?;
            let mut copied = 0usize;
            for p in &photos {
                let src = photos_dir.join(&p.original_path);
                if std::fs::copy(&src, dest.join(format!("{}.jpg", p.ulid))).is_ok() {
                    copied += 1;
                }
            }
            Ok((copied, dest.to_string_lossy().to_string()))
        })
        .await
        .map_err(|e| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .map_err(|e| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(
        json!({ "copied": copied, "total": total, "path": dest_str }),
    ))
}
