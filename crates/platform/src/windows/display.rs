//! Primary-display metrics, and the one call that has to happen before any of
//! them are worth reading.
//!
//! Windows reports a *virtualised* desktop to a process that has not declared
//! itself DPI-aware: on a panel at 150% scaling, a 2560×1440 screen answers
//! 1707×960 and every capture comes back stretched from a lie. The agent's
//! coordinates are offsets into the image it was shown, so a lie here puts
//! every click a third of the screen away from its target.
//!
//! [`ensure_dpi_aware`] is therefore called by every entry point below rather
//! than left to a caller to remember, and it must run before the process makes
//! its first device context — which is why it is here, beside the first code
//! that would.
//!
//! The other conversion is [`pixels_to_absolute`]. `SendInput` does not take
//! pixels: an absolute mouse event carries a 0..65535 coordinate that the OS
//! maps onto the primary display, and that mapping is this file's business
//! alone, exactly as the Retina point/pixel ratio is the macOS backend's.

#![expect(
    unsafe_code,
    reason = "the Win32 metrics calls are C; each carries its own SAFETY note"
)]

use std::sync::Once;

use ::windows::Win32::UI::HiDpi::{
    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetProcessDpiAwarenessContext,
};
use ::windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};

/// Declare this process DPI-aware, once.
///
/// Failure is ignored on purpose: the call refuses when awareness is already
/// set — by an embedded manifest, or by an earlier call — and both of those
/// are the state this function exists to reach.
pub fn ensure_dpi_aware() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        // SAFETY: no pointers; sets a process-wide flag before any device
        // context exists.
        unsafe { SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) }.ok();
    });
}

/// The primary display's size in physical pixels.
///
/// The primary display, not the virtual desktop spanning every monitor, and
/// the same choice the macOS backend makes about the main display: capture,
/// geometry and the absolute coordinate space below all have to agree on one
/// rectangle, and `SendInput`'s own default is this one.
pub fn size_in_pixels() -> (i64, i64) {
    ensure_dpi_aware();
    // SAFETY: no pointers; reads two process-wide metrics.
    unsafe {
        (
            GetSystemMetrics(SM_CXSCREEN) as i64,
            GetSystemMetrics(SM_CYSCREEN) as i64,
        )
    }
}

/// A pixel on the primary display → the absolute unit `SendInput` wants.
///
/// The event stream carries no pixels: `MOUSEEVENTF_ABSOLUTE` maps 0 to the
/// first column and 65535 to the last, so the divisor is the span between
/// them and not the width.
pub fn pixels_to_absolute(x: i64, y: i64) -> (i32, i32) {
    let (width, height) = size_in_pixels();
    (normalize(x, width), normalize(y, height))
}

/// Map a coordinate into 0..=65535 over an `extent`-wide axis.
///
/// Rounded rather than truncated: the two are a whole pixel apart at the top
/// of a 4K axis, and a click that lands one pixel short of a one-pixel border
/// lands on the wrong side of it.
fn normalize(value: i64, extent: i64) -> i32 {
    let span = (extent - 1).max(1);
    let value = value.clamp(0, span);
    ((value * 65_535 + span / 2) / span) as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_axis_ends_land_on_the_axis_ends() {
        assert_eq!(normalize(0, 1920), 0);
        assert_eq!(normalize(1919, 1920), 65_535);
    }

    #[test]
    fn a_coordinate_past_the_edge_is_pinned_to_it() {
        assert_eq!(normalize(-5, 1920), 0);
        assert_eq!(normalize(9_999, 1920), 65_535);
    }

    #[test]
    fn the_midpoint_is_the_midpoint() {
        // Truncation would answer 32_750 here, which is a whole pixel short.
        assert_eq!(normalize(960, 1921), 32_768);
    }

    #[test]
    fn a_one_pixel_axis_does_not_divide_by_zero() {
        assert_eq!(normalize(0, 1), 0);
    }
}
