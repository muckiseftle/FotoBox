//! Cross-platform webcam backend via nokhwa.
//!
//! Windows → Media Foundation, macOS → AVFoundation, Linux → v4l2. This gives a
//! real camera (capture + live view) on every desktop OS, which is what makes
//! SnapStation a fully working photo booth on Windows and macOS, not just Linux.
//!
//! macOS note: camera access requires the TCC permission (and the .app bundle's
//! `NSCameraUsageDescription`); the user grants it on first run.

use crate::{CameraBackend, CameraError, CameraInfo, CaptureResult};
use image::{ImageEncoder, RgbImage};
use nokhwa::pixel_format::RgbFormat;
use nokhwa::utils::{CameraIndex, RequestedFormat, RequestedFormatType};
use nokhwa::Camera;

pub struct WebcamCamera {
    cam: Camera,
    model: String,
}

impl WebcamCamera {
    /// Open the default webcam (index 0).
    pub fn open() -> Result<Self, CameraError> {
        Self::open_index(0)
    }

    /// Open a specific webcam by index at its highest resolution.
    pub fn open_index(index: u32) -> Result<Self, CameraError> {
        #[cfg(target_os = "macos")]
        {
            // Trigger the AVFoundation/TCC permission flow once.
            nokhwa::nokhwa_initialize(|_granted| {});
            std::thread::sleep(std::time::Duration::from_millis(300));
        }

        let format =
            RequestedFormat::new::<RgbFormat>(RequestedFormatType::AbsoluteHighestResolution);
        let mut cam = Camera::new(CameraIndex::Index(index), format)
            .map_err(|e| CameraError::Backend(format!("Webcam öffnen: {e}")))?;
        cam.open_stream()
            .map_err(|e| CameraError::Backend(format!("Webcam-Stream: {e}")))?;
        let model = cam.info().human_name();
        Ok(Self { cam, model })
    }
}

/// List available webcams as `(index, name)`.
pub fn list_cameras() -> Vec<(u32, String)> {
    match nokhwa::query(nokhwa::utils::ApiBackend::Auto) {
        Ok(list) => list
            .iter()
            .enumerate()
            .map(|(i, info)| {
                let idx = match info.index() {
                    CameraIndex::Index(n) => *n,
                    CameraIndex::String(_) => i as u32,
                };
                (idx, info.human_name())
            })
            .collect(),
        Err(_) => Vec::new(),
    }
}

impl CameraBackend for WebcamCamera {
    fn info(&self) -> CameraInfo {
        CameraInfo {
            model: if self.model.is_empty() {
                "Webcam".to_string()
            } else {
                self.model.clone()
            },
            port: "webcam".to_string(),
            backend: "webcam",
        }
    }

    fn capture(&mut self) -> Result<CaptureResult, CameraError> {
        let img = grab(&mut self.cam)?;
        let (width, height) = (img.width(), img.height());
        Ok(CaptureResult {
            jpeg: encode(&img, 90)?,
            width,
            height,
        })
    }

    fn preview_frame(&mut self) -> Result<Vec<u8>, CameraError> {
        let img = grab(&mut self.cam)?;
        // Downscale wide frames to keep the live stream light.
        let img = if img.width() > 960 {
            let h = img.height() * 960 / img.width().max(1);
            image::imageops::resize(&img, 960, h.max(1), image::imageops::FilterType::Triangle)
        } else {
            img
        };
        encode(&img, 70)
    }
}

fn grab(cam: &mut Camera) -> Result<RgbImage, CameraError> {
    let frame = cam
        .frame()
        .map_err(|e| CameraError::Backend(format!("Frame: {e}")))?;
    frame
        .decode_image::<RgbFormat>()
        .map_err(|e| CameraError::Backend(format!("Decode: {e}")))
}

fn encode(img: &RgbImage, quality: u8) -> Result<Vec<u8>, CameraError> {
    let mut buf = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, quality)
        .write_image(
            img.as_raw(),
            img.width(),
            img.height(),
            image::ExtendedColorType::Rgb8,
        )
        .map_err(|e| CameraError::Backend(format!("JPEG: {e}")))?;
    Ok(buf)
}
