use std::sync::Arc;
use std::time::{Duration, Instant};

use nest_rs::core::{hooks, injectable};
use platform::coords;
use platform::screen::{self, ImageFormat, Region};

use crate::config::GhostdeskConfig;

/// How long to keep waiting for two identical frames before giving up and
/// returning the latest one. A genuinely animating screen must not block the
/// agent forever.
const STABILITY_TIMEOUT: Duration = Duration::from_millis(2500);
const STABILITY_POLL: Duration = Duration::from_millis(150);

/// One encoded capture, ready for the wire.
pub struct Capture {
    pub bytes: Vec<u8>,
    pub format: ImageFormat,
}

#[injectable]
pub struct ScreenService {
    #[inject]
    config: Arc<GhostdeskConfig>,
}

#[hooks]
impl ScreenService {
    /// Publish the configured screen size to the platform layer.
    ///
    /// Runs in `on_module_init`, the earlier of the two init phases, so the
    /// size is in place before `InputService` opens its Wayland connection in
    /// `on_application_bootstrap` — the virtual pointer reports absolute
    /// motion against exactly these extents.
    #[on_module_init]
    async fn install_screen_size(&self) {
        coords::set_screen(self.config.screen_width, self.config.screen_height);
        tracing::info!(
            target: "ghostdesk::screen",
            width = self.config.screen_width,
            height = self.config.screen_height,
            "screen geometry installed",
        );
    }
}

impl ScreenService {
    /// Capture the screen and encode it for the wire.
    pub async fn capture(
        &self,
        region: Option<Region>,
        format: ImageFormat,
        stabilize: bool,
        quality: u8,
    ) -> anyhow::Result<Capture> {
        let region = region.map(Region::clamped);

        let raw = if stabilize {
            self.capture_until_stable(region).await?
        } else {
            screen::capture_png(region, None).await?
        };

        Ok(Capture {
            bytes: screen::reencode(&raw, format, quality)?,
            format,
        })
    }

    /// Poll grim until two consecutive frames are pixel-stable — this is what
    /// catches a page still animating after a click or a navigation.
    async fn capture_until_stable(&self, region: Option<Region>) -> anyhow::Result<Vec<u8>> {
        let mut previous = screen::capture_png(region, None).await?;
        let deadline = Instant::now() + STABILITY_TIMEOUT;

        while Instant::now() < deadline {
            let current = screen::capture_png(region, None).await?;
            if screen::screens_stable(&previous, &current) {
                return Ok(current);
            }
            tokio::time::sleep(STABILITY_POLL).await;
            previous = current;
        }

        Ok(previous)
    }
}
