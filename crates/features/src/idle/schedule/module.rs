use nest_rs::core::module;

use super::tasks::IdleTasks;
use crate::idle::IdleModule;

#[module(
    imports = [IdleModule],
    providers = [IdleTasks],
)]
pub struct IdleScheduleModule;
