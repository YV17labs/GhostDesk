use nest_rs::config::ConfigModule;
use nest_rs::core::module;

use super::config::SessionConfig;
use super::idle::IdleService;
use super::tasks::IdleTasks;
use crate::host::HostModule;

/// Without `ScheduleModule` in the root module, `#[every("5s")]` compiles in
/// and never ticks. The app imports it; this module only owns the providers.
#[module(
    imports = [ConfigModule::for_feature::<SessionConfig>(), HostModule],
    providers = [IdleService, IdleTasks],
)]
pub struct SessionModule;
