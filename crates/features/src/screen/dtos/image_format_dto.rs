use platform::screen::ImageFormat;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Encoding for a returned capture.
///
/// Derives directly rather than through `#[input]`: a plain choice has
/// nothing for a validator to check.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ImageFormatDto {
    #[default]
    Webp,
    Png,
}

impl From<ImageFormatDto> for ImageFormat {
    fn from(value: ImageFormatDto) -> Self {
        match value {
            ImageFormatDto::Webp => Self::Webp,
            ImageFormatDto::Png => Self::Png,
        }
    }
}
