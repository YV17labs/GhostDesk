use nest_rs::config::ConfigModule;
use nest_rs::core::module;

use super::config::IdleConfig;
use super::service::IdleService;
use crate::host::HostModule;

#[module(
    imports = [ConfigModule::for_feature::<IdleConfig>(), HostModule],
    providers = [IdleService],
)]
pub struct IdleModule;
