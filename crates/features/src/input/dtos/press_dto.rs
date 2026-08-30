use nest_rs::core::input;

#[input]
#[derive(Debug)]
pub struct PressDto {
    pub keys: String,
}
