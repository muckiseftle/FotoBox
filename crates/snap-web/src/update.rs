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

/// Running version, with an optional `SNAP_FAKE_VERSION` override so the update
/// path can be exercised locally (pretend to be an older build).
fn current_version() -> String {
    std::env::var("SNAP_FAKE_VERSION").unwrap_or_else(|_| snap_core::VERSION.to_string())
}

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

/// Asset disambiguator: the release contains several files whose names include
/// the target string. Restrict the self-updater to the right one.
///
/// Windows ships a *raw* `…-windows-x86_64.exe` for the updater (the
/// `Compress-Archive` zip is not readable by `self_update`'s unzip), so the
/// downloaded file is self-replaced directly without extraction.
#[cfg(target_os = "windows")]
const UPD_IDENTIFIER: &str = "-windows-x86_64.exe";
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
const UPD_IDENTIFIER: &str = ".tar.gz";
#[cfg(not(any(
    target_os = "windows",
    all(target_os = "linux", target_arch = "x86_64")
)))]
const UPD_IDENTIFIER: &str = "";

pub async fn check(_admin: AdminAuth) -> Result<Json<Value>, ApiError> {
    let current = current_version();
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
        .identifier(UPD_IDENTIFIER)
        .show_download_progress(false)
        .no_confirm(true)
        .current_version(&current_version())
        .build()
        .map_err(|e| {
            tracing::error!("update: configure failed: {e}");
            format!("Update-Konfiguration fehlgeschlagen: {e}")
        })?
        .update()
        .map_err(|e| {
            tracing::error!("update: apply failed: {e}");
            format!("Update fehlgeschlagen: {e}")
        })?;
    Ok(status.version().to_string())
}
