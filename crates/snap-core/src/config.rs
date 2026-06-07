//! Runtime configuration derived from the environment.
//!
//! This is *infrastructure* config (where to store data, what address to bind).
//! All *user-facing* settings (event name, language, theme, chroma-key
//! calibration, …) live in the database `settings` table and are edited through
//! the web interface — never in config files. See [`crate::settings`].

use std::net::SocketAddr;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct Config {
    /// Base directory for all mutable state.
    pub data_dir: PathBuf,
    /// SQLite database file path.
    pub db_path: PathBuf,
    /// Directory holding captured originals, thumbnails and renders.
    pub photos_dir: PathBuf,
    /// Address the HTTP server binds to.
    pub bind: SocketAddr,
}

impl Config {
    /// Build configuration from environment variables, falling back to
    /// platform-appropriate defaults.
    ///
    /// * `SNAP_DATA_DIR` – base data directory.
    /// * `SNAP_BIND`     – `ip:port` to bind (default `0.0.0.0:8080`).
    pub fn from_env() -> Self {
        let data_dir = std::env::var_os("SNAP_DATA_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(default_data_dir);

        let bind = std::env::var("SNAP_BIND")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or_else(|| SocketAddr::from(([0, 0, 0, 0], 8080)));

        Self {
            db_path: data_dir.join("snapstation.db"),
            photos_dir: data_dir.join("photos"),
            data_dir,
            bind,
        }
    }

    /// Create the data and photos directories if they do not yet exist.
    pub fn ensure_dirs(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.data_dir)?;
        std::fs::create_dir_all(&self.photos_dir)?;
        Ok(())
    }
}

fn default_data_dir() -> PathBuf {
    // A per-user, writable location so SnapStation runs WITHOUT admin rights even
    // when installed under Program Files / Applications:
    //   Windows: %APPDATA%\SnapStation
    //   macOS:   ~/Library/Application Support/SnapStation
    //   Linux:   ~/.local/share/SnapStation
    // The systemd service overrides this via SNAP_DATA_DIR=/var/lib/snapstation.
    dirs::data_dir()
        .map(|d| d.join("SnapStation"))
        .unwrap_or_else(|| PathBuf::from("data"))
}
