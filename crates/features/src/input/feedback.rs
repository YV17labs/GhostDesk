use super::action::Action;

/// What one act did, and what the screen did about it.
///
/// The verdict is advisory and deliberately weak: `screen_changed` false means
/// nothing visibly moved within the watch, which is an ordinary outcome for a
/// hover with no effect and a useful signal for a click that missed. The act
/// itself is recorded whether or not this is ever produced.
#[derive(Debug, Clone)]
pub struct Feedback {
    pub action: Action,
    pub screen_changed: bool,
    pub reaction_time_ms: u64,
}
