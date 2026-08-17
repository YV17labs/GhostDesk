use nest_rs::core::module;
use nest_rs::mcp::McpToolContext;

use features::idle::IdleModule;
use features::telemetry::TelemetryModule;

use super::context::DesktopContext;

/// The per-call binding's own module, so the composition root stays pure
/// imports. It imports the two ports [`DesktopContext`] injects: the access
/// graph demands that whoever provides a consumer can reach its dependencies.
#[module(
    imports = [IdleModule, TelemetryModule],
    providers = [DesktopContext as dyn McpToolContext],
)]
pub struct DesktopContextModule;
