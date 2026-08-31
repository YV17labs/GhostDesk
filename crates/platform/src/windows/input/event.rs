//! `SendInput` construction and posting — the only unsafe in the backend.
//!
//! Every function here is synchronous and self-contained: the `INPUT` array is
//! built on the stack, handed to `SendInput` and dropped before returning.
//! Nothing outlives the call, so no raw handle can be captured by an `async`
//! future above.
//!
//! Events are injected at the bottom of the stack, so they behave like
//! hardware: every hook, every application and the window manager itself see
//! them. What they cannot reach is a window running at a higher integrity
//! level than this process — Windows refuses that silently, which is why
//! [`on_the_input_desktop`] exists.
//!
//! # One chord, one call
//!
//! Windows has no equivalent of Quartz's modifier flags stamped on an event:
//! a modifier is an ordinary key that has to be pressed before the chord and
//! released after it. A press and its release in two separate `SendInput`
//! calls can be separated by a failure, and a modifier left held down
//! corrupts every keystroke the user types afterwards. So a whole chord —
//! modifiers down, keys tapped, modifiers up — is one array in one call, and
//! the release cannot be lost without the press being lost with it.

#![expect(
    unsafe_code,
    reason = "SendInput and the desktop handles are C; each call carries its own SAFETY note"
)]

use anyhow::{Result, bail};

use ::windows::Win32::System::StationsAndDesktops::{
    CloseDesktop, DESKTOP_CONTROL_FLAGS, DESKTOP_READOBJECTS, OpenInputDesktop,
};
use ::windows::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBD_EVENT_FLAGS, KEYBDINPUT, KEYEVENTF_KEYUP,
    KEYEVENTF_UNICODE, MOUSE_EVENT_FLAGS, MOUSEEVENTF_ABSOLUTE, MOUSEEVENTF_HWHEEL,
    MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP,
    MOUSEEVENTF_MOVE, MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP, MOUSEEVENTF_WHEEL, SendInput,
    VIRTUAL_KEY,
};

use crate::input::{Button, ScrollDirection};
use crate::windows::display;

/// One wheel notch, as every Windows application expects to be told it.
const WHEEL_DELTA: i32 = 120;

/// What a refused injection means.
///
/// `SendInput` reports a count, never a reason, and in practice there is one
/// cause: the target window sits at a higher integrity level than this
/// process, or the secure desktop has taken over the session.
const BLOCKED: &str = "the OS refused the input event — GhostDesk cannot drive a window \
                       running at a higher integrity level than itself, and reaches \
                       nothing at all while the secure desktop (a UAC prompt, or the \
                       lock screen) is in front";

/// Hand a whole batch to the input stream, all or nothing.
fn send(inputs: &[INPUT]) -> Result<()> {
    // SAFETY: `inputs` is a live slice for the duration of the call and its
    // element size is passed alongside it, which is the whole contract.
    let sent = unsafe { SendInput(inputs, size_of::<INPUT>() as i32) };
    if sent as usize != inputs.len() {
        bail!("{BLOCKED}");
    }
    Ok(())
}

