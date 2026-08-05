//! The window-management contract — enumerate what is open, close a window.
//!
//! This is the seam that keeps compositor JSON out of the feature layer: a
//! backend hands back [`WindowInfo`] values it built itself, never a raw
//! protocol payload for someone upstream to dig through.

use std::any::Any;
use std::fmt;
use std::sync::Arc;

use anyhow::{Result, bail};
use async_trait::async_trait;

/// A window's identity, opaque outside the backend that produced it.
///
/// Sway has a `con_id` — one integer, stable, addressable for a kill. macOS
/// has no equivalent: a `CGWindowID` enumerates but cannot act, an
/// `AXUIElement` acts but does not enumerate, and bridging the two is the
/// backend's problem. A bare `i64` here would encode Sway's luck as the
/// contract and force a redesign the day a second backend lands.
#[derive(Clone)]
pub struct WindowId {
    payload: Arc<dyn Any + Send + Sync>,
    /// How the id prints in logs (`con_id=42`). Carried alongside the payload
    /// because `dyn Any` cannot render itself.
    display: String,
}

impl WindowId {
    pub fn new(payload: impl Any + Send + Sync, display: impl Into<String>) -> Self {
        Self {
            payload: Arc::new(payload),
            display: display.into(),
        }
    }

    /// Recover the backend payload.
    ///
    /// The error means the id came from another backend — a programming
    /// error, not a runtime condition. The diagnostic lives here rather than
    /// in each backend so every implementation reports it the same way.
    pub fn payload<T: Any>(&self) -> Result<&T> {
        match self.payload.downcast_ref() {
            Some(payload) => Ok(payload),
            None => bail!("window id was produced by a different window manager"),
        }
    }
}

impl fmt::Display for WindowId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.display)
    }
}

impl fmt::Debug for WindowId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "WindowId({})", self.display)
    }
}

/// One open application window, as the platform layer sees it.
///
/// Deliberately not `Serialize` — same doctrine as `DesktopApp`: the wire
/// shape belongs to the MCP adapter, so renaming a field here cannot silently
/// change what clients receive.
#[derive(Debug, Clone)]
pub struct WindowInfo {
    pub id: WindowId,
    /// Stable application identity: `app_id` (else the X11 class) on
    /// Wayland, the bundle identifier on macOS.
    pub app: String,
    /// The window's current title.
    pub title: String,
    pub pid: i64,
    pub focused: bool,
}

/// Enumerate and close application windows, one implementation per OS.
///
/// Only real client windows are listed — never the desktop's own
/// infrastructure (compositor, bar, notification daemon, this server).
/// That exclusion is a security property the idle watchdog relies on, so it
/// is part of the contract, not a backend courtesy.
#[async_trait]
pub trait WindowManager: Send + Sync {
    async fn windows(&self) -> Result<Vec<WindowInfo>>;

    /// Ask one window to close, gracefully: the client receives the OS's
    /// close event and may flush state before exiting.
    async fn close(&self, window: &WindowId) -> Result<()>;
}
