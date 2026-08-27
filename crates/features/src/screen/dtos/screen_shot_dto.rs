use nest_rs::core::input;

use super::{ImageFormatDto, RegionDto};

fn default_true() -> bool {
    true
}

fn default_quality() -> u8 {
    platform::screen::DEFAULT_WEBP_QUALITY
}

#[input]
#[derive(Debug)]
pub struct ScreenShotDto {
    #[serde(default)]
    pub region: Option<RegionDto>,

    #[serde(default)]
    pub format: ImageFormatDto,

    #[serde(default = "default_true")]
    pub stabilize: bool,

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
