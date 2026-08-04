//! Auth ≡ TLS, decided at boot by whether an operator mounted a cert+key.
//!
//! * **Cert mounted** (`NESTRS_HTTP__TLS_CERT_FILE` + `..._KEY_FILE`, which
//!   `docker/init/entrypoint.sh` points at `/etc/ghostdesk/tls/server.{crt,key}`
//!   when those files exist) — the transport serves `https://` through rustls
//!   and [`BearerMcpGuard`] rejects any operation missing
//!   `Authorization: Bearer $GHOSTDESK_AUTH_TOKEN`. The token is mandatory in
//!   this posture; the guard fails boot without it.
//!
//! * **No cert** — plain `http://` and [`OpenMcpGuard`] lets every operation
//!   through. Shipping a static bearer token over cleartext would be security
//!   theatre (no rotation, no per-user identity), so the surface is left open
//!   and the operator is expected to mount a cert or keep the port on a
//!   trusted loopback. See SECURITY.md § Authentication.
//!
//! Both guards are real `dyn McpOperationGuard` bindings: a bare `#[mcp]`
//! endpoint is deny-all, so the open posture has to *say* it is open. Which
//! one is active is in the boot log.

use nest_rs::config::ConfigModule;
use nest_rs::core::module;
use nest_rs::http::{HttpConfig, HttpModule};
use nest_rs::mcp::McpModule;
use nest_rs::schedule::ScheduleModule;

use features::mcp::GhostdeskMcpModule;
use features::session::SessionModule;

/// Loopback by default, per the MCP transports spec. The container
/// entrypoint exports `NESTRS_HTTP__HOST=0.0.0.0` so the endpoint is
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
