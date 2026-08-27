use std::fmt;

use platform::input::{Button, ScrollDirection};

#[derive(Debug, Clone)]
pub enum Action {
    Move {
        x: i64,
        y: i64,
    },
    Click {
        button: Button,
        x: i64,
        y: i64,
    },
    DoubleClick {
        button: Button,
        x: i64,
        y: i64,
    },
    Drag {
        button: Button,
        from: (i64, i64),
        to: (i64, i64),
    },
    Scroll {
        direction: ScrollDirection,
        amount: u32,
        x: i64,
        y: i64,
    },
    Type {
        chars: usize,
    },
    Key {
        keys: String,
    },
}

impl Action {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Move { .. } => "move",
            Self::Click { .. } => "click",
            Self::DoubleClick { .. } => "double_click",
            Self::Drag { .. } => "drag",
            Self::Scroll { .. } => "scroll",
            Self::Type { .. } => "type",
            Self::Key { .. } => "key",
        }
    }

    pub fn button(&self) -> Option<&'static str> {
        match self {
            Self::Click { button, .. }
            | Self::DoubleClick { button, .. }
            | Self::Drag { button, .. } => Some(button.as_str()),
            _ => None,
        }
    }

    pub fn direction(&self) -> Option<&'static str> {
        match self {
            Self::Scroll { direction, .. } => Some(direction.as_str()),
            _ => None,
        }
    }

    pub fn amount(&self) -> Option<u32> {
        match self {
            Self::Scroll { amount, .. } => Some(*amount),
            _ => None,
        }
    }

    pub fn x(&self) -> Option<i64> {
        match self {
            Self::Move { x, .. }
            | Self::Click { x, .. }
            | Self::DoubleClick { x, .. }
            | Self::Scroll { x, .. } => Some(*x),
            Self::Drag { from, .. } => Some(from.0),
            _ => None,
        }
    }

    pub fn y(&self) -> Option<i64> {
        match self {
            Self::Move { y, .. }
            | Self::Click { y, .. }
            | Self::DoubleClick { y, .. }
            | Self::Scroll { y, .. } => Some(*y),
            Self::Drag { from, .. } => Some(from.1),
            _ => None,
        }
    }

    pub fn to_x(&self) -> Option<i64> {
        match self {
            Self::Drag { to, .. } => Some(to.0),
            _ => None,
        }
    }

    pub fn to_y(&self) -> Option<i64> {
        match self {
            Self::Drag { to, .. } => Some(to.1),
            _ => None,
        }
    }

    pub fn chars(&self) -> Option<usize> {
        match self {
            Self::Type { chars } => Some(*chars),
            _ => None,
        }
    }

    pub fn keys(&self) -> Option<&str> {
        match self {
            Self::Key { keys } => Some(keys),
            _ => None,
        }
    }
}

impl fmt::Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Move { x, y } => write!(f, "Moved cursor to ({x}, {y})"),
            Self::Click { button, x, y } => {
                write!(f, "Clicked {} at ({x}, {y})", button.as_str())
            }
            Self::DoubleClick { button, x, y } => {
                write!(f, "Double-clicked {} at ({x}, {y})", button.as_str())
            }
            Self::Drag { from, to, .. } => write!(
                f,
                "Dragged from ({}, {}) to ({}, {})",
                from.0, from.1, to.0, to.1
            ),
            Self::Scroll {
                direction,
                amount,
                x,
                y,
            } => write!(
                f,
                "Scrolled {} {amount} clicks at ({x}, {y})",
                direction.as_str()
            ),
            Self::Type { chars } => write!(f, "Typed {chars} characters"),
            Self::Key { keys } => write!(f, "Pressed {keys}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
                Action::Drag {
                    button: Button::Left,
                    from: (1, 2),
                    to: (3, 4),
                },
                "Dragged from (1, 2) to (3, 4)",
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
            assert_eq!(action.to_string(), expected);
        }
    }

    #[test]
    fn typed_text_is_counted_never_carried() {
        let action = Action::Type { chars: 9 };
        assert_eq!(action.chars(), Some(9));
        assert_eq!(action.keys(), None);
        assert!(!format!("{action:?}").contains("password"));
    }

    #[test]
    fn a_drag_reports_both_ends() {
        let action = Action::Drag {
            button: Button::Left,
            from: (1, 2),
            to: (3, 4),
        };
        assert_eq!((action.x(), action.y()), (Some(1), Some(2)));
        assert_eq!((action.to_x(), action.to_y()), (Some(3), Some(4)));
    }
}
