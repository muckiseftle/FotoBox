//! A synthetic camera backend used for development on hosts without a DSLR
//! (e.g. Windows/macOS) and in tests. Preview frames animate over time so the
//! live-view stream is visibly alive; captures are higher-resolution stills.

use crate::{CameraBackend, CameraError, CameraInfo, CaptureResult};
use image::{ImageEncoder, Rgb, RgbImage};

pub struct MockCamera {
    frame: u64,
}

impl MockCamera {
    pub fn new() -> Self {
        Self { frame: 0 }
    }
}

impl Default for MockCamera {
    fn default() -> Self {
        Self::new()
    }
}

impl CameraBackend for MockCamera {
    fn info(&self) -> CameraInfo {
        CameraInfo {
            model: "SnapStation Mock Camera".to_string(),
            port: "mock://0".to_string(),
            backend: "mock",
        }
    }

    fn capture(&mut self) -> Result<CaptureResult, CameraError> {
        self.frame = self.frame.wrapping_add(1);
        let (w, h) = (1920, 1280);
        let img = render(w, h, self.frame, true);
        let jpeg = encode_jpeg(&img, 90)?;
        Ok(CaptureResult {
            jpeg,
            width: w,
            height: h,
        })
    }

    fn preview_frame(&mut self) -> Result<Vec<u8>, CameraError> {
        self.frame = self.frame.wrapping_add(1);
        let (w, h) = (640, 480);
        let img = render(w, h, self.frame, false);
        encode_jpeg(&img, 70)
    }
}

/// Draw a calm, slowly drifting studio-style gradient. There is no harsh moving
/// line; a gentle brightness pulse keeps the live MJPEG stream visibly updating
/// without being distracting.
fn render(w: u32, h: u32, frame: u64, capture: bool) -> RgbImage {
    let pulse = 0.5 + 0.5 * (frame as f32 * 0.03).sin(); // slow 0..1 drift
    let base = if capture { 36.0 } else { 22.0 };
    let mut img = RgbImage::new(w, h);
    for y in 0..h {
        let fy = y as f32 / h as f32;
        for x in 0..w {
            let fx = x as f32 / w as f32;
            let r = (base + 26.0 * fx + 8.0 * pulse).min(255.0) as u8;
            let g = (base + 70.0 * (1.0 - fy) + 10.0 * pulse).min(255.0) as u8;
            let b = (base + 96.0 * fy + 10.0 * pulse).min(255.0) as u8;
            img.put_pixel(x, y, Rgb([r, g, b]));
        }
    }
    img
}

fn encode_jpeg(img: &RgbImage, quality: u8) -> Result<Vec<u8>, CameraError> {
    let mut buf = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, quality)
        .write_image(
            img.as_raw(),
            img.width(),
            img.height(),
            image::ExtendedColorType::Rgb8,
        )
        .map_err(|e| CameraError::Backend(format!("jpeg encode: {e}")))?;
    Ok(buf)
}
