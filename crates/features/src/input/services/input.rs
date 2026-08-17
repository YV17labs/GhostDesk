//! Mouse and keyboard control, delegated to the host's [`InputBackend`].
//!
//! This service owns the *policy* — the model-space conversion, feedback
//! capture, the message an agent reads back — and none of the mechanism. How a
//! click actually happens (a Wayland virtual pointer, a CGEvent) is the
//! backend's business. Argument bounds are the DTO's, checked by the pipe.

use std::sync::Arc;

use nest_rs::core::{hooks, injectable};
use platform::coords;
use platform::input::{Button, InputBackend, ScrollDirection};

use super::feedback::{Feedback, FeedbackService};
use crate::input::error::InputError;

type Result<T> = std::result::Result<T, InputError>;

#[injectable]
pub struct InputService {
    #[inject]
    svc: Arc<FeedbackService>,
    #[inject]
    backend: Arc<dyn InputBackend>,
}

#[hooks]
impl InputService {
    /// Warm the input backend up at boot.
    ///
    /// A host missing what the backend needs (a Wayland protocol, an OS
    /// permission) is a deployment error, and this hook is what turns it
    /// into a failed boot with a clear message instead of a puzzling failure
    /// on the agent's first click. Init hooks are strict — the first error
    /// aborts, and nothing is listening yet.
    #[on_application_bootstrap]
    async fn warm_up(&self) -> anyhow::Result<()> {
        let started = std::time::Instant::now();
        self.backend.warm_up().await?;
        tracing::info!(
            target: "features::input",
            elapsed_ms = started.elapsed().as_millis() as u64,
            "input backend ready",
        );
        Ok(())
    }
}

impl InputService {
    /// Move the cursor without pressing anything — hover-only UI reactions.
    pub async fn mouse_move(&self, x: i64, y: i64) -> Result<Feedback> {
        let (x, y) = coords::to_pixels(x, y);
        let before = self.svc.capture_before().await?;
        self.backend
            .move_to(x, y)
            .await
            .map_err(InputError::Backend)?;
        self.svc
            .observe(format!("Moved cursor to ({x}, {y})"), &before)
            .await
    }

    /// Click once at screen coordinates.
    ///
    /// The baseline is captured *after* the pointer is in place, so the
    /// cursor's own repaint is not what gets reported as a change.
    pub async fn mouse_click(&self, x: i64, y: i64, button: Button) -> Result<Feedback> {
        let (x, y) = coords::to_pixels(x, y);
        self.backend
            .move_to(x, y)
            .await
            .map_err(InputError::Backend)?;
        let before = self.svc.capture_before().await?;
        self.backend
            .click(button)
            .await
            .map_err(InputError::Backend)?;
        self.svc
            .observe(
                format!("Clicked {} at ({x}, {y})", button.as_str()),
                &before,
            )
            .await
    }

    /// Two clicks in a row — open a file, select a word.
    pub async fn mouse_double_click(&self, x: i64, y: i64, button: Button) -> Result<Feedback> {
        let (x, y) = coords::to_pixels(x, y);
        self.backend
            .move_to(x, y)
            .await
            .map_err(InputError::Backend)?;
        let before = self.svc.capture_before().await?;
        self.backend
            .click(button)
            .await
            .map_err(InputError::Backend)?;
        self.backend
            .click(button)
            .await
            .map_err(InputError::Backend)?;
        self.svc
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
    ) -> Result<Feedback> {
        let from = coords::to_pixels(from.0, from.1);
        let to = coords::to_pixels(to.0, to.1);
        let before = self.svc.capture_before().await?;
        self.backend
            .drag(from, to, button)
            .await
            .map_err(InputError::Backend)?;
        self.svc
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
    ) -> Result<Feedback> {
        let (x, y) = coords::to_pixels(x, y);
        self.backend
            .move_to(x, y)
            .await
            .map_err(InputError::Backend)?;
        let before = self.svc.capture_before().await?;
        self.backend
            .scroll(direction, amount)
            .await
            .map_err(InputError::Backend)?;
        self.svc
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
    pub async fn key_type(&self, text: &str) -> Result<Feedback> {
        let before = self.svc.capture_before().await?;
        self.backend
            .type_text(text)
            .await
            .map_err(InputError::Backend)?;
        self.svc
            .observe(
                format!("Typed {} characters", text.chars().count()),
                &before,
            )
            .await
    }

    /// Press a key or a chord.
    pub async fn key_press(&self, keys: &str) -> Result<Feedback> {
        // Resolve before capturing: an unknown key name should fail without
        // costing a screenshot.
        let chord = self
            .backend
            .resolve_chord(keys)
            .map_err(InputError::Chord)?;
        let before = self.svc.capture_before().await?;
        self.backend
            .press_chord(chord)
            .await
            .map_err(InputError::Backend)?;
        self.svc.observe(format!("Pressed {keys}"), &before).await
    }
}
