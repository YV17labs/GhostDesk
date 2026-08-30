//! Pixel work on a captured frame: decode, compare, re-encode.
//!
//! Separate from the [`ScreenBackend`](crate::screen::ScreenBackend) seam
//! because none of it touches an OS. A backend hands back PNG bytes and stops;
//! everything below is the same arithmetic on every target, and keeping it out
//! of the contract file is what lets that file be read as a contract.

use anyhow::Result;
use image::{ImageFormat as ImageIoFormat, RgbImage};

/// Max ratio of changed pixels below which two consecutive captures count as
/// "stable enough" — small enough to ignore a blinking caret or a clock tick,
/// large enough to catch a popup or a filled row of cells.
pub const STABILITY_MAX_DIFF_RATIO: f64 = 0.005;

/// The default WebP quality, chosen for compact payloads.
pub const DEFAULT_WEBP_QUALITY: u8 = 50;

/// Capture scale used while polling for post-action feedback. Smaller is a
/// cheaper capture *and* a natural downsample that filters out
/// single-character changes.
pub const FEEDBACK_SCALE: f32 = 0.25;

/// Decode PNG bytes to RGB.
///
/// RGB, not RGBA, on purpose: alpha is a constant 255 on captures, so a
/// fourth channel would be a third more bytes to compare for no information.
pub fn decode_rgb(bytes: &[u8]) -> Result<RgbImage> {
    let decoded = image::load_from_memory_with_format(bytes, ImageIoFormat::Png)?;
    Ok(decoded.to_rgb8())
}

/// Ratio of pixels that differ between two frames, or `None` on a size
/// mismatch.
///
/// Changed *pixels*, not the box around them: the bounding rectangle of two
/// disjoint changes spans everything between them, and a desktop is never
/// quiet in only one place. Measured on both OSes: a clock digit plus an app
/// spinner — 0.04 % of the pixels — box out to half the screen, so under the
/// old metric every action on a lived-in desktop read as "landed".
/// True when more than `ratio` of the pixels differ. A size mismatch counts as
/// a difference.
fn differs_by_more_than(a: &RgbImage, b: &RgbImage, ratio: f64) -> bool {
    if a.dimensions() != b.dimensions() {
        return true;
    }

    let (width, height) = a.dimensions();

    // A threshold, not a census: the answer is settled the moment enough
    // pixels have changed, and every tick of both polling loops that finds a
    // moving screen is such a case. On 1920x1080 the limit is ~10k pixels —
    // six rows — so a screen mid-transition answers in a fraction of the walk
    // a full count would pay.
    let limit = (ratio * f64::from(width) * f64::from(height)).ceil() as u64;
    if limit == 0 {
        return false;
    }

    // Row-at-a-time over the raw buffers. Comparing whole rows first is the
    // point: an unchanged row is one vectorised memcmp instead of `width`
    // bounds-checked `get_pixel` pairs, and on a desktop capture almost every
    // row *is* unchanged. This runs ~15x per input action and once per
    // stabilisation tick, so it is the hottest loop in the server.
    const CHANNELS: usize = 3;
    let stride = width as usize * CHANNELS;
    let (raw_a, raw_b) = (a.as_raw(), b.as_raw());

    let mut changed: u64 = 0;
    for y in 0..height {
        let start = y as usize * stride;
        let row_a = &raw_a[start..start + stride];
        let row_b = &raw_b[start..start + stride];
        if row_a == row_b {
            continue;
        }

        // `as_chunks` over `chunks_exact`: the stride is a whole number of
        // pixels, so the remainder is always empty, and a fixed-size array
        // compares without the length check a slice pair carries.
        let (pixels_a, _) = row_a.as_chunks::<CHANNELS>();
        let (pixels_b, _) = row_b.as_chunks::<CHANNELS>();
        changed += pixels_a
            .iter()
            .zip(pixels_b)
            .filter(|(pa, pb)| pa != pb)
            .count() as u64;
        if changed >= limit {
            return true;
        }
    }

    false
}

/// True when two decoded frames differ by more than the stability threshold.
/// A size mismatch counts as a difference.
///
/// Every caller holds decoded frames: the feedback loop keeps one fixed
/// baseline, and the stabilisation loop carries the previous frame forward.
/// So a frame is decoded exactly once no matter how many comparisons it takes
/// part in.
pub fn differ_rgb(before: &RgbImage, after: &RgbImage) -> bool {
    differs_by_more_than(before, after, STABILITY_MAX_DIFF_RATIO)
}

/// Encode a decoded frame as lossy WebP.
///
/// `quality` is 1-100. PNG needs no counterpart on the outbound path: a
/// backend already hands us PNG, and re-encoding a lossless format to itself
/// is pure waste.
pub fn encode_webp(image: &RgbImage, quality: u8) -> Vec<u8> {
    let (width, height) = image.dimensions();
    webp::Encoder::from_rgb(image.as_raw(), width, height)
        .encode(f32::from(quality))
        .to_vec()
}

#[cfg(test)]
mod tests {
    use image::Rgb;

    use super::*;

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
    fn two_distant_specks_stay_under_the_threshold() {
        // Two far-apart pixels are 0.02% of the frame — a clock digit and a
        // spinner, not an action landing. Their union bounding box is 41x41,
        // 16.8% of the frame: the old box-area metric read exactly this
        // pattern as a change, which is the false positive this test pins.
        let before = solid(100, 100, [0, 0, 0]);
        let mut after = before.clone();
        after.put_pixel(30, 30, Rgb([255, 255, 255]));
        after.put_pixel(70, 70, Rgb([255, 255, 255]));
        assert!(!differ_rgb(&before, &after));
    }

    #[test]
    fn disjoint_changes_that_are_jointly_large_still_register() {
        // Two 20x20 blocks in opposite corners: 8% of the pixels, over the
        // threshold on pixel count alone — disjointness must not hide a
        // genuinely large change.
        let before = solid(100, 100, [0, 0, 0]);
        let mut after = before.clone();
        for y in 0..20 {
            for x in 0..20 {
                after.put_pixel(x, y, Rgb([255, 255, 255]));
            }
        }
        for y in 80..100 {
            for x in 80..100 {
                after.put_pixel(x, y, Rgb([255, 255, 255]));
            }
        }
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
        // 1/10000 of the pixels — a blinking caret, not a real change.
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
}
