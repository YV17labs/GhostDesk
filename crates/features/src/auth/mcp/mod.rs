//! The auth domain's MCP adapter — the bearer check the transport runs
//! before it dispatches anything.

mod guard;
mod module;

pub use module::AuthMcpModule;
