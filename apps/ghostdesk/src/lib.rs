//! The GhostDesk binary's composition root. Everything it does is wire
//! modules — no transport is constructed here, each one attaches itself at
//! boot from the import below.

mod module;

pub use module::GhostdeskModule;
