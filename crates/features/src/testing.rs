//! Shared test doubles: one per platform seam, so a domain service can be
//! exercised without a desktop. Every one of them answers its contract and
//! nothing more.
//!
//! Reading a logged act back is `nest_rs::testing::LogCapture`, not something
//! here — it is the framework's, it is already thread-local for the same
//! reason, and it hands back structured fields where a rendered line could
//! only be grepped.

use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU8, Ordering};

use nest_rs::core::async_trait;
use platform::chord::{self, Chord};
use platform::clipboard::Clipboard;
use platform::desktop::{AppCatalog, DesktopApp};
use platform::input::{Button, InputBackend, ScrollDirection};
use platform::screen::{Region, ScreenBackend};
use platform::window::{WindowId, WindowInfo, WindowManager};

// --- windows ---------------------------------------------------------------

pub struct NoWindows;

#[async_trait]
impl WindowManager for NoWindows {
    async fn windows(&self) -> anyhow::Result<Vec<WindowInfo>> {
        Ok(Vec::new())
    }

    async fn close(&self, _window: &WindowId) -> anyhow::Result<()> {
        Ok(())
    }
}

pub struct OneWindow(pub i64);

#[async_trait]
impl WindowManager for OneWindow {
    async fn windows(&self) -> anyhow::Result<Vec<WindowInfo>> {
        Ok(vec![WindowInfo {
            id: WindowId::new(0u8, "test-window"),
            app: "test-app".into(),
            title: "Test Window".into(),
            pid: self.0,
            focused: true,
        }])
    }

    async fn close(&self, _window: &WindowId) -> anyhow::Result<()> {
        Ok(())
    }
}

// --- catalogue -------------------------------------------------------------

pub struct EmptyCatalog;

impl AppCatalog for EmptyCatalog {
    fn apps(&self) -> Vec<DesktopApp> {
        Vec::new()
    }

    fn resolve(&self, _exec: &str) -> Option<PathBuf> {
        None
    }
}

// --- screen ----------------------------------------------------------------

/// A solid PNG, which is all any caller here decodes.
fn png(shade: u8) -> Vec<u8> {
    let image = image::RgbImage::from_pixel(32, 32, image::Rgb([shade, shade, shade]));
    let mut out = std::io::Cursor::new(Vec::new());
    image
        .write_to(&mut out, image::ImageFormat::Png)
        .expect("a PNG encodes");
    out.into_inner()
}

/// A screen that answers the same frame every time — a settled desktop.
pub struct StillScreen;

#[async_trait]
impl ScreenBackend for StillScreen {
    async fn capture_png(
        &self,
        _region: Option<Region>,
        _scale: Option<f32>,
    ) -> anyhow::Result<Vec<u8>> {
        Ok(png(7))
    }

    fn geometry(&self) -> Option<(i64, i64)> {
        None
    }
}

/// A screen whose every capture differs wholly from the last — a desktop that
/// never settles, which is the branch a stabilisation deadline exists for.
#[derive(Default)]
pub struct RestlessScreen(AtomicU8);

#[async_trait]
impl ScreenBackend for RestlessScreen {
    async fn capture_png(
        &self,
        _region: Option<Region>,
        _scale: Option<f32>,
    ) -> anyhow::Result<Vec<u8>> {
        Ok(png(self.0.fetch_add(37, Ordering::Relaxed)))
    }

    fn geometry(&self) -> Option<(i64, i64)> {
        Some((1920, 1080))
    }
}

/// A screen that answers the baseline and then goes away.
///
/// The desktop was touched and the watch cannot report on it — the one state
/// where an act must already be on the record.
#[derive(Default)]
pub struct FailingWatch(AtomicU8);

impl FailingWatch {
    /// The same screen with its one good capture already spent — a backend
    /// that cannot capture at all.
    pub fn from_the_first_capture() -> Self {
        Self(AtomicU8::new(1))
    }
}

