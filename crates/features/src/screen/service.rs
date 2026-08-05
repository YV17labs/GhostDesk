use std::sync::Arc;
use std::time::{Duration, Instant};

use image::RgbImage;
use nest_rs::core::{hooks, injectable};
use platform::coords;
use platform::screen::{self, ImageFormat, Region, ScreenBackend};

use super::config::ScreenConfig;

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
    config: Arc<ScreenConfig>,
    #[inject]
    backend: Arc<dyn ScreenBackend>,
}

#[hooks]
impl ScreenService {
    /// Publish the screen size to the platform layer.
    ///
    /// The backend wins when it has real hardware to report: the agent's
    /// coordinates are offsets into the image it was shown, so a configured
    /// size that disagrees with the panel would put every click in the wrong
    /// place. A headless virtual display answers `None`, and there the
    /// operator's setting is the only truth there is.
    ///
    /// Runs in `on_module_init`, the earlier of the two init phases, so the
    /// size is in place before `InputService` warms the input backend up in
    /// `on_application_bootstrap` — Wayland's virtual pointer reports
    /// absolute motion against exactly these extents.
    #[on_module_init]
    async fn install_screen_size(&self) {
        // Remembered, not re-derived: comparing the two afterwards would
        // report "config" for a panel that merely happens to match it.
        let ((width, height), source) = match self.backend.geometry() {
            Some(geometry) => (geometry, "display"),
            None => ((self.config.width, self.config.height), "config"),
        };

        coords::set_screen(width, height);
        tracing::info!(
            target: "ghostdesk::screen",
            width,
            height,
            source,
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

        // The stabilisation loop already decoded the frame it settled on, so
        // the WebP encode below reuses it rather than decoding a third time
        // (capture PNG → compare → encode used to decode the same bytes
        // twice per tick plus once at the end).
        let (png, decoded) = if stabilize {
            let (png, rgb) = self.capture_until_stable(region).await?;
            (png, Some(rgb))
        } else {
            (self.backend.capture_png(region, None).await?, None)
        };

        let bytes = match format {
            // The backend already hands us PNG; re-encoding lossless to
            // lossless would cost a decode and an encode to produce the same
            // picture.
            ImageFormat::Png => png,
            ImageFormat::Webp => {
                let rgb = match decoded {
                    Some(rgb) => rgb,
                    None => screen::decode_rgb(&png)?,
                };
                screen::encode_webp(&rgb, quality)
            }
        };

        Ok(Capture { bytes, format })
    }

    /// Poll the backend until two consecutive frames are pixel-stable — this
    /// is what catches a page still animating after a click or a navigation.
    ///
    /// Returns the settled frame both encoded and decoded; the decoded half
    /// is what the caller re-uses instead of decoding it again.
    async fn capture_until_stable(
        &self,
        region: Option<Region>,
    ) -> anyhow::Result<(Vec<u8>, RgbImage)> {
        let mut previous_png = self.backend.capture_png(region, None).await?;
        let mut previous = screen::decode_rgb(&previous_png)?;
        let deadline = Instant::now() + STABILITY_TIMEOUT;

        while Instant::now() < deadline {
            let current_png = self.backend.capture_png(region, None).await?;
            let current = screen::decode_rgb(&current_png)?;
            if !screen::differ_rgb(&previous, &current) {
                return Ok((current_png, current));
            }
            tokio::time::sleep(STABILITY_POLL).await;
            previous_png = current_png;
            previous = current;
        }

        Ok((previous_png, previous))
    }
}
