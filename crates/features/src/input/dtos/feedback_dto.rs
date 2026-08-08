use schemars::JsonSchema;
use serde::Serialize;

use crate::input::services::Feedback;

/// What every input tool answers with.
///
/// `screen_changed: false` is the single most useful signal the agent gets,
/// and the endpoint's session brief teaches it by name — which is exactly why
/// the field belongs to a type the wire owns rather than to the service.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct FeedbackDto {
    /// What was performed, in words.
    pub action: String,
    /// Whether the screen visibly changed within the feedback window.
    pub screen_changed: bool,
    /// How quickly the change was detected.
    pub reaction_time_ms: u64,
}

impl From<Feedback> for FeedbackDto {
    fn from(feedback: Feedback) -> Self {
        Self {
            action: feedback.action,
            screen_changed: feedback.screen_changed,
            reaction_time_ms: feedback.reaction_time_ms,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_published_verdict_shape_is_what_clients_already_read() {
        let feedback = FeedbackDto::from(Feedback {
            action: "Clicked left at (200, 830)".into(),
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
