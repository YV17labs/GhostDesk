use std::sync::Arc;
use std::time::{Duration, Instant};

use image::RgbImage;
use nest_rs::core::{hooks, injectable};
use platform::coords;
use platform::screen::{self, ImageFormat, Region, ScreenBackend};

use super::config::ScreenConfig;
use super::error::ScreenError;

type Result<T> = std::result::Result<T, ScreenError>;

const STABILITY_TIMEOUT: Duration = Duration::from_millis(2500);
const STABILITY_POLL: Duration = Duration::from_millis(150);

pub struct Capture {
    pub bytes: Vec<u8>,
    pub format: ImageFormat,
}

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
        screen::decode_rgb(png).map_err(ScreenError::Decode)
    }

    pub async fn capture_settled(&self) -> Result<Capture> {
        self.capture(None, ImageFormat::Webp, true, screen::DEFAULT_WEBP_QUALITY)
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
            let (png, rgb, grab) = self.capture_until_stable(region).await?;
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
                (screen::encode_webp(&rgb, quality), Some(dimensions))
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

    async fn capture_until_stable(
        &self,
        region: Option<Region>,
    ) -> Result<(Vec<u8>, RgbImage, Grab)> {
        let mut previous_png = self.grab(region).await?;
        let mut previous = Self::decode(&previous_png)?;
        let mut frames = 1;
        let deadline = Instant::now() + STABILITY_TIMEOUT;

        while Instant::now() < deadline {
            let current_png = self.grab(region).await?;
            let current = Self::decode(&current_png)?;
            frames += 1;
            if !screen::differ_rgb(&previous, &current) {
                return Ok((
                    current_png,
                    current,
                    Grab {
                        frames,
                        settled: Some(true),
                    },
                ));
            }
            tokio::time::sleep(STABILITY_POLL).await;
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
