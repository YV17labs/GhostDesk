//! Main-display metrics, and the one conversion Retina forces on us.
//!
//! macOS runs two coordinate systems at once. Quartz **points** are what the
//! window server and every `CGEvent` speak; **pixels** are what a capture
//! actually contains. On a Retina panel the second is twice the first, and on
//! a mixed setup the ratio is whatever the user picked in Displays.
//!
//! GhostDesk's neutral layer is defined in pixels — the agent's coordinates
//! are offsets into the image it was shown, and nothing else would make
//! sense. So this module is where pixels become points, and it is the only
//! place in the crate that knows the difference. Keeping the factor here
//! rather than in `coords` is deliberate: model-space conversion is a
//! product concept shared by every OS, while this ratio is an artefact of
//! one window server.

use objc2_core_graphics::{
    CGDisplayBounds, CGDisplayCopyDisplayMode, CGDisplayMode, CGMainDisplayID,
};

/// The main display's size in captured pixels.
pub fn size_in_pixels() -> (i64, i64) {
    match CGDisplayCopyDisplayMode(CGMainDisplayID()) {
        Some(mode) => (
            CGDisplayMode::pixel_width(Some(&mode)) as i64,
            CGDisplayMode::pixel_height(Some(&mode)) as i64,
        ),
        // No current mode is a display that is asleep or being reconfigured.
        // The point size is the honest fallback: it is never larger than the
        // pixel size, so coordinates stay inside the panel.
        None => size_in_points(),
    }
}

/// The main display's size in Quartz points.
pub fn size_in_points() -> (i64, i64) {
    let bounds = CGDisplayBounds(CGMainDisplayID());
    (bounds.size.width as i64, bounds.size.height as i64)
}

/// Captured pixels per Quartz point — 2.0 on a Retina panel, 1.0 otherwise.
///
/// Recomputed per call rather than cached: a display can be swapped, scaled
/// or reconfigured mid-session, and a stale factor would put every click in
/// the wrong place with nothing to explain why.
pub fn backing_scale() -> f64 {
    let (pixels, _) = size_in_pixels();
    let (points, _) = size_in_points();
    if points <= 0 || pixels <= 0 {
        return 1.0;
    }
    pixels as f64 / points as f64
}

/// A pixel coordinate in the agent's space → the Quartz point a `CGEvent`
/// wants.
pub fn pixels_to_points(x: i64, y: i64) -> (f64, f64) {
    let scale = backing_scale();
    (x as f64 / scale, y as f64 / scale)
}
