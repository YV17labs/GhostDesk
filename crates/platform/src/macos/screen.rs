//! The macOS [`ScreenBackend`] — the `screencapture` tool.
//!
//! Same shape as the Linux backend: shell out to the OS's own capture
//! utility and hand back PNG bytes. `screencapture` needs Screen Recording
//! permission, which is granted to the *binary* in System Settings and
//! cannot be requested from code — an unauthorised run comes back as a
//! failure here, not as a black image.
//!
//! Unlike `grim` it has no downsample flag, so a scaled capture is resized in
//! process. That costs a decode and a re-encode on the feedback path, which
//! polls a few times per action; a native path (ScreenCaptureKit) would
//! avoid it and is the obvious next optimisation.

use std::time::Duration;

use anyhow::{Context, Result};
use async_trait::async_trait;
use image::{ImageEncoder, ImageFormat};

use super::display;
use crate::cmd;
use crate::screen::{Region, ScreenBackend};

/// Ceiling for one capture, matching the Linux backend.
const CAPTURE_TIMEOUT: Duration = Duration::from_secs(10);

/// The [`ScreenBackend`] the host selector hands out on macOS.
pub struct ScreenCapture;

#[async_trait]
impl ScreenBackend for ScreenCapture {
    async fn capture_png(&self, region: Option<Region>, scale: Option<f32>) -> Result<Vec<u8>> {
        // `screencapture` only writes to a file. A per-call unique name keeps
        // two concurrent captures — the feedback poll and a screenshot tool —
        // from reading each other's bytes.
        let path = std::env::temp_dir().join(format!(
            "ghostdesk-capture-{}-{}.png",
            std::process::id(),
            next_capture_id(),
        ));

        let geometry = region.map(to_points);
        let target = path.to_string_lossy().into_owned();

        // `-x` silences the shutter, `-t png` fixes the format the neutral
        // pipeline decodes, `-o` drops window shadows from a region grab.
        let mut argv: Vec<&str> = vec!["screencapture", "-x", "-o", "-t", "png"];
        if let Some(geometry) = geometry.as_deref() {
            argv.extend_from_slice(&["-R", geometry]);
        }
        argv.push(&target);

        let captured = cmd::run(&argv, CAPTURE_TIMEOUT).await;
        // Read and delete whatever landed, whether or not the tool was happy:
        // leaving temp PNGs of the user's screen behind is the one failure
        // mode here that outlives the process.
        let bytes = tokio::fs::read(&path).await;
        tokio::fs::remove_file(&path).await.ok();

        captured.context(
            "screencapture failed — GhostDesk needs Screen Recording permission \
             (System Settings ▸ Privacy & Security ▸ Screen Recording)",
        )?;
        let bytes = bytes.context("screencapture reported success but wrote no file")?;

        match scale {
            // Decode, resize and re-encode are CPU-bound on a full-resolution
            // frame, and this runs a few times per second while the feedback
            // loop watches for a change. On a runtime worker it would park
            // the whole server for the duration.
            Some(scale) if scale < 1.0 => {
                tokio::task::spawn_blocking(move || downsample(&bytes, scale)).await?
            }
            _ => Ok(bytes),
        }
    }

    /// macOS owns its geometry: the panel is whatever hardware is attached,
    /// and the agent's coordinates have to match the pixels it is shown.
    fn geometry(&self) -> Option<(i64, i64)> {
        Some(display::size_in_pixels())
    }
}

/// A pixel region → the `x,y,w,h` point rectangle `-R` expects.
fn to_points(region: Region) -> String {
    let (x, y) = display::pixels_to_points(region.x, region.y);
    let (width, height) = display::pixels_to_points(region.width, region.height);
    format!("{x},{y},{width},{height}")
}

/// Shrink a captured PNG by `scale` and re-encode it.
///
/// `screencapture` has no scale flag, so the shrink that `grim -s` does at
/// capture time happens here instead.
fn downsample(png: &[u8], scale: f32) -> Result<Vec<u8>> {
    let decoded = image::load_from_memory_with_format(png, ImageFormat::Png)?;
    let width = ((decoded.width() as f32 * scale).round() as u32).max(1);
    let height = ((decoded.height() as f32 * scale).round() as u32).max(1);

    // Resize first, convert after: `to_rgb8()` on the full frame would
    // allocate and walk every pixel of an image that exists only to be
    // sampled down 16×.
    //
    // Triangle filtering, not nearest: the point of a scaled capture is to
    // *average away* one-character changes, and nearest-neighbour would keep
    // whichever pixel it happened to land on.
    let resized = decoded
        .resize_exact(width, height, image::imageops::FilterType::Triangle)
        .to_rgb8();

    // Fast deflate, not the default maximum: these bytes are never shown to
    // anyone — the caller decodes them straight back to diff two frames — so
    // spending CPU on a smaller temporary buys nothing.
    let mut out = std::io::Cursor::new(Vec::new());
    image::codecs::png::PngEncoder::new_with_quality(
        &mut out,
        image::codecs::png::CompressionType::Fast,
        image::codecs::png::FilterType::NoFilter,
    )
    .write_image(
        resized.as_raw(),
        width,
        height,
        image::ExtendedColorType::Rgb8,
    )?;
    Ok(out.into_inner())
}

/// A process-unique capture counter.
fn next_capture_id() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    NEXT.fetch_add(1, Ordering::Relaxed)
}
