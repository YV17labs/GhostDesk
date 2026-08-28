//! The input contract every backend implements, plus the neutral vocabulary
//! (`Button`, `ScrollDirection`) the feature layer speaks.
//!
//! Nothing here names a protocol or an OS. The Wayland backend presses evdev
//! keycodes through a virtual keyboard; the macOS backend posts `CGEvent`s; the
//! feature crate cannot tell the difference — that opacity is what makes the
//! next OS a new directory under this crate instead of a sweep through
//! `features`. The chord grammar the two share is [`chord`](crate::chord).

use anyhow::Result;
use async_trait::async_trait;

use crate::chord::Chord;

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

/// The shortcut vocabulary of a desktop, as the agent must be told it.
///
/// This is the one place the port reaches the agent contract, and it is not
/// cosmetic: on macOS copy is `cmd+c`, and an agent told to send `ctrl+c`
/// types a control character into the document instead of copying. Every
/// other difference between the backends is invisible above `platform`;
/// this one has to travel all the way out to the model, so it is a value
/// [`host::conventions`](crate::host::conventions) states rather than a string
/// the feature layer guesses.
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

    #[test]
    fn a_drag_path_ends_on_the_destination() {
        let points: Vec<_> = drag::path((0, 0), (100, 50)).collect();
        assert_eq!(points.len(), 20);
        assert_eq!(points[0], (5, 3), "the start point is not re-emitted");
        assert_eq!(*points.last().unwrap(), (100, 50));
    }

    #[test]
    fn a_shortcut_hangs_off_the_desktops_own_modifier() {
        let mac = Conventions {
            desktop: "macOS",
            primary_modifier: "cmd",
        };
        assert_eq!(mac.shortcut("c"), "cmd+c");
    }
}
