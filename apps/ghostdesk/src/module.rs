use std::time::Duration;

use nest_rs::config::ConfigModule;
use nest_rs::core::module;
use nest_rs::health::HealthModule;
use nest_rs::http::{HttpConfig, HttpModule};
use nest_rs::mcp::McpOptions;
use nest_rs::schedule::ScheduleModule;

use features::auth::AuthModule;
use features::clipboard::ClipboardMcpModule;
use features::idle::IdleScheduleModule;
use features::input::InputMcpModule;
use features::programs::ProgramsMcpModule;
use features::screen::ScreenMcpModule;

use crate::mcp::{McpModule, identity};

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
