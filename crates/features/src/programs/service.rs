//! Program launching and process tracking.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use nest_rs::core::injectable;
use platform::desktop::{AppCatalog, DesktopApp};
use platform::window::{WindowInfo, WindowManager};
use tokio::process::Command;

use super::error::ProgramsError;

type Result<T> = std::result::Result<T, ProgramsError>;

/// Where launched programs' stdout and stderr land.
const LOG_DIR: &str = "/tmp/ghostdesk";

/// Debian policy §9.1.1 installs games under `/usr/games`, which is not in a
/// non-interactive shell's default PATH. Extending it is what lets a
/// `.desktop` `Exec=` basename like `gnome-chess` resolve.
const EXTRA_PATH: &str = "/usr/games:/usr/local/games";

/// The server's own environment namespace, swept wholesale before a GUI
/// program is spawned.
///
/// The MCP server needs these; a browser the agent spawns does not — and any
/// code execution inside that browser would otherwise inherit them. It is a
/// single prefix rather than a list of names to keep in step with the config
/// structs: everything the server reads is under it, so nothing new can be
/// added on one side and forgotten here.
///
/// A literal rather than the framework's own prefix: the container's own
/// knobs are set directly on the process without passing through the
/// framework at all, so following the framework's prefix would drop them the
/// moment the two diverged — which the entrypoint refuses to let happen.
///
/// Sweeping non-secrets along with the bearer token is deliberate. A launched
/// browser has no use for the server's settings, and a prefix rule that
/// admits exceptions stops being a rule.
const SCRUBBED_PREFIX: &str = "GHOSTDESK_";

/// Bound on the post-launch wait for a first window. Cold starts in a fresh
/// container (a browser's first run) sit in single-digit seconds; anything
/// past this is either a UI-less process or a problem `app_status` is better
/// placed to explain.
const WINDOW_WAIT_TIMEOUT: Duration = Duration::from_secs(10);

/// How often the desktop is re-asked while waiting. A window list is a cheap
/// compositor round trip, not a capture, so this leans brisk.
const WINDOW_WAIT_POLL: Duration = Duration::from_millis(150);

/// One open window.
///
/// Not the wire shape — [`WindowDto`](super::dtos::WindowDto) is, and
/// converts from this. A field renamed here is a refactor; renamed there it
/// is a breaking change for every client, and the two should not be the same
/// edit.
#[derive(Debug, Clone)]
pub struct RunningWindow {
    /// Stable application identity, as this desktop reports it.
    pub app: String,
    pub title: String,
    pub pid: i64,
    pub focused: bool,
}

impl From<WindowInfo> for RunningWindow {
    fn from(window: WindowInfo) -> Self {
        Self {
            app: window.app,
            title: window.title,
            pid: window.pid,
            focused: window.focused,
        }
    }
}

/// What a successful [`launch`](ProgramsService::launch) produced.
///
/// The window a launch goes on to open is not here: waiting for one is a
/// second call, and composing the two is the adapter's job.
#[derive(Debug, Clone)]
pub struct Launched {
    pub pid: u32,
    pub log_file: String,
    pub action: String,
}

/// How a post-launch window wait ended.
///
/// Every arm carries how long it waited, so the caller never has to know what
/// the bound was to describe the outcome. [`WINDOW_WAIT_TIMEOUT`] stays
/// private for exactly that reason.
#[derive(Debug)]
pub enum WindowWait {
    /// The desktop mapped a window this launch produced.
    Appeared {
        window: RunningWindow,
        waited_ms: u64,
    },
    /// The bound expired with the process already gone and nothing mapped —
    /// a crash (the log tail knows) or a single-instance program that handed
    /// off to a running peer, whose new window keeps the peer's old PID and
    /// is indistinguishable from the rest of its windows.
    ProcessExited { waited_ms: u64 },
    /// The bound expired with the process alive and still windowless.
    TimedOut { waited_ms: u64 },
}

/// What [`status`](ProgramsService::status) found.
#[derive(Debug, Clone)]
pub struct ProgramStatus {
    pub pid: u32,
    pub running: bool,
    pub log_file: String,
    pub tail: String,
}

