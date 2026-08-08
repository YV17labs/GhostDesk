use platform::input::ScrollDirection;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Which way to scroll.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
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
