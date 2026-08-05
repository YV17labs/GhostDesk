//! The one place an OS is chosen.
//!
//! Each function answers `Arc<dyn Trait>` for one of the five seams, picked
//! by `target_os` at compile time. Porting GhostDesk to another OS is a new
//! sibling of `linux`/`macos` implementing the five contracts, plus one arm
//! in each selector below — no other file in the workspace names an OS.

use std::sync::Arc;

use crate::clipboard::Clipboard;
use crate::desktop::AppCatalog;
use crate::input::InputBackend;
use crate::screen::ScreenBackend;
use crate::window::WindowManager;

/// The mouse-and-keyboard backend for this OS.
pub fn input() -> Arc<dyn InputBackend> {
    #[cfg(target_os = "linux")]
    return Arc::new(crate::linux::input::Wayland::default());
    #[cfg(target_os = "macos")]
    return Arc::new(crate::macos::input::Quartz);
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    return Arc::new(crate::unsupported::Unsupported);
}

/// The screen-capture backend for this OS.
pub fn screen() -> Arc<dyn ScreenBackend> {
    #[cfg(target_os = "linux")]
    return Arc::new(crate::linux::screen::Grim);
    #[cfg(target_os = "macos")]
    return Arc::new(crate::macos::screen::ScreenCapture);
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    return Arc::new(crate::unsupported::Unsupported);
}

/// The window manager for this OS.
pub fn windows() -> Arc<dyn WindowManager> {
    #[cfg(target_os = "linux")]
    return Arc::new(crate::linux::window::Sway);
    #[cfg(target_os = "macos")]
    return Arc::new(crate::macos::window::Quartz);
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    return Arc::new(crate::unsupported::Unsupported);
}

/// The clipboard for this OS.
pub fn clipboard() -> Arc<dyn Clipboard> {
    #[cfg(target_os = "linux")]
    return Arc::new(crate::linux::clipboard::WlClipboard);
    #[cfg(target_os = "macos")]
    return Arc::new(crate::macos::clipboard::Pasteboard);
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    return Arc::new(crate::unsupported::Unsupported);
}

/// The installed-apps catalogue for this OS.
pub fn apps() -> Arc<dyn AppCatalog> {
    #[cfg(target_os = "linux")]
    return Arc::new(crate::linux::desktop::XdgEntries);
    #[cfg(target_os = "macos")]
    return Arc::new(crate::macos::desktop::AppBundles);
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    return Arc::new(crate::unsupported::Unsupported);
}
