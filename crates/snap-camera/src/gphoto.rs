//! Real DSLR / mirrorless backend via libgphoto2 (Linux only).
//!
//! IMPORTANT: this backend cannot be compiled or tested on the Windows/macOS
//! development host — libgphoto2 and the `gphoto2` crate are Linux-only here.
//! It is gated behind `#[cfg(target_os = "linux")]` and must be built and
//! verified on the Ubuntu target against real camera hardware (project plan,
//! Teil 8: "durchgängig auf der Zielhardware testen"). The exact borrow forms
//! of `FilePath::folder()/name()` may need a minor adjustment on first build.
//!
//! All calls here are blocking (`Task::wait()`); they run on the dedicated
//! camera-actor thread, never on the async runtime.

use crate::{CameraBackend, CameraError, CameraInfo, CaptureResult};
use std::io::Cursor;

pub struct GPhotoCamera {
    context: gphoto2::Context,
    camera: gphoto2::Camera,
}

impl GPhotoCamera {
    /// Autodetect the first connected camera.
    pub fn detect() -> Result<Self, CameraError> {
        let context = gphoto2::Context::new().map_err(backend_err)?;
        let camera = context
            .autodetect_camera()
            .wait()
            .map_err(|_| CameraError::NotConnected)?;
        Ok(Self { context, camera })
    }
}

impl CameraBackend for GPhotoCamera {
    fn info(&self) -> CameraInfo {
        // TODO(hardware): populate model/port from `camera.abilities()` /
        // `camera.port_info()` once verified against a real device.
        CameraInfo {
            model: "gPhoto2 Kamera".to_string(),
            port: String::new(),
            backend: "gphoto2",
        }
    }

    fn capture(&mut self) -> Result<CaptureResult, CameraError> {
        let path = self.camera.capture_image().wait().map_err(backend_err)?;
        let file = self
            .camera
            .fs()
            .download(path.folder().as_ref(), path.name().as_ref())
            .wait()
            .map_err(backend_err)?;
        let bytes = file
            .get_data(&self.context)
            .wait()
            .map_err(backend_err)?
            .to_vec();
        let (width, height) = jpeg_dimensions(&bytes).unwrap_or((0, 0));
        Ok(CaptureResult {
            jpeg: bytes,
            width,
            height,
        })
    }

    fn preview_frame(&mut self) -> Result<Vec<u8>, CameraError> {
        let file = self.camera.capture_preview().wait().map_err(backend_err)?;
        let bytes = file
            .get_data(&self.context)
            .wait()
            .map_err(backend_err)?
            .to_vec();
        Ok(bytes)
    }
}

fn backend_err<E: std::fmt::Display>(e: E) -> CameraError {
    CameraError::Backend(e.to_string())
}

/// Read JPEG dimensions from the header without fully decoding the image.
fn jpeg_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    image::ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .ok()?
        .into_dimensions()
        .ok()
}
