//! Apps domain — application launching and process tracking.

mod module;
mod service;

pub use module::AppsModule;
pub use service::{
    AppStatus, AppsService, DEFAULT_TAIL, Launched, RunningApp, WINDOW_WAIT_TIMEOUT, WindowWait,
};
