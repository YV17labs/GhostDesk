//! Input domain — mouse and keyboard control.

pub mod feedback;
pub mod keys;
mod module;
mod service;

pub use feedback::{Feedback, FeedbackService};
pub use module::InputModule;
pub use service::InputService;
