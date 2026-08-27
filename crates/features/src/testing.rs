use std::path::PathBuf;

use nest_rs::core::async_trait;
use platform::desktop::{AppCatalog, DesktopApp};
use platform::window::{WindowId, WindowInfo, WindowManager};

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

pub struct EmptyCatalog;

impl AppCatalog for EmptyCatalog {
    fn apps(&self) -> Vec<DesktopApp> {
        Vec::new()
    }

    fn resolve(&self, _exec: &str) -> Option<PathBuf> {
        None
    }
}
