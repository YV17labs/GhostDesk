//! `GHOSTDESK_SESSION__*` — how long a session may sit idle.

use nest_rs::config::{Config, ConfigService, config};

#[config(namespace = "session")]
#[derive(Clone, Debug)]
pub struct SessionConfig {
    /// Seconds of MCP silence after which every application window is
    /// closed. `0` disables the watchdog.
    ///
    /// Long-running agents leak GUI apps: every Firefox tab, every `foot`
    /// shell, every `mousepad` window stays resident until the container is
    /// killed. This is the deployment's safety net, and the agent cannot
    /// override it.
    pub idle_timeout_secs: u64,
}

impl Default for SessionConfig {
    fn default() -> Self {
        // Half an hour. A derived `Default` would land on `0` here, which is
        // the one value that means "no watchdog at all".
        Self {
            idle_timeout_secs: 1800,
        }
    }
}

impl SessionConfig {
    /// True when the idle watchdog should run at all.
    pub fn idle_watchdog_armed(&self) -> bool {
        self.idle_timeout_secs > 0
    }
}

impl Config for SessionConfig {
    fn from_env(env: &ConfigService, base: Self) -> nest_rs::config::Result<Self> {
        Ok(Self {
            // A malformed value aborts the boot naming the variable rather
            // than quietly falling back — a typo in the idle timeout used to
            // be indistinguishable from deliberately choosing the default.
            idle_timeout_secs: env
                .parse("IDLE_TIMEOUT_SECS")?
                .unwrap_or(base.idle_timeout_secs),
        })
    }
}
