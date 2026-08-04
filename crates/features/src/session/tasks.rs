use std::sync::Arc;

use nest_rs::core::{hooks, injectable};
use nest_rs::schedule::scheduled;

use super::idle::IdleService;

#[injectable]
pub struct IdleTasks {
    #[inject]
    idle: Arc<IdleService>,
}

#[hooks]
impl IdleTasks {
    #[on_application_bootstrap]
    async fn announce(&self) {
        match self.idle.timeout_secs() {
            0 => tracing::info!(
                target: "ghostdesk::idle",
                "watchdog disabled (NESTRS_GHOSTDESK__IDLE_TIMEOUT_SECS=0)",
            ),
            timeout => tracing::info!(
                target: "ghostdesk::idle",
                timeout_secs = timeout,
                "watchdog armed",
            ),
        }
        // Reset the clock at boot: a slow startup must not read as an idle
        // session and close the desktop before the agent's first request.
        self.idle.mark_activity();
    }
}

#[scheduled]
impl IdleTasks {
    /// Poll the idle clock and, past the threshold, close every view.
    ///
    /// A fixed 5s cadence rather than a timeout-derived one: the check itself
    /// is a single atomic load, so polling often costs nothing and keeps the
    /// watchdog firing within seconds of the deadline whatever it is set to.
    /// A failed run is logged and the schedule keeps ticking.
    #[every("5s")]
    async fn close_idle_views(&self) -> anyhow::Result<()> {
        if !self.idle.is_expired() {
            return Ok(());
        }

        tracing::info!(
            target: "ghostdesk::idle",
            idle_secs = self.idle.idle_secs(),
            threshold_secs = self.idle.timeout_secs(),
            "idle threshold reached — closing all views",
        );

        let closed = self.idle.cleanup_views().await;

        tracing::info!(target: "ghostdesk::idle", closed, "cleanup done");
        // Restart the clock so the next sweep is a full timeout away rather
        // than every tick from here on.
        self.idle.mark_activity();
        Ok(())
    }
}
