//! Screen domain — capture and inspect the display.

mod config;
mod module;
mod service;

pub use config::ScreenConfig;
pub use module::ScreenModule;
pub use service::{Capture, ScreenService};
