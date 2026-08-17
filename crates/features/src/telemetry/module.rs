use nest_rs::config::ConfigModule;
use nest_rs::core::module;

use super::config::TelemetryConfig;
use super::service::TelemetryService;

#[module(
    imports = [ConfigModule::for_feature::<TelemetryConfig>()],
    providers = [TelemetryService],
)]
pub struct TelemetryModule;
