use nest_rs::core::module;
use nest_rs::mcp::McpToolContext;

use features::idle::IdleModule;

use super::context::DesktopContext;
use super::guard::CallTrail;

/// What the app owns about every call, which no single feature spans: the
/// per-call binding over idle and the coordinate space, and the line that
/// names the operation a caller asked for.
#[module(
    imports = [IdleModule],
    providers = [DesktopContext as dyn McpToolContext, CallTrail],
)]
pub struct McpModule;
