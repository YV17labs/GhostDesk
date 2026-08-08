use nest_rs::core::input;

/// What `clipboard_set` accepts.
#[input]
#[derive(Debug)]
pub struct ClipboardSetDto {
    pub text: String,
}
