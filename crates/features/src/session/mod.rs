//! Session domain — the idle watchdog that keeps a long-lived agent from
//! accumulating orphaned windows.

mod idle;
mod module;
mod tasks;

pub use idle::IdleService;
pub use module::SessionModule;
pub use tasks::IdleTasks;
