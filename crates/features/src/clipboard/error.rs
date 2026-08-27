#[derive(Debug, thiserror::Error)]
pub enum ClipboardError {
    #[error("the clipboard could not be written")]
    Write(#[source] anyhow::Error),
}
