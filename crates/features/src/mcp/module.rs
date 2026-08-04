use nest_rs::config::ConfigModule;
use nest_rs::core::module;
use nest_rs::mcp::{McpOperationGuard, McpToolContext};

use super::context::GhostdeskToolContext;
use super::guard::{McpAuthGuard, McpSecurityPosture};
use super::server::GhostdeskMcp;
use crate::apps::AppsModule;
use crate::clipboard::ClipboardModule;
use crate::config::GhostdeskConfig;
use crate::input::InputModule;
use crate::screen::ScreenModule;
use crate::session::SessionModule;

/// The two `as dyn` bindings are what turn a deny-all endpoint into a
/// working one: without a `dyn McpOperationGuard` every request answers 401,
/// and without a `dyn McpToolContext` the model-space header and the idle
/// clock never reach a tool body.
#[module(
    imports = [
        ConfigModule::for_feature::<GhostdeskConfig>(),
        ScreenModule,
        InputModule,
        AppsModule,
        ClipboardModule,
        SessionModule,
    ],
    providers = [
        GhostdeskMcp,
        McpSecurityPosture,
        McpAuthGuard as dyn McpOperationGuard,
        GhostdeskToolContext as dyn McpToolContext,
    ],
)]
pub struct GhostdeskMcpModule;
