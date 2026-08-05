//! The composition root: which modules the binary imports, and why.
//!
//! Each transport attaches itself at boot from the import list below — there
//! is no transport constructed here, and no logic. The security posture the
//! MCP endpoint runs under is owned by `features::mcp::guard`, which is the
//! one copy of that doctrine.

use nest_rs::config::ConfigModule;
use nest_rs::core::module;
use nest_rs::http::{HttpConfig, HttpModule};
use nest_rs::mcp::McpModule;
use nest_rs::schedule::ScheduleModule;

use features::mcp::GhostdeskMcpModule;
use features::session::SessionModule;

/// Loopback by default, per the MCP transports spec. The container
/// entrypoint exports `GHOSTDESK_HTTP__HOST=0.0.0.0` so the endpoint is
/// reachable outside the container; a pin is a base, and the real
/// environment overlays it field by field.
#[module(imports = [
    ConfigModule::for_root(),
    HttpModule::for_root(HttpConfig {
        host: "127.0.0.1".into(),
        port: 3000,
        // grim hands back full-screen PNGs and `key_type` can carry a whole
        // spreadsheet, so 2 MiB is too tight for this endpoint's traffic.
        max_body_bytes: Some(8 * 1024 * 1024),
        // A screenshot that waits for the UI to stabilise, then re-encodes,
        // can outlive the framework's 30s default on a loaded desktop.
        request_timeout_secs: Some(120),
        ..Default::default()
    }),
    McpModule::for_root(None),
    ScheduleModule,
    GhostdeskMcpModule,
    SessionModule,
])]
pub struct GhostdeskModule;
