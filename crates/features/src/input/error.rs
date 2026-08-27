#[derive(Debug, thiserror::Error)]
pub enum InputError {
    #[error("{0}")]
    Chord(#[source] anyhow::Error),

    #[error("the input backend refused the action")]
    Backend(#[source] anyhow::Error),

    #[error("the action was performed but its effect could not be observed")]
    Feedback(#[source] anyhow::Error),
}

impl InputError {
    pub(crate) fn blames_the_caller(&self) -> bool {
        match self {
            Self::Chord(_) => true,
            Self::Backend(_) | Self::Feedback(_) => false,
        }
    }
}
