//! Image pipeline for SnapStation.
//!
//! The centerpiece (Phase 4) is high-quality chroma keying; this Phase-0 module
//! provides the decode/encode and thumbnail helpers the capture path needs.
//! Optional ONNX-based background removal will live behind the `ai` feature
//! (64-bit only) and is added in Phase 4.

pub mod chroma;
pub use chroma::{analyze, composite, matte, Calibration, ChromaKey};

use image::{DynamicImage, ImageEncoder};
use std::io::Cursor;

#[derive(Debug, thiserror::Error)]
pub enum ImagingError {
    #[error("decode error: {0}")]
    Decode(String),
    #[error("encode error: {0}")]
    Encode(String),
}

pub type Result<T> = std::result::Result<T, ImagingError>;

/// Upper bound on the dimensions of a decoded image, guarding against
/// decompression bombs (a tiny file that expands to gigabytes of pixels).
/// 64 MP comfortably covers any real DSLR/webcam frame while capping a single
/// decode to roughly 256 MB of RGBA.
const MAX_IMAGE_PIXELS: u64 = 64 * 1024 * 1024;
const MAX_IMAGE_SIDE: u32 = 16_384;

/// Decode image bytes (format auto-detected) into a [`DynamicImage`].
///
/// Dimension/allocation limits are enforced so a maliciously crafted upload
/// cannot exhaust memory (see [`MAX_IMAGE_PIXELS`]).
pub fn decode(bytes: &[u8]) -> Result<DynamicImage> {
    let mut reader = image::ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| ImagingError::Decode(e.to_string()))?;

    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_IMAGE_SIDE);
    limits.max_image_height = Some(MAX_IMAGE_SIDE);
    // Cap the decoder's intermediate allocations (~256 MB for 64 MP RGBA).
    limits.max_alloc = Some(MAX_IMAGE_PIXELS * 4);
    reader.limits(limits);

    let img = reader
        .decode()
        .map_err(|e| ImagingError::Decode(e.to_string()))?;

    if u64::from(img.width()) * u64::from(img.height()) > MAX_IMAGE_PIXELS {
        return Err(ImagingError::Decode(format!(
            "Bild zu groß ({}×{} Pixel, Maximum {} MP)",
            img.width(),
            img.height(),
            MAX_IMAGE_PIXELS / (1024 * 1024)
        )));
    }
    Ok(img)
}

/// Encode an image as JPEG at the given quality (0–100).
pub fn encode_jpeg(img: &DynamicImage, quality: u8) -> Result<Vec<u8>> {
    let rgb = img.to_rgb8();
    let mut buf = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, quality)
        .write_image(
            rgb.as_raw(),
            rgb.width(),
            rgb.height(),
            image::ExtendedColorType::Rgb8,
        )
        .map_err(|e| ImagingError::Encode(e.to_string()))?;
    Ok(buf)
}

/// Produce a JPEG thumbnail that fits within `max_dim` on its longest side,
/// preserving aspect ratio. Intended to run on a blocking thread pool.
pub fn thumbnail_jpeg(original_jpeg: &[u8], max_dim: u32, quality: u8) -> Result<Vec<u8>> {
    let img = decode(original_jpeg)?;
    let thumb = img.thumbnail(max_dim, max_dim);
    encode_jpeg(&thumb, quality)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageEncoder, RgbImage};

    fn sample_jpeg(w: u32, h: u32) -> Vec<u8> {
        let img = RgbImage::from_fn(w, h, |x, y| {
            image::Rgb([(x % 256) as u8, (y % 256) as u8, 120])
        });
        let mut buf = Vec::new();
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, 90)
            .write_image(img.as_raw(), w, h, image::ExtendedColorType::Rgb8)
            .unwrap();
        buf
    }

    #[test]
    fn thumbnail_fits_within_bounds() {
        let src = sample_jpeg(1600, 1200);
        let thumb = thumbnail_jpeg(&src, 480, 80).unwrap();
        let dims = decode(&thumb).unwrap();
        assert!(dims.width() <= 480 && dims.height() <= 480);
        assert!(dims.width() > 0 && dims.height() > 0);
    }
}
