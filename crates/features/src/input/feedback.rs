//! Post-action visual feedback — poll the screen until it changes.
//!
//! Every input tool answers with the same three fields. `screen_changed:
//! false` is the single most useful signal the agent gets: it means the input
//! landed nowhere, and the correct next call is a screenshot, not a retry.

use std::time::{Duration, Instant};

use nest_rs::core::injectable;
use platform::screen::{self, FEEDBACK_SCALE};

pub const POLL_INTERVAL: Duration = Duration::from_millis(100);
pub const POLL_TIMEOUT: Duration = Duration::from_secs(2);

/// The standard payload every input tool returns.
#[derive(Debug, Clone, serde::Serialize, schemars::JsonSchema)]
pub struct Feedback {
    /// What was performed, in words.
    pub action: String,
    /// Whether the screen visibly changed within [`POLL_TIMEOUT`].
    pub screen_changed: bool,
    /// How quickly the change was detected.
    pub reaction_time_ms: u64,
}

#[injectable]
#[derive(Default)]
pub struct FeedbackService;

impl FeedbackService {
    /// Capture the full screen at reduced resolution, before an action.
    ///
    /// The downsample is both a faster grim encode and a filter: at a quarter
    /// scale a blinking caret or a ticking clock digit stops registering.
    pub async fn capture_before(&self) -> anyhow::Result<Vec<u8>> {
        Ok(screen::capture_png(None, Some(FEEDBACK_SCALE)).await?)
    }

    /// Poll until the screen differs from `before`, or the timeout expires.
    pub async fn observe(
        &self,
        action: impl Into<String>,
        before: &[u8],
    ) -> anyhow::Result<Feedback> {
        // Decode the baseline once — the loop would otherwise re-decode the
        // same bytes on every tick.
        let baseline = screen::decode_rgb(before)?;
        let start = Instant::now();

        let mut screen_changed = false;
        while start.elapsed() < POLL_TIMEOUT {
            tokio::time::sleep(POLL_INTERVAL).await;
            let now = screen::capture_png(None, Some(FEEDBACK_SCALE)).await?;
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
                target: "ghostdesk::input",
                action = %feedback.action,
                timeout_ms = POLL_TIMEOUT.as_millis() as u64,
                "no visible screen change",
            );
        }
        Ok(feedback)
    }
}
