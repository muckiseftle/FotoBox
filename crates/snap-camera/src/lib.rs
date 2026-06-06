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
pub mod webcam;

#[cfg(target_os = "linux")]
pub mod gphoto;

use serde::Serialize;
use std::sync::Arc;

pub use actor::{spawn, CameraHandle, PreviewGuard};
pub use mock::MockCamera;
pub use webcam::WebcamCamera;

/// Detect and spawn the best available camera backend.
///
/// Auto-detection order: DSLR via gphoto2 (Linux) → **webcam** (all platforms) →
/// mock. Override with `SNAP_CAMERA=mock|webcam|gphoto`. The webcam path is what
/// makes SnapStation a real photo booth on Windows and macOS, not just Linux.
pub fn spawn_auto() -> CameraHandle {
    spawn(Box::new(detect_backend))
}

/// Choose a camera backend. Runs on the actor thread (so non-Send SDK handles
/// like the webcam are fine), and always returns something — falling back to the
/// mock so the app keeps running even with no camera attached.
fn detect_backend() -> Box<dyn CameraBackend> {
    let choice = std::env::var("SNAP_CAMERA").unwrap_or_default();

    match choice.as_str() {
        "mock" => {
            tracing::info!("camera: mock backend (SNAP_CAMERA=mock)");
            return Box::new(MockCamera::new());
        }
        "webcam" => return open_webcam().unwrap_or_else(mock_fallback),
        #[cfg(target_os = "linux")]
        "gphoto" | "dslr" => return open_gphoto().unwrap_or_else(mock_fallback),
        _ => {}
    }

    // Auto: prefer a tethered DSLR on Linux, then any webcam, then mock.
    #[cfg(target_os = "linux")]
    if let Some(b) = open_gphoto() {
        return b;
    }
    if let Some(b) = open_webcam() {
        return b;
    }
    mock_fallback()
}

fn mock_fallback() -> Box<dyn CameraBackend> {
    tracing::warn!("camera: no real camera found; using mock backend");
    Box::new(MockCamera::new())
}

fn open_webcam() -> Option<Box<dyn CameraBackend>> {
    match webcam::WebcamCamera::open() {
        Ok(cam) => {
            tracing::info!("camera: using webcam backend");
            Some(Box::new(cam))
        }
        Err(e) => {
            tracing::warn!("camera: webcam unavailable ({e})");
            None
        }
    }
}

#[cfg(target_os = "linux")]
fn open_gphoto() -> Option<Box<dyn CameraBackend>> {
    match gphoto::GPhotoCamera::detect() {
        Ok(cam) => {
            tracing::info!("camera: using gphoto2 DSLR backend");
            Some(Box::new(cam))
        }
        Err(e) => {
            tracing::warn!("camera: no DSLR detected ({e})");
            None
        }
    }
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
/// dedicated actor thread, never on the async runtime. The backend is built on
/// that thread and never crosses threads, so it need not be `Send`.
pub trait CameraBackend {
    /// Static info about the connected camera.
    fn info(&self) -> CameraInfo;

    /// Trigger the shutter and return the captured full-resolution JPEG.
    fn capture(&mut self) -> Result<CaptureResult, CameraError>;

    /// Grab a single live-view preview frame as JPEG bytes.
    fn preview_frame(&mut self) -> Result<Vec<u8>, CameraError>;
}
