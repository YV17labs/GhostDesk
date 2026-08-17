//! Split from the service because the two answer to different callers: the
//! service is what the endpoint's per-call context reaches on every
//! operation, this is what the scheduler drives and nothing else may.

use std::sync::Arc;

use nest_rs::core::{hooks, injectable};
use nest_rs::schedule::scheduled;

use crate::idle::service::IdleService;

#[injectable]
pub struct IdleTasks {
    #[inject]
    svc: Arc<IdleService>,
}

#[hooks]
impl IdleTasks {
    #[on_application_bootstrap]
    async fn announce(&self) {
        match self.svc.timeout_secs() {
            0 => tracing::info!(
                target: "features::idle",
                timeout_secs = 0,
                "watchdog disabled",
            ),
            timeout => tracing::info!(
                target: "features::idle",
                timeout_secs = timeout,
                "watchdog armed",
            ),
        }
        // Reset the clock at boot: a slow startup must not read as an idle
        // session and close the desktop before the agent's first request.
        self.svc.mark_activity();
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
        if !self.svc.is_expired() {
            return Ok(());
        }

        tracing::info!(
            target: "features::idle",
            idle_secs = self.svc.idle_secs(),
            threshold_secs = self.svc.timeout_secs(),
            "idle threshold reached — closing all views",
        );

        let closed = self.svc.cleanup_views().await;

        tracing::info!(target: "features::idle", closed, "cleanup done");
        // Restart the clock so the next sweep is a full timeout away rather
        // than every tick from here on.
        self.svc.mark_activity();
        Ok(())
    }
}
