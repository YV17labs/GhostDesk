use schemars::JsonSchema;
use serde::Serialize;

use crate::input::services::Feedback;

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct FeedbackDto {
    pub action: String,
    pub screen_changed: bool,
    pub reaction_time_ms: u64,
}

impl From<Feedback> for FeedbackDto {
    fn from(feedback: Feedback) -> Self {
        Self {
            action: feedback.action.to_string(),
            screen_changed: feedback.screen_changed,
            reaction_time_ms: feedback.reaction_time_ms,
        }
    }
}

#[cfg(test)]
mod tests {
    use platform::input::Button;

    use super::*;
    use crate::input::action::Action;

    #[test]
    fn the_published_verdict_shape_is_what_clients_already_read() {
        let feedback = FeedbackDto::from(Feedback {
            action: Action::Click {
                button: Button::Left,
                x: 200,
                y: 830,
            },
            screen_changed: false,
            reaction_time_ms: 2000,
        });
        assert_eq!(
            serde_json::to_value(&feedback).unwrap(),
            serde_json::json!({
                "action": "Clicked left at (200, 830)",
                "screen_changed": false,
                "reaction_time_ms": 2000,
            }),
        );
    }
}
