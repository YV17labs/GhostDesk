use schemars::JsonSchema;
use serde::Serialize;

use crate::input::action::Action;
use crate::input::feedback::Feedback;

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct FeedbackDto {
    pub action: String,
    pub screen_changed: bool,
    pub reaction_time_ms: u64,
}

/// What the agent is told it just did.
///
/// Here rather than on `Action` because it is the wire's wording, not the
/// domain's: the trail spells the same act in fields, and a sentence answers
/// none of the questions asked of a trail. Both are phrased from the same
/// value, so they cannot drift.
fn prose(action: &Action) -> String {
    match action {
        Action::Move { x, y } => format!("Moved cursor to ({x}, {y})"),
        Action::Click { button, x, y } => {
            format!("Clicked {} at ({x}, {y})", button.as_str())
        }
        Action::DoubleClick { button, x, y } => {
            format!("Double-clicked {} at ({x}, {y})", button.as_str())
        }
        Action::Drag {
            x, y, to_x, to_y, ..
        } => format!("Dragged from ({x}, {y}) to ({to_x}, {to_y})"),
        Action::Scroll {
            direction,
            amount,
            x,
            y,
        } => format!(
            "Scrolled {} {amount} clicks at ({x}, {y})",
            direction.as_str(),
        ),
        // The count, never the text: the agent supplied it and does not need
        // it read back, and this string is the one that travels.
        Action::Type { chars } => format!("Typed {chars} characters"),
        Action::Key { keys } => format!("Pressed {keys}"),
    }
}

impl From<Feedback> for FeedbackDto {
    fn from(feedback: Feedback) -> Self {
        Self {
            action: prose(&feedback.action),
            screen_changed: feedback.screen_changed,
            reaction_time_ms: feedback.reaction_time_ms,
        }
    }
}

#[cfg(test)]
mod tests {
    use platform::input::{Button, ScrollDirection};

    use super::*;

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

    #[test]
    fn the_prose_the_agent_reads_is_unchanged() {
        let cases = [
            (Action::Move { x: 10, y: 20 }, "Moved cursor to (10, 20)"),
            (
                Action::Click {
                    button: Button::Left,
                    x: 612,
                    y: 335,
                },
                "Clicked left at (612, 335)",
            ),
            (
                Action::DoubleClick {
                    button: Button::Right,
                    x: 1,
                    y: 2,
                },
                "Double-clicked right at (1, 2)",
            ),
            (
                Action::Drag {
                    button: Button::Left,
                    x: 1,
                    y: 2,
                    to_x: 3,
                    to_y: 4,
                },
                "Dragged from (1, 2) to (3, 4)",
            ),
            (
                Action::Scroll {
                    direction: ScrollDirection::Down,
                    amount: 3,
                    x: 5,
                    y: 6,
                },
                "Scrolled down 3 clicks at (5, 6)",
            ),
            (Action::Type { chars: 15 }, "Typed 15 characters"),
            (
                Action::Key {
                    keys: "Return".into(),
                },
                "Pressed Return",
            ),
        ];
        for (action, expected) in cases {
            assert_eq!(prose(&action), expected);
        }
    }

    #[test]
    fn typed_text_never_reaches_the_wire() {
        let typed = "correct horse battery staple";
        let rendered = prose(&Action::Type {
            chars: typed.chars().count(),
        });
        assert_eq!(rendered, "Typed 28 characters");
        assert!(!rendered.contains("horse"));
    }
}
