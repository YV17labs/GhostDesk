//! Input domain — mouse and keyboard control.

pub mod feedback;
mod module;
mod service;

pub use feedback::{Feedback, FeedbackService};
pub use module::InputModule;
pub use service::InputService;
