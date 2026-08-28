use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use nest_rs::core::injectable;

use super::error::ProgramsError;

type Result<T> = std::result::Result<T, ProgramsError>;

const LOG_DIR: &str = "/tmp/ghostdesk";

/// Which programs this session started, and where their output went.
///
/// A registry rather than part of the service, because it answers a different
/// question: the service decides *whether* a command may run, this remembers
/// *what ran*. The membership test is a security control — `app_status` reads
/// a file path derived from a pid, so a pid this session never launched must
/// be refused before that path is built, or the tool becomes a reader of
/// arbitrary `/tmp` files.
#[injectable]
#[derive(Default)]
pub struct LaunchRegistry {
    launched: Mutex<HashSet<u32>>,
}

impl LaunchRegistry {
    /// Where a launched program's output lives, addressed by pid alone.
    pub fn log_path(&self, pid: u32) -> PathBuf {
        PathBuf::from(LOG_DIR).join(format!("proc-{pid}.log"))
    }

    /// A log file for a launch that has not happened yet, and the two handles
    /// its output is redirected to.
    ///
    /// Staged under a name the pid cannot yet supply and renamed once the
    /// process exists: the file has to be open *before* the spawn, and only
    /// the spawn can say what to call it.
    pub fn stage_log(&self) -> Result<(PathBuf, Stdio, Stdio)> {
        std::fs::create_dir_all(LOG_DIR).map_err(ProgramsError::Log)?;

        let path = PathBuf::from(LOG_DIR).join(format!(
            "proc-{}-{}.log",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|elapsed| elapsed.as_nanos())
                .unwrap_or(0),
        ));
        let out = std::fs::File::create(&path).map_err(ProgramsError::Log)?;
        let errors = out.try_clone().map_err(ProgramsError::Log)?;
        Ok((path, Stdio::from(out), Stdio::from(errors)))
    }

    /// Take ownership of a spawned pid and give its log its final name.
    pub fn adopt(&self, pid: u32, staged: &Path) -> Result<PathBuf> {
        let path = self.log_path(pid);
        std::fs::rename(staged, &path).map_err(ProgramsError::Log)?;
        self.launched
            .lock()
            .expect("launched-pid registry poisoned")
            .insert(pid);
        Ok(path)
    }

    /// Drop the staged log of a launch that never started.
    pub fn discard(&self, staged: &Path) {
        std::fs::remove_file(staged).ok();
    }

    pub fn tracked(&self, pid: u32) -> bool {
        self.launched
            .lock()
            .expect("launched-pid registry poisoned")
            .contains(&pid)
    }

    /// Is the process still alive?
    ///
    /// `kill(pid, 0)` asks the kernel rather than reading a cached exit status:
    /// the child is reaped by a detached task, so nothing here holds a handle
    /// to ask.
    pub fn is_running(&self, pid: u32) -> bool {
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
}

/// The last `lines` of a file, or nothing at all.
///
/// A missing log is an empty tail rather than an error: a program that has not
/// written yet and one whose file was swept are the same answer to the agent,
/// and neither is worth a failed tool call.
pub(super) fn tail(path: &Path, lines: usize) -> String {
    let Ok(body) = std::fs::read(path) else {
        return String::new();
    };
    let text = String::from_utf8_lossy(&body);
    let all: Vec<&str> = text.lines().collect();
    all[all.len().saturating_sub(lines)..].join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dead_pid() -> u32 {
        let mut child = std::process::Command::new("true").spawn().unwrap();
        let pid = child.id();
        child.wait().unwrap();
        pid
    }

    #[test]
    fn a_pid_is_untracked_until_it_is_adopted() {
        let registry = LaunchRegistry::default();
        assert!(!registry.tracked(4242));

        let (staged, _out, _err) = registry.stage_log().expect("a staged log");
        let path = registry.adopt(4242, &staged).expect("adopted");

        assert!(registry.tracked(4242));
        assert!(
            path.ends_with("proc-4242.log"),
            "a log is addressed by pid alone: {}",
            path.display(),
        );
        assert!(!staged.exists(), "the staged name is gone");
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn a_launch_that_never_started_leaves_no_file_behind() {
        let registry = LaunchRegistry::default();
        let (staged, _out, _err) = registry.stage_log().expect("a staged log");
        assert!(staged.exists());

        registry.discard(&staged);
        assert!(!staged.exists());
    }

    #[test]
    fn liveness_is_asked_of_the_kernel() {
        let registry = LaunchRegistry::default();
        assert!(registry.is_running(std::process::id()));
        assert!(!registry.is_running(dead_pid()));
    }

    #[test]
    fn tail_returns_the_last_lines_and_tolerates_a_missing_file() {
        let path = std::env::temp_dir().join(format!("gd-tail-{}.log", std::process::id()));
        std::fs::write(&path, "a\nb\nc\nd\n").unwrap();
        assert_eq!(tail(&path, 2), "c\nd");
        assert_eq!(tail(&path, 99), "a\nb\nc\nd");
        std::fs::remove_file(&path).unwrap();
        assert_eq!(tail(&path, 2), "");
    }
}
