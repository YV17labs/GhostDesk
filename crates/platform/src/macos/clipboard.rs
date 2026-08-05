//! The macOS [`Clipboard`] — `pbcopy` / `pbpaste`.
//!
//! The CLI pair rather than `NSPasteboard` on purpose: the Objective-C
//! interface would pull AppKit in for two calls, and AppKit expects a main
//! run loop this server does not have. The tools need no permission at all.

use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;

use crate::clipboard::Clipboard;
use crate::cmd;

/// How long `pbcopy` gets to consume its stdin and exit.
const COPY_TIMEOUT: Duration = Duration::from_secs(5);

/// The [`Clipboard`] the host selector hands out on macOS.
pub struct Pasteboard;

#[async_trait]
impl Clipboard for Pasteboard {
    /// `pbpaste` writes nothing for an empty pasteboard and fails for content
    /// it cannot render as text — both are the "empty string" contract.
    async fn get(&self) -> String {
        cmd::run(&["pbpaste"], cmd::DEFAULT_TIMEOUT)
            .await
            .unwrap_or_default()
    }

    async fn set(&self, text: &str) -> Result<()> {
        Ok(cmd::run_with_stdin(&["pbcopy"], text.as_bytes(), COPY_TIMEOUT).await?)
    }
}
