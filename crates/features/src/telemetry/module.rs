use nest_rs::config::ConfigModule;
use nest_rs::core::module;

use super::config::TelemetryConfig;
use super::service::TelemetryService;

/// The instrumentation's single provider.
///
/// Safe for the MCP adapter to import precisely because it depends on nothing
/// but its own config: importing this module can never introduce a cycle.
#[module(
    imports = [ConfigModule::for_feature::<TelemetryConfig>()],
    providers = [TelemetryService],
)]
pub struct TelemetryModule;
