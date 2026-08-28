use crate::blame::Blame;

#[derive(Debug, thiserror::Error)]
pub enum ClipboardError {
    #[error("the clipboard could not be written")]
    Write(#[source] anyhow::Error),
}

impl Blame for ClipboardError {
    fn blames_the_caller(&self) -> bool {
        // A clipboard that refuses a write is the desktop's failure, and the
        // helper's own message names a binary the model must not learn about.
        match self {
            Self::Write(_) => false,
        }
    }
}
