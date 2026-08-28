use crate::blame::Blame;

#[derive(Debug, thiserror::Error)]
pub enum ScreenError {
    #[error("the screen could not be captured")]
    Capture(#[source] anyhow::Error),

    #[error("a captured frame could not be decoded")]
    Decode(#[source] anyhow::Error),
}

impl Blame for ScreenError {
    fn blames_the_caller(&self) -> bool {
        // Nothing here is the model's doing: it asked for a picture of a screen
        // it cannot see, and a compositor that will not answer is not a
        // question it can rephrase. Declared all the same, so a variant added
        // later has to say which side it falls on before it compiles.
        match self {
            Self::Capture(_) | Self::Decode(_) => false,
        }
    }
}
