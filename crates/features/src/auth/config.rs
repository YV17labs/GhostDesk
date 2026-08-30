use nest_rs::config::{Config, ConfigService, config};

#[config(namespace = "auth")]
#[derive(Clone, Default)]
pub struct AuthConfig {
    pub token: Option<String>,
}

impl Config for AuthConfig {
    fn from_env(env: &ConfigService, base: Self) -> nest_rs::config::Result<Self> {
        Ok(Self {
            token: env.get("TOKEN").or(base.token),
        })
    }
}
