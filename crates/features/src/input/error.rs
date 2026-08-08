//! What an input action can go wrong with.
//!
//! The distinction the enum buys is real: an unresolvable chord is the
//! caller's mistake and is worth retrying with a different spelling, while a
//! backend that will not press anything is not. The two used to be told apart
//! by *which tool* had failed rather than by what failed — every error out of
//! `key_press` was reported as a bad key name, including a dead compositor.

/// A failure from the input domain.
#[derive(Debug, thiserror::Error)]
pub enum InputError {
    /// The backend does not know a key in the chord the caller sent.
    #[error("{0}")]
    Chord(#[source] anyhow::Error),

    /// The pointer or keyboard could not be driven.
    #[error("the input backend refused the action")]
    Backend(#[source] anyhow::Error),

    /// The before/after screens the verdict is measured from could not be
    /// taken. The action itself may well have landed.
    #[error("the action was performed but its effect could not be observed")]
    Feedback(#[source] anyhow::Error),
}
