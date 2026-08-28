//! The one place an OS is chosen.
//!
//! Five selectors answer `Arc<dyn Trait>`, one per seam, picked by `target_os`
//! at compile time; [`conventions`] answers a plain value for the sixth thing
//! a port has to state. Porting GhostDesk to another OS is a new sibling of
//! `linux`/`macos` implementing the five contracts, plus one arm in each
//! selector below — no other file in the workspace names an OS.

use std::sync::Arc;

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
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
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
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    return crate::unsupported::CONVENTIONS;
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
