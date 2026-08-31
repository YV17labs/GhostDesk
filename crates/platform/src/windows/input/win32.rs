//! The trait answer: [`Win32`], the Windows [`InputBackend`].

use anyhow::{Result, bail};
use async_trait::async_trait;
use tokio::time::sleep;

use ::windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY;

use super::{chord, event};
use crate::chord::Chord;
use crate::input::{Button, Conventions, InputBackend, ScrollDirection, drag};
use crate::windows::display;

/// The [`InputBackend`] the host selector hands out on Windows.
///
/// Stateless, like the macOS backend and unlike the Wayland one: `SendInput`
/// reaches the window manager through the session this process already sits
/// in, so there is no connection to hold open and nothing to hand back if it
/// dies. What has to be true before the first click is a *desktop*, not a
/// handshake — which is what `warm_up` asks about.
pub struct Win32;

/// The payload behind a [`Chord`] this backend resolved: the modifiers held,
/// and the keys tapped under them.
struct Win32Chord {
    held: Vec<VIRTUAL_KEY>,
    keys: Vec<VIRTUAL_KEY>,
}

/// What this desktop calls its shortcuts.
///
/// Control, as on Linux — this is the desktop macOS disagrees with, not the
/// other way round. The same const [`chord`](super::chord) builds its
/// modifier table against, so the value
/// [`host::conventions`](crate::host::conventions) publishes and the value the
/// keyboard presses are the same one.
pub const CONVENTIONS: Conventions = Conventions {
    desktop: "Windows",
    primary_modifier: "ctrl",
};

#[async_trait]
impl InputBackend for Win32 {
    /// Refuse the boot when there is no interactive desktop to drive.
    ///
    /// Two ways to have none, and both are silent otherwise: a server started
    /// as a Windows service lives in session 0, which has no desktop a user
    /// can see, and a session showing the secure desktop belongs to Winlogon
    /// rather than to us. In either case every event is accepted and
    /// discarded, so this is worth a boot failure that says which it is.
    ///
    /// The DPI declaration rides along here because a warm boot is the last
    /// moment it can still be made: it must precede the process's first
    /// device context, and the capture backend is about to create one.
    async fn warm_up(&self) -> Result<()> {
        display::ensure_dpi_aware();

        if !event::on_the_input_desktop() {
            bail!(
                "GhostDesk cannot reach the desktop receiving input, so every mouse \
                 and keyboard event it sends would be silently discarded. Run it as \
                 the signed-in user in an interactive session — a Windows service in \
                 session 0 has no desktop to drive — and note that it reaches nothing \
                 at all while a UAC prompt or the lock screen is in front."
            );
        }
        Ok(())
    }

    /// The same question, for the same reason it was worth a boot failure: the
    /// answer changes underneath a running server. A UAC prompt, a lock, or a
    /// fast user switch replaces the input desktop with one this process may
    /// not open, and nothing in the command path notices — the events keep
    /// being accepted and keep going nowhere.
    async fn ping(&self) -> Result<()> {
        if !event::on_the_input_desktop() {
            bail!(
                "GhostDesk can no longer reach the desktop receiving input — every \
                 mouse and keyboard event it sends is being discarded. A UAC prompt, \
                 the lock screen or a switched user takes the session away from it."
            );
        }
        Ok(())
    }

    async fn move_to(&self, x: i64, y: i64) -> Result<()> {
        event::move_to(x, y)
    }

    async fn click(&self, button: Button) -> Result<()> {
        event::button(button, true)?;
        event::button(button, false)
    }

    /// Press at `from`, drag to `to`, release, on the shared [`drag`] pacing.
    ///
    /// The motion is an ordinary move: Windows has no distinct drag event, so
    /// what makes this a drag rather than a warp is the button held across it
    /// — and the dwell that lets the application's own hit testing notice.
    async fn drag(&self, from: (i64, i64), to: (i64, i64), button: Button) -> Result<()> {
        event::move_to(from.0, from.1)?;
        sleep(drag::BEFORE_PRESS).await;
        event::button(button, true)?;
        sleep(drag::AFTER_PRESS).await;

        for (x, y) in drag::path(from, to) {
            event::move_to(x, y)?;
            sleep(drag::BETWEEN_STEPS).await;
        }

        sleep(drag::BEFORE_RELEASE).await;
        event::button(button, false)
    }

    async fn scroll(&self, direction: ScrollDirection, amount: u32) -> Result<()> {
        for _ in 0..amount {
            event::scroll(direction)?;
        }
        Ok(())
    }

    /// Type text as Unicode, one character per event pair.
    ///
    /// Per character rather than one batch for the whole string: a paragraph
    /// would otherwise be a single `SendInput` array of thousands of entries,
    /// and an application that drops the tail of one is indistinguishable
    /// from one that never received it.
    async fn type_text(&self, text: &str) -> Result<()> {
        for ch in text.chars() {
            event::type_char(ch)?;
        }
        Ok(())
    }

    fn resolve_chord(&self, keys: &str) -> Result<Chord> {
        let (held, keys) = chord::resolve(keys)?;
        Ok(Chord::new(Win32Chord { held, keys }))
    }

    async fn press_chord(&self, chord: Chord) -> Result<()> {
        let Win32Chord { held, keys } = chord.take()?;

        if keys.is_empty() {
            // A chord of modifiers alone (`ctrl+shift`) has nothing to tap.
            // Pressing and releasing them changes nothing observable, so this
            // is a no-op rather than an error — and rather than a press whose
            // release the caller would have to be trusted to send.
            return Ok(());
        }
        event::tap_chord(&held, &keys)
    }
}
