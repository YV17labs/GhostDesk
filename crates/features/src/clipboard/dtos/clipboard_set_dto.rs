use nest_rs::core::input;

#[input]
pub struct ClipboardSetDto {
    pub text: String,
}
