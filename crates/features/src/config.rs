//! GhostDesk's own settings.
//!
//! Everything the *deployment* decides and the agent cannot: how big the
//! virtual screen is, how long a session may idle before its windows are
//! closed, and the bearer token that gates the endpoint when TLS is on.
//!
//! Transport settings are not here — host, port, TLS and CORS belong to
//! `NESTRS_HTTP__*`, and the MCP `Host` allow-list to `NESTRS_MCP__*`. This
//! namespace holds only what is GhostDesk's to decide.

use nest_rs::config::{Config, ConfigService, config};

#[config(namespace = "ghostdesk")]
#[derive(Clone, Debug)]
pub struct GhostdeskConfig {
    /// Virtual screen width in pixels. Must match the compositor's output,
    /// or every coordinate the agent computes lands in the wrong place.
    #[validate(range(min = 1, max = 16384))]
    pub screen_width: i64,

    /// Virtual screen height in pixels.
    #[validate(range(min = 1, max = 16384))]
    pub screen_height: i64,

    /// Seconds of MCP silence after which every Sway view is closed. `0`
    /// disables the watchdog.
    ///
    /// Long-running agents leak GUI apps: every Firefox tab, every `foot`
    /// shell, every `mousepad` window stays resident until the container is
    /// killed. This is the deployment's safety net, and the agent cannot
    /// override it.
    pub idle_timeout_secs: u64,

    /// Bearer token required on every MCP operation when TLS is on.
    /// Mandatory in that posture — the guard fails boot without it.
    pub auth_token: Option<String>,
}

impl Default for GhostdeskConfig {
    fn default() -> Self {
        Self {
            screen_width: 1280,
            screen_height: 1024,
            idle_timeout_secs: 1800,
            auth_token: None,
        }
    }
}

impl Config for GhostdeskConfig {
    fn from_env(env: &ConfigService, base: Self) -> nest_rs::config::Result<Self> {
        Ok(Self {
            screen_width: env.parse("SCREEN_WIDTH")?.unwrap_or(base.screen_width),
            screen_height: env.parse("SCREEN_HEIGHT")?.unwrap_or(base.screen_height),
            // A malformed value aborts the boot naming the variable rather
            // than quietly falling back — a typo in the idle timeout used to
            // be indistinguishable from deliberately choosing the default.
            idle_timeout_secs: env
                .parse("IDLE_TIMEOUT_SECS")?
                .unwrap_or(base.idle_timeout_secs),
            auth_token: env.get("AUTH_TOKEN").or(base.auth_token),
        })
    }
}

impl GhostdeskConfig {
    /// True when the idle watchdog should run at all.
    pub fn idle_watchdog_armed(&self) -> bool {
        self.idle_timeout_secs > 0
    }
}
