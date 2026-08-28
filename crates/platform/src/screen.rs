//! The screen-capture contract, and the geometry a caller addresses it with.
//!
//! Pixel work on what comes back — decoding, comparing, re-encoding — is
//! [`frame`](crate::frame): none of it touches an OS, and a contract file that
//! also carried a codec could not be read as one.

use anyhow::Result;
use async_trait::async_trait;

use crate::coords::screen;

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
    /// Clamp to the screen bounds so a backend never sees a negative offset
    /// or an extent running past the edge.
    pub fn clamped(self) -> Self {
        let (width, height) = screen();
        let x = self.x.clamp(0, width);
        let y = self.y.clamp(0, height);
        Self {
            x,
            y,
            width: self.width.clamp(0, width - x),
            height: self.height.clamp(0, height - y),
        }
    }
}

/// Screen capture, one implementation per OS. Always answers PNG bytes —
/// the neutral pipeline in [`frame`](crate::frame) (decode, diff, re-encode)
/// is built around exactly one wire-in format.
#[async_trait]
pub trait ScreenBackend: Send + Sync {
    /// Capture the screen (or `region` of it) as raw PNG bytes.
    ///
    /// `scale` below 1.0 downsamples the capture, which is what makes
    /// comparison-only captures cheap; `None` means native size. `region` is
    /// already clamped to the screen by the caller.
    async fn capture_png(&self, region: Option<Region>, scale: Option<f32>) -> Result<Vec<u8>>;

    /// The display's geometry in captured pixels, when the OS owns it.
    ///
    /// `None` leaves the operator's configured size in charge: the Linux
    /// container drives a virtual display whose extent is a deployment
    /// decision, and no API can second-guess that. A backend attached to real
    /// hardware answers `Some` — the agent's coordinates have to match the
    /// pixels it is actually shown, and that is not negotiable by config.
    ///
    /// Required rather than defaulted: a new backend that forgot it would
    /// silently inherit an answer about hardware it does not own.
    fn geometry(&self) -> Option<(i64, i64)>;
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(clamped.width, screen().0);
        assert_eq!(clamped.height, 20);
    }
}
