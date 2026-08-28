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
//! loudly at boot instead of on the first click.

use std::io::Write as _;
use std::os::fd::AsFd;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

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
use crate::coords::screen;
use crate::input::{Button, ScrollDirection};

// --- Linux input-event-codes.h ------------------------------------------

const BTN_LEFT: u32 = 0x110;
const BTN_RIGHT: u32 = 0x111;
const BTN_MIDDLE: u32 = 0x112;

const KEY_RELEASED: u32 = 0;
const KEY_PRESSED: u32 = 1;

const KEYMAP_FORMAT_XKB_V1: u32 = 1;

/// How long a compositor that is not there *yet* is given to appear.
///
/// A supervisor starts the compositor and this server in the same breath, so
/// the boot hook can reach its connect while the display socket is still
/// being bound — the same tenth of a second, on a cold desktop. Without this
/// window that ordering detail is a crashed boot, and the supervisor's answer
/// to a crashed boot is to start the whole process again: the wait costs one
/// retry loop, its absence costs a restart cycle.
///
/// Only the connect is retried. Everything past it stays fatal on the first
/// attempt — a compositor that answers without carrying the virtual-input
/// protocols is a misdeployment, and no amount of waiting turns it into a
/// desktop this server can drive.
const CONNECT_DEADLINE: Duration = Duration::from_secs(30);

/// Gap between two connect attempts.
const CONNECT_RETRY: Duration = Duration::from_millis(250);

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

/// Connect to the compositor, retrying while the display is merely absent.
///
/// The retry is deliberately blind to *why* the connect failed: the client
/// library reports "no compositor" for a socket that does not exist yet and
/// for one that will never exist, and guessing which from the message would
/// be reading a string the library is free to change. So the first failure is
/// logged with its reason — an operator watching a boot sees immediately
/// whether the display name is wrong — and the deadline is what decides.
fn connect_within(deadline: Duration) -> Result<Connection> {
    retry_connect(deadline, Connection::connect_to_env)
}

/// The wait itself, over the connector it is handed rather than over the
/// process environment.
///
/// What it promises — one more attempt after the first failure, and a deadline
/// that ends it — is provable against a socket the caller controls. Pointing
/// `WAYLAND_DISPLAY` at a fake display would mean writing state every other
/// test in the process shares, to prove a loop that never needed to read it.
fn retry_connect<C, E>(deadline: Duration, connect: C) -> Result<Connection>
where
    C: Fn() -> std::result::Result<Connection, E>,
    E: std::error::Error + Send + Sync + 'static,
{
    let started = Instant::now();
    let mut attempts: u32 = 0;

    loop {
        attempts += 1;
        match connect() {
            Ok(connection) => {
                if attempts > 1 {
                    tracing::info!(
                        target: "platform::input",
                        attempts,
                        waited_ms = started.elapsed().as_millis() as u64,
                        "Wayland display appeared",
                    );
                }
                return Ok(connection);
            }
            Err(err) if started.elapsed() + CONNECT_RETRY < deadline => {
                if attempts == 1 {
                    tracing::info!(
                        target: "platform::input",
                        deadline_secs = deadline.as_secs(),
                        error = %err,
                        "waiting for the Wayland display",
                    );
                }
                std::thread::sleep(CONNECT_RETRY);
            }
            Err(err) => {
                return Err(anyhow::Error::new(err).context(format!(
                    "no Wayland display answered within {}s",
                    deadline.as_secs(),
                )));
            }
        }
    }
}

impl Session {
    fn open() -> Result<Self> {
        let connection = connect_within(CONNECT_DEADLINE)?;
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
        // before the first keystroke; real keymaps are pushed lazily, once
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
        let (width, height) = screen();
        self.pointer.motion_absolute(
            now_ms(),
            x.max(0) as u32,
            y.max(0) as u32,
            width as u32,
            height as u32,
        );
        self.pointer.frame();
    }

    fn button(&self, button: Button, state: ButtonState) {
        self.pointer.button(now_ms(), button_code(button), state);
        self.pointer.frame();
    }

    fn execute(&mut self, command: Command) -> Result<()> {
        match command {
            // Nothing to send: the round-trip every command ends on *is* the
            // check, and it is the only one that touches the socket without
            // touching the desktop.
            Command::Ping => {}
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

#[cfg(test)]
mod tests {
    use std::os::unix::net::{UnixListener, UnixStream};

    use super::*;

    /// Both halves of the wait in one test: the deadline that ends it when
    /// nothing answers, and the first successful connect, which ends it at once.
    #[test]
    fn the_wait_ends_when_a_display_appears_and_not_before() {
        let dir = std::env::temp_dir().join(format!("ghostdesk-probe-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a directory for the fake display");
        let socket = dir.join("wayland-probe");
        let _ = std::fs::remove_file(&socket);

        // The same two steps `connect_to_env` performs once it has read the
        // environment — which is the half this test deliberately replaces.
        let connect = || {
            let stream = UnixStream::connect(&socket)?;
            Connection::from_socket(stream).map_err(std::io::Error::other)
        };

        // Nothing there: the deadline is what ends it, not the first failure.
        // A boot that gave up on attempt one is the bug this exists to close.
        let started = Instant::now();
        let err = retry_connect(Duration::from_millis(600), connect).expect_err("no display");
        let waited = started.elapsed();
        assert!(
            waited >= CONNECT_RETRY,
            "gave up before retrying once, after {waited:?}",
        );
        assert!(
            format!("{err:#}").contains("no Wayland display answered"),
            "the failure names the deadline it spent: {err:#}",
        );

        // Present: the loop stops at the connect. A listener is not a
        // compositor and the bind that follows will fail — deliberately not
        // this function's business, which is the split the retry rests on.
        let _listener = UnixListener::bind(&socket).expect("bind the fake display");
        assert!(
            retry_connect(Duration::from_millis(200), connect).is_ok(),
            "a display that accepts a connection has to end the wait",
        );

        let _ = std::fs::remove_file(&socket);
    }
}
