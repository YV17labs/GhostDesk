use nest_rs::mcp::ContentBlock;

use crate::screen::service::Capture;

pub struct CaptureDto(ContentBlock);

impl CaptureDto {
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
