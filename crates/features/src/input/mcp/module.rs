use nest_rs::core::module;

use super::tool::InputTool;
use crate::input::InputModule;
use crate::telemetry::TelemetryMcpModule;

#[module(
    imports = [InputModule, TelemetryMcpModule],
    providers = [InputTool],
)]
pub struct InputMcpModule;
