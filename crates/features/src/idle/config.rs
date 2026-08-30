use nest_rs::config::{Config, ConfigService, config};

#[config(namespace = "idle")]
#[derive(Clone, Debug)]
pub struct IdleConfig {
    pub timeout_secs: u64,
}

impl Default for IdleConfig {
    fn default() -> Self {
        Self { timeout_secs: 1800 }
    }
}

impl IdleConfig {
    pub fn armed(&self) -> bool {
        self.timeout_secs > 0
    }
}

impl Config for IdleConfig {
    fn from_env(env: &ConfigService, base: Self) -> nest_rs::config::Result<Self> {
        Ok(Self {
            timeout_secs: env.parse("TIMEOUT_SECS")?.unwrap_or(base.timeout_secs),
        })
    }
}
