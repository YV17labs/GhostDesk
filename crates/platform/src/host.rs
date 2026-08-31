//! The one place an OS is chosen.
//!
//! Five selectors answer `Arc<dyn Trait>`, one per seam, picked by `target_os`
//! at compile time; [`conventions`] answers a plain value for the sixth thing
//! a port has to state, and [`process_is_running`] and [`process_detach`] the
//! two things the OS decides about a child the server launched. Porting
//! GhostDesk to another OS is a new sibling of `linux`/`macos`/`windows`
//! implementing the five contracts, plus one arm in each selector below — no
//! other file in the workspace names an operating system.

use std::sync::Arc;

use tokio::process::Command;

use crate::clipboard::Clipboard;
use crate::desktop::AppCatalog;
use crate::input::{Conventions, InputBackend};
use crate::screen::ScreenBackend;
use crate::window::WindowManager;

/// The mouse-and-keyboard backend for this OS.
pub fn input() -> Arc<dyn InputBackend> {
    #[cfg(target_os = "linux")]
    return Arc::new(crate::linux::input::Wayland::default());
    #[cfg(target_os = "macos")]
    return Arc::new(crate::macos::input::Quartz);
    #[cfg(target_os = "windows")]
    return Arc::new(crate::windows::input::Win32);
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    return Arc::new(crate::unsupported::Unsupported);
}

/// How this OS's desktop spells its standard shortcuts.
///
/// The same const the backend's chord table is built against, so the value a
/// caller publishes and the value the keyboard actually presses can never be
/// two different things. A free function rather than a trait method: the brief
/// is rendered at composition time, and instantiating a second seam to read a
/// compile-target constant would put two backends in a process the `Chord`
/// contract says has one — which is why the method the trait used to carry had
/// no caller at all.
pub fn conventions() -> Conventions {
    #[cfg(target_os = "linux")]
    return crate::linux::input::CONVENTIONS;
    #[cfg(target_os = "macos")]
    return crate::macos::input::CONVENTIONS;
    #[cfg(target_os = "windows")]
    return crate::windows::input::CONVENTIONS;
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    return crate::unsupported::CONVENTIONS;
}

/// The screen-capture backend for this OS.
pub fn screen() -> Arc<dyn ScreenBackend> {
    #[cfg(target_os = "linux")]
    return Arc::new(crate::linux::screen::GrimScreen);
    #[cfg(target_os = "macos")]
    return Arc::new(crate::macos::screen::ScreenCapture);
    #[cfg(target_os = "windows")]
    return Arc::new(crate::windows::screen::GdiScreen);
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    return Arc::new(crate::unsupported::Unsupported);
}

/// The window manager for this OS.
pub fn windows() -> Arc<dyn WindowManager> {
    #[cfg(target_os = "linux")]
    return Arc::new(crate::linux::window::SwayWindows);
    #[cfg(target_os = "macos")]
    return Arc::new(crate::macos::window::QuartzWindows);
    #[cfg(target_os = "windows")]
    return Arc::new(crate::windows::window::Win32Windows);
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    return Arc::new(crate::unsupported::Unsupported);
}

/// The clipboard for this OS.
pub fn clipboard() -> Arc<dyn Clipboard> {
    #[cfg(target_os = "linux")]
    return Arc::new(crate::linux::clipboard::WlClipboard);
    #[cfg(target_os = "macos")]
    return Arc::new(crate::macos::clipboard::PbClipboard);
    #[cfg(target_os = "windows")]
    return Arc::new(crate::windows::clipboard::Win32Clipboard);
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    return Arc::new(crate::unsupported::Unsupported);
}

/// The installed-apps catalogue for this OS.
pub fn apps() -> Arc<dyn AppCatalog> {
    #[cfg(target_os = "linux")]
    return Arc::new(crate::linux::desktop::XdgDesktop);
    #[cfg(target_os = "macos")]
    return Arc::new(crate::macos::desktop::BundleDesktop);
    #[cfg(target_os = "windows")]
    return Arc::new(crate::windows::desktop::StartMenuDesktop);
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    return Arc::new(crate::unsupported::Unsupported);
}

/// Is a process this server launched still alive?
///
/// Not a desktop seam — a launched program is not a window — but per-OS all
/// the same: the Unix hosts ask the kernel with `kill(pid, 0)` and Windows
/// waits on a process handle. A free function for the reason [`conventions`]
/// is one: there is no state to hold, and nothing to inject.
pub fn process_is_running(pid: u32) -> bool {
    #[cfg(target_os = "linux")]
    return crate::linux::process::is_running(pid);
    #[cfg(target_os = "macos")]
    return crate::macos::process::is_running(pid);
    #[cfg(target_os = "windows")]
    return crate::windows::process::is_running(pid);
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    return crate::unsupported::process::is_running(pid);
}

/// Keep a child out of the server's own signal group, where the OS has one.
///
/// A Ctrl-C on a foreground run signals the whole group on Unix, and an
/// agent's browser dying with the server that launched it is not what either
/// of them asked for. Windows has no such inheritance, so the call is a no-op
/// there — which is a fact about the OS, and the reason it is stated here
/// rather than guessed at by the caller.
pub fn process_detach(command: &mut Command) {
    #[cfg(target_os = "linux")]
    crate::linux::process::detach(command);
    #[cfg(target_os = "macos")]
    crate::macos::process::detach(command);
    #[cfg(target_os = "windows")]
    crate::windows::process::detach(command);
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    crate::unsupported::process::detach(command);
}
