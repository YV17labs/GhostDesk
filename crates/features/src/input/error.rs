use crate::blame::Blame;

#[derive(Debug, thiserror::Error)]
pub enum InputError {
    #[error("{0}")]
    Chord(#[source] anyhow::Error),

    #[error("the input backend refused the action")]
    Backend(#[source] anyhow::Error),

    #[error("the action was performed but its effect could not be observed")]
    Feedback(#[source] anyhow::Error),
}

impl Blame for InputError {
    fn blames_the_caller(&self) -> bool {
        match self {
            // A mis-spelled chord is the one thing the model can fix and retry;
            // the other two are the desktop's failure, and naming a compositor
            // protocol at the model would teach it nothing it can act on.
            Self::Chord(_) => true,
            Self::Backend(_) | Self::Feedback(_) => false,
        }
    }
}
