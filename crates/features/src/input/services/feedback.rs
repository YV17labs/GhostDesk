use std::sync::Arc;
use std::time::{Duration, Instant};

use nest_rs::core::injectable;
use platform::screen::{self, FEEDBACK_SCALE, ScreenBackend};

use crate::input::action::Action;
use crate::input::error::InputError;

type Result<T> = std::result::Result<T, InputError>;

pub const POLL_INTERVAL: Duration = Duration::from_millis(100);
pub const POLL_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Debug, Clone)]
pub struct Feedback {
    pub action: Action,
    pub screen_changed: bool,
    pub reaction_time_ms: u64,
}

#[injectable]
pub struct FeedbackService {
    #[inject]
    screen: Arc<dyn ScreenBackend>,
}

impl FeedbackService {
    pub async fn capture_before(&self) -> Result<Vec<u8>> {
        self.screen
            .capture_png(None, Some(FEEDBACK_SCALE))
            .await
            .map_err(InputError::Feedback)
    }

    pub async fn observe(&self, action: Action, before: &[u8]) -> Result<Feedback> {
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
            action,
            screen_changed,
            reaction_time_ms: start.elapsed().as_millis() as u64,
        };

        let act = &feedback.action;
        tracing::info!(
            target: "features::input",
            action = act.kind(),
            button = act.button(),
            x = act.x(),
            y = act.y(),
            to_x = act.to_x(),
            to_y = act.to_y(),
            direction = act.direction(),
            amount = act.amount(),
            chars = act.chars(),
            keys = act.keys(),
            screen_changed,
            reaction_time_ms = feedback.reaction_time_ms,
            timeout_ms = POLL_TIMEOUT.as_millis() as u64,
            "input action",
        );
        Ok(feedback)
    }
}
