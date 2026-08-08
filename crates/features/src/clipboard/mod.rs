//! Clipboard domain — read and write the system clipboard.

mod dtos;
mod error;
mod mcp;
mod module;
mod service;

pub use mcp::ClipboardMcpModule;
pub(crate) use module::ClipboardModule;
