use nest_rs::core::module;

use super::tool::ProgramsTool;
use crate::programs::ProgramsModule;
use crate::screen::ScreenModule;
use crate::telemetry::TelemetryMcpModule;

/// One of several hosts on `/mcp`. It never learns that it shares the path —
/// the endpoint merges the tool tables, and a name claimed twice fails boot
/// naming both hosts.
#[module(
    imports = [ProgramsModule, ScreenModule, TelemetryMcpModule],
    providers = [ProgramsTool],
)]
pub struct ProgramsMcpModule;
