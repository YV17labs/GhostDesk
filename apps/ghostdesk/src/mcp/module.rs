use nest_rs::core::module;
use nest_rs::mcp::McpToolContext;

use features::idle::IdleModule;

use super::context::DesktopContext;
use super::guard::CallTrailGuard;

#[module(
    imports = [IdleModule],
    providers = [DesktopContext as dyn McpToolContext, CallTrailGuard],
)]
pub struct McpModule;
