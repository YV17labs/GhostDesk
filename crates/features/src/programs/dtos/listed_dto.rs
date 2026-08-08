use schemars::JsonSchema;
use serde::Serialize;

/// A list result.
///
/// `structuredContent` is typed as a JSON *object* by the spec, so a tool
/// whose natural result is a list needs a field to hang it on. `result` is
/// the name the Python original used, kept so existing prompts and clients
/// see the same shape.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct ListedDto<T> {
    pub result: Vec<T>,
}

impl<T> ListedDto<T> {
    pub fn new(result: impl IntoIterator<Item = T>) -> Self {
        Self {
            result: result.into_iter().collect(),
        }
    }
}
