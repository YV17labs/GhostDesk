//! The `telemetry` namespace — when a session counts as over.

use nest_rs::config::{Config, ConfigService, config};

#[config(namespace = "telemetry")]
#[derive(Clone, Debug)]
pub struct TelemetryConfig {
    /// Seconds of silence after which a session is considered over and its
    /// summary is logged.
    ///
    /// There is no verbosity setting here on purpose: how much of this
    /// reaches the log is `<PREFIX>_LOG`'s job, like every other target in
    /// the server. Per-call lines are `debug`, summaries `info`, repeated
    /// futile actions `warn`.
    #[validate(range(min = 5))]
    pub session_idle_secs: u64,
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        // Two minutes. Long enough that a model thinking hard between two
        // tool calls never has its session split in half, short enough that a
        // summary lands while the run is still interesting.
        Self {
            session_idle_secs: 120,
        }
    }
}

impl Config for TelemetryConfig {
    fn from_env(env: &ConfigService, base: Self) -> nest_rs::config::Result<Self> {
        Ok(Self {
            session_idle_secs: env
                .parse("SESSION_IDLE_SECS")?
                .unwrap_or(base.session_idle_secs),
        })
    }
}
