use nest_rs::core::input;

/// What `key_press` accepts.
#[input]
#[derive(Debug)]
pub struct PressDto {
    /// A key or a chord, `+`-separated: `ctrl+shift+tab`, `alt+tab`, `f5`.
    pub keys: String,
}
