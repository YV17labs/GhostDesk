//! The input contract every backend implements, plus the neutral vocabulary
//! (`Button`, `ScrollDirection`) the feature layer speaks.
//!
//! Nothing here names a protocol or an OS. The Wayland backend presses evdev
//! keycodes through a virtual keyboard; a macOS backend posts `CGEvent`s; the
//! feature crate cannot tell the difference — that opacity is what makes the
//! next OS a new directory under this crate instead of a sweep through
//! `features`.

use std::any::Any;

use anyhow::{Result, bail};
use async_trait::async_trait;

/// Which pointer button an action uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Button {
    #[default]
    Left,
    Middle,
    Right,
}

impl Button {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Left => "left",
            Self::Middle => "middle",
            Self::Right => "right",
        }
    }
}

/// Which way a scroll goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ScrollDirection {
    Up,
    #[default]
    Down,
    Left,
    Right,
}

impl ScrollDirection {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Up => "up",
            Self::Down => "down",
            Self::Left => "left",
            Self::Right => "right",
        }
    }
}

/// A chord resolved by the backend that will press it.
///
/// The payload is opaque on purpose: on Wayland a chord is an XKB modifier
/// mask plus keysyms, on macOS it would be `(CGEventFlags, CGKeyCode)` — no
/// neutral encoding covers both without lying to one of them. Only the
/// backend that produced a `Chord` can consume it, and only one backend ever
/// exists per process.
pub struct Chord(Box<dyn Any + Send>);

impl Chord {
    pub fn new(payload: impl Any + Send) -> Self {
        Self(Box::new(payload))
    }

    /// Recover the payload.
    ///
    /// The error means a backend was handed a chord it did not resolve — a
    /// programming error, not a runtime condition. The diagnostic lives here
    /// rather than in each backend so every implementation reports it the
    /// same way without having to reword it.
    pub fn take<T: Any>(self) -> Result<T> {
        match self.0.downcast::<T>() {
            Ok(payload) => Ok(*payload),
            Err(_) => bail!("chord was resolved by a different input backend"),
        }
    }
}

/// How a drag is paced, shared by every backend.
///
/// The numbers are not arbitrary and they are not per-OS. A gesture
/// recogniser only arms when it sees a press that *persists* while motion
/// arrives, so a bundled press→warp→release burst is read as a plain click
/// and true drag-and-drop (moving a selection, a file, a tab) never starts —
/// even though it happens to work for text selection, which only tracks raw
/// button+position. Every desktop tested so far needs the same dwell, so the
/// pacing lives here and a backend supplies only the three verbs.
pub mod drag {
    use std::time::Duration;

    /// Dwell after arriving at the start point, before pressing.
    pub const BEFORE_PRESS: Duration = Duration::from_millis(200);
    /// Dwell with the button held, before the first motion.
    pub const AFTER_PRESS: Duration = Duration::from_millis(300);
    /// Gap between two waypoints.
    pub const BETWEEN_STEPS: Duration = Duration::from_millis(30);
    /// Dwell at the destination, before releasing.
    pub const BEFORE_RELEASE: Duration = Duration::from_millis(300);

    /// Waypoints between `from` and `to`, excluding the start and including
    /// the destination.
    pub fn path(from: (i64, i64), to: (i64, i64)) -> impl Iterator<Item = (i64, i64)> {
        const STEPS: i64 = 20;
        (1..=STEPS).map(move |step| {
            let progress = step as f64 / STEPS as f64;
            (
                (from.0 as f64 + (to.0 - from.0) as f64 * progress).round() as i64,
                (from.1 as f64 + (to.1 - from.1) as f64 * progress).round() as i64,
            )
        })
    }
}

/// Split a `+`-separated chord into normalised tokens, applying `aliases`.
///
/// One definition rather than a convention two backends happen to agree on:
/// the grammar is published to callers, so it cannot be per-OS. The alias
/// *tables* are deliberately not shared: `cmd` means
/// Super on Linux and Command on macOS, and merging them would reintroduce
/// exactly the bug [`Conventions`] exists to prevent.
pub fn normalize_chord(keys: &str, aliases: &[(&str, &str)]) -> Vec<String> {
    keys.split('+')
        .filter(|token| !token.trim().is_empty())
        .map(|token| {
            let key = token.trim().to_lowercase();
            aliases
                .iter()
                .find_map(|(from, to)| (*from == key).then(|| (*to).to_string()))
                .unwrap_or(key)
        })
        .collect()
}

