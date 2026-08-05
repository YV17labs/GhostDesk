//! The MCP adapter — one endpoint, every GhostDesk capability.

mod config;
mod context;
mod dto;
mod guard;
mod icons;
mod instructions;
mod module;
mod server;

pub use config::AuthConfig;
pub use context::GhostdeskToolContext;
pub use guard::{McpAuthGuard, McpSecurityPosture};
pub use instructions::instructions;
pub use module::GhostdeskMcpModule;
pub use server::GhostdeskMcp;
