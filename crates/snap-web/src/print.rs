//! Printing API: printer status, submitting prints, the job queue, print
//! settings, plus auto-print and per-photo print limits.

use crate::auth::AdminAuth;
use crate::error::ApiError;
use crate::state::AppState;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use snap_core::models::{print_jobs, Photo};
use snap_core::settings::{self, keys};
use snap_print::{JobState, PrintError, PrintOptions};

#[derive(Debug, Serialize, Deserialize)]
pub struct PrintSettings {
    pub printer: String,
    pub copies: u32,
    pub media: String,
    pub auto_print: bool,
    /// Max prints per photo, 0 = unlimited.
    pub limit: i64,
}

fn print_err(e: PrintError) -> ApiError {
    ApiError::new(StatusCode::SERVICE_UNAVAILABLE, e.to_string())
}

fn job_state_str(s: JobState) -> &'static str {
    match s {
        JobState::Queued => "queued",
        JobState::Printing => "printing",
        JobState::Done => "done",
        JobState::Error => "error",
        JobState::Unknown => "unknown",
    }
}

/// Detected printers plus the current print settings.
pub async fn printers(
    _admin: AdminAuth,
    State(st): State<AppState>,
) -> Result<Json<Value>, ApiError> {
    let printers = st.printer.list_printers().await.map_err(print_err)?;
    let settings = load_settings(&st).await?;
    Ok(Json(json!({ "printers": printers, "settings": settings })))
}

#[derive(Deserialize)]
pub struct PrintReq {
    token: String,
}

/// Print a photo identified by its share token.
pub async fn print_photo(
    State(st): State<AppState>,
    Json(req): Json<PrintReq>,
) -> Result<Json<Value>, ApiError> {
    let photo = Photo::by_token(&st.db, &req.token).await?;
    let result = do_print(&st, &photo).await?;
    Ok(Json(result))
}

/// Recent print jobs, with active ones refreshed from the backend.
pub async fn jobs(_admin: AdminAuth, State(st): State<AppState>) -> Result<Json<Value>, ApiError> {
    let mut jobs = print_jobs::recent(&st.db, 50).await?;
    for job in jobs.iter_mut() {
        if job.status == "printing" || job.status == "queued" {
            if let Some(job_ref) = &job.job_ref {
                if let Ok(state) = st.printer.job_status(&job.printer, job_ref).await {
                    let new_status = job_state_str(state);
                    if new_status != job.status {
                        let _ = print_jobs::update_status(
                            &st.db,
                            job.id,
                            new_status,
                            job.message.as_deref(),
                        )
                        .await;
                        job.status = new_status.to_string();
                    }
                }
            }
        }
    }
    Ok(Json(json!({ "jobs": jobs })))
}

pub async fn get_settings(
    _admin: AdminAuth,
    State(st): State<AppState>,
) -> Result<Json<PrintSettings>, ApiError> {
    Ok(Json(load_settings(&st).await?))
}

pub async fn set_settings(
    _admin: AdminAuth,
    State(st): State<AppState>,
    Json(s): Json<PrintSettings>,
) -> Result<Json<Value>, ApiError> {
    settings::set(&st.db, keys::PRINTER, &s.printer).await?;
    settings::set(&st.db, keys::PRINT_COPIES, &s.copies.to_string()).await?;
    settings::set(&st.db, keys::PRINT_MEDIA, &s.media).await?;
    settings::set(
        &st.db,
        keys::AUTO_PRINT,
        if s.auto_print { "true" } else { "false" },
    )
    .await?;
    settings::set(&st.db, keys::PRINT_LIMIT, &s.limit.to_string()).await?;
    Ok(Json(json!({ "ok": true })))
}

// ---- Shared logic ----------------------------------------------------------

pub(crate) async fn load_settings(st: &AppState) -> Result<PrintSettings, ApiError> {
    Ok(PrintSettings {
        printer: settings::get_or(&st.db, keys::PRINTER, "").await?,
        copies: settings::get_or(&st.db, keys::PRINT_COPIES, "1")
            .await?
            .parse()
            .unwrap_or(1),
        media: settings::get_or(&st.db, keys::PRINT_MEDIA, "").await?,
        auto_print: settings::get_or(&st.db, keys::AUTO_PRINT, "false").await? == "true",
        limit: settings::get_or(&st.db, keys::PRINT_LIMIT, "0")
            .await?
            .parse()
            .unwrap_or(0),
    })
}

/// Submit a print for a photo, enforcing the limit and recording the job.
/// Honest failures (e.g. "kein Papier") are stored and returned as errors.
pub(crate) async fn do_print(st: &AppState, photo: &Photo) -> Result<Value, ApiError> {
    let settings = load_settings(st).await?;

    let available = st.printer.list_printers().await.map_err(print_err)?;
    if available.is_empty() {
        return Err(ApiError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "kein Drucker verfügbar",
        ));
    }
    let target = if !settings.printer.is_empty() {
        settings.printer.clone()
    } else {
        available
            .iter()
            .find(|p| p.is_default)
            .or_else(|| available.first())
            .map(|p| p.name.clone())
            .unwrap_or_default()
    };

    if settings.limit > 0 {
        let made = print_jobs::count_for_photo(&st.db, photo.id).await?;
        if made >= settings.limit {
            return Err(ApiError::new(
                StatusCode::TOO_MANY_REQUESTS,
                format!("Druck-Limit erreicht ({made}/{})", settings.limit),
            ));
        }
    }

    let opts = PrintOptions {
        copies: settings.copies.max(1),
        media: (!settings.media.is_empty()).then(|| settings.media.clone()),
        fit: true,
    };
    let file = st.config.photos_dir.join(&photo.original_path);

    match st.printer.submit(&target, &file, &opts).await {
        Ok(job_ref) => {
            let state = st
                .printer
                .job_status(&target, &job_ref)
                .await
                .unwrap_or(JobState::Queued);
            let status = job_state_str(state);
            let id =
                print_jobs::insert(&st.db, photo.id, &target, Some(&job_ref), status, None).await?;
            Ok(json!({ "ok": true, "job_id": id, "printer": target, "status": status }))
        }
        Err(e) => {
            let message = e.to_string();
            print_jobs::insert(&st.db, photo.id, &target, None, "error", Some(&message)).await?;
            Err(ApiError::new(StatusCode::SERVICE_UNAVAILABLE, message))
        }
    }
}

/// Print automatically after capture when enabled (best-effort).
pub(crate) async fn maybe_autoprint(st: &AppState, photo: &Photo) {
    let auto = settings::get_or(&st.db, keys::AUTO_PRINT, "false")
        .await
        .unwrap_or_default()
        == "true";
    if !auto {
        return;
    }
    match do_print(st, photo).await {
        Ok(_) => tracing::info!("auto-print queued for photo {}", photo.ulid),
        Err(e) => tracing::warn!("auto-print failed: {}", e.message),
    }
}
