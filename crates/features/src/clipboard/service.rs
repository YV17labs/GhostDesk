//! Clipboard access, delegated to the host's [`Clipboard`] backend.

use std::sync::Arc;

use nest_rs::core::injectable;
use platform::clipboard::Clipboard;

use super::error::ClipboardError;

#[injectable]
pub struct ClipboardService {
    #[inject]
    backend: Arc<dyn Clipboard>,
}

impl ClipboardService {
    /// The current clipboard as text — empty when the clipboard is empty or
    /// holds non-text content, neither of which is worth an agent turn.
    pub async fn get(&self) -> String {
        self.backend.get().await
    }

    /// Write text to the clipboard. The message is agent-facing prose, so it
    /// belongs to this layer, not to the backend.
    pub async fn set(&self, text: &str) -> Result<String, ClipboardError> {
        self.backend
            .set(text)
            .await
            .map_err(ClipboardError::Write)?;
        Ok(format!(
            "Clipboard set ({} characters)",
            text.chars().count()
        ))
    }
}
