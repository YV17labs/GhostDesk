//! `GHOSTDESK_AUTH__*` — what the endpoint demands of a caller.
//!
//! Its own namespace rather than a field under `mcp`: `GHOSTDESK_MCP__*` is
//! the framework's, and it already owns the transport's `Host` allow-list. A
//! feature config declaring that namespace would be reading someone else's
//! contract.

use nest_rs::config::{Config, ConfigService, config};

#[config(namespace = "auth")]
#[derive(Clone, Debug, Default)]
pub struct AuthConfig {
    /// Bearer token required on every MCP operation when TLS is on.
    /// Mandatory in that posture — the guard fails boot without it.
    pub token: Option<String>,
}

impl Config for AuthConfig {
    fn from_env(env: &ConfigService, base: Self) -> nest_rs::config::Result<Self> {
        Ok(Self {
            token: env.get("TOKEN").or(base.token),
        })
    }
}
