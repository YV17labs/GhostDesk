use nest_rs::mcp::ContentBlock;

use crate::screen::service::Capture;

/// A capture in the form the agent receives it.
///
/// Both tools that hand the model a picture — `screen_shot` and the settled
/// frame `app_launch` returns — come through here, so how a capture is
/// encoded and what mime it is announced under is decided in one place.
pub struct CaptureDto(ContentBlock);

impl CaptureDto {
    /// The block, ready to push onto a tool result.
    pub fn block(self) -> ContentBlock {
        self.0
    }
}

impl From<&Capture> for CaptureDto {
    fn from(capture: &Capture) -> Self {
        use base64::Engine as _;
        let encoded = base64::engine::general_purpose::STANDARD.encode(&capture.bytes);
        Self(ContentBlock::image(encoded, capture.format.mime()))
    }
}
