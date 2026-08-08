//! Input domain — mouse and keyboard control.

mod dtos;
mod error;
mod mcp;
mod module;
mod services;

pub use mcp::InputMcpModule;
pub(crate) use module::InputModule;
