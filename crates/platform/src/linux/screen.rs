//! The Linux [`ScreenBackend`] — `grim`, the wlroots screencopy client.

use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;

use crate::cmd;
use crate::screen::{Region, ScreenBackend};

/// The [`ScreenBackend`] the host selector hands out on Linux.
pub struct Grim;

#[async_trait]
impl ScreenBackend for Grim {
    /// Single grim invocation — raw PNG bytes.
    ///
    /// grim writes to stdout when the output path is `-`. Region geometry
    /// follows the standard Wayland `X,Y WxH` format. `scale` below 1.0
    /// downsamples the encoded image, which is what makes comparison-only
    /// captures cheap.
    async fn capture_png(&self, region: Option<Region>, scale: Option<f32>) -> Result<Vec<u8>> {
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

        Ok(cmd::run_bytes(&argv, Duration::from_secs(10)).await?)
    }

    /// The compositor's output is sized from `GHOSTDESK_SCREEN__*` by the
    /// container entrypoint, so the operator's setting is the only truth
    /// there is — there is nothing here to discover.
    fn geometry(&self) -> Option<(i64, i64)> {
        None
    }
}
