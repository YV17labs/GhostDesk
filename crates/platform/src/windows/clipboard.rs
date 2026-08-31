//! The Windows [`Clipboard`] — the Win32 clipboard, in process.
//!
//! The API rather than a helper, for the reason the rest of this backend
//! shells out to nothing: Windows has no `pbcopy`, and the nearest equivalent
//! is a PowerShell process per call — a third of a second of start-up, and a
//! text encoding to argue with. The C interface is fiddly instead, and the
//! fiddliness is all here.
//!
//! The clipboard is a single global lock any application may be holding for a
//! moment, so a refusal is normal rather than exceptional and both directions
//! retry briefly before giving up.

#![expect(
    unsafe_code,
    reason = "the clipboard and its memory are C; each call carries its own SAFETY note"
)]

use std::time::Duration;

use anyhow::{Result, bail};
use async_trait::async_trait;

use ::windows::Win32::Foundation::{GlobalFree, HANDLE, HGLOBAL};
use ::windows::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, GetClipboardData, OpenClipboard, SetClipboardData,
};
use ::windows::Win32::System::Memory::{GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalUnlock};
use ::windows::Win32::System::Ole::CF_UNICODETEXT;
use ::windows::core::PCWSTR;

use crate::clipboard::Clipboard;

/// How many times to ask for the clipboard before calling it held.
const ATTEMPTS: u32 = 10;

/// How long to wait between two attempts.
const BETWEEN_ATTEMPTS: Duration = Duration::from_millis(20);

/// The [`Clipboard`] the host selector hands out on Windows.
pub struct Win32Clipboard;

#[async_trait]
impl Clipboard for Win32Clipboard {
    /// Empty for an empty clipboard and for content that is not text — both
    /// are the contract's "empty string", and neither is worth an agent turn.
    async fn get(&self) -> String {
        tokio::task::spawn_blocking(read)
            .await
            .ok()
            .and_then(|read| read.ok())
            .unwrap_or_default()
    }

    async fn set(&self, text: &str) -> Result<()> {
        let text = text.to_owned();
        tokio::task::spawn_blocking(move || write(&text)).await?
    }
}

/// The clipboard, held for as long as one read or write takes.
///
/// A guard rather than a higher-order wrapper, because the wrapper could not
/// keep the promise it made: both callers run on a blocking-pool thread, and a
/// panic there unwound straight past the close — leaving the clipboard open
/// for the life of the process, which is the exact failure it existed to
/// prevent.
struct ClipboardLock;

impl ClipboardLock {
    fn open() -> Result<Self> {
        for attempt in 0..ATTEMPTS {
            // SAFETY: `None` associates the clipboard with the current task
            // rather than with a window, which this server does not have.
            if unsafe { OpenClipboard(None) }.is_ok() {
                return Ok(Self);
            }
            if attempt + 1 < ATTEMPTS {
                std::thread::sleep(BETWEEN_ATTEMPTS);
            }
        }
        bail!("another application has held the clipboard open for too long")
    }
}

impl Drop for ClipboardLock {
    fn drop(&mut self) {
        // SAFETY: opened on this thread by `open`, and closed nowhere else.
        unsafe { CloseClipboard() }.ok();
    }
}

/// The clipboard's text, if it holds any.
fn read() -> Result<String> {
    let _clipboard = ClipboardLock::open()?;

    // SAFETY: the returned handle is owned by the clipboard, so it is locked
    // and unlocked here but never freed.
    let handle = unsafe { GetClipboardData(CF_UNICODETEXT.0 as u32) }?;
    let memory = HGLOBAL(handle.0);

    // SAFETY: a clipboard `CF_UNICODETEXT` handle is a global memory block.
    let text = unsafe { GlobalLock(memory) };
    if text.is_null() {
        bail!("the clipboard's text could not be locked for reading");
    }

    // SAFETY: the block is NUL-terminated UTF-16 — that is what the format
    // means — and stays locked for the length of this read.
    let wide = PCWSTR(text.cast());
    let read = String::from_utf16_lossy(unsafe { wide.as_wide() });

    // SAFETY: balances the lock above.
    unsafe { GlobalUnlock(memory) }.ok();
    Ok(read)
}

/// Put text on the clipboard.
fn write(text: &str) -> Result<()> {
    let mut utf16: Vec<u16> = text.encode_utf16().collect();
    utf16.push(0);
    let bytes = std::mem::size_of_val(utf16.as_slice());

    let _clipboard = ClipboardLock::open()?;

    // Emptied before anything is allocated, not after: every exit between the
    // allocation and the hand-over has to free the block itself, and this is
    // one fewer of them.
    // SAFETY: empties the clipboard this thread already owns.
    unsafe { EmptyClipboard() }?;

    // Moveable, not fixed: the clipboard takes ownership of the block and
    // frees it with the flags it was allocated under.
    // SAFETY: the size is the one the copy below writes.
    let memory = unsafe { GlobalAlloc(GMEM_MOVEABLE, bytes) }?;

    // SAFETY: freshly allocated and not yet handed to anyone.
    let destination = unsafe { GlobalLock(memory) };
    if destination.is_null() {
        // SAFETY: nothing else holds this block.
        unsafe { GlobalFree(Some(memory)) }.ok();
        bail!("the clipboard buffer could not be locked for writing");
    }
    // SAFETY: both sides are live and `bytes` is the allocation's own size.
    unsafe { std::ptr::copy_nonoverlapping(utf16.as_ptr().cast(), destination, bytes) };
    // SAFETY: balances the lock above.
    unsafe { GlobalUnlock(memory) }.ok();

    // SAFETY: on success the clipboard owns `memory` and this process must not
    // free it — which is why the free below is only on the error path.
    match unsafe { SetClipboardData(CF_UNICODETEXT.0 as u32, Some(HANDLE(memory.0))) } {
        Ok(_) => Ok(()),
        Err(err) => {
            // SAFETY: the clipboard refused ownership, so it is still ours.
            unsafe { GlobalFree(Some(memory)) }.ok();
            bail!("the clipboard refused the text ({err})")
        }
    }
}
