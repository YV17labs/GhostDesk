//! The OS substrate GhostDesk drives, behind five OS-neutral contracts:
//! [`input::InputBackend`], [`screen::ScreenBackend`],
//! [`window::WindowManager`], [`clipboard::Clipboard`] and
//! [`desktop::AppCatalog`]. [`host`] picks the implementation for the compile
//! target — Wayland/Sway/grim/wl-clipboard/XDG under [`linux`], Quartz/
//! Accessibility/`screencapture`/pasteboard under [`macos`] — and a port to a
//! third OS is a new sibling module, since nothing outside this crate names
//! an OS.
//!
//! `unsupported` is the stand-in that keeps every other target compiling, so a
//! backend type leaking into the neutral layer breaks the build immediately
//! rather than the day someone tries the port.
//!
//! Four more modules are neutral too, and are not contracts: [`chord`] is the
//! chord grammar both backends serve, [`frame`] the pixel work on what a
//! capture returns, [`cmd`] the runner for the helper binaries a backend
//! shells out to, and [`coords`] the model-space conversion. Each exists
//! because two backends would otherwise write it twice and drift.
//!
//! Nothing here knows about MCP or NestRS. The feature crate wraps each seam
//! in an `#[injectable]` service; this crate is what those services call.
//! Keeping the split means the desktop-driving code is testable on its own
//! and the framework never leaks into a Wayland event loop.

pub mod chord;
pub mod clipboard;
pub mod cmd;
pub mod coords;
pub mod desktop;
pub mod frame;
pub mod host;
pub mod input;
pub mod screen;
pub mod window;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(not(any(target_os = "linux", target_os = "macos")))]
mod unsupported;
