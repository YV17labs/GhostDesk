mod capture;
mod config;
mod dtos;
mod error;
mod mcp;
mod module;
mod service;

// `CaptureDto`, `ScreenService` and `ScreenModule` cross to one consumer and
// one only: `programs`' MCP adapter, whose `app_launch` answers with the
// settled frame so the agent needs no follow-up `screen_shot()`. Deliberate,
// and narrow on purpose — see AGENTS.md, *Known deviations*.
pub(crate) use dtos::CaptureDto;
pub use mcp::ScreenMcpModule;
pub(crate) use module::ScreenModule;
pub(crate) use service::ScreenService;
