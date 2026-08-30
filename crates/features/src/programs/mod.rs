mod dtos;
mod error;
mod launched;
mod mcp;
mod module;
mod program_status;
mod registry;
mod running_window;
mod service;
mod window_wait;

pub use mcp::ProgramsMcpModule;
pub(crate) use module::ProgramsModule;
