use nest_rs::core::input;

/// What `key_type` accepts.
#[input]
#[derive(Debug)]
pub struct TypeDto {
    pub text: String,
}
