//! Mouse and keyboard control, driven by the Wayland virtual-input protocols.
//!
//! Text entry is layout-independent: GhostDesk pushes its own XKB keymap and
//! never consults the compositor's, so a French AZERTY host and a US QWERTY
//! one produce byte-identical output.

use std::sync::Arc;

use nest_rs::core::{hooks, injectable};
use platform::wayland::keysym;
use platform::wayland::{Button, ScrollDirection, WaylandInput};
use tokio::sync::OnceCell;

use super::feedback::{Feedback, FeedbackService};
use super::keys;

/// Wheel notches allowed in a single call. Long pages are scrolled by
/// chaining calls, each with its own screenshot — one call that scrolls a
/// whole page would fly past content the agent never sees.
const SCROLL_MIN: u32 = 1;
const SCROLL_MAX: u32 = 5;

#[injectable]
pub struct InputService {
    #[inject]
    feedback: Arc<FeedbackService>,
    /// The connection is opened once and reused for the process lifetime.
    /// `OnceCell` rather than a plain field because the boot hook and the
    /// first tool call must not be able to open two.
    wayland: OnceCell<WaylandInput>,
}

#[hooks]
impl InputService {
    /// Bind the virtual pointer and keyboard at boot.
    ///
    /// A compositor missing either protocol is a deployment error, and this
    /// hook is what turns it into a failed boot with a clear message instead
    /// of a puzzling failure on the agent's first click. Init hooks are
    /// strict — the first error aborts, and nothing is listening yet.
    #[on_application_bootstrap]
    async fn warm_up(&self) -> anyhow::Result<()> {
        self.wayland().await?;
        tracing::info!(
            target: "ghostdesk::input",
            "Wayland input ready (virtual pointer + keyboard bound)",
        );
        Ok(())
    }
}

impl InputService {
    async fn wayland(&self) -> anyhow::Result<&WaylandInput> {
        self.wayland.get_or_try_init(WaylandInput::connect).await
    }

    /// Move the cursor without pressing anything — hover-only UI reactions.
    pub async fn mouse_move(&self, x: i64, y: i64) -> anyhow::Result<Feedback> {
        let before = self.feedback.capture_before().await?;
        self.wayland().await?.move_to(x, y).await?;
        self.feedback
            .observe(format!("Moved cursor to ({x}, {y})"), &before)
            .await
    }

    /// Click once at screen coordinates.
    ///
    /// The baseline is captured *after* the pointer is in place, so the
    /// cursor's own repaint is not what gets reported as a change.
    pub async fn mouse_click(&self, x: i64, y: i64, button: Button) -> anyhow::Result<Feedback> {
        let wayland = self.wayland().await?;
        wayland.move_to(x, y).await?;
        let before = self.feedback.capture_before().await?;
        wayland.click(button).await?;
        self.feedback
            .observe(
                format!("Clicked {} at ({x}, {y})", button.as_str()),
                &before,
            )
            .await
    }

    /// Two clicks in a row — open a file, select a word.
    pub async fn mouse_double_click(
        &self,
        x: i64,
        y: i64,
        button: Button,
    ) -> anyhow::Result<Feedback> {
        let wayland = self.wayland().await?;
        wayland.move_to(x, y).await?;
        let before = self.feedback.capture_before().await?;
        wayland.click(button).await?;
        wayland.click(button).await?;
        self.feedback
            .observe(
                format!("Double-clicked {} at ({x}, {y})", button.as_str()),
                &before,
            )
            .await
    }

    /// Press, drag and release.
    pub async fn mouse_drag(
        &self,
        from: (i64, i64),
        to: (i64, i64),
        button: Button,
    ) -> anyhow::Result<Feedback> {
        let before = self.feedback.capture_before().await?;
        self.wayland().await?.drag(from, to, button).await?;
        self.feedback
            .observe(
                format!(
                    "Dragged from ({}, {}) to ({}, {})",
                    from.0, from.1, to.0, to.1
                ),
                &before,
            )
            .await
    }

    /// Scroll the region under the pointer.
    pub async fn mouse_scroll(
        &self,
        x: i64,
        y: i64,
        direction: ScrollDirection,
        amount: u32,
    ) -> anyhow::Result<Feedback> {
        let amount = amount.clamp(SCROLL_MIN, SCROLL_MAX);
        let wayland = self.wayland().await?;
        wayland.move_to(x, y).await?;
        let before = self.feedback.capture_before().await?;
        wayland.scroll(direction, amount).await?;
        self.feedback
            .observe(
                format!(
                    "Scrolled {} {amount} clicks at ({x}, {y})",
                    direction.as_str()
                ),
                &before,
            )
            .await
    }

    /// Type text at the current keyboard focus.
    pub async fn key_type(&self, text: &str) -> anyhow::Result<Feedback> {
        let keysyms: Vec<u32> = text.chars().map(keysym::keysym_for_char).collect();
        let before = self.feedback.capture_before().await?;
        self.wayland().await?.type_keysyms(keysyms).await?;
        self.feedback
            .observe(
                format!("Typed {} characters", text.chars().count()),
                &before,
            )
            .await
    }

    /// Press a key or a chord.
    pub async fn key_press(&self, keys: &str) -> anyhow::Result<Feedback> {
        // Resolve before capturing: an unknown key name should fail without
        // costing a screenshot.
        let (mask, keysyms) = keys::resolve_chord(keys)?;
        let before = self.feedback.capture_before().await?;
        self.wayland().await?.press_chord(mask, keysyms).await?;
        self.feedback
            .observe(format!("Pressed {keys}"), &before)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scroll_amounts_are_clamped_into_the_allowed_band() {
        assert_eq!(0u32.clamp(SCROLL_MIN, SCROLL_MAX), 1);
        assert_eq!(3u32.clamp(SCROLL_MIN, SCROLL_MAX), 3);
        assert_eq!(99u32.clamp(SCROLL_MIN, SCROLL_MAX), 5);
    }
}
