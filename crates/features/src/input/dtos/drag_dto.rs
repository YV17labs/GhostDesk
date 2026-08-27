use nest_rs::core::input;

use super::ButtonDto;

#[input]
#[derive(Debug)]
pub struct DragDto {
    pub from_x: i64,
    pub from_y: i64,
    pub to_x: i64,
    pub to_y: i64,
    #[serde(default)]
    pub button: ButtonDto,
}
