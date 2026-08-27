mod config;
mod dtos;
mod error;
mod mcp;
mod module;
mod service;

pub(crate) use dtos::CaptureDto;
pub use mcp::ScreenMcpModule;
pub(crate) use module::ScreenModule;
pub(crate) use service::ScreenService;
