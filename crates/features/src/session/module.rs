use nest_rs::config::ConfigModule;
use nest_rs::core::module;

use super::idle::IdleService;
use super::tasks::IdleTasks;
use crate::config::GhostdeskConfig;

/// Without `ScheduleModule` in the root module, `#[every("5s")]` compiles in
/// and never ticks. The app imports it; this module only owns the providers.
#[module(
    imports = [ConfigModule::for_feature::<GhostdeskConfig>()],
    providers = [IdleService, IdleTasks],
)]
pub struct SessionModule;
