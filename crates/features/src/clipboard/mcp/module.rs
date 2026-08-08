use nest_rs::core::module;

use super::tool::ClipboardTool;
use crate::clipboard::ClipboardModule;
use crate::telemetry::TelemetryMcpModule;

#[module(
    imports = [ClipboardModule, TelemetryMcpModule],
    providers = [ClipboardTool],
)]
pub struct ClipboardMcpModule;
