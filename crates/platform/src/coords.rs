//! Coordinate conversion between the LLM's normalised space and real pixels.
//!
//! The active model space is **per operation**: the MCP endpoint reads the
//! `GhostDesk-Model-Space` header on the HTTP request and the feature crate's
//! `McpToolContext` re-installs it inside rmcp's spawned dispatch, where the
//! request no longer exists. `0` (the default) means pass-through native
//! pixels; `1000` selects Qwen-VL's 0-1000 space; any positive integer is a
//! custom normalised space.

use std::future::Future;
use std::sync::OnceLock;

use crate::screen::Region;

/// Fallbacks, used only if nothing installs a size — a unit test reaching
/// into `screen` or `wayland` without booting the app.
const DEFAULT_WIDTH: i64 = 1280;
const DEFAULT_HEIGHT: i64 = 1024;

static SCREEN: OnceLock<(i64, i64)> = OnceLock::new();

/// Install the screen size for the process.
///
/// The value belongs to `ScreenConfig`, which is loaded and validated by
/// the framework at boot; this crate deliberately reads no environment of its
/// own, so there is exactly one place the size can come from. Idempotent —
/// a second call is ignored, which keeps concurrent tests honest.
pub fn set_screen(width: i64, height: i64) {
    let _ = SCREEN.set((width, height));
}

fn screen() -> (i64, i64) {
    *SCREEN.get_or_init(|| (DEFAULT_WIDTH, DEFAULT_HEIGHT))
}

/// Screen width in pixels.
pub fn screen_width() -> i64 {
    screen().0
}

/// Screen height in pixels.
pub fn screen_height() -> i64 {
    screen().1
}

tokio::task_local! {
    /// Set by the MCP tool context for the duration of one operation. A
    /// task-local (not a global) is what keeps concurrent calls carrying
    /// different headers from reading each other's space.
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
    (
        rescale(mx, space, screen_width()),
        rescale(my, space, screen_height()),
    )
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
    Region {
        x: rescale(region.x, space, screen_width()),
        y: rescale(region.y, space, screen_height()),
        width: rescale(region.width, space, screen_width()),
        height: rescale(region.height, space, screen_height()),
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
