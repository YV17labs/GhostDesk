//! The channel to the Wayland thread.
//!
//! # Why a dedicated thread
//!
//! A Wayland event queue is driven by blocking round-trips and owns `!Sync`
//! dispatch state. Rather than smear that across the async runtime, the
//! connection lives on one OS thread that owns everything and processes
//! commands off a channel, one at a time. That single consumer *is* the
//! serialisation the protocol needs — the ordering guarantee comes from the
//! channel, not from a lock every call site has to remember to take.

use anyhow::{Context, Result, anyhow};
use tokio::sync::{mpsc, oneshot};
use tokio::time::sleep;

use super::session;
use crate::input::{Button, ScrollDirection, drag};

/// One unit of work for the Wayland thread. Each variant ends in a
/// round-trip, so the compositor has seen everything by the time the reply
/// comes back.
pub(super) enum Command {
    Motion {
        x: i64,
        y: i64,
    },
    Button {
        button: Button,
        pressed: bool,
    },
    Click {
        button: Button,
    },
    Scroll {
        direction: ScrollDirection,
        amount: u32,
    },
    Type {
        keysyms: Vec<u32>,
    },
    Chord {
        mask: u32,
        keysyms: Vec<u32>,
    },
}

/// A command and the channel its outcome goes back on.
pub(super) type Job = (Command, oneshot::Sender<Result<()>>);

/// Handle to the Wayland thread. Cheap to clone; every clone drives the same
/// connection.
#[derive(Clone)]
pub(super) struct Connection {
    tx: mpsc::UnboundedSender<Job>,
}

impl Connection {
    /// Connect, bind the two virtual-input protocols, and spawn the thread
    /// that owns them. Returns once the compositor has acknowledged the
    /// bindings, so a missing protocol surfaces here rather than mid-session.
    pub(super) async fn open() -> Result<Self> {
        let (tx, rx) = mpsc::unbounded_channel::<Job>();
        let (ready_tx, ready_rx) = oneshot::channel::<Result<()>>();

        std::thread::Builder::new()
            .name("ghostdesk-wayland".into())
            .spawn(move || session::run(rx, ready_tx))
            .context("spawning the Wayland input thread")?;

        ready_rx
            .await
            .map_err(|_| anyhow!("the Wayland input thread exited before it was ready"))??;

        Ok(Self { tx })
    }

    async fn send(&self, command: Command) -> Result<()> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.tx
            .send((command, reply_tx))
            .map_err(|_| anyhow!("the Wayland input thread is gone"))?;
        reply_rx
            .await
            .map_err(|_| anyhow!("the Wayland input thread dropped a reply"))?
    }

    pub(super) async fn move_to(&self, x: i64, y: i64) -> Result<()> {
        self.send(Command::Motion { x, y }).await
    }

    async fn button(&self, button: Button, pressed: bool) -> Result<()> {
        self.send(Command::Button { button, pressed }).await
    }

    /// One press/release pair inside a single round-trip.
    pub(super) async fn click(&self, button: Button) -> Result<()> {
        self.send(Command::Click { button }).await
    }

    pub(super) async fn scroll(&self, direction: ScrollDirection, amount: u32) -> Result<()> {
        self.send(Command::Scroll { direction, amount }).await
    }

    /// Press and release every keysym in order.
    pub(super) async fn type_keysyms(&self, keysyms: Vec<u32>) -> Result<()> {
        if keysyms.is_empty() {
            return Ok(());
        }
        self.send(Command::Type { keysyms }).await
    }

    /// Hold `mask` while tapping each non-modifier keysym.
    pub(super) async fn press_chord(&self, mask: u32, keysyms: Vec<u32>) -> Result<()> {
        if mask == 0 && keysyms.is_empty() {
            return Ok(());
        }
        self.send(Command::Chord { mask, keysyms }).await
    }

    /// Press at `from`, drag to `to`, release.
    ///
    /// Press, N intermediate motions and release go out as separate
    /// round-trips, spaced by the shared [`drag`] pacing.
    pub(super) async fn drag(
        &self,
        from: (i64, i64),
        to: (i64, i64),
        button: Button,
    ) -> Result<()> {
        self.move_to(from.0, from.1).await?;
        sleep(drag::BEFORE_PRESS).await;
        self.button(button, true).await?;
        sleep(drag::AFTER_PRESS).await;

        for (x, y) in drag::path(from, to) {
            self.move_to(x, y).await?;
            sleep(drag::BETWEEN_STEPS).await;
        }

        sleep(drag::BEFORE_RELEASE).await;
        self.button(button, false).await
    }
}
