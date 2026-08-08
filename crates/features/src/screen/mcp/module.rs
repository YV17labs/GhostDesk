use nest_rs::core::module;

use super::tool::ScreenTool;
use crate::screen::ScreenModule;
use crate::telemetry::TelemetryMcpModule;

#[module(
    imports = [ScreenModule, TelemetryMcpModule],
    providers = [ScreenTool],
)]
pub struct ScreenMcpModule;
