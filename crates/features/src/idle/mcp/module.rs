use nest_rs::core::module;
use nest_rs::mcp::McpModule;

use super::interceptor::IdleInterceptor;
use crate::idle::IdleModule;

#[module(imports = [IdleModule, McpModule], providers = [IdleInterceptor])]
pub struct IdleMcpModule;
