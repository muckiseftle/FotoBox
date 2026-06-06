//! Camera abstraction for SnapStation.
//!
//! A single [`actor`] task owns the camera exclusively and serves commands over
//! a channel. Because only that one thread ever touches the camera, the
//! transition between live-view and capture is atomic — no "device busy" races.
//!
//! Backends implement [`CameraBackend`]. [`MockCamera`] runs on every platform
//! (so the whole web/UI stack is developable on Windows/macOS), while the real
//! gphoto2 DSLR backend is compiled only on Linux.

pub mod actor;
pub mod mock;

#[cfg(target_os = "linux")]
pub mod gphoto;

use serde::Serialize;
use std::sync::Arc;

pub use actor::{spawn, CameraHandle, PreviewGuard};
pub use mock::MockCamera;

/// Detect and spawn the best available camera backend.
///
/// On Linux it tries the real gphoto2 DSLR backend and falls back to the mock
/// if no camera is connected. On other platforms — and whenever the environment
/// variable `SNAP_CAMERA=mock` is set — it always uses the mock backend.
pub fn spawn_auto() -> CameraHandle {
    let force_mock = std::env::var("SNAP_CAMERA")
        .map(|v| v == "mock")
        .unwrap_or(false);

    #[cfg(target_os = "linux")]
    if !force_mock {
        match gphoto::GPhotoCamera::detect() {
            Ok(cam) => {
                tracing::info!("camera: using gphoto2 DSLR backend");
                return spawn(Box::new(cam));
            }
            Err(e) => tracing::warn!("camera: no DSLR detected ({e}); using mock"),
        }
    }

    let _ = force_mock;
    tracing::info!("camera: using mock backend");
    spawn(Box::new(MockCamera::new()))
}

/// Static description of a connected camera.
#[derive(Debug, Clone, Serialize)]
pub struct CameraInfo {
    pub model: String,
    pub port: String,
    /// Which backend produced this camera: `"gphoto2"` or `"mock"`.
    pub backend: &'static str,
}

/// Coarse camera lifecycle state, broadcast to the UI for honest status.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(tag = "state", rename_all = "lowercase")]
pub enum CameraState {
    Disconnected,
    Idle,
    Previewing,
    Capturing,
    Error { message: String },
}

/// A full-resolution capture: JPEG bytes plus decoded dimensions.
#[derive(Debug, Clone)]
pub struct CaptureResult {
    pub jpeg: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

/// The latest preview frame (JPEG), shared cheaply across all live-view clients.
pub type PreviewFrame = Option<Arc<Vec<u8>>>;

#[derive(Debug, thiserror::Error)]
pub enum CameraError {
    #[error("no camera connected")]
    NotConnected,
    #[error("the camera actor is no longer running")]
    ActorGone,
    #[error("camera error: {0}")]
    Backend(String),
}

/// A camera backend. Methods are synchronous and blocking — they run on the
/// dedicated actor thread, never on the async runtime.
pub trait CameraBackend: Send {
    /// Static info about the connected camera.
    fn info(&self) -> CameraInfo;

    /// Trigger the shutter and return the captured full-resolution JPEG.
    fn capture(&mut self) -> Result<CaptureResult, CameraError>;

    /// Grab a single live-view preview frame as JPEG bytes.
    fn preview_frame(&mut self) -> Result<Vec<u8>, CameraError>;
}
