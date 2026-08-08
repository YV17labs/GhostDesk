//! Split because [`FeedbackService`] is the expensive half — two captures and
//! a poll per action — and is worth reading, and testing, without
//! [`InputService`]'s seven near-identical methods in the way.

mod feedback;
mod input;

pub use feedback::{Feedback, FeedbackService};
pub use input::InputService;
