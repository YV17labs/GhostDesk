use nest_rs::core::output;

#[output]
#[derive(Debug, Clone)]
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
