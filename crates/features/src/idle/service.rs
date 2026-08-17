//! The idle watchdog — close every application window after N seconds of MCP
//! silence.
//!
//! Long-running agents leak GUI apps: every Firefox tab, every `foot` shell,
//! every `mousepad` window stays resident until the container is killed.
//! Configuration is operator-side, never client-side — the agent cannot
//! extend or disable its own leash.
//!
//! The desktop's own infrastructure (compositor, notification daemon, VNC
//! bridge, this server) is spared: the [`WindowManager`] contract lists only
//! real application windows, so the sweep never reaches them.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, LazyLock};
use std::time::Instant;

use nest_rs::core::injectable;
use platform::window::WindowManager;
use tokio::task::JoinSet;

use super::config::IdleConfig;

/// Monotonic origin for the idle clock — a property of the process, not of
/// the service, which is why it is not a field.
static ORIGIN: LazyLock<Instant> = LazyLock::new(Instant::now);

#[injectable]
pub struct IdleService {
    #[inject]
    config: Arc<IdleConfig>,
    #[inject]
    windows: Arc<dyn WindowManager>,
    /// Milliseconds since [`ORIGIN`] at the last MCP operation. An atomic,
    /// not a lock: this is written on every single operation and read by a
    /// timer, and neither should ever wait on the other.
    last_activity_ms: AtomicU64,
}

impl IdleService {
    /// Reset the idle clock. Called on every MCP operation.
    pub fn mark_activity(&self) {
        self.last_activity_ms
            .store(ORIGIN.elapsed().as_millis() as u64, Ordering::Relaxed);
    }

    /// Seconds elapsed since the last [`mark_activity`](Self::mark_activity).
    pub fn idle_secs(&self) -> u64 {
        let now = ORIGIN.elapsed().as_millis() as u64;
        let last = self.last_activity_ms.load(Ordering::Relaxed);
        now.saturating_sub(last) / 1000
    }

    /// Whether the desktop has been untouched past the configured threshold.
    pub fn is_expired(&self) -> bool {
        self.config.armed() && self.idle_secs() >= self.config.timeout_secs
    }

    pub fn timeout_secs(&self) -> u64 {
        self.config.timeout_secs
    }

    /// Close every application window. Returns how many were closed.
    ///
    /// Each close is a *graceful* request: the client receives the OS's
    /// close event and may flush state before exiting. They run concurrently
    /// so one slow-closing client cannot hold up the rest.
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
        // Even with the clock never marked, nothing expires.
        assert!(!idle.is_expired());
    }

    #[test]
    fn silence_past_the_threshold_expires_the_session() {
        let idle = service(1);
        // Rewind the last activity two seconds into the past.
        idle.last_activity_ms.store(0, Ordering::Relaxed);
        std::thread::sleep(std::time::Duration::from_millis(1100));
        assert!(idle.idle_secs() >= 1);
        assert!(idle.is_expired());
    }
}
