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
    pub async fn get(&self) -> String {
        let text = self.backend.get().await;
        tracing::info!(
            target: "features::clipboard",
            chars = text.chars().count(),
            "clipboard read",
        );
        text
    }

    pub async fn set(&self, text: &str) -> Result<String, ClipboardError> {
        self.backend
            .set(text)
            .await
            .map_err(ClipboardError::Write)?;
        tracing::info!(
            target: "features::clipboard",
            chars = text.chars().count(),
            "clipboard written",
        );
        Ok(format!(
            "Clipboard set ({} characters)",
            text.chars().count()
        ))
    }
}
