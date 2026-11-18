use nest_rs::core::module;
use nest_rs::guards::GlobalGuards;
use nest_rs::health::HealthModule;
use nest_rs::http::{HttpConfig, HttpModule};
use nest_rs::mcp::McpModule;

use features::auth::{AuthModule, AuthnGuard};
use features::clipboard::ClipboardMcpModule;
use features::idle::{IdleModule, IdleScheduleModule};
use features::input::InputMcpModule;
use features::programs::ProgramsMcpModule;
use features::screen::ScreenMcpModule;

use crate::interceptor::DesktopInterceptor;
use crate::mcp::identity;

/// The server is pinned to loopback and to its published port: the framework
/// binds every interface by default, and a desk reachable from the network by
/// default is a desktop handed to whoever finds the port. A deployment moves
/// either on the process — `GHOSTDESK_HTTP__HOST`, `GHOSTDESK_HTTP__PORT` — as
/// the container's entrypoint does. The body ceiling is raised for what a call
/// carries in: text typed or set on the clipboard.
#[module(
    imports = [
        HttpModule::for_root(HttpConfig {
            host: "127.0.0.1".into(),
            port: 3000,
            max_body_bytes: 8 * 1024 * 1024,
            ..Default::default()
        }),
        McpModule::for_root(identity()),
        HealthModule,
        AuthModule,
        ScreenMcpModule,
        InputMcpModule,
        ProgramsMcpModule,
        ClipboardMcpModule,
        IdleModule,
        IdleScheduleModule,
    ],
    providers = [GlobalGuards<(AuthnGuard,)>, DesktopInterceptor],
)]
pub struct GhostdeskModule;
