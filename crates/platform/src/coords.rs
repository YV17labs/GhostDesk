//! Coordinate conversion between the LLM's normalised space and real pixels.
//!
//! The active model space is **per operation** and installed by the caller for
//! the duration of one: this crate holds the conversion, never the decision of
//! which space is in force. `0` (the default) means pass-through native pixels;
//! `1000` selects Qwen-VL's 0-1000 space; any positive integer is a custom
//! normalised space.

use std::future::Future;
use std::sync::Mutex;

use crate::screen::Region;

/// Fallbacks, used only while nothing has installed a size — a unit test
/// reaching into `screen` or the Wayland session without booting the app.
const DEFAULT_WIDTH: i64 = 1280;
const DEFAULT_HEIGHT: i64 = 1024;

/// The installed size, if anything has installed one.
///
/// `Option` in one cell, and not a `OnceLock`, for the reason the previous
/// shape got wrong: `get_or_init` made a **read** install the fallback, so real
/// geometry arriving afterwards was discarded in silence — on a backend that
/// owns its geometry, every coordinate wrong for the life of the process with
/// nothing logged and nothing to notice. Here a read cannot write, so the size
/// in force is whichever was installed last. One cell rather than two also
/// means a reader can never pair a new width with an old height.
///
/// Installed by `ScreenService`'s boot hook, which is also what logs the
/// geometry and where it came from — that line's absence is how an operator
/// knows nothing installed one.
static SCREEN: Mutex<Option<(i64, i64)>> = Mutex::new(None);

/// Install the screen size for the process.
///
/// This crate reads no environment of its own, so the size arrives from the
/// one caller that resolved it. Idempotent in the only sense that matters: the
/// same size installed twice changes nothing, and a display reconfigured
/// mid-session can install its new one.
pub fn set_screen(width: i64, height: i64) {
    *SCREEN.lock().expect("screen size poisoned") = Some((width, height));
}

/// The size in force, in pixels.
///
/// The pair, never one half at a time: a caller taking the width and the
/// height in two reads could straddle a display reconfiguration and pair a new
/// width with an old height, which is the one thing the single cell exists to
/// prevent. It is also the read on the pointer path, where a motion event
/// would otherwise pay two acquisitions.
pub fn screen() -> (i64, i64) {
    SCREEN
        .lock()
        .expect("screen size poisoned")
        .unwrap_or((DEFAULT_WIDTH, DEFAULT_HEIGHT))
}

tokio::task_local! {
    /// Installed by the caller for the duration of one operation. A
    /// task-local (not a global) is what keeps concurrent operations asking
    /// for different spaces from reading each other's.
    static MODEL_SPACE: i64;
}

/// Run `inner` with `space` installed as the ambient model space.
pub async fn with_model_space<F: Future>(space: i64, inner: F) -> F::Output {
    MODEL_SPACE.scope(space, inner).await
}

/// The active model space, or `0` outside any operation.
pub fn model_space() -> i64 {
    MODEL_SPACE.try_with(|space| *space).unwrap_or(0)
}

/// Map `value` out of a `from`-wide space into a `to`-wide one.
fn rescale(value: i64, from: i64, to: i64) -> i64 {
    ((value as f64) * (to as f64) / (from as f64)).round() as i64
}

/// Model coords → screen pixels. Pass-through when disabled.
pub fn to_pixels(mx: i64, my: i64) -> (i64, i64) {
    let space = model_space();
    if space == 0 {
        return (mx, my);
    }
    let (width, height) = screen();
    (rescale(mx, space, width), rescale(my, space, height))
}

/// A model-space region → a pixel region. Pass-through when disabled.
///
/// Only this direction exists: nothing ever reports coordinates back to the
/// agent, so a pixels→model counterpart would be an API with no caller.
pub fn region_to_pixels(region: Region) -> Region {
    let space = model_space();
    if space == 0 {
        return region;
    }
    let (width, height) = screen();
    Region {
        x: rescale(region.x, space, width),
        y: rescale(region.y, space, height),
        width: rescale(region.width, space, width),
        height: rescale(region.height, space, height),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FULL: Region = Region {
        x: 0,
        y: 0,
        width: 1000,
        height: 1000,
    };

    #[tokio::test]
    async fn pass_through_when_no_space_is_installed() {
        assert_eq!(to_pixels(383, 22), (383, 22));
        assert_eq!(region_to_pixels(FULL), FULL);
    }

    #[tokio::test]
    async fn rescales_from_a_normalised_space() {
        with_model_space(1000, async {
            // 500/1000 of a 1280x1024 screen.
            assert_eq!(to_pixels(500, 500), (640, 512));
            // The whole normalised square covers the whole screen.
            assert_eq!(
                region_to_pixels(FULL),
                Region {
                    x: 0,
                    y: 0,
                    width: 1280,
                    height: 1024,
                },
            );
        })
        .await;
    }

    #[tokio::test]
    async fn model_space_does_not_leak_between_tasks() {
        with_model_space(1000, async {
            let outside = tokio::spawn(async { model_space() }).await.unwrap();
            assert_eq!(outside, 0, "a spawned task inherits no model space");
            assert_eq!(model_space(), 1000);
        })
        .await;
    }
}