fn mouse(dx: i32, dy: i32, flags: MOUSE_EVENT_FLAGS, data: i32) -> INPUT {
    INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: ::windows::Win32::UI::Input::KeyboardAndMouse::MOUSEINPUT {
                dx,
                dy,
                // Signed on the wire and unsigned in the binding: a negative
                // wheel notch is the same bits either way, and this is the one
                // place that has to know it.
                mouseData: data as u32,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

fn key(virtual_key: VIRTUAL_KEY, scan: u16, flags: KEYBD_EVENT_FLAGS) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: virtual_key,
                wScan: scan,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

/// `(down, up)` for one pointer button.
fn button_flags(button: Button) -> (MOUSE_EVENT_FLAGS, MOUSE_EVENT_FLAGS) {
    match button {
        Button::Left => (MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP),
        Button::Right => (MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP),
        Button::Middle => (MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP),
    }
}

/// `(axis flag, signed notch)` for one scroll step.
///
/// Windows signs the vertical axis the way the wheel turns — positive is away
/// from the user — and the horizontal axis the way the *content* moves, so
/// positive is to the right. The two disagree, which is a Win32 wart and not
/// a mistake here; macOS signs both the first way, and this is the file where
/// the difference is absorbed.
fn scroll_step(direction: ScrollDirection) -> (MOUSE_EVENT_FLAGS, i32) {
    match direction {
        ScrollDirection::Up => (MOUSEEVENTF_WHEEL, WHEEL_DELTA),
        ScrollDirection::Down => (MOUSEEVENTF_WHEEL, -WHEEL_DELTA),
        ScrollDirection::Right => (MOUSEEVENTF_HWHEEL, WHEEL_DELTA),
        ScrollDirection::Left => (MOUSEEVENTF_HWHEEL, -WHEEL_DELTA),
    }
}

/// Move the cursor to a pixel coordinate.
///
/// This is also what a drag's motion is: unlike AppKit, Windows has no
/// separate "dragged" event type — a move with a button held *is* a drag, and
/// the application's own hit testing decides the rest.
pub(super) fn move_to(x: i64, y: i64) -> Result<()> {
    let (dx, dy) = display::pixels_to_absolute(x, y);
    send(&[mouse(dx, dy, MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE, 0)])
}

/// Press or release a button at the cursor's current position.
pub(super) fn button(which: Button, pressed: bool) -> Result<()> {
    let (down, up) = button_flags(which);
    send(&[mouse(0, 0, if pressed { down } else { up }, 0)])
}

/// One wheel notch under the cursor.
pub(super) fn scroll(direction: ScrollDirection) -> Result<()> {
    let (axis, notch) = scroll_step(direction);
    send(&[mouse(0, 0, axis, notch)])
}

/// Type one character by injecting its UTF-16 directly.
///
/// `KEYEVENTF_UNICODE` is the simplification Windows hands us, and it is the
/// same one macOS does: the character rides on the event, so there is no
/// keycode to look up and no layout to be wrong about. A character outside
/// the basic plane arrives as a surrogate pair, and both halves go in one
/// call — an application that saw only the first would render a replacement
/// glyph.
pub(super) fn type_char(ch: char) -> Result<()> {
    let mut utf16 = [0u16; 2];
    let encoded = ch.encode_utf16(&mut utf16);

    let mut batch = Vec::with_capacity(encoded.len() * 2);
    for unit in encoded.iter() {
        batch.push(key(VIRTUAL_KEY(0), *unit, KEYEVENTF_UNICODE));
    }
    for unit in encoded.iter() {
        batch.push(key(
            VIRTUAL_KEY(0),
            *unit,
            KEYEVENTF_UNICODE | KEYEVENTF_KEYUP,
        ));
    }
    send(&batch)
}

/// Tap `keys` with `modifiers` held, as one indivisible batch.
///
/// See the module header: the release of a modifier is in the same call as
/// its press, so nothing can leave one latched.
pub(super) fn tap_chord(modifiers: &[VIRTUAL_KEY], keys: &[VIRTUAL_KEY]) -> Result<()> {
    let mut batch = Vec::with_capacity(modifiers.len() * 2 + keys.len() * 2);

    for modifier in modifiers {
        batch.push(key(*modifier, 0, KEYBD_EVENT_FLAGS(0)));
    }
    for pressed in keys {
        batch.push(key(*pressed, 0, KEYBD_EVENT_FLAGS(0)));
        batch.push(key(*pressed, 0, KEYEVENTF_KEYUP));
    }
    // Released in reverse, the way a hand leaves a chord.
    for modifier in modifiers.iter().rev() {
        batch.push(key(*modifier, 0, KEYEVENTF_KEYUP));
    }

    send(&batch)
}

/// Can this process reach the desktop that is receiving input right now?
///
/// The passive half of what a permission check is on macOS. A server started
/// as a service lives in session 0 and has no interactive desktop at all; a
/// UAC prompt or the lock screen swaps the input desktop for a secure one no
/// ordinary process may open. In both cases every event this backend sends is
/// accepted and discarded, and asking the desktop is the only way to know
/// without pressing something — which a probe must not do.
pub(super) fn on_the_input_desktop() -> bool {
    // SAFETY: no pointers in; the returned handle is closed below.
    let opened = unsafe { OpenInputDesktop(DESKTOP_CONTROL_FLAGS(0), false, DESKTOP_READOBJECTS) };
    match opened {
        Ok(desktop) => {
            // SAFETY: `desktop` was just handed to us by `OpenInputDesktop`
            // and is not used again.
            unsafe { CloseDesktop(desktop) }.ok();
            true
        }
        Err(_) => false,
    }
}
