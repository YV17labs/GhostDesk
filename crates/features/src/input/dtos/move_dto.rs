use nest_rs::core::input;

#[input]
#[derive(Debug)]
pub struct MoveDto {
    pub x: i64,
    pub y: i64,
}
