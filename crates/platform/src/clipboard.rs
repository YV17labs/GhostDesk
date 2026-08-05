//! The clipboard contract — read and write the system clipboard as text.

use anyhow::Result;
use async_trait::async_trait;

/// System clipboard access, one implementation per OS.
#[async_trait]
pub trait Clipboard: Send + Sync {
    /// The current clipboard as text. Empty when the clipboard is empty or
    /// holds non-text content — neither is an error worth spending an agent
    /// turn on, so the distinction is deliberately not surfaced.
    async fn get(&self) -> String;

    /// Write text to the clipboard.
    async fn set(&self, text: &str) -> Result<()>;
}