#[injectable]
pub struct ProgramsService {
    #[inject]
    windows: Arc<dyn WindowManager>,
    #[inject]
    catalog: Arc<dyn AppCatalog>,
    /// PIDs launched by this session. `app_status` refuses anything else, so
    /// the tool cannot be turned into a general-purpose process prober.
    launched: Mutex<HashSet<u32>>,
}

impl ProgramsService {
    /// The catalogue of installed GUI programs.
    pub fn list(&self) -> Vec<DesktopApp> {
        self.catalog.apps()
    }

    /// The program windows currently open on the desktop.
    pub async fn running(&self) -> Result<Vec<RunningWindow>> {
        Ok(self
            .windows
            .windows()
            .await
            .map_err(ProgramsError::Desktop)?
            .into_iter()
            .map(RunningWindow::from)
            .collect())
    }

    /// Wait, bounded by [`WINDOW_WAIT_TIMEOUT`], for the desktop to show the
    /// window a launch produced.
    ///
    /// This is the composition `app_launch`'s wait is built on: the
    /// launch → poll-until-a-window dance the agent used to run over two to
    /// four tool calls, run server-side in one. A window matches when its
    /// PID is the launched one — the common case — or, failing that, when
    /// its PID was not on screen before the launch: a `.desktop` `Exec` that
    /// wraps the real binary puts the window on a child PID, and that window
    /// is still the one this launch produced.
    ///
    /// The spawned process dying does *not* end the wait early: wrapper
    /// scripts exit the moment their child is running, often seconds before
    /// the child maps its window, and reporting "exited, no window" in that
    /// gap would be a fast wrong answer. The full bound is spent looking;
    /// only then does the verdict distinguish a dead process from a slow or
    /// UI-less one.
    pub async fn wait_for_window(
        &self,
        pid: u32,
        seen_before: &HashSet<i64>,
    ) -> Result<WindowWait> {
        self.wait_for_window_within(pid, seen_before, WINDOW_WAIT_TIMEOUT)
            .await
    }

    /// [`wait_for_window`](Self::wait_for_window) with the bound injectable,
    /// so tests exercise the timeout arms in milliseconds.
    async fn wait_for_window_within(
        &self,
        pid: u32,
        seen_before: &HashSet<i64>,
        timeout: Duration,
    ) -> Result<WindowWait> {
        let start = Instant::now();
        loop {
            let mut windows = self
                .windows
                .windows()
                .await
                .map_err(ProgramsError::Desktop)?;
            let matched = windows
                .iter()
                .position(|window| window.pid == i64::from(pid))
                .or_else(|| {
                    windows
                        .iter()
                        .position(|window| !seen_before.contains(&window.pid))
                });
            if let Some(index) = matched {
                return Ok(WindowWait::Appeared {
                    window: windows.swap_remove(index).into(),
                    waited_ms: start.elapsed().as_millis() as u64,
                });
            }

            if start.elapsed() >= timeout {
                let waited_ms = start.elapsed().as_millis() as u64;
                return Ok(if is_running(pid) {
                    WindowWait::TimedOut { waited_ms }
                } else {
                    WindowWait::ProcessExited { waited_ms }
                });
            }
            tokio::time::sleep(WINDOW_WAIT_POLL).await;
        }
    }

    /// Start a GUI program in the background.
    ///
    /// Only a bare executable name from [`list`](Self::list) is accepted.
    /// Arguments are refused outright: programs that take `--exec` / `-e`
    /// style flags would otherwise turn this into arbitrary command execution
    /// (CWE-78), and the `.desktop` catalogue is the whitelist that keeps the
    /// executable itself from being anything the agent likes.
    pub async fn launch(&self, command: &str) -> Result<Launched> {
        let parts = shlex::split(command).ok_or_else(|| ProgramsError::Syntax(command.into()))?;

        let [executable] = parts.as_slice() else {
            return Err(if parts.is_empty() {
                ProgramsError::Empty
            } else {
                ProgramsError::Arguments(command.into())
            });
        };

        let name = Path::new(executable)
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();

        // One call is both the whitelist check and the lookup, so what is
        // admitted and what is spawned can never be two different things.
        let Some(program) = self.catalog.resolve(&name) else {
            return Err(ProgramsError::Unknown(name));
        };

        std::fs::create_dir_all(LOG_DIR).map_err(ProgramsError::Log)?;

        // A unique temp name first: two concurrent launches must not race for
        // the same path before either knows its PID.
        let staging = PathBuf::from(LOG_DIR).join(format!(
            "proc-{}-{}.log",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0),
        ));
        let log = std::fs::File::create(&staging).map_err(ProgramsError::Log)?;
        let log_err = log.try_clone().map_err(ProgramsError::Log)?;

