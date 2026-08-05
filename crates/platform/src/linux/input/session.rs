//! What the Wayland thread owns: the event queue, the virtual pointer and
//! the virtual keyboard, for the lifetime of the process.
//!
//! Two unstable protocol extensions are required:
//!
//! - `zwlr_virtual_pointer_manager_v1` — any wlroots-based compositor (Sway,
//!   Hyprland, Wayfire, labwc, river, …).
//! - `zwp_virtual_keyboard_manager_v1` — the same set, plus a few others.
//!
//! Either one missing is a hard error at connect time, so the server fails
//! loudly at boot instead of on the first `mouse_click`.

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

use super::connection::{Command, Job};
use super::keymap;
use crate::coords::{screen_height, screen_width};
use crate::input::{Button, ScrollDirection};

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

/// The evdev button code for a neutral [`Button`].
fn button_code(button: Button) -> u32 {
    match button {
        Button::Left => BTN_LEFT,
        Button::Middle => BTN_MIDDLE,
        Button::Right => BTN_RIGHT,
    }
}

/// `(axis, wl_fixed motion value, discrete notch count)`.
///
/// The `wl_pointer` protocol requires `sign(value) == sign(discrete)`
/// within a frame. wlroots exposes both, and Firefox trusts `discrete`
/// for wheel sources, so a mismatched pair silently inverts the scroll.
fn scroll_vector(direction: ScrollDirection) -> (Axis, f64, i32) {
    match direction {
        ScrollDirection::Up => (Axis::VerticalScroll, -WHEEL_STEP, -1),
        ScrollDirection::Down => (Axis::VerticalScroll, WHEEL_STEP, 1),
        ScrollDirection::Left => (Axis::HorizontalScroll, -WHEEL_STEP, -1),
        ScrollDirection::Right => (Axis::HorizontalScroll, WHEEL_STEP, 1),
    }
}

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

/// The thread body: open a session, report readiness, then serve commands
/// until the channel closes.
pub(super) fn run(mut rx: mpsc::UnboundedReceiver<Job>, ready: oneshot::Sender<Result<()>>) {
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
        self.pointer.button(now_ms(), button_code(button), state);
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
                let (axis, value, discrete) = scroll_vector(direction);
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
