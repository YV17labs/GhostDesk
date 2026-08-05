use nest_rs::config::ConfigModule;
use nest_rs::core::module;
use nest_rs::http::{HttpConfig, HttpModule};
use nest_rs::mcp::McpModule;
use nest_rs::schedule::ScheduleModule;

use features::mcp::GhostdeskMcpModule;
use features::session::SessionModule;

#[module(imports = [
    ConfigModule::for_root(),
    HttpModule::for_root(HttpConfig {
        host: "127.0.0.1".into(),
        port: 3000,
        max_body_bytes: Some(8 * 1024 * 1024),
        request_timeout_secs: Some(120),
        ..Default::default()
    }),
    McpModule::for_root(None),
    ScheduleModule,
    GhostdeskMcpModule,
    SessionModule,
])]
pub struct GhostdeskModule;
