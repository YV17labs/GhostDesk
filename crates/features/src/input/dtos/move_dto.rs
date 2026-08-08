use nest_rs::core::input;

/// What `mouse_move` accepts.
#[input]
#[derive(Debug)]
pub struct MoveDto {
    pub x: i64,
    pub y: i64,
}
