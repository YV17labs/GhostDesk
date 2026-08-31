//! The Windows [`WindowManager`] — `EnumWindows` and `WM_CLOSE`.
//!
//! # Why the id is a number and not a handle
//!
//! An `HWND` is a raw pointer, so it is neither `Send` nor `Sync`, and the
//! idle watchdog closes windows concurrently. The handle's *value* is, so
//! that is what a [`WindowId`] carries, alongside the pid that owned it.
//! Windows recycles handles, and a recycled one addressed by number alone
//! would close whatever inherited it — so the pid is re-checked before
//! anything is sent. Sway stores one integer here, macOS a `(pid, title)`
//! pair and this a `(handle, pid)` one; the feature layer is none the wiser,
//! which is what the opaque id was introduced for.
//!
//! # Which windows count
//!
//! The contract says real client windows and nothing else, and on Windows the
//! desktop is full of things that are technically windows: every tray helper
//! owns a hidden message-only window, and every Store application leaves a
//! *cloaked* one behind on every virtual desktop it is not on. The filter
//! below is the one the task switcher itself uses — visible, not cloaked,
//! titled, not a tool window, and either unowned or explicitly marked as an
//! application window. Without the cloaking test in particular the list comes
//! back several times longer than what the user can see.

#![expect(
    unsafe_code,
    reason = "the Win32 window API is C; each call carries its own SAFETY note"
)]

use std::collections::HashMap;

use anyhow::{Result, bail};
use async_trait::async_trait;

use ::windows::Win32::Foundation::{CloseHandle, HWND, LPARAM, TRUE, WPARAM};
use ::windows::Win32::Graphics::Dwm::{DWMWA_CLOAKED, DwmGetWindowAttribute};
use ::windows::Win32::System::Threading::{
    OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
};
use ::windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GW_OWNER, GWL_EXSTYLE, GetForegroundWindow, GetWindow, GetWindowLongW,
    GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId, IsWindow, IsWindowVisible,
    PostMessageW, WM_CLOSE, WS_EX_APPWINDOW, WS_EX_TOOLWINDOW,
};
use ::windows::core::{BOOL, PWSTR};

use crate::window::{WindowId, WindowInfo, WindowManager};

/// What a `WindowId` carries on Windows.
struct Win32Window {
    handle: isize,
    pid: u32,
}

/// The [`WindowManager`] the host selector hands out on Windows.
pub struct Win32Windows;

#[async_trait]
impl WindowManager for Win32Windows {
    async fn windows(&self) -> Result<Vec<WindowInfo>> {
        // Synchronous and self-contained: every `HWND` is created and dropped
        // inside this call, so no `!Send` handle can be captured by the future
        // this `async fn` returns.
        enumerate()
    }

    async fn close(&self, window: &WindowId) -> Result<()> {
        ask_to_close(window.payload::<Win32Window>()?)
    }
}

/// Every application window currently open.
fn enumerate() -> Result<Vec<WindowInfo>> {
    let mut handles: Vec<HWND> = Vec::new();

    // SAFETY: the callback below is the one this pointer is built for, and
    // `handles` outlives the call — `EnumWindows` is synchronous.
    unsafe { EnumWindows(Some(collect), LPARAM(&raw mut handles as isize)) }?;

    // SAFETY: no pointers; reads which window currently has focus.
    let focused = unsafe { GetForegroundWindow() };
    let own_pid = std::process::id();

    // One process handle per pid rather than per window. Several windows from
    // one application is the ordinary case — a browser, an editor — and each
    // would otherwise reopen and reclose the same process to read the same
    // name back.
    let mut names: HashMap<u32, String> = HashMap::new();

    Ok(handles
        .into_iter()
        .filter(|handle| is_an_application_window(*handle))
        .filter_map(|handle| describe(handle, focused, own_pid, &mut names))
        .collect())
}

/// The `EnumWindows` callback: collect, decide nothing.
///
/// Every test lives in [`enumerate`] instead, because a panic across an FFI
/// boundary is undefined behaviour and the only way to be sure none happens
/// here is for the callback to do nothing that can fail.
unsafe extern "system" fn collect(handle: HWND, lparam: LPARAM) -> BOOL {
    // SAFETY: `lparam` is the `&mut Vec<HWND>` `enumerate` passed in, live for
    // the whole enumeration.
    let found = unsafe { &mut *(lparam.0 as *mut Vec<HWND>) };
    found.push(handle);
    TRUE
}

/// Is this a window the user would call open?
fn is_an_application_window(handle: HWND) -> bool {
    // SAFETY: `handle` came from `EnumWindows` and each of these reads a
    // property of it without taking a pointer.
    unsafe {
        if !IsWindowVisible(handle).as_bool() || GetWindowTextLengthW(handle) == 0 {
            return false;
        }
    }

    if is_cloaked(handle) {
        return false;
    }

    // SAFETY: reads the extended style word of a live window.
    let styles = unsafe { GetWindowLongW(handle, GWL_EXSTYLE) } as u32;
    if styles & WS_EX_TOOLWINDOW.0 != 0 {
        return false;
    }
    if styles & WS_EX_APPWINDOW.0 != 0 {
        return true;
    }

    // SAFETY: asks for this window's owner; an unowned window is an error
    // rather than a null handle, which is the case being tested for.
    unsafe { GetWindow(handle, GW_OWNER) }.is_err()
}

