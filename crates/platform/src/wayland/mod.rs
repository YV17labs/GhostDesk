//! Persistent Wayland input controller.
//!
//! Opens a single Wayland connection and reuses it for every pointer and
//! keyboard action for the lifetime of the process. Two unstable protocol
//! extensions are required:
//!
//! - `zwlr_virtual_pointer_manager_v1` — any wlroots-based compositor (Sway,
//!   Hyprland, Wayfire, labwc, river, …).
//! - `zwp_virtual_keyboard_manager_v1` — the same set, plus a few others.
//!
//! Either one missing is a hard error at connect time, so the server fails
//! loudly at boot instead of on the first `mouse_click`.
//!
//! # Why a dedicated thread
//!
//! A Wayland event queue is driven by blocking round-trips and owns
//! `!Sync` dispatch state. Rather than smear that across the async runtime,
//! the connection lives on one OS thread that owns everything and processes
//! commands off a channel, one at a time. That single consumer *is* the
//! serialisation the protocol needs — the ordering guarantee comes from the
//! channel, not from a lock every call site has to remember to take.

pub mod keymap;
pub mod keysym;

use std::io::Write as _;
use std::os::fd::AsFd;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, anyhow};
use tokio::sync::{mpsc, oneshot};
use wayland_client::globals::{GlobalListContents, registry_queue_init};
use wayland_client::protocol::wl_pointer::{Axis, AxisSource, ButtonState};
use wayland_client::protocol::wl_registry::WlRegistry;
use wayland_client::protocol::wl_seat::WlSeat;
use wayland_client::{Connection, Dispatch, EventQueue, QueueHandle, delegate_noop};
use wayland_protocols_misc::zwp_virtual_keyboard_v1::client::zwp_virtual_keyboard_manager_v1::ZwpVirtualKeyboardManagerV1;
use wayland_protocols_misc::zwp_virtual_keyboard_v1::client::zwp_virtual_keyboard_v1::ZwpVirtualKeyboardV1;
use wayland_protocols_wlr::virtual_pointer::v1::client::zwlr_virtual_pointer_manager_v1::ZwlrVirtualPointerManagerV1;
use wayland_protocols_wlr::virtual_pointer::v1::client::zwlr_virtual_pointer_v1::ZwlrVirtualPointerV1;

use crate::coords::{screen_height, screen_width};

// --- Linux input-event-codes.h ------------------------------------------

const BTN_LEFT: u32 = 0x110;
const BTN_RIGHT: u32 = 0x111;
const BTN_MIDDLE: u32 = 0x112;

const KEY_RELEASED: u32 = 0;
const KEY_PRESSED: u32 = 1;

const KEYMAP_FORMAT_XKB_V1: u32 = 1;

/// One wheel notch. wayland-rs converts `wl_fixed` to `f64` at the binding
/// boundary, so this is the notch count itself — not the 1/256 raw units the
/// wire carries.
const WHEEL_STEP: f64 = 10.0;

/// Which pointer button an action uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Button {
    #[default]
    Left,
    Middle,
    Right,
}

impl Button {
    fn code(self) -> u32 {
        match self {
            Self::Left => BTN_LEFT,
            Self::Middle => BTN_MIDDLE,
            Self::Right => BTN_RIGHT,
        }
    }

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
    /// `(axis, wl_fixed motion value, discrete notch count)`.
    ///
    /// The `wl_pointer` protocol requires `sign(value) == sign(discrete)`
    /// within a frame. wlroots exposes both, and Firefox trusts `discrete`
    /// for wheel sources, so a mismatched pair silently inverts the scroll.
    fn vector(self) -> (Axis, f64, i32) {
        match self {
            Self::Up => (Axis::VerticalScroll, -WHEEL_STEP, -1),
            Self::Down => (Axis::VerticalScroll, WHEEL_STEP, 1),
            Self::Left => (Axis::HorizontalScroll, -WHEEL_STEP, -1),
            Self::Right => (Axis::HorizontalScroll, WHEEL_STEP, 1),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Up => "up",
            Self::Down => "down",
            Self::Left => "left",
            Self::Right => "right",
        }
    }
}

