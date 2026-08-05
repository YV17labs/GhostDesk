//! Quartz event construction and posting — the only unsafe in the backend.
//!
//! Every function here is synchronous and self-contained: a `CGEvent` is
//! built, posted and dropped before returning. That is not a style choice.
//! `CGEvent` is neither `Send` nor `Sync`, and the backend above is an
//! `async` trait whose futures must be `Send` — keeping each event's whole
//! life inside one non-async call is what makes that true by construction
//! rather than by careful review of every `await`.
//!
//! Events go to `HIDEventTap`, the bottom of the stack, so they behave like
//! hardware: every tap, every app and the window server itself see them. That
//! placement is what needs Accessibility permission.

use anyhow::{Result, bail};
use objc2_core_foundation::CGPoint;
use objc2_core_graphics::{
    CGEvent, CGEventFlags, CGEventTapLocation, CGEventType, CGKeyCode, CGMouseButton,
    CGScrollEventUnit,
};

use super::super::display;
use crate::input::{Button, ScrollDirection};

/// The mouse-event types for one button: `(down, up, dragged)`.
fn mouse_types(button: Button) -> (CGEventType, CGEventType, CGEventType) {
    match button {
        Button::Left => (
            CGEventType::LeftMouseDown,
            CGEventType::LeftMouseUp,
            CGEventType::LeftMouseDragged,
        ),
        Button::Right => (
            CGEventType::RightMouseDown,
            CGEventType::RightMouseUp,
            CGEventType::RightMouseDragged,
        ),
        Button::Middle => (
            CGEventType::OtherMouseDown,
            CGEventType::OtherMouseUp,
            CGEventType::OtherMouseDragged,
        ),
    }
}

fn mouse_button(button: Button) -> CGMouseButton {
    match button {
        Button::Left => CGMouseButton::Left,
        Button::Right => CGMouseButton::Right,
        Button::Middle => CGMouseButton::Center,
    }
}

/// `(vertical, horizontal)` line deltas for one notch.
///
/// Quartz signs a scroll the way the wheel turns, not the way the content
/// moves: positive is away from the user (up) and to the left.
fn scroll_delta(direction: ScrollDirection) -> (i32, i32) {
    match direction {
        ScrollDirection::Up => (1, 0),
        ScrollDirection::Down => (-1, 0),
        ScrollDirection::Left => (0, 1),
        ScrollDirection::Right => (0, -1),
    }
}

fn post(event: &CGEvent) {
    CGEvent::post(CGEventTapLocation::HIDEventTap, Some(event));
}

/// The cursor's current position, in Quartz points.
///
/// Read from a fresh null event rather than tracked in a field: the user's
/// own hand moves the same cursor, and a click at a remembered position
/// would land wherever GhostDesk last put it instead of where it is.
fn cursor() -> Result<CGPoint> {
    let Some(probe) = CGEvent::new(None) else {
        bail!("{DENIED}")
    };
    Ok(CGEvent::location(Some(&probe)))
}

/// What a refused event creation means. Quartz returns null rather than an
/// error code, and in practice there is one cause.
const DENIED: &str = "the OS refused to create an input event — GhostDesk needs \
                      Accessibility permission (System Settings ▸ Privacy & \
                      Security ▸ Accessibility)";

/// Move the cursor to a pixel coordinate.
pub(super) fn move_to(x: i64, y: i64) -> Result<()> {
    let (px, py) = display::pixels_to_points(x, y);
    let Some(event) = CGEvent::new_mouse_event(
        None,
        CGEventType::MouseMoved,
        CGPoint::new(px, py),
        CGMouseButton::Left,
    ) else {
        bail!("{DENIED}")
    };
    post(&event);
    Ok(())
}

/// Press or release a button at the cursor's current position.
pub(super) fn button(button_: Button, pressed: bool) -> Result<()> {
    let (down, up, _) = mouse_types(button_);
    let Some(event) = CGEvent::new_mouse_event(
        None,
        if pressed { down } else { up },
        cursor()?,
        mouse_button(button_),
    ) else {
        bail!("{DENIED}")
    };
    post(&event);
    Ok(())
}

/// Move to a pixel coordinate with a button held — a drag, not a warp.
///
/// A held button plus `MouseMoved` is not a drag: AppKit's drag machinery
/// tracks the `*MouseDragged` types, and a view that only ever sees
/// `MouseMoved` treats the gesture as a hover that happens to end in a click.
pub(super) fn drag_to(x: i64, y: i64, button_: Button) -> Result<()> {
    let (_, _, dragged) = mouse_types(button_);
    let (px, py) = display::pixels_to_points(x, y);
    let Some(event) =
        CGEvent::new_mouse_event(None, dragged, CGPoint::new(px, py), mouse_button(button_))
    else {
        bail!("{DENIED}")
    };
    post(&event);
    Ok(())
}

/// One wheel notch.
pub(super) fn scroll(direction: ScrollDirection) -> Result<()> {
    let (vertical, horizontal) = scroll_delta(direction);
    let Some(event) = CGEvent::new_scroll_wheel_event2(
        None,
        // Lines, not pixels: a notch should move a document by whatever the
        // app considers one step, exactly as a physical wheel does.
        CGScrollEventUnit::Line,
        2,
        vertical,
        horizontal,
        0,
    ) else {
        bail!("{DENIED}")
    };
    post(&event);
    Ok(())
}

/// Type one character by injecting its UTF-16 directly.
///
/// This is the simplification macOS hands us: the character rides on the
/// event itself, so there is no keysym to look up, no keymap to upload, and
/// no layout to be wrong about. The virtual keycode is irrelevant and set to
/// 0 — the string is what the receiving app reads.
pub(super) fn type_char(ch: char) -> Result<()> {
    let mut utf16 = [0u16; 2];
    let encoded = ch.encode_utf16(&mut utf16);

    for down in [true, false] {
        let Some(event) = CGEvent::new_keyboard_event(None, 0, down) else {
            bail!("{DENIED}")
        };
        // SAFETY: `encoded` is a live slice for the duration of the call, and
        // its length is passed alongside it.
        unsafe {
            CGEvent::keyboard_set_unicode_string(
                Some(&event),
                encoded.len() as u64,
                encoded.as_ptr(),
            );
        }
        post(&event);
    }
    Ok(())
}

/// Tap one key with `flags` held down.
pub(super) fn tap_key(key: CGKeyCode, flags: CGEventFlags) -> Result<()> {
    for down in [true, false] {
        let Some(event) = CGEvent::new_keyboard_event(None, key, down) else {
            bail!("{DENIED}")
        };
        // Modifiers ride on the key event rather than going out as separate
        // `FlagsChanged` posts: a latched modifier left behind by a failure
        // between two posts would corrupt every keystroke that followed.
        CGEvent::set_flags(Some(&event), flags);
        post(&event);
    }
    Ok(())
}

/// Whether this process may post input events at all.
pub(super) fn is_trusted() -> bool {
    // SAFETY: no arguments, no pointers; reads a process-wide TCC decision.
    unsafe { objc2_application_services::AXIsProcessTrusted() }
}
