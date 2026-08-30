use std::time::Duration;

use nest_rs::config::ConfigModule;
use nest_rs::core::module;
use nest_rs::guards::{GuardSpec, guard};
use nest_rs::health::HealthModule;
use nest_rs::http::{HttpConfig, HttpModule};
use nest_rs::mcp::{McpIdentity, McpOptions};
use nest_rs::schedule::ScheduleModule;

use features::auth::{AuthModule, AuthnGuard};
use features::clipboard::ClipboardMcpModule;
use features::idle::IdleScheduleModule;
use features::input::InputMcpModule;
use features::programs::ProgramsMcpModule;
use features::screen::ScreenMcpModule;

use crate::mcp::{CallTrail, McpModule, icons, instructions};

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
            request_timeout: Some(Duration::from_secs(120)),
            ..Default::default()
        }),
        nest_rs::mcp::McpModule::for_root(McpOptions {
            server: Some(identity()),
            ..Default::default()
        }),
        ScheduleModule,
        HealthModule,
        AuthModule,
        ScreenMcpModule,
        InputMcpModule,
        ProgramsMcpModule,
        ClipboardMcpModule,
        IdleScheduleModule,
        McpModule,
    ],
)]
pub struct GhostdeskModule;

impl GhostdeskModule {
    pub fn guards() -> [GuardSpec; 2] {
        [guard::<AuthnGuard>(), guard::<CallTrail>()]
    }
}
