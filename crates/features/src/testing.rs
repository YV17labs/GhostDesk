//! Test doubles shared by the domain services' unit tests.
//!
//! Every service now injects a backend contract, so a service under test
//! needs a stand-in for the seams it does not exercise. One copy here rather
//! than one per test module: with five contracts, a third service would
//! otherwise arrive with a third hand-written pair.
//!
//! `platform::host`'s real backends are not a substitute — they talk to the
//! machine running the tests — and neither is the `Unsupported` stub, whose
//! methods fail rather than answering an empty list.

use std::path::PathBuf;

use nest_rs::core::async_trait;
use platform::desktop::{AppCatalog, DesktopApp};
use platform::window::{WindowId, WindowInfo, WindowManager};

/// A desktop with nothing open.
pub struct NoWindows;

#[async_trait]
impl WindowManager for NoWindows {
    async fn windows(&self) -> anyhow::Result<Vec<WindowInfo>> {
        Ok(Vec::new())
    }

    async fn close(&self, _window: &WindowId) -> anyhow::Result<()> {
        Ok(())
    }
}

/// A desktop showing exactly one window, owned by the given PID.
pub struct OneWindow(pub i64);

#[async_trait]
impl WindowManager for OneWindow {
    async fn windows(&self) -> anyhow::Result<Vec<WindowInfo>> {
        Ok(vec![WindowInfo {
            id: WindowId::new(0u8, "test-window"),
            app: "test-app".into(),
            title: "Test Window".into(),
            pid: self.0,
            focused: true,
        }])
    }

    async fn close(&self, _window: &WindowId) -> anyhow::Result<()> {
        Ok(())
    }
}

/// A machine with nothing installed — so every `app_launch` is refused.
pub struct EmptyCatalog;

impl AppCatalog for EmptyCatalog {
    fn apps(&self) -> Vec<DesktopApp> {
        Vec::new()
    }

    fn resolve(&self, _exec: &str) -> Option<PathBuf> {
        None
    }
}
