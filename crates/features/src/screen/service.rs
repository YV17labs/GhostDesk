use std::sync::Arc;
use std::time::{Duration, Instant};

use image::RgbImage;
use nest_rs::core::{hooks, injectable};
use platform::screen::{ImageFormat, Region, ScreenBackend};
use platform::{coords, frame};

use super::capture::Capture;
use super::config::ScreenConfig;
use super::error::ScreenError;

type Result<T> = std::result::Result<T, ScreenError>;

const STABILITY_TIMEOUT: Duration = Duration::from_millis(2500);
const STABILITY_POLL: Duration = Duration::from_millis(150);

/// How a frame was obtained, for the line that reports it. Private: nobody
/// upstream asks how many times the screen was read.
struct Grab {
    frames: u32,
    settled: Option<bool>,
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
    #[on_module_init]
    async fn install_screen_size(&self) {
        let ((width, height), source) = match self.backend.geometry() {
            Some(geometry) => (geometry, "display"),
            None => ((self.config.width, self.config.height), "config"),
        };

        coords::set_screen(width, height);
        tracing::info!(
            target: "features::screen",
            width,
            height,
            source,
            "screen geometry installed",
        );
    }
}

impl ScreenService {
    async fn grab(&self, region: Option<Region>) -> Result<Vec<u8>> {
        self.backend
            .capture_png(region, None)
            .await
            .map_err(ScreenError::Capture)
    }

    fn decode(png: &[u8]) -> Result<RgbImage> {
        frame::decode_rgb(png).map_err(ScreenError::Decode)
    }

    pub async fn capture_settled(&self) -> Result<Capture> {
        self.capture(None, ImageFormat::Webp, true, frame::DEFAULT_WEBP_QUALITY)
            .await
    }

    pub async fn capture(
        &self,
        region: Option<Region>,
        format: ImageFormat,
        stabilize: bool,
        quality: u8,
    ) -> Result<Capture> {
        let region = region.map(|region| coords::region_to_pixels(region).clamped());
        let started = Instant::now();

        let (png, decoded, grab) = if stabilize {
            let (png, rgb, grab) = self
                .capture_until_stable(region, STABILITY_TIMEOUT, STABILITY_POLL)
                .await?;
            (png, Some(rgb), grab)
        } else {
            let png = self.grab(region).await?;
            (
                png,
                None,
                Grab {
                    frames: 1,
                    settled: None,
                },
            )
        };
        let grab_ms = started.elapsed().as_millis() as u64;

        let encode_started = Instant::now();
        let (bytes, dimensions) = match format {
            ImageFormat::Png => {
                let dimensions = decoded.map(|rgb| rgb.dimensions());
                (png, dimensions)
            }
            ImageFormat::Webp => {
                let rgb = match decoded {
                    Some(rgb) => rgb,
                    None => Self::decode(&png)?,
                };
                let dimensions = rgb.dimensions();
                (frame::encode_webp(&rgb, quality), Some(dimensions))
            }
        };

        tracing::info!(
            target: "features::screen",
            bytes = bytes.len(),
            width = dimensions.map(|(width, _)| width),
            height = dimensions.map(|(_, height)| height),
            format = format.mime(),
            quality,
            stabilize,
            grab_ms,
            frames = grab.frames,
            settled = grab.settled,
            encode_ms = encode_started.elapsed().as_millis() as u64,
            total_ms = started.elapsed().as_millis() as u64,
            "captured",
        );

        Ok(Capture { bytes, format })
    }