#[async_trait]
impl ScreenBackend for FailingWatch {
    async fn capture_png(
        &self,
        _region: Option<Region>,
        _scale: Option<f32>,
    ) -> anyhow::Result<Vec<u8>> {
        match self.0.fetch_add(1, Ordering::Relaxed) {
            0 => Ok(png(7)),
            _ => anyhow::bail!("the compositor went away mid-watch"),
        }
    }

    fn geometry(&self) -> Option<(i64, i64)> {
        None
    }
}

// --- input -----------------------------------------------------------------

/// An input backend that accepts everything and remembers the order it was
/// asked in — the only way to tell a click that moved first from one that did
/// not.
#[derive(Default)]
pub struct RecordingInput {
    calls: Mutex<Vec<String>>,
}

impl RecordingInput {
    pub fn calls(&self) -> Vec<String> {
        self.calls.lock().expect("call log poisoned").clone()
    }

    fn note(&self, call: impl Into<String>) {
        self.calls
            .lock()
            .expect("call log poisoned")
            .push(call.into());
    }
}

#[async_trait]
impl InputBackend for RecordingInput {
    async fn warm_up(&self) -> anyhow::Result<()> {
        Ok(())
    }

    async fn ping(&self) -> anyhow::Result<()> {
        Ok(())
    }

    async fn move_to(&self, x: i64, y: i64) -> anyhow::Result<()> {
        self.note(format!("move_to({x},{y})"));
        Ok(())
    }

    async fn click(&self, button: Button) -> anyhow::Result<()> {
        self.note(format!("click({})", button.as_str()));
        Ok(())
    }

    async fn drag(&self, from: (i64, i64), to: (i64, i64), button: Button) -> anyhow::Result<()> {
        self.note(format!(
            "drag({},{}->{},{},{})",
            from.0,
            from.1,
            to.0,
            to.1,
            button.as_str(),
        ));
        Ok(())
    }

    async fn scroll(&self, direction: ScrollDirection, amount: u32) -> anyhow::Result<()> {
        self.note(format!("scroll({},{amount})", direction.as_str()));
        Ok(())
    }

    async fn type_text(&self, text: &str) -> anyhow::Result<()> {
        // The count, never the content: a double that echoed typed text into a
        // failure message would leak it exactly where the service will not.
        self.note(format!("type_text({} chars)", text.chars().count()));
        Ok(())
    }

    fn resolve_chord(&self, keys: &str) -> anyhow::Result<Chord> {
        // The published gate itself, not a stand-in for it: a double that
        // accepted a spelling no desktop serves would let a test prove the
        // opposite of what it claims.
        Ok(Chord::new(chord::normalize(keys, &[])?.join("+")))
    }

    async fn press_chord(&self, chord: Chord) -> anyhow::Result<()> {
        self.note(format!("press_chord({})", chord.take::<String>()?));
        Ok(())
    }
}

// --- clipboard -------------------------------------------------------------

/// A clipboard held in memory.
#[derive(Default)]
pub struct MemoryClipboard {
    text: Mutex<String>,
}

impl MemoryClipboard {
    pub fn holding(text: &str) -> Self {
        Self {
            text: Mutex::new(text.to_owned()),
        }
    }
}

#[async_trait]
impl Clipboard for MemoryClipboard {
    async fn get(&self) -> String {
        self.text.lock().expect("clipboard poisoned").clone()
    }

    async fn set(&self, text: &str) -> anyhow::Result<()> {
        *self.text.lock().expect("clipboard poisoned") = text.to_owned();
        Ok(())
    }
}

/// A clipboard that refuses to be written.
pub struct ReadOnlyClipboard;

#[async_trait]
impl Clipboard for ReadOnlyClipboard {
    async fn get(&self) -> String {
        String::new()
    }

    async fn set(&self, _text: &str) -> anyhow::Result<()> {
        anyhow::bail!("wl-copy exited 1")
    }
}