/// The shortcut vocabulary of a desktop, as the agent must be told it.
///
/// This is the one place the port reaches the agent contract, and it is not
/// cosmetic: on macOS copy is `cmd+c`, and an agent told to send `ctrl+c`
/// types a control character into the document instead of copying. Every
/// other difference between the backends is invisible above `platform`;
/// this one has to travel all the way out to the model, so it is a value the
/// backend states rather than a string the feature layer guesses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Conventions {
    /// The desktop the agent is driving, in the words it should think in.
    pub desktop: &'static str,
    /// The modifier the standard shortcuts hang off — `ctrl` or `cmd`.
    pub primary_modifier: &'static str,
}

impl Conventions {
    /// The chord for one shortcut on the primary modifier: `copy` → `cmd+c`.
    pub fn shortcut(&self, key: &str) -> String {
        format!("{}+{key}", self.primary_modifier)
    }
}

/// Mouse and keyboard control, one implementation per OS.
///
/// Coordinates are physical screen pixels: the model-space conversion has
/// already happened by the time a backend sees them.
#[async_trait]
pub trait InputBackend: Send + Sync {
    /// How this desktop spells its standard shortcuts.
    fn conventions(&self) -> Conventions;

    /// Open whatever the backend needs to open, once, and report whether it
    /// worked. Idempotent: a caller may bind eagerly at start-up so a missing
    /// protocol or permission is known before an agent's first click, and what
    /// it does with a refusal is its own decision — this seam states the fact,
    /// never the policy.
    async fn warm_up(&self) -> Result<()>;

    /// Is what [`warm_up`](Self::warm_up) established *still* true?
    ///
    /// Separate from `warm_up` because that one only has to succeed once and
    /// answers from what it opened: a desktop can revoke underneath a bound
    /// backend — a compositor restarts and leaves the connection dead, a
    /// permission is withdrawn — and nothing in the command path notices
    /// until an agent's click is swallowed. Neither condition can be repaired
    /// in place, so this is what a liveness probe calls to have the process
    /// replaced instead. It must not move the cursor, press a key, or open
    /// what was never opened: a probe reports the state, it does not create
    /// it.
    async fn ping(&self) -> Result<()>;

    /// Move the cursor without pressing anything.
    async fn move_to(&self, x: i64, y: i64) -> Result<()>;

    /// One press/release pair at the current cursor position.
    async fn click(&self, button: Button) -> Result<()>;

    /// Press at `from`, drag to `to`, release. Pacing belongs to the
    /// backend: how slowly a drag must move for the OS's gesture recognisers
    /// to arm is a platform property, not a feature decision.
    async fn drag(&self, from: (i64, i64), to: (i64, i64), button: Button) -> Result<()>;

    /// Scroll `amount` wheel notches under the cursor.
    async fn scroll(&self, direction: ScrollDirection, amount: u32) -> Result<()>;

    /// Type text at the current keyboard focus.
    ///
    /// Text, not keycodes: the Wayland backend maps chars to keysyms through
    /// its own XKB keymap, macOS injects the string directly — which encoding
    /// exists at all is a backend detail.
    async fn type_text(&self, text: &str) -> Result<()>;

    /// Resolve a `+`-separated chord (`ctrl+shift+t`) without pressing it.
    ///
    /// Split from [`press_chord`](Self::press_chord) so an unknown key name
    /// fails before the caller has paid for a baseline screenshot.
    fn resolve_chord(&self, keys: &str) -> Result<Chord>;

    /// Press a chord previously resolved by *this* backend.
    async fn press_chord(&self, chord: Chord) -> Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALIASES: &[(&str, &str)] = &[("return", "enter")];

    #[test]
    fn splits_a_chord_dropping_blanks_and_applying_aliases() {
        assert_eq!(
            normalize_chord("Ctrl+Shift+Return", ALIASES),
            vec!["ctrl", "shift", "enter"],
        );
        assert_eq!(normalize_chord("ctrl++a", ALIASES), vec!["ctrl", "a"]);
        assert_eq!(normalize_chord(" ", ALIASES), Vec::<String>::new());
    }

    #[test]
    fn a_drag_path_ends_on_the_destination() {
        let points: Vec<_> = drag::path((0, 0), (100, 50)).collect();
        assert_eq!(points.len(), 20);
        assert_eq!(points[0], (5, 3), "the start point is not re-emitted");
        assert_eq!(*points.last().unwrap(), (100, 50));
    }

    #[test]
    fn a_chord_handed_to_the_wrong_backend_says_so() {
        let chord = Chord::new(42u32);
        assert!(
            chord
                .take::<String>()
                .unwrap_err()
                .to_string()
                .contains("different input backend")
        );
    }
}
