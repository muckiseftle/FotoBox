//! High-quality chroma keying — the centerpiece of SnapStation's image pipeline.
//!
//! Approach (chosen for quality *and* "just works" calibration):
//!
//! * **Chroma-distance key.** Pixels are compared to the key colour in the
//!   CbCr chroma plane (luma ignored). This is far more lighting-robust than an
//!   RGB/HSV threshold and generalises to *any* sampled key colour, not just a
//!   fixed green — which is what makes one-tap auto-calibration work.
//! * **Soft dual-threshold matte.** A smooth ramp between `tolerance` and
//!   `tolerance + softness` yields feathered edges (hair, motion) instead of a
//!   hard 0/1 cut.
//! * **Despill.** Green/blue spill on foreground edges is suppressed so no
//!   coloured fringe remains.
//!
//! All heavy loops are parallelised with rayon.

use image::{imageops::FilterType, DynamicImage, RgbImage};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

/// A chroma-key configuration, persisted per event and editable in the wizard.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChromaKey {
    pub enabled: bool,
    /// Sampled background colour (RGB).
    pub key_color: [u8; 3],
    /// Chroma distance at/below which a pixel is fully background (transparent).
    pub tolerance: f32,
    /// Width of the ramp above `tolerance` to fully-opaque foreground.
    pub softness: f32,
    /// Spill-suppression strength, 0.0–1.0.
    pub despill: f32,
}

impl Default for ChromaKey {
    fn default() -> Self {
        Self {
            enabled: false,
            key_color: [40, 200, 70],
            tolerance: 38.0,
            softness: 26.0,
            despill: 0.7,
        }
    }
}

/// Result of analysing a background region for auto-calibration.
#[derive(Debug, Clone, Serialize)]
pub struct Calibration {
    pub key_color: [u8; 3],
    pub tolerance: f32,
    pub softness: f32,
    /// Lighting evenness across the sampled region, 0.0 (poor) – 1.0 (excellent).
    pub evenness: f32,
    /// Honest, human-readable assessment for the wizard.
    pub message: String,
}

#[inline]
fn chroma(rgb: [f32; 3]) -> (f32, f32) {
    let [r, g, b] = rgb;
    let cb = -0.168_736 * r - 0.331_264 * g + 0.5 * b;
    let cr = 0.5 * r - 0.418_688 * g - 0.081_312 * b;
    (cb, cr)
}

#[inline]
fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Foreground alpha for a single pixel: 0.0 = background, 1.0 = foreground.
#[inline]
fn alpha_for(rgb: [f32; 3], key_chroma: (f32, f32), key: &ChromaKey) -> f32 {
    let (cb, cr) = chroma(rgb);
    let d = ((cb - key_chroma.0).powi(2) + (cr - key_chroma.1).powi(2)).sqrt();
    let soft = key.softness.max(1.0);
    smoothstep((d - key.tolerance) / soft)
}

/// Suppress green/blue spill on a foreground pixel.
#[inline]
fn despill(mut rgb: [f32; 3], key_color: [u8; 3], strength: f32) -> [f32; 3] {
    if strength <= 0.0 {
        return rgb;
    }
    let [kr, kg, kb] = key_color;
    let [r, g, b] = rgb;
    if kg >= kr && kg >= kb {
        // Green screen: limit green that exceeds the brighter of red/blue.
        let limit = r.max(b);
        if g > limit {
            rgb[1] = g - (g - limit) * strength;
        }
    } else if kb >= kr && kb >= kg {
        // Blue screen.
        let limit = r.max(g);
        if b > limit {
            rgb[2] = b - (b - limit) * strength;
        }
    }
    rgb
}

