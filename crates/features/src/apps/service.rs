//! Application launching and process tracking.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use nest_rs::core::injectable;
use platform::desktop::{AppCatalog, DesktopApp};
use platform::window::WindowManager;
use tokio::process::Command;

/// Where launched apps' stdout and stderr land.
const LOG_DIR: &str = "/tmp/ghostdesk";

/// Debian policy §9.1.1 installs games under `/usr/games`, which is not in a
/// non-interactive shell's default PATH. Extending it is what lets a
/// `.desktop` `Exec=` basename like `gnome-chess` resolve.
const EXTRA_PATH: &str = "/usr/games:/usr/local/games";

/// The server's own environment namespace, swept wholesale before a GUI app
/// is spawned.
///
/// The MCP server needs these; a browser the agent spawns does not — and any
/// code execution inside that browser would otherwise inherit them. Under
/// `NESTRS_ENV_PREFIX=GHOSTDESK` this is a single prefix rather than a list of
/// names to keep in step with the config structs: everything the server reads
/// is under it, so nothing new can be added on one side and forgotten here.
///
/// A literal rather than `EnvPrefix::current()`: the container's own knobs
/// (`GHOSTDESK_VNC_PASSWORD`) carry this spelling without passing through the
/// framework at all, so following the framework's prefix would drop them the
/// moment the two diverged — which the entrypoint refuses to let happen.
///
/// Sweeping non-secrets (`GHOSTDESK_SCREEN__WIDTH`) along with the bearer
/// token is deliberate. A launched `firefox` has no use for the server's
/// settings, and a prefix rule that admits exceptions stops being a rule.
const SCRUBBED_PREFIX: &str = "GHOSTDESK_";

/// Trailing log lines returned by default.
pub const DEFAULT_TAIL: usize = 50;

/// One open window, as the agent sees it.
#[derive(Debug, Clone, serde::Serialize, schemars::JsonSchema)]
pub struct RunningApp {
    /// Stable application identity, as this desktop reports it.
    pub app: String,
    pub title: String,
    pub pid: i64,
    pub focused: bool,
}

/// What `app_launch` answers with.
#[derive(Debug, Clone, serde::Serialize, schemars::JsonSchema)]
pub struct Launched {
    pub pid: u32,
    pub log_file: String,
    pub action: String,
}

/// What `app_status` answers with.
#[derive(Debug, Clone, serde::Serialize, schemars::JsonSchema)]
pub struct AppStatus {
    pub pid: u32,
    pub running: bool,
    pub log_file: String,
    pub tail: String,
}

#[injectable]
pub struct AppsService {
    #[inject]
    windows: Arc<dyn WindowManager>,
    #[inject]
    catalog: Arc<dyn AppCatalog>,
    /// PIDs launched by this session. `app_status` refuses anything else, so
    /// the tool cannot be turned into a general-purpose process prober.
    launched: Mutex<HashSet<u32>>,
}

impl AppsService {
    /// The catalogue of installed GUI applications.
    pub fn list(&self) -> Vec<DesktopApp> {
        self.catalog.apps()
    }

    /// The application windows currently open on the desktop.
    pub async fn running(&self) -> anyhow::Result<Vec<RunningApp>> {
        Ok(self
            .windows
            .windows()
            .await?
            .into_iter()
            .map(|window| RunningApp {
                app: window.app,
                title: window.title,
                pid: window.pid,
                focused: window.focused,
            })
            .collect())
    }

    /// Start a GUI application in the background.
    ///
    /// Only a bare executable name from [`list`](Self::list) is accepted.
    /// Arguments are refused outright: apps that take `--exec` / `-e` style
    /// flags would otherwise turn this into arbitrary command execution
    /// (CWE-78), and the `.desktop` catalogue is the whitelist that keeps the
    /// executable itself from being anything the agent likes.
    pub async fn launch(&self, command: &str) -> anyhow::Result<Launched> {
        let parts = shlex::split(command)
            .ok_or_else(|| anyhow::anyhow!("Invalid command syntax: {command:?}"))?;

        let [executable] = parts.as_slice() else {
            if parts.is_empty() {
                anyhow::bail!("No command provided");
            }
            anyhow::bail!(
                "Arguments are not allowed — pass only the executable name \
                 from app_list(). Got: {command:?}"
            );
        };

        let name = Path::new(executable)
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();

        // One call is both the whitelist check and the lookup, so what is
        // admitted and what is spawned can never be two different things.
        let Some(program) = self.catalog.resolve(&name) else {
            anyhow::bail!(
                "{name:?} is not a known GUI app. Call app_list() to see what is available."
            );
        };

        std::fs::create_dir_all(LOG_DIR)?;

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
        let log = std::fs::File::create(&staging)?;
        let log_err = log.try_clone()?;

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
            Err(err) => {
                std::fs::remove_file(&staging).ok();
                anyhow::bail!("Command not found: {} ({err})", program.display());
            }
        };

        let pid = child
            .id()
            .ok_or_else(|| anyhow::anyhow!("the launched process exited before it was tracked"))?;

        let final_path = PathBuf::from(LOG_DIR).join(format!("proc-{pid}.log"));
        std::fs::rename(&staging, &final_path)?;

        self.launched
            .lock()
            .expect("launched-pid registry poisoned")
            .insert(pid);

        // Reap it in the background. Without this the child becomes a zombie
        // on exit, and `app_status` would report a dead app as running
        // forever — a zombie still answers `kill(pid, 0)`.
        tokio::spawn(async move {
            let mut child = child;
            let _ = child.wait().await;
        });

        tracing::info!(
            target: "ghostdesk::apps",
            app = %name,
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

    /// Check on an app started by [`launch`](Self::launch) in this session.
    pub fn status(&self, pid: u32, lines: usize) -> anyhow::Result<AppStatus> {
        if !self
            .launched
            .lock()
            .expect("launched-pid registry poisoned")
            .contains(&pid)
        {
            anyhow::bail!("PID {pid} was not launched by this session. Use app_launch() first.");
        }

        let log_file = PathBuf::from(LOG_DIR).join(format!("proc-{pid}.log"));
        Ok(AppStatus {
            pid,
            running: is_running(pid),
            tail: tail(&log_file, lines),
            log_file: log_file.to_string_lossy().into_owned(),
        })
    }
}

/// The environment a launched app gets: every server secret stripped, PATH
/// extended for Debian's game basenames.
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
    use crate::testing::{EmptyCatalog, NoWindows};

    fn service() -> AppsService {
        AppsService {
            windows: Arc::new(NoWindows),
            catalog: Arc::new(EmptyCatalog),
            launched: Mutex::default(),
        }
    }

    #[test]
    fn server_secrets_never_reach_a_launched_app() {
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
            "the whole GHOSTDESK_ namespace goes, not just the secrets in it",
        );
        assert!(
            keys.contains(&"XDG_RUNTIME_DIR"),
            "the Wayland plumbing a GUI app actually needs survives",
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
            .unwrap_err()
            .to_string();
        assert!(err.contains("Arguments are not allowed"), "got: {err}");
    }

    #[tokio::test]
    async fn an_unknown_executable_is_refused() {
        let err = service()
            .launch("definitely-not-installed")
            .await
            .unwrap_err()
            .to_string();
        assert!(err.contains("is not a known GUI app"), "got: {err}");
    }

    #[test]
    fn status_refuses_a_pid_this_session_did_not_launch() {
        let err = service().status(1, DEFAULT_TAIL).unwrap_err().to_string();
        assert!(
            err.contains("was not launched by this session"),
            "got: {err}"
        );
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
