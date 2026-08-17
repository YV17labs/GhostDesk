use nest_rs::config::ConfigModule;
use nest_rs::core::module;
use nest_rs::http::{HttpConfig, HttpModule};
use nest_rs::mcp::{McpIdentity, McpModule, McpOptions, McpToolContext};
use nest_rs::schedule::ScheduleModule;

use features::auth::AuthMcpModule;
use features::clipboard::ClipboardMcpModule;
use features::idle::IdleScheduleModule;
use features::input::InputMcpModule;
use features::programs::ProgramsMcpModule;
use features::screen::ScreenMcpModule;

use crate::mcp::{DesktopContext, icons, instructions};

/// The app's half of the endpoint's identity — the half no feature could
/// know: the deployment's version, and a session brief describing a surface
/// no single host can see. It carries no path, and neither does any host, so
/// the endpoint is wherever the framework's default puts it.
fn identity() -> McpIdentity {
    McpIdentity::new("ghostdesk", env!("CARGO_PKG_VERSION"))
        .title("GhostDesk")
        .description("MCP server to control a virtual desktop")
        .icons(icons())
        .instructions(instructions(&platform::host::conventions()))
}

#[module(
    imports = [
        ConfigModule::for_root(),
        HttpModule::for_root(HttpConfig {
            host: "127.0.0.1".into(),
            port: 3000,
            max_body_bytes: Some(8 * 1024 * 1024),
            request_timeout_secs: Some(120),
            ..Default::default()
        }),
        McpModule::for_root(McpOptions {
            server: Some(identity()),
            ..Default::default()
        }),
        ScheduleModule,
        AuthMcpModule,
        ScreenMcpModule,
        InputMcpModule,
        ProgramsMcpModule,
        ClipboardMcpModule,
        IdleScheduleModule,
    ],
    providers = [DesktopContext as dyn McpToolContext],
)]
pub struct GhostdeskModule;
