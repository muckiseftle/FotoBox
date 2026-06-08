//! Kiosk (guest screen) behaviour settings. `GET` is public (the kiosk needs
//! them); changes require an admin.

use crate::auth::AdminAuth;
use crate::error::ApiError;
use crate::state::AppState;
use axum::extract::State;
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use snap_core::settings::{self, keys};

#[derive(Serialize, Deserialize)]
pub struct KioskSettings {
    pub show_gallery: bool,
    pub show_print: bool,
    pub show_qr: bool,
    /// Auto-return to live view after a capture, in seconds (0 = manual).
    pub result_seconds: i64,
    /// Embed the event logo in the centre of QR codes.
    pub qr_logo: bool,
    /// Allow guests to delete a photo from the gallery.
    pub allow_delete_gallery: bool,
    /// Allow guests to delete the photo just taken (result screen).
    pub allow_delete_after_capture: bool,
    /// Auto-return from the gallery to the kiosk after N idle seconds (0 = off).
    pub gallery_idle_seconds: i64,
}

fn flag(v: bool) -> &'static str {
    if v {
        "true"
    } else {
        "false"
    }
}

pub async fn get_kiosk(State(st): State<AppState>) -> Result<Json<KioskSettings>, ApiError> {
    Ok(Json(KioskSettings {
        show_gallery: settings::get_or(&st.db, keys::KIOSK_SHOW_GALLERY, "true").await? == "true",
        show_print: settings::get_or(&st.db, keys::KIOSK_SHOW_PRINT, "true").await? == "true",
        show_qr: settings::get_or(&st.db, keys::KIOSK_SHOW_QR, "true").await? == "true",
        result_seconds: settings::get_or(&st.db, keys::KIOSK_RESULT_SECONDS, "0")
            .await?
            .parse()
            .unwrap_or(0),
        qr_logo: settings::get_or(&st.db, keys::QR_LOGO, "false").await? == "true",
        allow_delete_gallery: settings::get_or(&st.db, keys::ALLOW_DELETE_GALLERY, "false").await?
            == "true",
        allow_delete_after_capture: settings::get_or(
            &st.db,
            keys::ALLOW_DELETE_AFTER_CAPTURE,
            "false",
        )
        .await?
            == "true",
        gallery_idle_seconds: settings::get_or(&st.db, keys::GALLERY_IDLE_SECONDS, "0")
            .await?
            .parse()
            .unwrap_or(0),
    }))
}

pub async fn set_kiosk(
    _admin: AdminAuth,
    State(st): State<AppState>,
    Json(s): Json<KioskSettings>,
) -> Result<Json<Value>, ApiError> {
    settings::set(&st.db, keys::KIOSK_SHOW_GALLERY, flag(s.show_gallery)).await?;
    settings::set(&st.db, keys::KIOSK_SHOW_PRINT, flag(s.show_print)).await?;
    settings::set(&st.db, keys::KIOSK_SHOW_QR, flag(s.show_qr)).await?;
    settings::set(
        &st.db,
        keys::KIOSK_RESULT_SECONDS,
        &s.result_seconds.clamp(0, 120).to_string(),
    )
    .await?;
    settings::set(&st.db, keys::QR_LOGO, flag(s.qr_logo)).await?;
    settings::set(
        &st.db,
        keys::ALLOW_DELETE_GALLERY,
        flag(s.allow_delete_gallery),
    )
    .await?;
    settings::set(
        &st.db,
        keys::ALLOW_DELETE_AFTER_CAPTURE,
        flag(s.allow_delete_after_capture),
    )
    .await?;
    settings::set(
        &st.db,
        keys::GALLERY_IDLE_SECONDS,
        &s.gallery_idle_seconds.clamp(0, 600).to_string(),
    )
    .await?;
    Ok(Json(json!({ "ok": true })))
}
