use platform::input::{Button, ScrollDirection};

/// One act performed on the desktop, in the fields the audit trail asks for.
///
/// The accessors below all answer `Option`, and `tracing` records nothing for
/// a `None` — which is how a field that does not apply to an act stays absent
/// rather than empty. `x` on a keypress would be a coordinate the act never
/// had, and a trail queried for "every click below this line" cannot afford
/// one.
///
/// The prose an agent reads is deliberately not here: it belongs to the wire
/// type, phrased from these same values so the two cannot drift.
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
        x: i64,
        y: i64,
        to_x: i64,
        to_y: i64,
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
            | Self::Scroll { x, .. }
            | Self::Drag { x, .. } => Some(*x),
            _ => None,
        }
    }

    pub fn y(&self) -> Option<i64> {
        match self {
            Self::Move { y, .. }
            | Self::Click { y, .. }
            | Self::DoubleClick { y, .. }
            | Self::Scroll { y, .. }
            | Self::Drag { y, .. } => Some(*y),
            _ => None,
        }
    }

    pub fn to_x(&self) -> Option<i64> {
        match self {
            Self::Drag { to_x, .. } => Some(*to_x),
            _ => None,
        }
    }

    pub fn to_y(&self) -> Option<i64> {
        match self {
            Self::Drag { to_y, .. } => Some(*to_y),
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

#[cfg(test)]
mod tests {
    use super::*;

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
            x: 1,
            y: 2,
            to_x: 3,
            to_y: 4,
        };
        assert_eq!((action.x(), action.y()), (Some(1), Some(2)));
        assert_eq!((action.to_x(), action.to_y()), (Some(3), Some(4)));
    }
}
