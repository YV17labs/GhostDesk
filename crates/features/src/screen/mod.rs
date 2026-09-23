mod capture;
mod config;
mod dtos;
mod error;
mod mcp;
mod module;
mod service;

// `RegionDto` crosses to `input`, and the crossing is the point: a region is
// one statement made twice — once to say what to capture, once to say what a
// coordinate was read against — and the two spellings have to be the same
// spelling. A second copy in `input` would compile, serialise identically,
// and drift the first time either side gains a field.
pub(crate) use dtos::RegionDto;
pub use mcp::ScreenMcpModule;
pub(crate) use module::ScreenModule;