/// Is the window present but drawn nowhere — the shell's own word for it?
///
/// A Store application keeps a window alive on every virtual desktop it is not
/// showing on, and the desktop switcher keeps one for the animation. Both are
/// visible, titled, and invisible to the user.
fn is_cloaked(handle: HWND) -> bool {
    let mut cloaked = 0u32;
    // SAFETY: the out-pointer and its size describe the same `u32`, which
    // outlives the call.
    let asked = unsafe {
        DwmGetWindowAttribute(
            handle,
            DWMWA_CLOAKED,
            (&raw mut cloaked).cast(),
            size_of::<u32>() as u32,
        )
    };
    asked.is_ok() && cloaked != 0
}

/// One handle → the window the feature layer sees.
fn describe(
    handle: HWND,
    focused: HWND,
    own_pid: u32,
    names: &mut HashMap<u32, String>,
) -> Option<WindowInfo> {
    let mut pid = 0u32;
    // SAFETY: the out-pointer is a live `u32` for the duration of the call.
    unsafe { GetWindowThreadProcessId(handle, Some(&raw mut pid)) };

    // This server's own windows are not the agent's to act on — the contract
    // excludes the desktop's infrastructure, and the idle watchdog relies on
    // it.
    if pid == 0 || pid == own_pid {
        return None;
    }

    let title = window_title(handle);
    Some(WindowInfo {
        id: WindowId::new(
            Win32Window {
                handle: handle.0 as isize,
                pid,
            },
            format!("hwnd={:#x} title={title:?}", handle.0 as isize),
        ),
        // The executable's own name, which is what survives a retitled window
        // — the closest Windows has to an `app_id` or a bundle identifier.
        app: names
            .entry(pid)
            .or_insert_with(|| executable_name(pid).unwrap_or_else(|| format!("pid {pid}")))
            .clone(),
        title,
        pid: pid as i64,
        focused: handle == focused,
    })
}

/// A window's title, or the empty string when it has none.
fn window_title(handle: HWND) -> String {
    // SAFETY: reads a length, taking no pointer.
    let length = unsafe { GetWindowTextLengthW(handle) };
    if length <= 0 {
        return String::new();
    }

    // One more than the reported length: the call writes a terminating NUL and
    // truncates to fit, so a buffer of exactly the title's length loses its
    // last character.
    let mut buffer = vec![0u16; length as usize + 1];
    // SAFETY: the buffer is live and its length is what bounds the write.
    let written = unsafe { GetWindowTextW(handle, &mut buffer) };
    String::from_utf16_lossy(&buffer[..written.max(0) as usize])
}

/// The basename of the executable behind a pid.
fn executable_name(pid: u32) -> Option<String> {
    // SAFETY: opens a handle with the narrowest right that answers the
    // question; closed on every path out.
    let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) }.ok()?;

    let mut buffer = [0u16; 260];
    let mut length = buffer.len() as u32;
    // SAFETY: `buffer` and `length` describe the same live array, and the call
    // writes the used length back into `length`.
    let queried = unsafe {
        QueryFullProcessImageNameW(
            process,
            PROCESS_NAME_WIN32,
            PWSTR(buffer.as_mut_ptr()),
            &raw mut length,
        )
    };
    // SAFETY: `process` is not used again.
    unsafe { CloseHandle(process) }.ok();
    queried.ok()?;

    let path = String::from_utf16_lossy(&buffer[..length as usize]);
    std::path::Path::new(&path)
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
}

/// Ask one window to close, gracefully.
///
/// `WM_CLOSE` posted rather than sent: it is exactly what clicking the ✕ does,
/// so the application runs its own save-and-quit path and may put a dialog up
/// instead of exiting. Posted, so a window whose thread is busy does not hold
/// this call — and so an application that asks the user a question does not
/// block the watchdog closing the next one.
fn ask_to_close(target: &Win32Window) -> Result<()> {
    let handle = HWND(target.handle as *mut std::ffi::c_void);

    // SAFETY: `IsWindow` is defined for any value, valid handle or not — that
    // is what makes it the test to run first.
    if !unsafe { IsWindow(Some(handle)) }.as_bool() {
        // The window closed on its own between the sweep and now — the outcome
        // the caller wanted, so not an error.
        return Ok(());
    }

    let mut pid = 0u32;
    // SAFETY: the out-pointer is a live `u32` for the duration of the call.
    unsafe { GetWindowThreadProcessId(handle, Some(&raw mut pid)) };
    if pid != target.pid {
        // Windows reuses handle values. The number is still a window, but it
        // belongs to something else now, and closing it would be closing a
        // window nobody asked about.
        return Ok(());
    }

    // SAFETY: the handle is live and both message parameters are unused by
    // `WM_CLOSE`.
    match unsafe { PostMessageW(Some(handle), WM_CLOSE, WPARAM(0), LPARAM(0)) } {
        Ok(()) => Ok(()),
        Err(err) => bail!("the window refused the close message ({err})"),
    }
}
