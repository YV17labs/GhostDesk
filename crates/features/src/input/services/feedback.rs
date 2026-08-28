use std::sync::Arc;
use std::time::{Duration, Instant};

use nest_rs::core::injectable;
use platform::frame::{self, FEEDBACK_SCALE};
use platform::screen::ScreenBackend;

use crate::input::action::Action;
use crate::input::error::InputError;
use crate::input::feedback::Feedback;

type Result<T> = std::result::Result<T, InputError>;

/// How long an act's effect is watched for, and how often the screen is read.
///
/// A field with a `Default` rather than two constants read inside the loop:
/// `InputService`'s tests reach this service through the verbs, so the only
/// way for them not to spend the real cadence once per act is for the cadence
/// to be something they can hand over. The container builds the default, so
/// production never states it.
#[derive(Debug, Clone, Copy)]
pub(super) struct Watch {
    pub window: Duration,
    pub poll: Duration,
}

impl Default for Watch {
    /// Two seconds is what a desktop is given to react — long enough for a menu
    /// to animate, short enough that an agent is not left waiting on a click
    /// that missed. A tenth of a second between looks catches the reaction
    /// without turning the watch into a capture budget of its own.
    fn default() -> Self {
        Self {
            window: Duration::from_secs(2),
            poll: Duration::from_millis(100),
        }
    }
}

#[injectable]
pub struct FeedbackService {
    #[inject]
    screen: Arc<dyn ScreenBackend>,
    watch: Watch,
}

impl FeedbackService {
    pub async fn capture_before(&self) -> Result<Vec<u8>> {
        self.screen
            .capture_png(None, Some(FEEDBACK_SCALE))
            .await
            .map_err(InputError::Feedback)
    }

    pub async fn observe(&self, action: Action, before: &[u8]) -> Result<Feedback> {
        let baseline = frame::decode_rgb(before).map_err(InputError::Feedback)?;
        let start = Instant::now();

        let mut screen_changed = false;
        while start.elapsed() < self.watch.window {
            tokio::time::sleep(self.watch.poll).await;
            let now = self.capture_before().await?;
            if frame::decode_rgb(&now).is_ok_and(|now| frame::differ_rgb(&baseline, &now)) {
                screen_changed = true;
                break;
            }
        }

        let feedback = Feedback {
            action,
            screen_changed,
            reaction_time_ms: start.elapsed().as_millis() as u64,
        };

        // `debug`, and joined to the act by `action`: the act itself is a
        // security record and was filed by the service that performed it, while
        // whether the screen moved afterwards is a diagnosis of this watch.
        tracing::debug!(
            target: "features::input",
            action = feedback.action.kind(),
            screen_changed,
            reaction_time_ms = feedback.reaction_time_ms,
            watch_ms = self.watch.window.as_millis() as u64,
            "action verdict",
        );
        Ok(feedback)
    }
}

#[cfg(test)]
impl FeedbackService {
    /// The service a test needs, without a container and without its cadence.
    pub(super) fn watching(screen: Arc<dyn ScreenBackend>, watch: Watch) -> Self {
        Self { screen, watch }
    }
}

#[cfg(test)]
mod tests {
    use nest_rs::testing::LogCapture;

    use super::*;
    use crate::testing::{FailingWatch, RestlessScreen, StillScreen};

    fn watching(screen: Arc<dyn ScreenBackend>, window_ms: u64) -> FeedbackService {
        FeedbackService::watching(
            screen,
            Watch {
                window: Duration::from_millis(window_ms),
                poll: Duration::from_millis(2),
            },
        )
    }

    fn typed() -> Action {
        Action::Type { chars: 3 }
    }

    #[tokio::test]
    async fn a_screen_that_moves_is_reported_as_changed() {
        let service = watching(Arc::new(RestlessScreen::default()), 200);
        let before = service.capture_before().await.expect("a baseline");
        let feedback = service.observe(typed(), &before).await.expect("observed");

        assert!(feedback.screen_changed);
        assert!(
            feedback.reaction_time_ms < 200,
            "the watch ends on the change, not on the deadline",
        );
    }

    #[tokio::test]
    async fn a_screen_that_does_not_move_is_reported_unchanged_at_the_deadline() {
        let service = watching(Arc::new(StillScreen), 40);
        let before = service.capture_before().await.expect("a baseline");
        let feedback = service.observe(typed(), &before).await.expect("observed");

        assert!(!feedback.screen_changed);
        assert!(feedback.reaction_time_ms >= 40, "the deadline was spent");
    }

    #[tokio::test]
    async fn a_baseline_that_cannot_be_taken_is_a_feedback_failure() {
        let service = watching(Arc::new(FailingWatch::from_the_first_capture()), 40);
        let err = service.capture_before().await.unwrap_err();
        assert!(matches!(err, InputError::Feedback(_)), "got: {err}");
    }

    #[tokio::test]
    async fn a_watch_that_loses_the_screen_fails_rather_than_reporting_quiet() {
        // A capture that stops answering mid-watch is not evidence that nothing
        // happened, and reporting `screen_changed: false` would tell the agent
        // its click missed when nobody knows.
        let service = watching(Arc::new(FailingWatch::default()), 200);
        let before = service.capture_before().await.expect("a baseline");
        let err = service.observe(typed(), &before).await.unwrap_err();
        assert!(matches!(err, InputError::Feedback(_)), "got: {err}");
    }

    #[tokio::test]
    async fn the_verdict_names_the_act_it_belongs_to() {
        let logs = LogCapture::install();
        let service = watching(Arc::new(RestlessScreen::default()), 200);
        let before = service.capture_before().await.expect("a baseline");
        service.observe(typed(), &before).await.expect("observed");

        let verdict = logs.expect_one("features::input", "action verdict");
        assert_eq!(verdict.field("action").as_deref(), Some("type"));
        assert_eq!(verdict.field("screen_changed").as_deref(), Some("true"));
    }

    #[tokio::test]
    async fn the_default_cadence_is_what_a_desktop_is_really_given() {
        // The container builds this one, so it is the only place the production
        // numbers are stated — and the only place a slip would go unnoticed.
        let watch = Watch::default();
        assert_eq!(watch.window, Duration::from_secs(2));
        assert_eq!(watch.poll, Duration::from_millis(100));
    }
}
