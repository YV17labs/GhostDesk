use nest_rs::core::input;

use super::{ImageFormatDto, RegionDto};

fn default_true() -> bool {
    true
}

fn default_quality() -> u8 {
    platform::screen::DEFAULT_WEBP_QUALITY
}

/// What `screen_shot` accepts.
#[input]
#[derive(Debug)]
pub struct ScreenShotDto {
    /// Area to capture. Omit to capture the full screen — almost always the
    /// right choice.
    #[serde(default)]
    pub region: Option<RegionDto>,

    /// `webp` (default, small payload, visually stable at UI resolutions) or
    /// `png` (lossless, larger).
    #[serde(default)]
    pub format: ImageFormatDto,

    /// Wait up to 2.5 s for two consecutive frames to be identical before
    /// returning — catches pages still animating after a click or a
    /// navigation. Set false only to observe a genuinely animating UI.
    #[serde(default = "default_true")]
    pub stabilize: bool,

    /// WebP encoder quality. The default is visually indistinguishable from
    /// 80 on typical UI content but roughly half the bytes; raise it to read
    /// fine pixels (small fonts in a PDF, design mockups, photos). Ignored
    /// for PNG.
    #[serde(default = "default_quality")]
    #[validate(range(min = 1, max = 100))]
    pub quality: u8,
}

#[cfg(test)]
mod tests {
    use nest_rs::core::validator::Validate;

    use super::*;

    #[test]
    fn omitted_fields_fall_back_to_the_agent_facing_defaults() {
        let params: ScreenShotDto = serde_json::from_str("{}").unwrap();
        assert!(params.region.is_none());
        assert_eq!(params.format, ImageFormatDto::Webp);
        assert!(params.stabilize);
        assert_eq!(params.quality, 50);
    }

    #[test]
    fn out_of_range_quality_fails_validation() {
        let params: ScreenShotDto = serde_json::from_str(r#"{"quality":0}"#).unwrap();
        assert!(params.validate().is_err());

        let params: ScreenShotDto = serde_json::from_str(r#"{"quality":100}"#).unwrap();
        assert!(params.validate().is_ok());
    }
}
