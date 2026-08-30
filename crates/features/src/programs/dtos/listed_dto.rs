use schemars::JsonSchema;
use serde::Serialize;

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
