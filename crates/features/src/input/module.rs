use nest_rs::core::module;

use super::feedback::FeedbackService;
use super::service::InputService;

#[module(providers = [FeedbackService, InputService])]
pub struct InputModule;
