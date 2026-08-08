use nest_rs::core::module;
use nest_rs::mcp::McpOperationGuard;

use super::guard::AuthGuard;
use crate::auth::AuthModule;

/// The `as dyn` binding is what turns a deny-all endpoint into a working one:
/// without a `dyn McpOperationGuard` every request answers 401.
///
/// One binding for the whole path, not one per tool host — the guard runs on
/// the poem endpoint, before rmcp sees the operation at all.
#[module(
    imports = [AuthModule],
    providers = [AuthGuard as dyn McpOperationGuard],
)]
pub struct AuthMcpModule;
