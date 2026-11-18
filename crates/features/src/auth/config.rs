use nest_rs::config::config;
use nest_rs::core::Secret;

#[config(namespace = "auth")]
#[derive(Clone, Debug, Default)]
pub struct AuthConfig {
    pub token: Option<Secret<String>>,
}
