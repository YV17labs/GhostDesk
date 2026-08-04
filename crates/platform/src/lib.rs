//! The OS substrate GhostDesk drives: the Wayland compositor, Sway's IPC
//! socket, `grim`, `wl-copy`/`wl-paste`, and `/usr/share/applications`.
//!
//! Nothing here knows about MCP or NestRS. The feature crate wraps each of
//! these in an `#[injectable]` service; this crate is what those services
//! call. Keeping the split means the desktop-driving code is testable on its
//! own and the framework never leaks into a Wayland event loop.

pub mod cmd;
pub mod coords;
pub mod desktop;
pub mod screen;
pub mod sway;
pub mod wayland;
