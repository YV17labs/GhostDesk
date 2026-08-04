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

/// Fallbacks, used only if nothing installs a size — a unit test reaching
/// into `screen` or `wayland` without booting the app.
const DEFAULT_WIDTH: i64 = 1280;
const DEFAULT_HEIGHT: i64 = 1024;

static SCREEN: OnceLock<(i64, i64)> = OnceLock::new();

/// Install the screen size for the process.
///
/// The value belongs to `GhostdeskConfig`, which is loaded and validated by
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

/// True when coordinate normalisation is active.
pub fn is_enabled() -> bool {
    model_space() > 0
}

fn rescale(value: i64, extent: i64, from: i64, to: i64) -> i64 {
    // Same rounding as Python's `round(v * a / b)` for the non-negative
    // values coordinates actually take.
    let _ = extent;
    ((value as f64) * (to as f64) / (from as f64)).round() as i64
}

/// Model coords → screen pixels. Pass-through when disabled.
pub fn to_pixels(mx: i64, my: i64) -> (i64, i64) {
    let space = model_space();
    if space == 0 {
        return (mx, my);
    }
    (
        rescale(mx, 0, space, screen_width()),
        rescale(my, 0, space, screen_height()),
    )
}

/// Screen pixels → model coords. Pass-through when disabled.
pub fn to_model(px: i64, py: i64) -> (i64, i64) {
    let space = model_space();
    if space == 0 {
        return (px, py);
    }
    (
        rescale(px, 0, screen_width(), space),
        rescale(py, 0, screen_height(), space),
    )
}

/// A model-space `{x, y, width, height}` → pixel region.
pub fn region_to_pixels(x: i64, y: i64, w: i64, h: i64) -> (i64, i64, i64, i64) {
    let space = model_space();
    if space == 0 {
        return (x, y, w, h);
    }
    (
        rescale(x, 0, space, screen_width()),
        rescale(y, 0, space, screen_height()),
        rescale(w, 0, space, screen_width()),
        rescale(h, 0, space, screen_height()),
    )
}

/// A pixel region → model space.
pub fn region_to_model(x: i64, y: i64, w: i64, h: i64) -> (i64, i64, i64, i64) {
    let space = model_space();
    if space == 0 {
        return (x, y, w, h);
    }
    (
        rescale(x, 0, screen_width(), space),
        rescale(y, 0, screen_height(), space),
        rescale(w, 0, screen_width(), space),
        rescale(h, 0, screen_height(), space),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn pass_through_when_no_space_is_installed() {
        assert_eq!(to_pixels(383, 22), (383, 22));
        assert!(!is_enabled());
    }

    #[tokio::test]
    async fn rescales_from_a_normalised_space() {
        with_model_space(1000, async {
            assert!(is_enabled());
            // 500/1000 of a 1280x1024 screen.
            assert_eq!(to_pixels(500, 500), (640, 512));
            assert_eq!(to_model(640, 512), (500, 500));
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
