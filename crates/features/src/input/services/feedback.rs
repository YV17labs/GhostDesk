//! Post-action visual feedback — poll the screen until it changes.
//!
//! Every input tool answers with the same three fields. `screen_changed:
//! false` is the single most useful signal the agent gets: it means the input
//! landed nowhere, and the correct next call is a screenshot, not a retry.

use std::sync::Arc;
use std::time::{Duration, Instant};

use nest_rs::core::injectable;
use platform::screen::{self, FEEDBACK_SCALE, ScreenBackend};

use crate::input::error::InputError;

type Result<T> = std::result::Result<T, InputError>;

pub const POLL_INTERVAL: Duration = Duration::from_millis(100);
pub const POLL_TIMEOUT: Duration = Duration::from_secs(2);

/// What one observed action came back with.
///
/// Not the wire shape — [`FeedbackDto`](crate::input::dtos::FeedbackDto) is,
/// and converts from this. Both `telemetry` and the adapter read these three
/// fields, and only one of them publishes them.
#[derive(Debug, Clone)]
pub struct Feedback {
    /// What was performed, in words.
    pub action: String,
    /// Whether the screen visibly changed within [`POLL_TIMEOUT`].
    pub screen_changed: bool,
    /// How quickly the change was detected.
    pub reaction_time_ms: u64,
}

#[injectable]
pub struct FeedbackService {
    #[inject]
    screen: Arc<dyn ScreenBackend>,
}

impl FeedbackService {
    /// Capture the full screen at reduced resolution, before an action.
    ///
    /// The downsample is both a faster capture encode and a filter: at a
    /// quarter scale a blinking caret or a ticking clock digit stops
    /// registering.
    pub async fn capture_before(&self) -> Result<Vec<u8>> {
        self.screen
            .capture_png(None, Some(FEEDBACK_SCALE))
            .await
            .map_err(InputError::Feedback)
    }

    /// Poll until the screen differs from `before`, or the timeout expires.
    pub async fn observe(&self, action: impl Into<String>, before: &[u8]) -> Result<Feedback> {
        // Decode the baseline once — the loop would otherwise re-decode the
        // same bytes on every tick.
        let baseline = screen::decode_rgb(before).map_err(InputError::Feedback)?;
        let start = Instant::now();

        let mut screen_changed = false;
        while start.elapsed() < POLL_TIMEOUT {
            tokio::time::sleep(POLL_INTERVAL).await;
            let now = self
                .screen
                .capture_png(None, Some(FEEDBACK_SCALE))
                .await
                .map_err(InputError::Feedback)?;
            if screen::decode_rgb(&now).is_ok_and(|now| screen::differ_rgb(&baseline, &now)) {
                screen_changed = true;
                break;
            }
        }

        let feedback = Feedback {
            action: action.into(),
            screen_changed,
            reaction_time_ms: start.elapsed().as_millis() as u64,
        };

        if !screen_changed {
            tracing::warn!(
                target: "features::input",
                action = %feedback.action,
                timeout_ms = POLL_TIMEOUT.as_millis() as u64,
                "no visible screen change",
            );
        }
        Ok(feedback)
    }
}
