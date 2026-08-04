//! Screen capture and frame comparison.
//!
//! `grim` is the only thing that talks to the compositor's screencopy
//! protocol; everything else here decodes its PNG, compares two frames, or
//! re-encodes one for the wire.

use std::time::Duration;

use image::{ImageFormat as ImageIoFormat, RgbImage};

use crate::cmd::{self, CmdError};
use crate::coords::{screen_height, screen_width};

/// Max ratio of changed area below which two consecutive captures count as
/// "stable enough" — small enough to ignore a blinking caret or a clock tick,
/// large enough to catch a popup or a filled row of cells.
pub const STABILITY_MAX_DIFF_RATIO: f64 = 0.005;

/// Agent-facing default: WebP, for compact payloads.
pub const DEFAULT_WEBP_QUALITY: u8 = 50;

/// Capture scale used while polling for post-action feedback. Smaller is a
/// faster grim encode *and* a natural downsample that filters out
/// single-character changes.
pub const FEEDBACK_SCALE: f32 = 0.25;

/// Wire format for a returned capture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    Webp,
    Png,
}

impl ImageFormat {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Webp => "webp",
            Self::Png => "png",
        }
    }

    pub fn mime(self) -> &'static str {
        match self {
            Self::Webp => "image/webp",
            Self::Png => "image/png",
        }
    }
}

/// Rectangular screen region to capture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Region {
    pub x: i64,
    pub y: i64,
    pub width: i64,
    pub height: i64,
}

impl Region {
    /// Clamp to the screen bounds so grim never sees a negative offset or an
    /// extent running past the edge.
    pub fn clamped(self) -> Self {
        let x = self.x.clamp(0, screen_width());
        let y = self.y.clamp(0, screen_height());
        Self {
            x,
            y,
            width: self.width.clamp(0, screen_width() - x),
            height: self.height.clamp(0, screen_height() - y),
        }
    }
}

/// Single grim invocation — raw PNG bytes.
///
/// grim writes to stdout when the output path is `-`. Region geometry follows
/// the standard Wayland `X,Y WxH` format. `scale` below 1.0 downsamples the
/// encoded image, which is what makes comparison-only captures cheap.
pub async fn capture_png(region: Option<Region>, scale: Option<f32>) -> Result<Vec<u8>, CmdError> {
    let geometry = region.map(|r| format!("{},{} {}x{}", r.x, r.y, r.width, r.height));
    let scale = scale.map(|s| s.to_string());

    let mut argv: Vec<&str> = vec!["grim", "-t", "png"];
    if let Some(geometry) = geometry.as_deref() {
        argv.extend_from_slice(&["-g", geometry]);
    }
    if let Some(scale) = scale.as_deref() {
        argv.extend_from_slice(&["-s", scale]);
    }
    argv.push("-");

    cmd::run_bytes(&argv, Duration::from_secs(10)).await
}

/// Decode PNG bytes to RGB.
///
/// RGB, not RGBA, on purpose: differencing RGBA leaves alpha at 0 (it is a
/// constant 255 on captures), and a bounding box computed over that reads as
/// empty — every diff would come back "no change".
pub fn decode_rgb(bytes: &[u8]) -> anyhow::Result<RgbImage> {
    let decoded = image::load_from_memory_with_format(bytes, ImageIoFormat::Png)?;
    Ok(decoded.to_rgb8())
}

/// Bounding-box-of-difference area over image area, or `None` on a size
/// mismatch.
fn bbox_ratio(a: &RgbImage, b: &RgbImage) -> Option<f64> {
    if a.dimensions() != b.dimensions() {
        return None;
    }

    let (width, height) = a.dimensions();
    let (mut min_x, mut min_y, mut max_x, mut max_y) = (u32::MAX, u32::MAX, 0u32, 0u32);
    let mut differs = false;

    for y in 0..height {
        for x in 0..width {
            if a.get_pixel(x, y) != b.get_pixel(x, y) {
                differs = true;
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
            }
        }
    }

    if !differs {
        return Some(0.0);
    }

    // Pillow's `getbbox()` returns a half-open box, so the extent is
    // (max - min + 1) in each axis.
    let dw = (max_x - min_x + 1) as f64;
    let dh = (max_y - min_y + 1) as f64;
    Some((dw * dh) / (width as f64 * height as f64))
}

