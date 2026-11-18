use nest_rs::core::wire_enum;
use platform::input::ScrollDirection;

#[wire_enum]
#[derive(Default)]
#[serde(rename_all = "lowercase")]
pub enum ScrollDirectionDto {
    Up,
    #[default]
    Down,
    Left,
    Right,
}

impl From<ScrollDirectionDto> for ScrollDirection {
    fn from(value: ScrollDirectionDto) -> Self {
        match value {
            ScrollDirectionDto::Up => Self::Up,
            ScrollDirectionDto::Down => Self::Down,
            ScrollDirectionDto::Left => Self::Left,
            ScrollDirectionDto::Right => Self::Right,
        }
    }
}
