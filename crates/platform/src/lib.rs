//! The OS substrate GhostDesk drives, behind five OS-neutral contracts:
//! [`input::InputBackend`], [`screen::ScreenBackend`],
//! [`window::WindowManager`], [`clipboard::Clipboard`] and
//! [`desktop::AppCatalog`]. [`host`] picks the implementation for the compile
//! target — Wayland/Sway/grim/wl-clipboard/XDG under [`linux`], Quartz/
//! Accessibility/NSPasteboard/app bundles under [`macos`] — and a port to a
//! third OS is a new sibling module, since nothing outside this crate names
//! an OS.
//!
//! Nothing here knows about MCP or NestRS. The feature crate wraps each seam
//! in an `#[injectable]` service; this crate is what those services call.
//! Keeping the split means the desktop-driving code is testable on its own
//! and the framework never leaks into a Wayland event loop.

pub mod clipboard;
pub mod cmd;
pub mod coords;
pub mod desktop;
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
