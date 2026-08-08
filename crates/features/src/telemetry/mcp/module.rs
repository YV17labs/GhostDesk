use nest_rs::core::module;

use super::journal::CallJournal;
use crate::telemetry::TelemetryModule;

/// Imported by every module that hosts MCP tools, and by nothing else.
#[module(
    imports = [TelemetryModule],
    providers = [CallJournal],
)]
pub struct TelemetryMcpModule;
