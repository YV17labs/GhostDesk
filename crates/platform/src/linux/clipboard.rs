//! The Linux [`Clipboard`] — `wl-copy` / `wl-paste`.

use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;

use crate::clipboard::Clipboard;
use crate::cmd;

/// How long `wl-copy` gets to take the content and fork its daemon.
const COPY_TIMEOUT: Duration = Duration::from_secs(5);

/// The [`Clipboard`] the host selector hands out on Linux.
pub struct WlClipboard;

#[async_trait]
impl Clipboard for WlClipboard {
    /// `wl-paste` exits non-zero for both an empty clipboard and non-text
    /// content, which is exactly the "empty string" contract.
    async fn get(&self) -> String {
        cmd::run(&["wl-paste", "--no-newline"], cmd::DEFAULT_TIMEOUT)
            .await
            .unwrap_or_default()
    }

    async fn set(&self, text: &str) -> Result<()> {
        Ok(cmd::run_with_stdin(&["wl-copy"], text.as_bytes(), COPY_TIMEOUT).await?)
    }
}
