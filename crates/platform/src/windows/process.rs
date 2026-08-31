//! What this OS decides about a child process the server launched.
//!
//! The Windows answers to the two questions the Unix files beside this one
//! answer with `kill(pid, 0)` and a process group. Neither has a Win32
//! equivalent, and neither needed one — which is the whole reason this seam
//! exists rather than a `cfg` in the feature crate.

#![expect(
    unsafe_code,
    reason = "the process handle API is C; each call carries its own SAFETY note"
)]

use ::windows::Win32::Foundation::{CloseHandle, WAIT_OBJECT_0};
use ::windows::Win32::System::Threading::{OpenProcess, PROCESS_SYNCHRONIZE, WaitForSingleObject};
use tokio::process::Command;

/// Is the process still alive?
///
/// Waited on for zero milliseconds rather than asked for its exit code. A
/// process handle is signalled the moment the process ends, so the wait times
/// out exactly while it is running — where `GetExitCodeProcess` answers
/// `STILL_ACTIVE`, which is the number 259, and cannot tell a running process
/// from one that exited with 259.
pub fn is_running(pid: u32) -> bool {
    // SAFETY: opens a handle with the narrowest right that answers the
    // question; closed on every path out. A pid that no longer exists is a
    // refusal, which is the answer.
    let Ok(process) = (unsafe { OpenProcess(PROCESS_SYNCHRONIZE, false, pid) }) else {
        return false;
    };

    // SAFETY: the handle is live and the wait does not block.
    let signalled = unsafe { WaitForSingleObject(process, 0) } == WAIT_OBJECT_0;
    // SAFETY: the handle is not used again.
    unsafe { CloseHandle(process) }.ok();

    !signalled
}

/// Nothing to detach from.
///
/// A Unix child inherits its parent's process group and is signalled with it,
/// so it has to be moved out of the way. Windows has no such inheritance: a
/// child spawned here already outlives the server, and console control events
/// — the nearest equivalent — do not reach a program with no console, which is
/// every application this catalogue can launch.
pub fn detach(_command: &mut Command) {}
