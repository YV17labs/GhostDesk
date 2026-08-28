//! The trait answer: [`Wayland`], the Linux [`InputBackend`].

use anyhow::{Result, bail};
use async_trait::async_trait;
use tokio::sync::OnceCell;

use super::connection::Connection;
use super::{chord, keysym};
use crate::chord::Chord;
use crate::input::{Button, Conventions, InputBackend, ScrollDirection};

/// The [`InputBackend`] the host selector hands out on Linux.
///
/// Construction is cheap and synchronous (the container builds providers
/// synchronously); the connection opens on [`warm_up`](InputBackend::warm_up).
/// A `OnceCell` rather than a plain field because the boot hook and a racing
/// first tool call must not be able to open two connections.
#[derive(Default)]
pub struct Wayland {
    connection: OnceCell<Connection>,
}

/// The payload behind a [`Chord`] this backend resolved: the XKB modifier
/// mask and the non-modifier keysyms it holds down.
struct WaylandChord {
    mask: u32,
    keysyms: Vec<u32>,
}

impl Wayland {
    async fn connection(&self) -> Result<&Connection> {
        self.connection.get_or_try_init(Connection::open).await
    }
}

/// What this desktop calls its shortcuts.
///
/// The same const [`chord`](super::chord) builds its modifier table against,
/// so the value [`host::conventions`](crate::host::conventions) publishes at
/// composition time and the value the keyboard presses are the same one.
pub const CONVENTIONS: Conventions = Conventions {
    desktop: "Linux (Wayland)",
    primary_modifier: "ctrl",
};

#[async_trait]
impl InputBackend for Wayland {
    async fn warm_up(&self) -> Result<()> {
        self.connection().await?;
        Ok(())
    }

    /// Reports on the connection as it stands; never opens one. A probe that
    /// connected would answer "up" for a server whose boot never bound
    /// anything, which is the one state worth hearing about.
    async fn ping(&self) -> Result<()> {
        match self.connection.get() {
            Some(connection) => connection.ping().await,
            None => bail!("the input backend is not bound to a compositor"),
        }
    }

    async fn move_to(&self, x: i64, y: i64) -> Result<()> {
        self.connection().await?.move_to(x, y).await
    }

    async fn click(&self, button: Button) -> Result<()> {
        self.connection().await?.click(button).await
    }

    async fn drag(&self, from: (i64, i64), to: (i64, i64), button: Button) -> Result<()> {
        self.connection().await?.drag(from, to, button).await
    }

    async fn scroll(&self, direction: ScrollDirection, amount: u32) -> Result<()> {
        self.connection().await?.scroll(direction, amount).await
    }

    /// Text is mapped to keysyms through GhostDesk's own XKB keymap, never
    /// the compositor's — a French AZERTY host and a US QWERTY one produce
    /// byte-identical output.
    async fn type_text(&self, text: &str) -> Result<()> {
        let keysyms: Vec<u32> = text.chars().map(keysym::keysym_for_char).collect();
        self.connection().await?.type_keysyms(keysyms).await
    }

    fn resolve_chord(&self, keys: &str) -> Result<Chord> {
        let (mask, keysyms) = chord::resolve(keys)?;
        Ok(Chord::new(WaylandChord { mask, keysyms }))
    }

    async fn press_chord(&self, chord: Chord) -> Result<()> {
        let WaylandChord { mask, keysyms } = chord.take()?;
        self.connection().await?.press_chord(mask, keysyms).await
    }
}
