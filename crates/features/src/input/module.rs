use nest_rs::core::module;

use super::feedback::FeedbackService;
use super::service::InputService;
use crate::host::HostModule;

#[module(
    imports = [HostModule],
    providers = [FeedbackService, InputService],
)]
pub struct InputModule;
