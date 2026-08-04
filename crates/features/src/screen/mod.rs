//! Screen domain — capture and inspect the display.

mod module;
mod service;

pub use module::ScreenModule;
pub use service::{Capture, ScreenService};
