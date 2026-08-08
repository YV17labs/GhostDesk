use nest_rs::core::module;

use super::tasks::IdleTasks;
use crate::idle::IdleModule;

/// Without `ScheduleModule` in the root module, `#[every("5s")]` compiles in
/// and never ticks. The app imports it; this module only owns the provider.
#[module(
    imports = [IdleModule],
    providers = [IdleTasks],
)]
pub struct IdleScheduleModule;
