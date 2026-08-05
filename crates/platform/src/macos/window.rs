//! The macOS [`WindowManager`] — the Accessibility API.
//!
//! # Why Accessibility and not `CGWindowListCopyWindowInfo`
//!
//! The window list enumerates well and acts not at all: it hands back a
//! `CGWindowID`, and there is no public call that closes one. Accessibility
//! hands back an `AXUIElement`, which has a close button you can press. Since
//! the contract here is *enumerate and close*, only one of the two can answer
//! it, and the input backend already requires the same permission.
//!
//! # Why the id is a `(pid, title)` pair
//!
//! An `AXUIElement` is a `CFRetained` handle that is neither `Send` nor
//! `Sync`, so it cannot live in a `WindowId` that crosses tasks — and the
//! watchdog closes windows concurrently. The handle is therefore re-derived
//! at close time from data that *is* portable. This is exactly the redesign
//! the opaque `WindowId` was introduced to absorb: Sway stores one integer
//! here, macOS stores a pair, and the feature layer is none the wiser.

use std::ptr::NonNull;

use anyhow::{Result, anyhow, bail};
use async_trait::async_trait;
use objc2_application_services::{AXError, AXUIElement};
use objc2_core_foundation::{
    CFArray, CFBoolean, CFDictionary, CFNumber, CFRetained, CFString, CFType,
};
use objc2_core_graphics::{CGWindowListCopyWindowInfo, CGWindowListOption};

use crate::window::{WindowId, WindowInfo, WindowManager};

/// Accessibility attribute and action names.
///
/// Spelled out because the SDK defines them as `CFSTR(...)` macros in a
/// header, so there is no symbol to link against — the strings *are* the ABI.
///
/// Built per call rather than cached in a static: `CFString` is neither
/// `Send` nor `Sync`, so a shared one would need a thread-local, and the
/// allocation it saves is microseconds against a cross-process AX round trip.
const AX_WINDOWS: &str = "AXWindows";
const AX_TITLE: &str = "AXTitle";
const AX_CLOSE_BUTTON: &str = "AXCloseButton";
const AX_FOCUSED: &str = "AXFocused";
const AX_PRESS: &str = "AXPress";

const CG_OWNER_PID: &str = "kCGWindowOwnerPID";
const CG_OWNER_NAME: &str = "kCGWindowOwnerName";

/// What a `WindowId` carries on macOS.
struct MacWindow {
    pid: i32,
    title: String,
}

/// The [`WindowManager`] the host selector hands out on macOS.
pub struct Quartz;

#[async_trait]
impl WindowManager for Quartz {
    async fn windows(&self) -> Result<Vec<WindowInfo>> {
        // Synchronous and self-contained: every `AXUIElement` is created and
        // dropped inside this call, so no `!Send` handle can be captured by
        // the future this `async fn` returns.
        enumerate()
    }

    async fn close(&self, window: &WindowId) -> Result<()> {
        press_close_button(window.payload::<MacWindow>()?)
    }
}

/// Every application window currently open, with the owning app's name.
///
/// The app list comes from the window server rather than from
/// `NSRunningApplication`: it already excludes agents and daemons that have
/// no window, so asking Accessibility about them is work that would answer
/// nothing.
fn enumerate() -> Result<Vec<WindowInfo>> {
    let mut windows = Vec::new();

    for (pid, app_name) in window_owners()? {
        // A process that refuses Accessibility (or has died since the window
        // list was taken) contributes nothing rather than failing the sweep:
        // one uncooperative app must not hide every other open window.
        for (title, focused) in app_windows(pid) {
            windows.push(WindowInfo {
                id: WindowId::new(
                    MacWindow {
                        pid,
                        title: title.clone(),
                    },
                    format!("pid={pid} title={title:?}"),
                ),
                app: app_name.clone(),
                title,
                pid: pid as i64,
                focused,
            });
        }
    }

    Ok(windows)
}

/// `(pid, application name)` for every process owning an on-screen window.
///
/// `CGWindowListCopyWindowInfo` is used only for this: the owner name and pid
/// are readable without Screen Recording permission, where window *titles*
/// are not — which is the other reason titles come from Accessibility.
fn window_owners() -> Result<Vec<(i32, String)>> {
    let list = CGWindowListCopyWindowInfo(
        // On-screen windows only, minus the desktop picture, Dock tiles and
        // the other window-server furniture the agent cannot act on.
        CGWindowListOption::OptionOnScreenOnly | CGWindowListOption::ExcludeDesktopElements,
        // Relative to no window: the whole list.
        0,
    )
    .ok_or_else(|| anyhow!("the window server returned no window list"))?;
    // SAFETY: the window list is documented to hold one CF dictionary per
    // window; the C signature simply has no element type to say so.
    let list: CFRetained<CFArray<CFType>> = unsafe { CFRetained::cast_unchecked(list) };

    let mut owners: Vec<(i32, String)> = Vec::new();
    for entry in list.iter() {
        let Some(entry) = window_entry(entry) else {
            continue;
        };
        if !owners.iter().any(|(pid, _)| *pid == entry.0) {
            owners.push(entry);
        }
    }
    Ok(owners)
}

