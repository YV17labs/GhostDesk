use nest_rs::config::config;

#[config(namespace = "screen")]
#[derive(Clone, Debug)]
pub struct ScreenConfig {
    #[validate(range(min = 1, max = 16384))]
    pub width: i64,

    #[validate(range(min = 1, max = 16384))]
    pub height: i64,
}

impl Default for ScreenConfig {
    fn default() -> Self {
        Self {
            width: 1280,
            height: 1024,
        }
    }
}
