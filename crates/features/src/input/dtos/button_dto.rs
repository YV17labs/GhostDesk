use nest_rs::core::wire_enum;
use platform::input::Button;

#[wire_enum]
#[derive(Default)]
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
