use nest_rs::core::input;
use platform::screen::Region;

/// A rectangle of the screen, in whatever coordinate space the connection
/// declared.
///
/// On a capture it says what to crop; on the act that follows it says what
/// the coordinates were read against. The two are one statement made twice,
/// and half of it points somewhere real but wrong.
#[input]
#[derive(Debug, Clone, Copy)]
pub struct RegionDto {
    pub x: i64,
    pub y: i64,
    pub width: i64,
    pub height: i64,
}

impl From<RegionDto> for Region {
    fn from(value: RegionDto) -> Self {
        Self {
            x: value.x,
            y: value.y,
            width: value.width,
            height: value.height,
        }
    }
}
