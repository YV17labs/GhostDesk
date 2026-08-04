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

    // Row-at-a-time over the raw buffers. Comparing whole rows first is the
    // point: an unchanged row is one vectorised memcmp instead of `width`
    // bounds-checked `get_pixel` pairs, and on a desktop capture almost every
    // row *is* unchanged. This runs ~15x per input action and once per
    // stabilisation tick, so it is the hottest loop in the server.
    const CHANNELS: usize = 3;
    let stride = width as usize * CHANNELS;
    let (raw_a, raw_b) = (a.as_raw(), b.as_raw());

    for y in 0..height {
        let start = y as usize * stride;
        let row_a = &raw_a[start..start + stride];
        let row_b = &raw_b[start..start + stride];
        if row_a == row_b {
            continue;
        }

        let first = row_a
            .chunks_exact(CHANNELS)
            .zip(row_b.chunks_exact(CHANNELS))
            .position(|(pa, pb)| pa != pb);
        let last = row_a
            .chunks_exact(CHANNELS)
            .zip(row_b.chunks_exact(CHANNELS))
            .rposition(|(pa, pb)| pa != pb);

        if let (Some(first), Some(last)) = (first, last) {
            differs = true;
            min_x = min_x.min(first as u32);
            max_x = max_x.max(last as u32);
            min_y = min_y.min(y);
            max_y = max_y.max(y);
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

/// True when two decoded frames differ by more than the stability threshold.
/// A size mismatch counts as a difference.
///
/// Every caller holds decoded frames: the feedback loop keeps one fixed
/// baseline, and the stabilisation loop carries the previous frame forward.
/// So a frame is decoded exactly once no matter how many comparisons it takes
/// part in.
pub fn differ_rgb(before: &RgbImage, after: &RgbImage) -> bool {
    bbox_ratio(before, after).is_none_or(|ratio| ratio >= STABILITY_MAX_DIFF_RATIO)
}

/// Encode a decoded frame as lossy WebP.
///
/// `quality` is 1-100. PNG needs no counterpart: grim already hands us PNG,
/// and re-encoding a lossless format to itself is pure waste.
pub fn encode_webp(image: &RgbImage, quality: u8) -> Vec<u8> {
    let (width, height) = image.dimensions();
    webp::Encoder::from_rgb(image.as_raw(), width, height)
        .encode(f32::from(quality))
        .to_vec()
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
        assert!(!differ_rgb(&img, &img.clone()));
    }

    #[test]
    fn the_difference_box_spans_every_changed_row_and_column() {
        // Two far-apart pixels: the box is their bounding rectangle, not two
        // separate specks — 41x41 of 100x100 is 16.8%, well over threshold.
        let before = solid(100, 100, [0, 0, 0]);
        let mut after = before.clone();
        after.put_pixel(30, 30, Rgb([255, 255, 255]));
        after.put_pixel(70, 70, Rgb([255, 255, 255]));
        assert!(differ_rgb(&before, &after));
    }

    #[test]
    fn a_change_in_the_last_row_and_column_is_seen() {
        // Guards the row-slice indexing: an off-by-one on `stride` would miss
        // the final row entirely.
        let before = solid(20, 20, [0, 0, 0]);
        let mut after = before.clone();
        for x in 0..20 {
            after.put_pixel(x, 19, Rgb([255, 255, 255]));
        }
        assert!(differ_rgb(&before, &after));
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
    fn a_decoded_frame_encodes_to_webp() {
        let img = solid(320, 240, [120, 120, 200]);
        let webp = encode_webp(&img, DEFAULT_WEBP_QUALITY);
        assert!(webp.starts_with(b"RIFF"), "a WebP payload starts with RIFF");
        assert!(webp.len() < to_png(&img).len() * 2);
    }

    #[test]
    fn a_png_decodes_to_the_pixels_it_was_built_from() {
        let img = solid(64, 48, [7, 8, 9]);
        let decoded = decode_rgb(&to_png(&img)).unwrap();
        assert_eq!(decoded.dimensions(), (64, 48));
        assert!(!differ_rgb(&img, &decoded));
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
