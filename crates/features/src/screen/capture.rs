use platform::screen::ImageFormat;

/// One encoded frame, and what it is encoded as.
///
/// The format travels with the bytes rather than being remembered by the
/// caller: an adapter has to name a MIME type, and a frame that arrived as
/// WebP announced as PNG is an image no client renders.
pub struct Capture {
    pub bytes: Vec<u8>,
    pub format: ImageFormat,
}
