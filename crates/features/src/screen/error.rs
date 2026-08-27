#[derive(Debug, thiserror::Error)]
pub enum ScreenError {
    #[error("the screen could not be captured")]
    Capture(#[source] anyhow::Error),

    #[error("a captured frame could not be decoded")]
    Decode(#[source] anyhow::Error),
}
