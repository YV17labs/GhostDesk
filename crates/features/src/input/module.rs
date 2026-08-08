use nest_rs::core::module;

use super::services::{FeedbackService, InputService};
use crate::host::HostModule;

#[module(
    imports = [HostModule],
    providers = [FeedbackService, InputService],
)]
pub struct InputModule;
