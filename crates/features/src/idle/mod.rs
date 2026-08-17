//! Idle domain — the watchdog that keeps a long-lived agent from
//! accumulating orphaned windows.
//!
//! Named for what it watches, not for what it watches over: "session" is
//! [`telemetry`](crate::telemetry)'s word, where it means one agent's
//! conversation with the endpoint. The two have nothing to do with each
//! other, and sharing the name made both harder to read.

mod config;
mod module;
mod schedule;
mod service;

/// Exported with [`IdleService`]: the app's per-call MCP context injects the
/// service, and whoever provides a consumer must import the module that
/// provides its dependencies.
pub use module::IdleModule;
pub use schedule::IdleScheduleModule;
/// Reset by the app's per-call MCP context on every operation — any traffic
/// at all counts as someone at the desk.
pub use service::IdleService;