        let mut command_builder = Command::new(&program);
        command_builder
            .stdin(Stdio::null())
            .stdout(Stdio::from(log))
            .stderr(Stdio::from(log_err))
            .env_clear()
            .envs(launch_env())
            // Its own process group, so a signal aimed at the server does not
            // take the agent's windows down with it.
            .process_group(0);

        let child = match command_builder.spawn() {
            Ok(child) => child,
            Err(source) => {
                std::fs::remove_file(&staging).ok();
                return Err(ProgramsError::Spawn { program, source });
            }
        };

        let pid = child.id().ok_or(ProgramsError::Vanished)?;

        let final_path = PathBuf::from(LOG_DIR).join(format!("proc-{pid}.log"));
        std::fs::rename(&staging, &final_path).map_err(ProgramsError::Log)?;

        self.launched
            .lock()
            .expect("launched-pid registry poisoned")
            .insert(pid);

        // Reap it in the background. Without this the child becomes a zombie
        // on exit, and `app_status` would report a dead program as running
        // forever — a zombie still answers `kill(pid, 0)`.
        tokio::spawn(async move {
            let mut child = child;
            let _ = child.wait().await;
        });

        tracing::info!(
            target: "features::programs",
            program = %name,
            pid,
            log = %final_path.display(),
            "launched",
        );

        Ok(Launched {
            pid,
            log_file: final_path.to_string_lossy().into_owned(),
            action: format!("Launched: {name}"),
        })
    }

    /// Check on a program started by [`launch`](Self::launch) in this session.
    pub fn status(&self, pid: u32, lines: usize) -> Result<ProgramStatus> {
        if !self
            .launched
            .lock()
            .expect("launched-pid registry poisoned")
            .contains(&pid)
        {
            return Err(ProgramsError::Untracked(pid));
        }

        let log_file = PathBuf::from(LOG_DIR).join(format!("proc-{pid}.log"));
        Ok(ProgramStatus {
            pid,
            running: is_running(pid),
            tail: tail(&log_file, lines),
            log_file: log_file.to_string_lossy().into_owned(),
        })
    }
}

/// The environment a launched program gets: every server secret stripped,
/// PATH extended for Debian's game basenames.
fn launch_env() -> Vec<(String, String)> {
    std::env::vars()
        .filter(|(key, _)| !key.starts_with(SCRUBBED_PREFIX))
        .map(|(key, value)| {
            if key == "PATH" {
                (key, format!("{value}:{EXTRA_PATH}"))
            } else {
                (key, value)
            }
        })
        .collect()
}

