//! What this OS decides about a child process the server launched.
//!
//! Two questions the standard library will not answer portably: is a pid
//! still alive, and how is a child kept out of the server's own signal group.
//! Neither belongs to one of the five desktop contracts — a launched program
//! is not a desktop seam — but both are per-OS, so they live here rather than
//! leaking a `cfg` into the feature crate.
//!
//! The macOS file beside this one is identical, and that is a price rather
//! than an argument. `cfg(unix)` would name the family honestly enough — it is
//! the compiler's own predicate, not a `shared/` — but a fourth top-level
//! directory would stop `host`'s arms and this crate's directory listing from
//! reading as one list, and two members of fifteen lines is the wrong side of
//! that trade to spend it on. What the copy costs meanwhile is worth knowing:
//! a change to the probe has to be made twice, and nothing catches a one-sided
//! edit, because neither build compiles the other file. A third Unix host is
//! the moment to revisit this.

use tokio::process::Command;

/// Is the process still alive?
///
/// `kill(pid, 0)` asks the kernel rather than reading a cached exit status:
/// the child is reaped by a detached task, so nothing upstream holds a handle
/// to ask.
pub fn is_running(pid: u32) -> bool {
    let Some(pid) = i32::try_from(pid)
        .ok()
        .and_then(rustix::process::Pid::from_raw)
    else {
        return false;
    };
    !matches!(
        rustix::process::test_kill_process(pid),
        Err(rustix::io::Errno::SRCH)
    )
}

/// Put the child in a process group of its own.
///
/// A terminal signalling the server — Ctrl-C on a foreground run — signals its
/// whole group, and an agent's browser dying with the server it was launched
/// from is not what either of them asked for.
pub fn detach(command: &mut Command) {
    command.process_group(0);
}
