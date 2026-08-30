use nest_rs::core::module;
use nest_rs::mcp::McpToolContext;

use features::idle::IdleModule;

use super::context::DesktopContext;
use super::guard::CallTrail;

#[module(
    imports = [IdleModule],
    providers = [DesktopContext as dyn McpToolContext, CallTrail],
)]
pub struct McpModule;
