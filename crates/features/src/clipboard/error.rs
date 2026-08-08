//! What clipboard access can go wrong with.
//!
//! Only writing can fail: reading answers an empty string for an empty
//! clipboard *and* for non-text content, neither of which is worth an agent
//! turn to distinguish.

/// A failure from the clipboard domain.
#[derive(Debug, thiserror::Error)]
pub enum ClipboardError {
    #[error("the clipboard could not be written")]
    Write(#[source] anyhow::Error),
}
