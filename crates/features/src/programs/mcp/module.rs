use nest_rs::core::module;

use super::tool::ProgramsTool;
use crate::programs::ProgramsModule;
use crate::screen::ScreenModule;
use crate::telemetry::TelemetryMcpModule;

#[module(
    imports = [ProgramsModule, ScreenModule, TelemetryMcpModule],
    providers = [ProgramsTool],
)]
pub struct ProgramsMcpModule;
