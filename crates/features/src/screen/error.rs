//! What a capture can go wrong with.
//!
//! Both variants are the server failing, never the caller: the one thing a
//! caller can get wrong — a quality outside the published band — is refused
//! by the DTO's own validation before a frame is grabbed.

/// A failure from the screen domain.
#[derive(Debug, thiserror::Error)]
pub enum ScreenError {
    /// The backend could not grab a frame.
    #[error("the screen could not be captured")]
    Capture(#[source] anyhow::Error),

    /// A frame came back that the encoder could not read.
    #[error("a captured frame could not be decoded")]
    Decode(#[source] anyhow::Error),
}