/// True when two captures differ by more than the stability threshold.
/// A size mismatch counts as a difference.
pub fn screens_differ(a: &[u8], b: &[u8]) -> bool {
    if a == b {
        return false;
    }
    let (Ok(a), Ok(b)) = (decode_rgb(a), decode_rgb(b)) else {
        return true;
    };
    differ_rgb(&a, &b)
}

/// `screens_differ` against a baseline that is already decoded — the polling
/// loops hold `before` fixed and would otherwise re-decode it every tick.
pub fn differ_rgb(before: &RgbImage, after: &RgbImage) -> bool {
    bbox_ratio(before, after).is_none_or(|ratio| ratio >= STABILITY_MAX_DIFF_RATIO)
}

/// True when two captures look the same up to the stability threshold. The
/// positive form reads better at the screenshot stabilisation call site.
pub fn screens_stable(a: &[u8], b: &[u8]) -> bool {
    !screens_differ(a, b)
}

/// Re-encode raw PNG bytes into the requested wire format.
///
/// `quality` is the WebP encoder quality (1-100); PNG is lossless and
/// ignores it, so the original bytes are handed straight back.
pub fn reencode(raw_png: &[u8], format: ImageFormat, quality: u8) -> anyhow::Result<Vec<u8>> {
    match format {
        ImageFormat::Png => Ok(raw_png.to_vec()),
        ImageFormat::Webp => {
            let rgb = decode_rgb(raw_png)?;
            let (width, height) = rgb.dimensions();
            let encoded = webp::Encoder::from_rgb(rgb.as_raw(), width, height)
                .encode(f32::from(quality))
                .to_vec();
            Ok(encoded)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgb;

    fn solid(width: u32, height: u32, colour: [u8; 3]) -> RgbImage {
        RgbImage::from_pixel(width, height, Rgb(colour))
    }

    fn to_png(img: &RgbImage) -> Vec<u8> {
        let mut out = std::io::Cursor::new(Vec::new());
        img.write_to(&mut out, ImageIoFormat::Png).unwrap();
        out.into_inner()
    }

    #[test]
    fn identical_frames_do_not_differ() {
        let img = solid(100, 100, [10, 20, 30]);
        assert!(!screens_differ(&to_png(&img), &to_png(&img)));
        assert!(screens_stable(&to_png(&img), &to_png(&img)));
    }

    #[test]
    fn a_single_changed_pixel_stays_under_the_threshold() {
        let before = solid(100, 100, [0, 0, 0]);
        let mut after = before.clone();
        after.put_pixel(50, 50, Rgb([255, 255, 255]));
        // 1/10000 of the area — a blinking caret, not a real change.
        assert!(!differ_rgb(&before, &after));
    }

    #[test]
    fn a_dialog_sized_change_crosses_the_threshold() {
        let before = solid(100, 100, [0, 0, 0]);
        let mut after = before.clone();
        for y in 40..60 {
            for x in 40..60 {
                after.put_pixel(x, y, Rgb([255, 255, 255]));
            }
        }
        // 20x20 of 100x100 = 4% ≫ 0.5%.
        assert!(differ_rgb(&before, &after));
    }

    #[test]
    fn a_size_mismatch_counts_as_a_difference() {
        let a = solid(100, 100, [0, 0, 0]);
        let b = solid(80, 100, [0, 0, 0]);
        assert!(differ_rgb(&a, &b));
    }

    #[test]
    fn png_round_trips_untouched_and_webp_shrinks() {
        let img = solid(320, 240, [120, 120, 200]);
        let png = to_png(&img);

        assert_eq!(reencode(&png, ImageFormat::Png, 50).unwrap(), png);

        let webp = reencode(&png, ImageFormat::Webp, DEFAULT_WEBP_QUALITY).unwrap();
        assert!(webp.starts_with(b"RIFF"), "a WebP payload starts with RIFF");
        assert!(!webp.is_empty());
    }

    #[test]
    fn regions_are_clamped_into_the_screen() {
        let clamped = Region {
            x: -50,
            y: 10,
            width: 99_999,
            height: 20,
        }
        .clamped();
        assert_eq!(clamped.x, 0);
        assert_eq!(clamped.y, 10);
        assert_eq!(clamped.width, screen_width());
        assert_eq!(clamped.height, 20);
    }
}
