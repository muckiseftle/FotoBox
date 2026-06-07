//! Key/value application settings, persisted in the database.
//!
//! Every user-facing configuration value goes through here so it can be edited
//! live from the web interface with no config files. Values are stored as text
//! (often JSON for structured settings).

use crate::db::Db;
use crate::Result;

/// Well-known setting keys.
pub mod keys {
    pub const SETUP_COMPLETE: &str = "setup_complete";
    pub const EVENT_NAME: &str = "event_name";
    pub const LANGUAGE: &str = "language";
    /// JSON blob: `{ "primary": "#rrggbb", "logo_url": "...", ... }`
    pub const THEME: &str = "theme";
    /// JSON blob of the current chroma-key calibration ([`snap_imaging::ChromaKey`]).
    pub const CHROMA_CALIBRATION: &str = "chroma_calibration";
    /// ULID of the currently selected background (empty = none).
    pub const CHROMA_BACKGROUND: &str = "chroma_background";

    /// Selected printer name (empty = use system default / first available).
    pub const PRINTER: &str = "printer";
    /// Copies per print.
    pub const PRINT_COPIES: &str = "print_copies";
    /// CUPS media name (empty = printer default).
    pub const PRINT_MEDIA: &str = "print_media";
    /// `"true"` to print automatically after each capture.
    pub const AUTO_PRINT: &str = "auto_print";
    /// Maximum prints per photo (0 = unlimited).
    pub const PRINT_LIMIT: &str = "print_limit";

    // SMTP / e-mail sharing.
    pub const SMTP_HOST: &str = "smtp_host";
    pub const SMTP_PORT: &str = "smtp_port";
    pub const SMTP_USER: &str = "smtp_user";
    pub const SMTP_PASS: &str = "smtp_pass";
    pub const SMTP_FROM: &str = "smtp_from";
    pub const EMAIL_ENABLED: &str = "email_enabled";

    // Slideshow / screensaver.
    pub const SLIDESHOW_ENABLED: &str = "slideshow_enabled";
    /// Idle seconds before the slideshow starts (0 = disabled).
    pub const SLIDESHOW_IDLE: &str = "slideshow_idle";
    /// Seconds each slide is shown.
    pub const SLIDESHOW_INTERVAL: &str = "slideshow_interval";

    // Camera / capture.
    /// Countdown length in seconds before the shutter (0 = no countdown).
    pub const COUNTDOWN_SECONDS: &str = "countdown_seconds";
    /// Play a beep on each countdown tick.
    pub const COUNTDOWN_SOUND: &str = "countdown_sound";
    /// Mirror the live preview (selfie view); captured photo stays un-mirrored.
    pub const MIRROR_PREVIEW: &str = "mirror_preview";
    /// Which webcam index to use when several are present.
    pub const CAMERA_INDEX: &str = "camera_index";
    /// Live-preview mode: `live` (always), `on_demand` (only after pressing the
    /// shutter), or `off` (show the kiosk background image instead).
    pub const PREVIEW_MODE: &str = "preview_mode";

    // Kiosk (guest screen) behaviour.
    pub const KIOSK_SHOW_GALLERY: &str = "kiosk_show_gallery";
    pub const KIOSK_SHOW_PRINT: &str = "kiosk_show_print";
    pub const KIOSK_SHOW_QR: &str = "kiosk_show_qr";
    /// Auto-return to the live view after a capture, in seconds (0 = manual).
    pub const KIOSK_RESULT_SECONDS: &str = "kiosk_result_seconds";
}

/// Fetch a raw setting value, if present.
pub async fn get(db: &Db, key: &str) -> Result<Option<String>> {
    let row: Option<(String,)> = sqlx::query_as("SELECT value FROM settings WHERE key = ?")
        .bind(key)
        .fetch_optional(db)
        .await?;
    Ok(row.map(|r| r.0))
}

/// Fetch a setting value or a provided default.
pub async fn get_or(db: &Db, key: &str, default: &str) -> Result<String> {
    Ok(get(db, key).await?.unwrap_or_else(|| default.to_string()))
}

/// Insert or update a setting value.
pub async fn set(db: &Db, key: &str, value: &str) -> Result<()> {
    sqlx::query(
        "INSERT INTO settings (key, value, updated_at) VALUES (?, ?, datetime('now'))
         ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = datetime('now')",
    )
    .bind(key)
    .bind(value)
    .execute(db)
    .await?;
    Ok(())
}

/// Whether the first-run setup wizard has been completed.
pub async fn is_setup_complete(db: &Db) -> Result<bool> {
    Ok(get(db, keys::SETUP_COMPLETE).await?.as_deref() == Some("true"))
}
