use platform::input::Button;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ButtonDto {
    #[default]
    Left,
    Middle,
    Right,
}

impl From<ButtonDto> for Button {
    fn from(value: ButtonDto) -> Self {
        match value {
            ButtonDto::Left => Self::Left,
            ButtonDto::Middle => Self::Middle,
            ButtonDto::Right => Self::Right,
        }
    }
}
