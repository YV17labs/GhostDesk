use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, LazyLock};
use std::time::Instant;

use nest_rs::core::injectable;
use platform::window::WindowManager;
use tokio::task::JoinSet;

use super::config::IdleConfig;

static ORIGIN: LazyLock<Instant> = LazyLock::new(Instant::now);

#[injectable]
pub struct IdleService {
    #[inject]
    config: Arc<IdleConfig>,
    #[inject]
    windows: Arc<dyn WindowManager>,
    last_activity_ms: AtomicU64,
}

impl IdleService {
    pub fn mark_activity(&self) {
        self.last_activity_ms
            .store(ORIGIN.elapsed().as_millis() as u64, Ordering::Relaxed);
    }

    pub fn idle_secs(&self) -> u64 {
        let now = ORIGIN.elapsed().as_millis() as u64;
        let last = self.last_activity_ms.load(Ordering::Relaxed);
        now.saturating_sub(last) / 1000
    }

    pub fn is_expired(&self) -> bool {
        self.config.armed() && self.idle_secs() >= self.config.timeout_secs
    }

    pub fn timeout_secs(&self) -> u64 {
        self.config.timeout_secs
    }

    pub async fn cleanup_views(&self) -> usize {
        let windows = match self.windows.windows().await {
            Ok(windows) => windows,
            Err(err) => {
                tracing::error!(
                    target: "features::idle",
                    error = %err,
                    "window enumeration failed — nothing closed",
                );
                return 0;
            }
        };

        let mut kills = JoinSet::new();
        for window in windows {
            let manager = Arc::clone(&self.windows);
            kills.spawn(async move {
                match manager.close(&window.id).await {
                    Ok(()) => {
                        tracing::info!(
                            target: "features::idle",
                            window = %window.id,
                            app = %window.app,
                            title = %window.title,
                            "closed window",
                        );
                        true
                    }
                    Err(err) => {
                        tracing::error!(
                            target: "features::idle",
                            window = %window.id,
                            app = %window.app,
                            title = %window.title,
                            error = %err,
                            "failed to close window",
                        );
                        false
                    }
                }
            });
        }

        let mut closed = 0;
        while let Some(outcome) = kills.join_next().await {
            if outcome.unwrap_or(false) {
                closed += 1;
            }
        }
        closed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::NoWindows;

    fn service(timeout_secs: u64) -> IdleService {
        IdleService {
            config: Arc::new(IdleConfig { timeout_secs }),
            windows: Arc::new(NoWindows),
            last_activity_ms: AtomicU64::new(ORIGIN.elapsed().as_millis() as u64),
        }
    }

    #[test]
    fn a_fresh_desktop_is_not_expired() {
        let idle = service(1800);
        idle.mark_activity();
        assert_eq!(idle.idle_secs(), 0);
        assert!(!idle.is_expired());
    }

    #[test]
    fn a_zero_timeout_disarms_the_watchdog() {
        let idle = service(0);
        assert!(!idle.is_expired());
    }

    #[test]
    fn silence_past_the_threshold_expires_the_session() {
        let idle = service(1);
        idle.last_activity_ms.store(0, Ordering::Relaxed);
        std::thread::sleep(std::time::Duration::from_millis(1100));
        assert!(idle.idle_secs() >= 1);
        assert!(idle.is_expired());
    }
}
