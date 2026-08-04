use std::sync::Arc;
use std::time::{Duration, Instant};

use image::RgbImage;
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

        // The stabilisation loop already decoded the frame it settled on, so
        // the WebP encode below reuses it rather than decoding a third time
        // (grim's PNG → compare → encode used to decode the same bytes twice
        // per tick plus once at the end).
        let (png, decoded) = if stabilize {
            let (png, rgb) = self.capture_until_stable(region).await?;
            (png, Some(rgb))
        } else {
            (screen::capture_png(region, None).await?, None)
        };

        let bytes = match format {
            // grim already hands us PNG; re-encoding lossless to lossless
            // would cost a decode and an encode to produce the same picture.
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

    /// Poll grim until two consecutive frames are pixel-stable — this is what
    /// catches a page still animating after a click or a navigation.
    ///
    /// Returns the settled frame both encoded and decoded; the decoded half
    /// is what the caller re-uses instead of decoding it again.
    async fn capture_until_stable(
        &self,
        region: Option<Region>,
    ) -> anyhow::Result<(Vec<u8>, RgbImage)> {
        let mut previous_png = screen::capture_png(region, None).await?;
        let mut previous = screen::decode_rgb(&previous_png)?;
        let deadline = Instant::now() + STABILITY_TIMEOUT;

        while Instant::now() < deadline {
            let current_png = screen::capture_png(region, None).await?;
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
