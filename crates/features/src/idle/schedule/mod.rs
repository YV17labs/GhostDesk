//! The idle domain's scheduler adapter — the clock is the only caller.

mod module;
mod tasks;

pub use module::IdleScheduleModule;
