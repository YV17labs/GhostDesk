use nest_rs::config::ConfigModule;
use nest_rs::core::module;

use super::config::ScreenConfig;
use super::service::ScreenService;
use crate::host::HostModule;

#[module(
    imports = [ConfigModule::for_feature::<ScreenConfig>(), HostModule],
    providers = [ScreenService],
)]
pub struct ScreenModule;
