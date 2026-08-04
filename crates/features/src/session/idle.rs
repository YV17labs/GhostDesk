//! Idle session watchdog — close every Sway view after N seconds of MCP
//! silence.
//!
//! Long-running agents leak GUI apps: every Firefox tab, every `foot` shell,
//! every `mousepad` window stays resident until the container is killed.
//! Configuration is operator-side, never client-side — the agent cannot
//! extend or disable its own leash.
//!
//! The desktop's own infrastructure (sway, mako, wayvnc, supervisord, this
//! server) is spared: none of those are Sway *views*, so walking the tree's
//! view nodes never reaches them.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, LazyLock};
use std::time::Instant;

use nest_rs::core::injectable;
use platform::sway;
use tokio::task::JoinSet;

use crate::config::GhostdeskConfig;

/// Monotonic origin for the idle clock.
///
/// A process-wide `LazyLock` rather than a field: `Instant` has no `Default`,
/// and the container builds a provider's non-injected fields from one. The
/// origin is a property of the process anyway, not of the service.
static ORIGIN: LazyLock<Instant> = LazyLock::new(Instant::now);

#[injectable]
pub struct IdleService {
    #[inject]
    config: Arc<GhostdeskConfig>,
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

    /// Whether the session has been silent past the configured threshold.
    pub fn is_expired(&self) -> bool {
        self.config.idle_watchdog_armed() && self.idle_secs() >= self.config.idle_timeout_secs
    }

    pub fn timeout_secs(&self) -> u64 {
        self.config.idle_timeout_secs
    }

    /// Close every Sway client window. Returns how many were closed.
    ///
    /// Each kill is a *graceful* close request: the client receives a Wayland
    /// close event and may flush state before exiting. They run concurrently
    /// so one slow-closing client cannot hold up the rest.
    pub async fn cleanup_views(&self) -> usize {
        let Some(tree) = sway::get_tree().await else {
            return 0;
        };

        let targets: Vec<(i64, String)> = sway::iter_views(&tree)
            .into_iter()
            .filter_map(|node| {
                let id = node.get("id").and_then(|v| v.as_i64())?;
                Some((id, sway::view_label(node)))
            })
            .collect();

        let mut kills = JoinSet::new();
        for (id, label) in targets {
            kills.spawn(async move {
                match sway::kill_view(id).await {
                    Ok(()) => {
                        tracing::info!(
                            target: "ghostdesk::idle",
                            con_id = id,
                            view = %label,
                            "closed view",
                        );
                        true
                    }
                    Err(err) => {
                        tracing::error!(
                            target: "ghostdesk::idle",
                            con_id = id,
                            view = %label,
                            error = %err,
                            "failed to close view",
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

    fn service(timeout_secs: u64) -> IdleService {
        IdleService {
            config: Arc::new(GhostdeskConfig {
                idle_timeout_secs: timeout_secs,
                ..GhostdeskConfig::default()
            }),
            last_activity_ms: AtomicU64::new(ORIGIN.elapsed().as_millis() as u64),
        }
    }

    #[test]
    fn a_fresh_session_is_not_expired() {
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