/// Composite the foreground over `background` using the chroma key, returning an
/// opaque RGB image the size of the foreground. The background is scaled to
/// cover the foreground (centre-cropped).
pub fn composite(foreground: &RgbImage, background: &RgbImage, key: &ChromaKey) -> RgbImage {
    let (w, h) = foreground.dimensions();
    let bg = DynamicImage::ImageRgb8(background.clone())
        .resize_to_fill(w, h, FilterType::Triangle)
        .to_rgb8();

    let key_chroma = chroma([
        key.key_color[0] as f32,
        key.key_color[1] as f32,
        key.key_color[2] as f32,
    ]);

    let fg_raw = foreground.as_raw();
    let bg_raw = bg.as_raw();
    let mut out = vec![0u8; fg_raw.len()];

    out.par_chunks_mut(3).enumerate().for_each(|(i, o)| {
        let j = i * 3;
        let fg = [fg_raw[j] as f32, fg_raw[j + 1] as f32, fg_raw[j + 2] as f32];
        let a = alpha_for(fg, key_chroma, key);
        let fg = despill(fg, key.key_color, key.despill);
        for c in 0..3 {
            let bgc = bg_raw[j + c] as f32;
            o[c] = (fg[c] * a + bgc * (1.0 - a)).round().clamp(0.0, 255.0) as u8;
        }
    });

    RgbImage::from_raw(w, h, out).expect("buffer matches dimensions")
}

/// Produce the alpha matte as a grayscale image (white = foreground). Useful for
/// the calibration preview's "matte" view.
pub fn matte(foreground: &RgbImage, key: &ChromaKey) -> image::GrayImage {
    let (w, h) = foreground.dimensions();
    let key_chroma = chroma([
        key.key_color[0] as f32,
        key.key_color[1] as f32,
        key.key_color[2] as f32,
    ]);
    let fg_raw = foreground.as_raw();
    let mut out = vec![0u8; (w * h) as usize];
    out.par_iter_mut().enumerate().for_each(|(i, o)| {
        let j = i * 3;
        let fg = [fg_raw[j] as f32, fg_raw[j + 1] as f32, fg_raw[j + 2] as f32];
        *o = (alpha_for(fg, key_chroma, key) * 255.0) as u8;
    });
    image::GrayImage::from_raw(w, h, out).expect("buffer matches dimensions")
}

