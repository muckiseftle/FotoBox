//! In-app self-update from GitHub releases (admin only).
//!
//! `check` compares the running version with the latest GitHub release. `apply`
//! downloads the matching asset for this platform and replaces the running
//! binary in place (a restart then runs the new version). Works where the
//! binary is user-writable (per-user install / portable). macOS uses a .dmg,
//! which is not auto-applied — there we point the user to the download.

use crate::auth::AdminAuth;
use crate::error::ApiError;
use axum::http::StatusCode;
use axum::Json;
use serde_json::{json, Value};

const OWNER: &str = "muckiseftle";
const REPO: &str = "FotoBox";

#[cfg(target_os = "windows")]
const UPD_TARGET: &str = "windows-x86_64";
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
const UPD_TARGET: &str = "linux-x86_64";
#[cfg(target_os = "macos")]
const UPD_TARGET: &str = "macos";
#[cfg(not(any(
    target_os = "windows",
    all(target_os = "linux", target_arch = "x86_64"),
    target_os = "macos"
)))]
const UPD_TARGET: &str = "";

#[cfg(target_os = "windows")]
const BIN_NAME: &str = "SnapStation.exe";
#[cfg(not(target_os = "windows"))]
const BIN_NAME: &str = "snapstation";

pub async fn check(_admin: AdminAuth) -> Result<Json<Value>, ApiError> {
    let current = snap_core::VERSION.to_string();
    let latest = tokio::task::spawn_blocking(fetch_latest)
        .await
        .map_err(|e| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .map_err(|e| ApiError::new(StatusCode::BAD_GATEWAY, e))?;
    let update_available =
        self_update::version::bump_is_greater(&current, &latest).unwrap_or(false);
    Ok(Json(json!({
        "current": current,
        "latest": latest,
        "update_available": update_available,
        "release_url": format!("https://github.com/{OWNER}/{REPO}/releases/latest"),
        "can_self_update": !UPD_TARGET.is_empty() && cfg!(not(target_os = "macos")),
    })))
}

fn fetch_latest() -> Result<String, String> {
    let releases = self_update::backends::github::ReleaseList::configure()
        .repo_owner(OWNER)
        .repo_name(REPO)
        .build()
        .map_err(|e| e.to_string())?
        .fetch()
        .map_err(|e| e.to_string())?;
    let latest = releases
        .first()
        .ok_or_else(|| "kein Release gefunden".to_string())?;
    Ok(latest.version.clone())
}

pub async fn apply(_admin: AdminAuth) -> Result<Json<Value>, ApiError> {
    #[cfg(target_os = "macos")]
    {
        return Err(ApiError::new(
            StatusCode::NOT_IMPLEMENTED,
            "Unter macOS bitte die neue .dmg von GitHub laden.",
        ));
    }
    #[cfg(not(target_os = "macos"))]
    {
        if UPD_TARGET.is_empty() {
            return Err(ApiError::new(
                StatusCode::NOT_IMPLEMENTED,
                "Selbst-Update auf dieser Plattform nicht unterstützt.",
            ));
        }
        let version = tokio::task::spawn_blocking(do_update)
            .await
            .map_err(|e| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
            .map_err(|e| ApiError::new(StatusCode::BAD_GATEWAY, e))?;
        Ok(Json(json!({
            "ok": true,
            "updated_to": version,
            "restart_required": true,
        })))
    }
}

#[cfg(not(target_os = "macos"))]
fn do_update() -> Result<String, String> {
    let status = self_update::backends::github::Update::configure()
        .repo_owner(OWNER)
        .repo_name(REPO)
        .bin_name(BIN_NAME)
        .target(UPD_TARGET)
        .show_download_progress(false)
        .no_confirm(true)
        .current_version(snap_core::VERSION)
        .build()
        .map_err(|e| e.to_string())?
        .update()
        .map_err(|e| e.to_string())?;
    Ok(status.version().to_string())
}
