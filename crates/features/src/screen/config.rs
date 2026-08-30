use nest_rs::config::{Config, ConfigService, config};

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

impl Config for ScreenConfig {
    fn from_env(env: &ConfigService, base: Self) -> nest_rs::config::Result<Self> {
        Ok(Self {
            width: env.parse("WIDTH")?.unwrap_or(base.width),
            height: env.parse("HEIGHT")?.unwrap_or(base.height),
        })
    }
}
