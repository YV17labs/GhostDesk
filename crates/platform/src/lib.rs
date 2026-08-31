//! The OS substrate GhostDesk drives, behind five OS-neutral contracts:
//! [`input::InputBackend`], [`screen::ScreenBackend`],
//! [`window::WindowManager`], [`clipboard::Clipboard`] and
//! [`desktop::AppCatalog`]. [`host`] picks the implementation for the compile
//! target — Wayland/Sway/grim/wl-clipboard/XDG on Linux, Quartz/
//! Accessibility/`screencapture`/pasteboard on macOS, `SendInput`/GDI/
//! `EnumWindows`/the Start Menu on Windows — and a port to a fourth OS is a
//! new sibling module, since nothing outside this crate names an OS.
//!
//! `host` also answers the two questions about a *launched child* that no
//! standard library call answers portably — is its pid still alive, and how is
//! it kept out of the server's own signal group. They are not desktop seams,
//! but they are per-OS, so each backend answers them in its own `process`
//! module rather than a `cfg` reaching into the feature crate.
//!
//! `unsupported` is the stand-in that keeps every other target compiling, so a
//! backend type leaking into the neutral layer breaks the build immediately
//! rather than the day someone tries the port.
//!
//! Four more modules are neutral too, and are not contracts: [`chord`] is the
//! chord grammar every backend serves, [`frame`] the pixel work on what a
//! capture returns, [`cmd`] the runner for the helper binaries a backend
//! shells out to, and [`coords`] the model-space conversion. Each exists
//! because every backend would otherwise write its own copy and drift.
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
#[cfg(target_os = "windows")]
mod windows;

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
mod unsupported;
