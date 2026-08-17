use nest_rs::core::module;
use nest_rs::mcp::McpOperationGuard;

use super::guard::AuthGuard;
use crate::auth::AuthModule;

#[module(
    imports = [AuthModule],
    providers = [AuthGuard as dyn McpOperationGuard],
)]
pub struct AuthMcpModule;
