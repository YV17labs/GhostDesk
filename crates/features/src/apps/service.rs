//! Application launching and process tracking.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use nest_rs::core::injectable;
use platform::desktop::{self, DesktopApp};
use platform::sway;
use tokio::process::Command;

/// Where launched apps' stdout and stderr land.
const LOG_DIR: &str = "/tmp/ghostdesk";

/// Debian policy §9.1.1 installs games under `/usr/games`, which is not in a
/// non-interactive shell's default PATH. Extending it is what lets a
/// `.desktop` `Exec=` basename like `gnome-chess` resolve.
const EXTRA_PATH: &str = "/usr/games:/usr/local/games";

/// Server-only secrets that must never reach a launched GUI app.
///
/// The MCP server needs them; a browser the agent spawns does not — and any
/// code execution inside that browser would otherwise inherit them. The
/// `NESTRS_` prefix is swept wholesale because it carries the bearer token,
/// inline TLS key material and every other transport secret.
const SCRUBBED_PREFIXES: &[&str] = &["NESTRS_", "GHOSTDESK_TLS_"];
const SCRUBBED_KEYS: &[&str] = &["GHOSTDESK_AUTH_TOKEN", "GHOSTDESK_VNC_PASSWORD"];

/// Trailing log lines returned by default.
pub const DEFAULT_TAIL: usize = 50;

/// One open window, as the agent sees it.
#[derive(Debug, Clone, serde::Serialize)]
pub struct RunningApp {
    /// `app_id` for Wayland-native clients, else the X11 window class.
    pub app: String,
    pub title: String,
    pub pid: i64,
    pub focused: bool,
}

/// What `app_launch` answers with.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Launched {
    pub pid: u32,
    pub log_file: String,
    pub action: String,
}

/// What `app_status` answers with.
#[derive(Debug, Clone, serde::Serialize)]
pub struct AppStatus {
    pub pid: u32,
    pub running: bool,
    pub log_file: String,
    pub tail: String,
}

#[injectable]
#[derive(Default)]
pub struct AppsService {
    /// PIDs launched by this session. `app_status` refuses anything else, so
    /// the tool cannot be turned into a general-purpose process prober.
    launched: Mutex<HashSet<u32>>,
}

impl AppsService {
    /// The catalogue of installed GUI applications.
    pub fn list(&self) -> Vec<DesktopApp> {
        desktop::desktop_apps()
    }

    /// The application windows currently open on the desktop.
    pub async fn running(&self) -> anyhow::Result<Vec<RunningApp>> {
        let tree = sway::get_tree()
            .await
            .ok_or_else(|| anyhow::anyhow!("swaymsg get_tree failed (see server logs)"))?;

        Ok(sway::iter_views(&tree)
            .into_iter()
            .map(|node| RunningApp {
                app: node
                    .get("app_id")
                    .and_then(|v| v.as_str())
                    .or_else(|| {
                        node.get("window_properties")
                            .and_then(|p| p.get("class"))
                            .and_then(|v| v.as_str())
                    })
                    .unwrap_or("?")
                    .to_string(),
                title: node
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string(),
                pid: node.get("pid").and_then(|v| v.as_i64()).unwrap_or_default(),
                focused: node
                    .get("focused")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false),
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
            anyhow::bail!(
                "{}",
                if parts.is_empty() {
                    "No command provided".to_string()
                } else {
                    format!(
                        "Arguments are not allowed — pass only the executable name \
                         from app_list(). Got: {command:?}"
                    )
                }
            );
        };

        let name = Path::new(executable)
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();

        if !desktop::known_executables().contains(&name) {
            anyhow::bail!(
                "{name:?} is not a known GUI app. Call app_list() to see what is available."
            );
        }

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

        let mut command_builder = Command::new(executable);
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
                anyhow::bail!("Command not found: {executable} ({err})");
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
        .filter(|(key, _)| {
            !SCRUBBED_KEYS.contains(&key.as_str())
                && !SCRUBBED_PREFIXES
                    .iter()
                    .any(|prefix| key.starts_with(prefix))
        })
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
    match rustix::process::test_kill_process(rustix::process::Pid::from_raw(pid as i32).unwrap()) {
        Ok(()) => true,
        Err(rustix::io::Errno::PERM) => true,
        Err(_) => false,
    }
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

    #[test]
    fn server_secrets_never_reach_a_launched_app() {
        unsafe {
            std::env::set_var("NESTRS_GHOSTDESK__AUTH_TOKEN", "super-secret");
            std::env::set_var("GHOSTDESK_VNC_PASSWORD", "hunter2");
            std::env::set_var("GHOSTDESK_SCREEN_WIDTH", "1280");
        }

        let env = launch_env();
        let keys: Vec<&str> = env.iter().map(|(k, _)| k.as_str()).collect();

        assert!(!keys.contains(&"NESTRS_GHOSTDESK__AUTH_TOKEN"));
        assert!(!keys.contains(&"GHOSTDESK_VNC_PASSWORD"));
        assert!(
            keys.contains(&"GHOSTDESK_SCREEN_WIDTH"),
            "non-secret GhostDesk vars still pass through",
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
        let service = AppsService::default();
        let err = service
            .launch("firefox --new-window https://example.com")
            .await
            .unwrap_err()
            .to_string();
        assert!(err.contains("Arguments are not allowed"), "got: {err}");
    }

    #[tokio::test]
    async fn an_unknown_executable_is_refused() {
        let service = AppsService::default();
        let err = service
            .launch("definitely-not-installed")
            .await
            .unwrap_err()
            .to_string();
        assert!(err.contains("is not a known GUI app"), "got: {err}");
    }

    #[test]
    fn status_refuses_a_pid_this_session_did_not_launch() {
        let service = AppsService::default();
        let err = service.status(1, DEFAULT_TAIL).unwrap_err().to_string();
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
