use nest_rs::core::input;

use super::ButtonDto;
use crate::screen::RegionDto;

#[input]
#[derive(Debug)]
pub struct DragDto {
    pub from_x: i64,
    pub from_y: i64,
    pub to_x: i64,
    pub to_y: i64,
    #[serde(default)]
    pub button: ButtonDto,

    /// One region for both ends: a drag that started in one frame and ended
    /// in another is a drag across two screens that no longer exist together.
    #[serde(default)]
    pub region: Option<RegionDto>,
}