    /// The wait, over the window and cadence it is handed rather than over the
    /// constants — so the give-up branch can be proved without spending the two
    /// and a half seconds a real desktop is given to settle.
    async fn capture_until_stable(
        &self,
        region: Option<Region>,
        wait: Duration,
        poll: Duration,
    ) -> Result<(Vec<u8>, RgbImage, Grab)> {
        let mut previous_png = self.grab(region).await?;
        let mut previous = Self::decode(&previous_png)?;
        let mut frames = 1;
        let deadline = Instant::now() + wait;

        while Instant::now() < deadline {
            let current_png = self.grab(region).await?;
            let current = Self::decode(&current_png)?;
            frames += 1;
            if !frame::differ_rgb(&previous, &current) {
                return Ok((
                    current_png,
                    current,
                    Grab {
                        frames,
                        settled: Some(true),
                    },
                ));
            }
            tokio::time::sleep(poll).await;
            previous_png = current_png;
            previous = current;
        }

        Ok((
            previous_png,
            previous,
            Grab {
                frames,
                settled: Some(false),
            },
        ))
    }
}

#[cfg(test)]
mod tests {
    use nest_rs::testing::LogCapture;

    use super::*;
    use crate::testing::{FailingWatch, RestlessScreen, StillScreen};

    fn service(backend: Arc<dyn ScreenBackend>) -> ScreenService {
        ScreenService {
            config: Arc::new(ScreenConfig::default()),
            backend,
        }
    }

    #[tokio::test]
    async fn a_png_capture_hands_back_the_backends_own_bytes() {
        let capture = service(Arc::new(StillScreen))
            .capture(None, ImageFormat::Png, false, 50)
            .await
            .expect("captured");

        assert_eq!(capture.format, ImageFormat::Png);
        assert!(
            capture.bytes.starts_with(&[0x89, b'P', b'N', b'G']),
            "a backend already answers PNG; re-encoding it would be pure waste",
        );
    }

    #[tokio::test]
    async fn the_settled_capture_is_webp() {
        let capture = service(Arc::new(StillScreen))
            .capture_settled()
            .await
            .expect("captured");

        assert_eq!(capture.format, ImageFormat::Webp);
        assert!(capture.bytes.starts_with(b"RIFF"));
    }

    #[tokio::test]
    async fn a_settled_desktop_stops_at_the_second_frame() {
        let (_png, _rgb, grab) = service(Arc::new(StillScreen))
            .capture_until_stable(None, Duration::from_secs(5), STABILITY_POLL)
            .await
            .expect("settled");

        assert_eq!(grab.settled, Some(true));
        assert_eq!(
            grab.frames, 2,
            "one baseline plus the frame that matched it"
        );
    }

    #[tokio::test]
    async fn a_desktop_that_never_settles_gives_up_and_says_so() {
        // The agent still gets a frame: a screen playing a video is not a
        // reason to refuse a screenshot, only a reason to say it is not
        // settled.
        let (png, _rgb, grab) = service(Arc::new(RestlessScreen::default()))
            .capture_until_stable(None, Duration::from_millis(40), Duration::from_millis(2))
            .await
            .expect("gave up with a frame");

        assert_eq!(grab.settled, Some(false));
        assert!(grab.frames > 1);
        assert!(!png.is_empty());
    }

    #[tokio::test]
    async fn a_backend_that_cannot_capture_is_a_capture_failure() {
        let refused = service(Arc::new(FailingWatch::from_the_first_capture()))
            .capture(None, ImageFormat::Png, false, 50)
            .await;
        assert!(
            matches!(refused, Err(ScreenError::Capture(_))),
            "a screen that cannot be read is not an empty screen",
        );
    }

    #[tokio::test]
    async fn a_backend_that_owns_its_geometry_outranks_the_configured_size() {
        let logs = LogCapture::install();
        service(Arc::new(RestlessScreen::default()))
            .install_screen_size()
            .await;

        let installed = logs.expect_one("features::screen", "screen geometry installed");
        assert_eq!(installed.field("source").as_deref(), Some("display"));
        assert_eq!(installed.field("width").as_deref(), Some("1920"));
    }

    #[tokio::test]
    async fn a_virtual_display_leaves_the_operators_size_in_charge() {
        let logs = LogCapture::install();
        service(Arc::new(StillScreen)).install_screen_size().await;

        let installed = logs.expect_one("features::screen", "screen geometry installed");
        assert_eq!(installed.field("source").as_deref(), Some("config"));
        assert_eq!(installed.field("width").as_deref(), Some("1280"));
    }
}