/// Whether a process is still alive. A live process we may not signal
/// (`EPERM`) still counts as running.
fn is_running(pid: u32) -> bool {
    // `Pid::from_raw` rejects 0 (which would mean "our whole process group")
    // and negatives. Only PIDs this session launched reach here, so the
    // `None` arm is unreachable in practice — it just refuses to guess.
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

/// The last `lines` lines of a file, or `""` when it is not there.
fn tail(path: &Path, lines: usize) -> String {
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
    use crate::testing::{EmptyCatalog, NoWindows, OneWindow};

    fn service() -> ProgramsService {
        ProgramsService {
            windows: Arc::new(NoWindows),
            catalog: Arc::new(EmptyCatalog),
            launched: Mutex::default(),
        }
    }

    fn service_showing(pid: i64) -> ProgramsService {
        ProgramsService {
            windows: Arc::new(OneWindow(pid)),
            catalog: Arc::new(EmptyCatalog),
            launched: Mutex::default(),
        }
    }

    /// A PID that has certainly exited: spawn `true(1)` and reap it.
    fn dead_pid() -> u32 {
        let mut child = std::process::Command::new("true").spawn().unwrap();
        let pid = child.id();
        child.wait().unwrap();
        pid
    }

    #[tokio::test]
    async fn a_window_owned_by_the_launched_pid_ends_the_wait() {
        match service_showing(4242)
            .wait_for_window(4242, &HashSet::new())
            .await
            .unwrap()
        {
            WindowWait::Appeared { window, .. } => assert_eq!(window.pid, 4242),
            other => panic!("expected Appeared, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn a_window_from_a_pid_unseen_before_the_launch_also_counts() {
        // The wrapper-script case: the spawned PID is already dead, the
        // window belongs to its child — a PID the pre-launch snapshot never
        // saw. That window is the launch's window.
        match service_showing(7777)
            .wait_for_window(dead_pid(), &HashSet::new())
            .await
            .unwrap()
        {
            WindowWait::Appeared { window, .. } => assert_eq!(window.pid, 7777),
            other => panic!("expected Appeared, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn a_window_already_there_before_the_launch_does_not_count() {
        // The only window on screen predates the launch and belongs to
        // another PID; the launched process is dead. Claiming that window
        // would hand the agent someone else's UI.
        match service_showing(7777)
            .wait_for_window_within(
                dead_pid(),
                &HashSet::from([7777]),
                Duration::from_millis(50),
            )
            .await
            .unwrap()
        {
            WindowWait::ProcessExited { .. } => {}
            other => panic!("expected ProcessExited, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn a_live_windowless_process_times_out_rather_than_reporting_death() {
        // Our own PID: alive for the whole test, guaranteed windowless in
        // the fake desktop.
        match service()
            .wait_for_window_within(
                std::process::id(),
                &HashSet::new(),
                Duration::from_millis(50),
            )
            .await
            .unwrap()
        {
            // The verdict carries its own measurement, so the caller never
            // needs to import the bound to describe the outcome.
            WindowWait::TimedOut { waited_ms } => assert!(waited_ms >= 50),
            other => panic!("expected TimedOut, got {other:?}"),
        }
    }

    #[test]
    fn server_secrets_never_reach_a_launched_program() {
        unsafe {
            std::env::set_var("GHOSTDESK_AUTH__TOKEN", "super-secret");
            std::env::set_var("GHOSTDESK_VNC_PASSWORD", "hunter2");
            std::env::set_var("GHOSTDESK_SCREEN__WIDTH", "1280");
            std::env::set_var("XDG_RUNTIME_DIR", "/run/user/1000");
        }

        let env = launch_env();
        let keys: Vec<&str> = env.iter().map(|(k, _)| k.as_str()).collect();

        assert!(!keys.contains(&"GHOSTDESK_AUTH__TOKEN"));
        assert!(!keys.contains(&"GHOSTDESK_VNC_PASSWORD"));
        assert!(
            !keys.contains(&"GHOSTDESK_SCREEN__WIDTH"),
            "the whole namespace goes, not just the secrets in it",
        );
        assert!(
            keys.contains(&"XDG_RUNTIME_DIR"),
            "the Wayland plumbing a GUI program actually needs survives",
        );
    }

    #[test]
    fn the_launch_path_reaches_debians_games_directory() {
        unsafe { std::env::set_var("PATH", "/usr/bin") }
        let env = launch_env();
        let path = env
            .iter()
            .find(|(k, _)| k == "PATH")
            .map(|(_, v)| v.as_str())
            .unwrap();
        assert!(path.contains("/usr/games"));
    }

    #[tokio::test]
    async fn arguments_are_refused_before_anything_is_spawned() {
        let err = service()
            .launch("firefox --new-window https://example.com")
            .await
            .unwrap_err();
        assert!(matches!(err, ProgramsError::Arguments(_)), "got: {err}");
    }

    #[tokio::test]
    async fn an_unknown_executable_is_refused() {
        let err = service()
            .launch("definitely-not-installed")
            .await
            .unwrap_err();
        assert!(matches!(err, ProgramsError::Unknown(_)), "got: {err}");
    }

    #[test]
    fn status_refuses_a_pid_this_session_did_not_launch() {
        let err = service().status(1, 50).unwrap_err();
        assert!(matches!(err, ProgramsError::Untracked(1)), "got: {err}");
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
