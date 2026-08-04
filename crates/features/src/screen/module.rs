use nest_rs::config::ConfigModule;
use nest_rs::core::module;

use super::service::ScreenService;
use crate::config::GhostdeskConfig;

#[module(
    imports = [ConfigModule::for_feature::<GhostdeskConfig>()],
    providers = [ScreenService],
)]
pub struct ScreenModule;