/// One unit of work for the Wayland thread. Each variant ends in a
/// round-trip, so the compositor has seen everything by the time the reply
/// comes back.
enum Command {
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

type Job = (Command, oneshot::Sender<Result<()>>);

/// Handle to the Wayland thread. Cheap to clone; every clone drives the same
/// connection.
#[derive(Clone)]
pub struct WaylandInput {
    tx: mpsc::UnboundedSender<Job>,
}

impl WaylandInput {
    /// Connect, bind the two virtual-input protocols, and spawn the thread
    /// that owns them. Returns once the compositor has acknowledged the
    /// bindings, so a missing protocol surfaces here rather than mid-session.
    pub async fn connect() -> Result<Self> {
        let (tx, rx) = mpsc::unbounded_channel::<Job>();
        let (ready_tx, ready_rx) = oneshot::channel::<Result<()>>();

        std::thread::Builder::new()
            .name("ghostdesk-wayland".into())
            .spawn(move || run_thread(rx, ready_tx))
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

    pub async fn move_to(&self, x: i64, y: i64) -> Result<()> {
        self.send(Command::Motion { x, y }).await
    }

    pub async fn button_down(&self, button: Button) -> Result<()> {
        self.send(Command::Button {
            button,
            pressed: true,
        })
        .await
    }

    pub async fn button_up(&self, button: Button) -> Result<()> {
        self.send(Command::Button {
            button,
            pressed: false,
        })
        .await
    }

    /// One press/release pair inside a single round-trip.
    pub async fn click(&self, button: Button) -> Result<()> {
        self.send(Command::Click { button }).await
    }

    pub async fn scroll(&self, direction: ScrollDirection, amount: u32) -> Result<()> {
        self.send(Command::Scroll { direction, amount }).await
    }

    /// Press and release every keysym in order.
    pub async fn type_keysyms(&self, keysyms: Vec<u32>) -> Result<()> {
        if keysyms.is_empty() {
            return Ok(());
        }
        self.send(Command::Type { keysyms }).await
    }

    /// Hold `mask` while tapping each non-modifier keysym.
    pub async fn press_chord(&self, mask: u32, keysyms: Vec<u32>) -> Result<()> {
        if mask == 0 && keysyms.is_empty() {
            return Ok(());
        }
        self.send(Command::Chord { mask, keysyms }).await
    }

    /// Press at `from`, drag to `to`, release.
    ///
    /// Press, N intermediate motions and release go out as separate
    /// round-trips with real time between them. `GtkGestureDrag` only arms
    /// when it sees a press that *persists* while motion arrives; a bundled
    /// press→warp→release burst is read as a plain click, which breaks true
    /// drag-and-drop (moving a selection, a file, a tab) even though it
    /// happens to work for text selection, which only tracks raw
    /// button+position.
    pub async fn drag(&self, from: (i64, i64), to: (i64, i64), button: Button) -> Result<()> {
        use tokio::time::{Duration, sleep};

        const STEPS: i64 = 20;

        self.move_to(from.0, from.1).await?;
        sleep(Duration::from_millis(200)).await;
        self.button_down(button).await?;
        sleep(Duration::from_millis(300)).await;

        for step in 1..=STEPS {
            let progress = step as f64 / STEPS as f64;
            let x = (from.0 as f64 + (to.0 - from.0) as f64 * progress).round() as i64;
            let y = (from.1 as f64 + (to.1 - from.1) as f64 * progress).round() as i64;
            self.move_to(x, y).await?;
            sleep(Duration::from_millis(30)).await;
        }

        sleep(Duration::from_millis(300)).await;
        self.button_up(button).await
    }
}

// --- the thread ---------------------------------------------------------

/// Dispatch state. Neither virtual-input interface emits events, and the
/// seat's capability announcements are irrelevant to a virtual device, so
/// there is nothing to keep here.
struct State;

impl Dispatch<WlRegistry, GlobalListContents> for State {
    fn event(
        _: &mut Self,
        _: &WlRegistry,
        _: <WlRegistry as wayland_client::Proxy>::Event,
        _: &GlobalListContents,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

delegate_noop!(State: ignore WlSeat);
delegate_noop!(State: ignore ZwlrVirtualPointerManagerV1);
delegate_noop!(State: ignore ZwlrVirtualPointerV1);
delegate_noop!(State: ignore ZwpVirtualKeyboardManagerV1);
delegate_noop!(State: ignore ZwpVirtualKeyboardV1);

/// Everything the thread owns for the process lifetime.
struct Session {
    queue: EventQueue<State>,
    pointer: ZwlrVirtualPointerV1,
    keyboard: ZwpVirtualKeyboardV1,
    /// Keysym pool, in keycode-slot order. Slot `i` is evdev keycode `i`.
    keysyms: Vec<u32>,
}

fn now_ms() -> u32 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis() as u32)
        .unwrap_or(0)
}

fn run_thread(mut rx: mpsc::UnboundedReceiver<Job>, ready: oneshot::Sender<Result<()>>) {
    let mut session = match Session::open() {
        Ok(session) => {
            let _ = ready.send(Ok(()));
            session
        }
        Err(err) => {
            let _ = ready.send(Err(err));
            return;
        }
    };

    while let Some((command, reply)) = rx.blocking_recv() {
        let _ = reply.send(session.execute(command));
    }
}

impl Session {
    fn open() -> Result<Self> {
        let connection =
            Connection::connect_to_env().context("connecting to the Wayland display")?;
        let (globals, mut queue) =
            registry_queue_init::<State>(&connection).context("initialising the registry")?;
        let qh = queue.handle();

        let seat = globals
            .bind::<WlSeat, _, _>(&qh, 1..=7, ())
            .map_err(|err| missing("wl_seat", err))?;
        let pointer_manager = globals
            .bind::<ZwlrVirtualPointerManagerV1, _, _>(&qh, 1..=2, ())
            .map_err(|err| missing("zwlr_virtual_pointer_manager_v1", err))?;
        let keyboard_manager = globals
            .bind::<ZwpVirtualKeyboardManagerV1, _, _>(&qh, 1..=1, ())
            .map_err(|err| missing("zwp_virtual_keyboard_manager_v1", err))?;

        let pointer = pointer_manager.create_virtual_pointer(Some(&seat), &qh, ());
        let keyboard = keyboard_manager.create_virtual_keyboard(&seat, &qh, ());
        queue.roundtrip(&mut State)?;

        let mut session = Self {
            queue,
            pointer,
            keyboard,
            keysyms: Vec::new(),
        };

        // Upload an empty keymap so the virtual keyboard is in a valid state
        // before the first `key_type`; real keymaps are pushed lazily, once
        // we know which keysyms the session actually needs.
        session.upload_keymap()?;
        Ok(session)
    }

