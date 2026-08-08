//! Programs domain — the catalogue of installed desktop applications,
//! launching one, and tracking what this session started.
//!
//! Named `programs` rather than `apps` because `apps` is the workspace's word
//! for a binary crate: a module that took it would make every path in the
//! tree ambiguous.

mod dtos;
mod error;
mod mcp;
mod module;
mod service;

pub use mcp::ProgramsMcpModule;
pub(crate) use module::ProgramsModule;
