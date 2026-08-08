//! The `idle` namespace — how long a desktop may sit untouched.

use nest_rs::config::{Config, ConfigService, config};

#[config(namespace = "idle")]
#[derive(Clone, Debug)]
pub struct IdleConfig {
    /// Seconds of MCP silence after which every application window is
    /// closed. `0` disables the watchdog.
    ///
    /// Long-running agents leak GUI apps: every Firefox tab, every `foot`
    /// shell, every `mousepad` window stays resident until the container is
    /// killed. This is the deployment's safety net, and the agent cannot
    /// override it.
    pub timeout_secs: u64,
}

impl Default for IdleConfig {
    fn default() -> Self {
        // Half an hour. A derived `Default` would land on `0` here, which is
        // the one value that means "no watchdog at all".
        Self { timeout_secs: 1800 }
    }
}

impl IdleConfig {
    /// True when the watchdog should run at all.
    pub fn armed(&self) -> bool {
        self.timeout_secs > 0
    }
}

impl Config for IdleConfig {
    fn from_env(env: &ConfigService, base: Self) -> nest_rs::config::Result<Self> {
        Ok(Self {
            // A malformed value aborts the boot naming the variable rather
            // than quietly falling back — a typo in the idle timeout used to
            // be indistinguishable from deliberately choosing the default.
            timeout_secs: env.parse("TIMEOUT_SECS")?.unwrap_or(base.timeout_secs),
        })
    }
}
