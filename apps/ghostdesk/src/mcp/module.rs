use nest_rs::core::module;
use nest_rs::mcp::McpToolContext;

use features::idle::IdleModule;
use features::telemetry::TelemetryModule;

use super::context::DesktopContext;

/// The per-call binding: glue over idle, telemetry and the coordinate
/// space, which no single feature spans.
#[module(
    imports = [IdleModule, TelemetryModule],
    providers = [DesktopContext as dyn McpToolContext],
)]
pub struct DesktopContextModule;
