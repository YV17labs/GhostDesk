//! Telemetry's MCP adapter — the measurement every tool host runs its
//! dispatch through.
//!
//! No `#[mcp]` host of its own: telemetry publishes no tool, it instruments
//! the ones other modules publish.

mod journal;
mod module;

pub use journal::CallJournal;
pub use module::TelemetryMcpModule;
