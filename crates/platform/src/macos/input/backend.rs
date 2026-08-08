//! The trait answer: [`Quartz`], the macOS [`InputBackend`].

use anyhow::{Result, bail};
use async_trait::async_trait;
use objc2_core_graphics::{CGEventFlags, CGKeyCode};
use tokio::time::sleep;

use super::{chord, event};
use crate::input::{Button, Chord, Conventions, InputBackend, ScrollDirection, drag};

/// The [`InputBackend`] the host selector hands out on macOS.
///
/// Stateless, unlike the Wayland backend: there is no connection to hold
/// open, because `CGEventPost` talks to the window server through the
/// process's existing session. What has to be true before the first click is
/// a *permission*, not a handshake — which is what `warm_up` checks.
pub struct Quartz;

/// The payload behind a [`Chord`] this backend resolved: the modifier flags
/// and the keycodes tapped under them.
struct QuartzChord {
    flags: CGEventFlags,
    keys: Vec<CGKeyCode>,
}

/// What this desktop calls its shortcuts.
///
/// Command, not Control. This one value is the whole reason `Conventions`
/// exists. A const, and the trait method below returns it, so the value
/// [`host::conventions`](crate::host::conventions) reads at composition time
/// and the value the keyboard presses are the same one.
pub const CONVENTIONS: Conventions = Conventions {
    desktop: "macOS",
    primary_modifier: "cmd",
};

#[async_trait]
impl InputBackend for Quartz {
    fn conventions(&self) -> Conventions {
        CONVENTIONS
    }

    /// Refuse the boot when the process is not trusted for Accessibility.
    ///
    /// TCC cannot be granted from code — it is a decision the user makes in
    /// System Settings about *this binary*, and it survives neither a rebuild
    /// with a different signature nor a move to a different path. Checking it
    /// here turns a silent no-op (every `CGEventPost` accepted and discarded)
    /// into a boot failure that says what to click.
    async fn warm_up(&self) -> Result<()> {
        if !event::is_trusted() {
            bail!(
                "GhostDesk is not trusted for Accessibility, so every mouse and \
                 keyboard event it posts would be silently discarded. Grant it in \
                 System Settings ▸ Privacy & Security ▸ Accessibility, then \
                 restart the server. Note the permission is tied to this exact \
                 binary — rebuilding or moving it revokes the grant."
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
    async fn drag(&self, from: (i64, i64), to: (i64, i64), button: Button) -> Result<()> {
        event::move_to(from.0, from.1)?;
        sleep(drag::BEFORE_PRESS).await;
        event::button(button, true)?;
        sleep(drag::AFTER_PRESS).await;

        for (x, y) in drag::path(from, to) {
            event::drag_to(x, y, button)?;
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

    /// Type text as Unicode, one character per event.
    ///
    /// Per character rather than one event carrying the whole string: many
    /// apps read only the first few UTF-16 units off a keyboard event, and a
    /// silently truncated paragraph is worse than a slower one.
    async fn type_text(&self, text: &str) -> Result<()> {
        for ch in text.chars() {
            event::type_char(ch)?;
        }
        Ok(())
    }

    fn resolve_chord(&self, keys: &str) -> Result<Chord> {
        let (flags, keys) = chord::resolve(keys)?;
        Ok(Chord::new(QuartzChord { flags, keys }))
    }

    async fn press_chord(&self, chord: Chord) -> Result<()> {
        let QuartzChord { flags, keys } = chord.take()?;

        if keys.is_empty() {
            // A chord of modifiers alone (`cmd+shift`) has nothing to tap.
            // Holding flags with no key does nothing observable, so this is a
            // no-op rather than an error.
            return Ok(());
        }
        for key in keys {
            event::tap_key(key, flags)?;
        }
        Ok(())
    }
}
