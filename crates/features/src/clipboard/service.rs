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

#[cfg(test)]
mod tests {
    use nest_rs::testing::LogCapture;

    use super::*;
    use crate::testing::{MemoryClipboard, ReadOnlyClipboard};

    fn service(backend: Arc<dyn Clipboard>) -> ClipboardService {
        ClipboardService { backend }
    }

    #[tokio::test]
    async fn what_was_written_is_what_comes_back() {
        let backend = Arc::new(MemoryClipboard::holding("before"));
        let clipboard = service(Arc::clone(&backend) as Arc<dyn Clipboard>);

        assert_eq!(clipboard.get().await, "before");
        clipboard.set("after").await.expect("written");
        assert_eq!(clipboard.get().await, "after");
    }

    #[tokio::test]
    async fn the_answer_counts_characters_not_bytes() {
        let clipboard = service(Arc::new(MemoryClipboard::default()));
        // Six characters, ten bytes. The agent is told what it wrote, and a
        // byte count would be a different number for the same text.
        let answer = clipboard.set("héllo…").await.expect("written");
        assert_eq!(answer, "Clipboard set (6 characters)");
    }

    #[tokio::test]
    async fn a_clipboard_that_cannot_be_written_says_so() {
        let err = service(Arc::new(ReadOnlyClipboard))
            .set("anything")
            .await
            .unwrap_err();
        assert!(matches!(err, ClipboardError::Write(_)), "got: {err}");
    }

    #[tokio::test]
    async fn the_trail_counts_what_crossed_and_never_quotes_it() {
        let logs = LogCapture::install();
        let clipboard = service(Arc::new(MemoryClipboard::holding("s3cret-token")));

        clipboard.get().await;
        clipboard.set("another-s3cret").await.expect("written");

        let read = logs.expect_one("features::clipboard", "clipboard read");
        assert_eq!(read.field("chars").as_deref(), Some("12"));
        let written = logs.expect_one("features::clipboard", "clipboard written");
        assert_eq!(written.field("chars").as_deref(), Some("14"));
        assert!(
            !format!("{:?}", logs.events()).contains("s3cret"),
            "a trail that leaks what it audits is worse than none",
        );
    }
}