/// Analyse a normalized background region (`x,y,w,h` in 0.0–1.0) to suggest a
/// key colour, tolerances and an honest lighting-evenness score.
pub fn analyze(img: &RgbImage, x: f32, y: f32, rw: f32, rh: f32) -> Calibration {
    let (iw, ih) = img.dimensions();
    let x0 = (x.clamp(0.0, 1.0) * iw as f32) as u32;
    let y0 = (y.clamp(0.0, 1.0) * ih as f32) as u32;
    let x1 = ((x + rw).clamp(0.0, 1.0) * iw as f32) as u32;
    let y1 = ((y + rh).clamp(0.0, 1.0) * ih as f32) as u32;
    let x1 = x1.max(x0 + 1).min(iw);
    let y1 = y1.max(y0 + 1).min(ih);

    // Subsample for speed (cap ~4096 samples).
    let step_x = (((x1 - x0) / 64).max(1)) as usize;
    let step_y = (((y1 - y0) / 64).max(1)) as usize;

    let mut sum = [0f64; 3];
    let mut n = 0u64;
    let mut samples: Vec<[f32; 3]> = Vec::new();
    for py in (y0..y1).step_by(step_y) {
        for px in (x0..x1).step_by(step_x) {
            let p = img.get_pixel(px, py).0;
            let rgb = [p[0] as f32, p[1] as f32, p[2] as f32];
            sum[0] += rgb[0] as f64;
            sum[1] += rgb[1] as f64;
            sum[2] += rgb[2] as f64;
            samples.push(rgb);
            n += 1;
        }
    }
    let n = n.max(1);
    let mean = [
        (sum[0] / n as f64) as f32,
        (sum[1] / n as f64) as f32,
        (sum[2] / n as f64) as f32,
    ];
    let key_chroma = chroma(mean);

    // Spread of chroma distance and luma around the mean.
    let mut dist_sum = 0f64;
    let mut dist_sq = 0f64;
    let mut luma_sum = 0f64;
    let mut luma_sq = 0f64;
    for s in &samples {
        let (cb, cr) = chroma(*s);
        let d = ((cb - key_chroma.0).powi(2) + (cr - key_chroma.1).powi(2)).sqrt() as f64;
        dist_sum += d;
        dist_sq += d * d;
        let luma = 0.299 * s[0] as f64 + 0.587 * s[1] as f64 + 0.114 * s[2] as f64;
        luma_sum += luma;
        luma_sq += luma * luma;
    }
    let nf = samples.len().max(1) as f64;
    let dist_mean = dist_sum / nf;
    let dist_std = (dist_sq / nf - dist_mean * dist_mean).max(0.0).sqrt();
    let luma_mean = luma_sum / nf;
    let luma_std = (luma_sq / nf - luma_mean * luma_mean).max(0.0).sqrt();

    // Tolerance must cover the background's own chroma spread; softness gives a
    // feathered edge proportional to that spread.
    let tolerance = (dist_mean + 3.0 * dist_std + 12.0) as f32;
    let softness = (2.0 * dist_std as f32 + 18.0).max(14.0);

    // Evenness: dominated by luma variation across the wall. ~12 luma std is
    // good; ~45 is poor.
    let evenness = (1.0 - ((luma_std - 12.0) / 33.0)).clamp(0.0, 1.0) as f32;
    let message = if evenness > 0.8 {
        "Hintergrund gleichmäßig ausgeleuchtet — sehr gut.".to_string()
    } else if evenness > 0.55 {
        "Ausleuchtung okay. Leichte Helligkeitsunterschiede im Hintergrund.".to_string()
    } else {
        "Hintergrund ungleichmäßig ausgeleuchtet — bitte gleichmäßiger beleuchten für saubere Kanten.".to_string()
    };

    Calibration {
        key_color: [
            mean[0].round() as u8,
            mean[1].round() as u8,
            mean[2].round() as u8,
        ],
        tolerance,
        softness,
        evenness,
        message,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgb;

    /// Left half green key, right half a red subject.
    fn green_and_red(w: u32, h: u32) -> RgbImage {
        RgbImage::from_fn(w, h, |x, _| {
            if x < w / 2 {
                Rgb([35, 205, 65]) // green screen
            } else {
                Rgb([200, 40, 40]) // subject
            }
        })
    }

    #[test]
    fn composite_replaces_key_keeps_subject() {
        let fg = green_and_red(80, 40);
        let bg = RgbImage::from_pixel(80, 40, Rgb([20, 30, 220])); // blue backdrop
        let key = ChromaKey {
            enabled: true,
            key_color: [35, 205, 65],
            tolerance: 40.0,
            softness: 20.0,
            despill: 0.8,
        };
        let out = composite(&fg, &bg, &key);

        // Left (was green) should now be ~blue backdrop.
        let left = out.get_pixel(10, 20).0;
        assert!(
            left[2] > 150 && left[1] < 90,
            "left not keyed to backdrop: {left:?}"
        );

        // Right (subject) should stay reddish.
        let right = out.get_pixel(70, 20).0;
        assert!(
            right[0] > 150 && right[2] < 90,
            "subject not preserved: {right:?}"
        );
    }

    #[test]
    fn analyze_recovers_key_color_and_reports_evenness() {
        // Perfectly even green wall.
        let even = RgbImage::from_pixel(100, 100, Rgb([40, 200, 70]));
        let c = analyze(&even, 0.1, 0.1, 0.8, 0.8);
        assert!((c.key_color[1] as i32 - 200).abs() < 5);
        assert!(
            c.evenness > 0.9,
            "even wall should score high: {}",
            c.evenness
        );

        // Noisy / unevenly lit region.
        let uneven = RgbImage::from_fn(100, 100, |x, y| {
            let v = ((x * 2 + y * 3) % 160) as u8;
            Rgb([20 + v / 4, 120 + v / 2, 40 + v / 5])
        });
        let u = analyze(&uneven, 0.1, 0.1, 0.8, 0.8);
        assert!(u.evenness < c.evenness, "uneven should score lower");
    }
}
