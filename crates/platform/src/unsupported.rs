//! The stand-in for a build target with no backend.
//!
//! One type answering all five contracts with the same refusal. It exists so
//! the workspace *compiles* on every OS, which is what keeps the seam honest:
//! a backend type leaking into the neutral layer breaks the build everywhere
//! else, immediately, rather than the day someone tries the port.
//!
//! `warm_up` is the boot hook, so an unsupported OS fails at boot with one
//! clear message rather than on the agent's first click; the per-call errors
//! only matter to a binary that skips the hook.

use std::path::PathBuf;

use anyhow::{Result, bail};
use async_trait::async_trait;

use crate::chord::Chord;
use crate::clipboard::Clipboard;
use crate::desktop::{AppCatalog, DesktopApp};
use crate::input::{Button, Conventions, InputBackend, ScrollDirection};
use crate::screen::{Region, ScreenBackend};
use crate::window::{WindowId, WindowInfo, WindowManager};

const NO_BACKEND: &str = "GhostDesk has no desktop backend for this OS \
                          (Linux/Wayland and macOS are the implemented ones)";

pub struct Unsupported;

/// What this desktop calls its shortcuts — nothing useful, but the endpoint
/// still has a brief to render.
pub const CONVENTIONS: Conventions = Conventions {
    desktop: "an unsupported desktop",
    primary_modifier: "ctrl",
};

#[async_trait]
impl InputBackend for Unsupported {
    async fn warm_up(&self) -> Result<()> {
        bail!("{NO_BACKEND}")
    }
    async fn ping(&self) -> Result<()> {
        bail!("{NO_BACKEND}")
    }
    async fn move_to(&self, _x: i64, _y: i64) -> Result<()> {
        bail!("{NO_BACKEND}")
    }
    async fn click(&self, _button: Button) -> Result<()> {
        bail!("{NO_BACKEND}")
    }
    async fn drag(&self, _from: (i64, i64), _to: (i64, i64), _button: Button) -> Result<()> {
        bail!("{NO_BACKEND}")
    }
    async fn scroll(&self, _direction: ScrollDirection, _amount: u32) -> Result<()> {
        bail!("{NO_BACKEND}")
    }
    async fn type_text(&self, _text: &str) -> Result<()> {
        bail!("{NO_BACKEND}")
    }
    fn resolve_chord(&self, _keys: &str) -> Result<Chord> {
        bail!("{NO_BACKEND}")
    }
    async fn press_chord(&self, _chord: Chord) -> Result<()> {
        bail!("{NO_BACKEND}")
    }
}

#[async_trait]
impl ScreenBackend for Unsupported {
    async fn capture_png(&self, _region: Option<Region>, _scale: Option<f32>) -> Result<Vec<u8>> {
        bail!("{NO_BACKEND}")
    }
    fn geometry(&self) -> Option<(i64, i64)> {
        None
    }
}

#[async_trait]
impl WindowManager for Unsupported {
    async fn windows(&self) -> Result<Vec<WindowInfo>> {
        bail!("{NO_BACKEND}")
    }
    async fn close(&self, _window: &WindowId) -> Result<()> {
        bail!("{NO_BACKEND}")
    }
}

#[async_trait]
impl Clipboard for Unsupported {
    async fn get(&self) -> String {
        String::new()
    }
    async fn set(&self, _text: &str) -> Result<()> {
        bail!("{NO_BACKEND}")
    }
}

impl AppCatalog for Unsupported {
    fn apps(&self) -> Vec<DesktopApp> {
        Vec::new()
    }
    fn resolve(&self, _exec: &str) -> Option<PathBuf> {
        None
    }
}
