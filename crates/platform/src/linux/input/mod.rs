//! The Linux [`InputBackend`](crate::input::InputBackend) — mouse and
//! keyboard through the wlroots virtual-input protocols.
//!
//! Three layers, one per file: [`backend`] answers the trait, [`connection`]
//! is the channel to the thread that owns the Wayland queue, and [`session`]
//! is the protocol machinery that thread drives. Text entry is
//! layout-independent — [`keymap`] builds GhostDesk's own XKB keymap out of
//! the [`keysym`]s a session has needed so far, so the compositor's own
//! layout never enters into it.

mod backend;
mod chord;
mod connection;
mod keymap;
mod keysym;
mod session;

pub use backend::Wayland;
