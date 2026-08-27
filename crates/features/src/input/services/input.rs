use std::sync::Arc;

use nest_rs::core::{hooks, injectable};
use nest_rs::health::indicators;
use platform::coords;
use platform::input::{Button, InputBackend, ScrollDirection};

use super::feedback::{Feedback, FeedbackService};
use crate::input::action::Action;
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
    #[on_application_bootstrap]
    async fn warm_up(&self) {
        let started = std::time::Instant::now();
        match self.backend.warm_up().await {
            Ok(()) => tracing::info!(
                target: "features::input",
                elapsed_ms = started.elapsed().as_millis() as u64,
                "input backend ready",
            ),
            Err(err) => tracing::warn!(
                target: "features::input",
                error = %err,
                elapsed_ms = started.elapsed().as_millis() as u64,
                "input backend unavailable at boot — serving unhealthy until it binds",
            ),
        }
    }
}

#[indicators]
impl InputService {
    #[liveness]
    async fn input_backend(&self) -> anyhow::Result<()> {
        self.backend.ping().await
    }
}

impl InputService {
    pub async fn mouse_move(&self, x: i64, y: i64) -> Result<Feedback> {
        let (x, y) = coords::to_pixels(x, y);
        let before = self.svc.capture_before().await?;
        self.backend
            .move_to(x, y)
            .await
            .map_err(InputError::Backend)?;
        self.svc.observe(Action::Move { x, y }, &before).await
    }

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
            .observe(Action::Click { button, x, y }, &before)
            .await
    }

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
            .observe(Action::DoubleClick { button, x, y }, &before)
            .await
    }

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
            .observe(Action::Drag { button, from, to }, &before)
            .await
    }

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
                Action::Scroll {
                    direction,
                    amount,
                    x,
                    y,
                },
                &before,
            )
            .await
    }

    pub async fn key_type(&self, text: &str) -> Result<Feedback> {
        let before = self.svc.capture_before().await?;
        self.backend
            .type_text(text)
            .await
            .map_err(InputError::Backend)?;
        self.svc
            .observe(
                Action::Type {
                    chars: text.chars().count(),
                },
                &before,
            )
            .await
    }

    pub async fn key_press(&self, keys: &str) -> Result<Feedback> {
        let chord = self
            .backend
            .resolve_chord(keys)
            .map_err(InputError::Chord)?;
        let before = self.svc.capture_before().await?;
        self.backend
            .press_chord(chord)
            .await
            .map_err(InputError::Backend)?;
        self.svc
            .observe(
                Action::Key {
                    keys: keys.to_owned(),
                },
                &before,
            )
            .await
    }
}
