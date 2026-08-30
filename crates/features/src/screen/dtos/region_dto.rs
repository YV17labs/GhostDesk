use nest_rs::core::input;
use platform::screen::Region;

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
