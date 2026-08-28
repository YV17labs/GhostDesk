use std::sync::Arc;

use nest_rs::core::{hooks, injectable};
use nest_rs::schedule::scheduled;

use crate::idle::IdleService;

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
        self.svc.mark_activity();
    }
}

#[scheduled]
impl IdleTasks {
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
        self.svc.mark_activity();
        Ok(())
    }
}
