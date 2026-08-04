//! The MCP adapter — one endpoint, every GhostDesk capability.

mod context;
mod dto;
mod guard;
mod icons;
mod instructions;
mod module;
mod server;

pub use context::GhostdeskToolContext;
pub use guard::{McpAuthGuard, McpSecurityPosture};
pub use instructions::INSTRUCTIONS;
pub use module::GhostdeskMcpModule;
pub use server::GhostdeskMcp;