/// One dictionary from the window list → `(pid, owner name)`.
fn window_entry(entry: CFRetained<CFType>) -> Option<(i32, String)> {
    let entry = entry.downcast::<CFDictionary>().ok()?;
    // SAFETY: every value in a `CGWindowListCopyWindowInfo` entry is a CF
    // object keyed by a `CFString`; the untyped form is only how the C API
    // spells that.
    let entry: CFRetained<CFDictionary<CFString, CFType>> =
        unsafe { CFRetained::cast_unchecked(entry) };

    let pid = entry
        .get(&CFString::from_str(CG_OWNER_PID))?
        .downcast::<CFNumber>()
        .ok()?
        .as_i32()?;
    let name = entry
        .get(&CFString::from_str(CG_OWNER_NAME))
        .and_then(|value| value.downcast::<CFString>().ok())
        .map(|name| name.to_string())
        // A window whose owner has no name is still a window the agent can
        // see, so it stays in the list under something addressable.
        .unwrap_or_else(|| format!("pid {pid}"));

    Some((pid, name))
}

/// The `AXWindows` of one process, or empty when it exposes none.
///
/// Empty rather than an error: a process that refuses Accessibility, has no
/// windows, or died since the window list was taken are the same thing to
/// every caller here, and the one `unsafe` lives in this function alone.
fn app_window_elements(pid: i32) -> Vec<CFRetained<AXUIElement>> {
    // SAFETY: a pid is all this takes; it returns a valid element even for a
    // process that has since exited (later calls then fail cleanly).
    let app = unsafe { AXUIElement::new_application(pid) };

    copy_attribute(&app, AX_WINDOWS)
        .and_then(object_array)
        .map(|windows| {
            windows
                .iter()
                .filter_map(|window| window.downcast::<AXUIElement>().ok())
                .collect()
        })
        .unwrap_or_default()
}

/// `(title, focused)` for each window of one application.
fn app_windows(pid: i32) -> Vec<(String, bool)> {
    app_window_elements(pid)
        .into_iter()
        .map(|window| {
            let focused = copy_attribute(&window, AX_FOCUSED)
                .and_then(|focused| focused.downcast::<CFBoolean>().ok())
                .is_some_and(|focused| focused.value());
            (window_title(&window), focused)
        })
        .collect()
}

/// Re-derive a window from its `(pid, title)` and press its close button.
///
/// A *graceful* close, exactly like Sway's: pressing the button is what the
/// user clicking the red dot does, so the app runs its own save-and-quit path
/// rather than being killed under it.
fn press_close_button(target: &MacWindow) -> Result<()> {
    for window in app_window_elements(target.pid) {
        if window_title(&window) != target.title {
            continue;
        }

        let Some(button) = copy_attribute(&window, AX_CLOSE_BUTTON) else {
            bail!("window {:?} has no close button", target.title)
        };
        let button: CFRetained<AXUIElement> = button
            .downcast()
            .map_err(|_| anyhow!("AXCloseButton was not an element"))?;

        // SAFETY: both the element and the action name are live here.
        let err = unsafe { button.perform_action(&CFString::from_str(AX_PRESS)) };
        if err != AXError::Success {
            bail!("pressing the close button failed ({err:?})");
        }
        return Ok(());
    }

    // The window closed on its own between the sweep and now — the outcome
    // the caller wanted, so not an error.
    Ok(())
}

/// An untyped CF value → an array of CF objects.
///
/// `ConcreteType` — what `downcast` needs — exists only for the untyped
/// `CFArray`, because the C type carries no element type to check against.
/// Re-typing it is therefore a cast, and it is sound here for the reason the
/// C API relies on: both arrays this crate reads (the window list, an
/// `AXWindows` attribute) are documented to hold CF objects.
fn object_array(value: CFRetained<CFType>) -> Option<CFRetained<CFArray<CFType>>> {
    let opaque = value.downcast::<CFArray>().ok()?;
    // SAFETY: see above — the elements are CF objects.
    Some(unsafe { CFRetained::cast_unchecked(opaque) })
}

/// A window's title, or the empty string when it has none.
fn window_title(window: &AXUIElement) -> String {
    copy_attribute(window, AX_TITLE)
        .and_then(|title| title.downcast::<CFString>().ok())
        .map(|title| title.to_string())
        .unwrap_or_default()
}

/// One Accessibility attribute, or `None` when it is absent or refused.
fn copy_attribute(element: &AXUIElement, attribute: &str) -> Option<CFRetained<CFType>> {
    let attribute = CFString::from_str(attribute);
    let mut value: *const CFType = std::ptr::null();

    // SAFETY: `value` is a live out-pointer for the duration of the call, and
    // the attribute name outlives it.
    let err = unsafe { element.copy_attribute_value(&attribute, NonNull::from(&mut value)) };
    if err != AXError::Success {
        return None;
    }
    // The call returns a +1 reference, which `from_raw` takes over.
    NonNull::new(value as *mut CFType).map(|value| unsafe { CFRetained::from_raw(value) })
}
