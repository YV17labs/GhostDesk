//! Screen domain — capture and inspect the display.

mod config;
mod dtos;
mod error;
mod mcp;
mod module;
mod service;

/// `app_launch` returns a settled frame, so the programs adapter composes
/// this domain's capture into its own result.
pub(crate) use dtos::CaptureDto;
pub use mcp::ScreenMcpModule;
pub(crate) use module::ScreenModule;
pub(crate) use service::ScreenService;
