use nest_rs::core::input;

use crate::screen::RegionDto;

#[input]
#[derive(Debug)]
pub struct MoveDto {
    pub x: i64,
    pub y: i64,

    #[serde(default)]
    pub region: Option<RegionDto>,
}
