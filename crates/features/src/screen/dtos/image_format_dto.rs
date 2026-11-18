use nest_rs::core::wire_enum;
use platform::screen::ImageFormat;

#[wire_enum]
#[derive(Default)]
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