    fn flush(&mut self) -> Result<()> {
        self.queue.roundtrip(&mut State)?;
        Ok(())
    }

    fn upload_keymap(&mut self) -> Result<()> {
        let mut serialised = keymap::build_keymap(&self.keysyms).into_bytes();
        // The compositor's parser expects a NUL-terminated buffer, and the
        // advertised size counts it.
        serialised.push(0);

        let fd = rustix::fs::memfd_create("ghostdesk-keymap", rustix::fs::MemfdFlags::CLOEXEC)
            .context("creating the keymap memfd")?;
        let mut file = std::fs::File::from(fd);
        file.write_all(&serialised)
            .context("writing the keymap memfd")?;

        self.keyboard
            .keymap(KEYMAP_FORMAT_XKB_V1, file.as_fd(), serialised.len() as u32);
        self.flush()
    }

    /// Assign a keycode to any unseen keysym, re-uploading the keymap when
    /// the pool grew. Returns the evdev keycode for each input keysym, in
    /// order.
    fn ensure_keysyms(&mut self, keysyms: &[u32]) -> Result<Vec<u32>> {
        let mut grew = false;
        for keysym in keysyms {
            if !self.keysyms.contains(keysym) {
                self.keysyms.push(*keysym);
                grew = true;
            }
        }
        if grew {
            self.upload_keymap()?;
        }
        Ok(keysyms
            .iter()
            .map(|keysym| {
                self.keysyms
                    .iter()
                    .position(|known| known == keysym)
                    .expect("just inserted") as u32
            })
            .collect())
    }

    fn motion(&self, x: i64, y: i64) {
        self.pointer.motion_absolute(
            now_ms(),
            x.max(0) as u32,
            y.max(0) as u32,
            screen_width() as u32,
            screen_height() as u32,
        );
        self.pointer.frame();
    }

    fn button(&self, button: Button, state: ButtonState) {
        self.pointer.button(now_ms(), button.code(), state);
        self.pointer.frame();
    }

    fn execute(&mut self, command: Command) -> Result<()> {
        match command {
            Command::Motion { x, y } => self.motion(x, y),
            Command::Button { button, pressed } => self.button(
                button,
                if pressed {
                    ButtonState::Pressed
                } else {
                    ButtonState::Released
                },
            ),
            Command::Click { button } => {
                self.button(button, ButtonState::Pressed);
                self.button(button, ButtonState::Released);
            }
            Command::Scroll { direction, amount } => {
                let (axis, value, discrete) = direction.vector();
                for _ in 0..amount {
                    // `axis_discrete` is self-contained: wlroots unpacks it
                    // into both the continuous delta and the step count, so a
                    // separate `axis` call would only overwrite the delta
                    // with the same value.
                    self.pointer.axis_source(AxisSource::Wheel);
                    self.pointer.axis_discrete(now_ms(), axis, value, discrete);
                    self.pointer.frame();
                }
            }
            Command::Type { keysyms } => {
                for keycode in self.ensure_keysyms(&keysyms)? {
                    self.keyboard.key(now_ms(), keycode, KEY_PRESSED);
                    self.keyboard.key(now_ms(), keycode, KEY_RELEASED);
                }
            }
            Command::Chord { mask, keysyms } => {
                let keycodes = self.ensure_keysyms(&keysyms)?;
                if mask != 0 {
                    self.keyboard.modifiers(mask, 0, 0, 0);
                }
                for keycode in keycodes {
                    self.keyboard.key(now_ms(), keycode, KEY_PRESSED);
                    self.keyboard.key(now_ms(), keycode, KEY_RELEASED);
                }
                if mask != 0 {
                    self.keyboard.modifiers(0, 0, 0, 0);
                }
            }
        }
        self.flush()
    }
}

fn missing(interface: &str, err: impl std::fmt::Display) -> anyhow::Error {
    anyhow!(
        "Wayland compositor is missing {interface} ({err}). GhostDesk needs \
         wl_seat, zwlr_virtual_pointer_manager_v1 (any wlroots compositor: \
         Sway, Hyprland, Wayfire, labwc, river) and \
         zwp_virtual_keyboard_manager_v1."
    )
}
