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

/// The app's half of the endpoint's identity — the half no feature could
/// know: the deployment's version, and a session brief describing a surface
/// no single host can see.
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
        // The probes stay reachable where the desk is gated: a probe an
        // orchestrator cannot read is a probe that reports nothing, and the
        // body carries indicator names and up/down with every reason kept to
        // the log.
        HealthModule,
        AuthModule,
        ScreenMcpModule,
        InputMcpModule,
        ProgramsMcpModule,
        ClipboardMcpModule,
        IdleScheduleModule,
        // Ours, not the framework's — the two share a name, which is why the
        // framework's is written in full above.
        McpModule,
    ],
)]
pub struct GhostdeskModule;

impl GhostdeskModule {
    /// The chain every request crosses, declared here rather than at the
    /// binary so the suite can assert the wiring the deployment actually
    /// ships — a test that redeclares its own list proves only itself.
    ///
    /// Global rather than per host: `/mcp` is one request carrying many
    /// operations, so admission is decided once, where the request still
    /// exists. `CallTrail` rides alongside to name the operation inside it.
    pub fn guards() -> [GuardSpec; 2] {
        [guard::<AuthnGuard>(), guard::<CallTrail>()]
    }
}
