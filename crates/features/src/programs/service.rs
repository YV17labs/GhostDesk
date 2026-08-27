use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use nest_rs::core::{EnvPrefix, injectable};
use nest_rs::health::indicators;
use platform::desktop::{AppCatalog, DesktopApp};
use platform::window::{WindowInfo, WindowManager};
use tokio::process::Command;

use super::error::ProgramsError;

type Result<T> = std::result::Result<T, ProgramsError>;

const LOG_DIR: &str = "/tmp/ghostdesk";

const EXTRA_PATH: &str = "/usr/games:/usr/local/games";

const SCRUBBED_PREFIX: &str = "GHOSTDESK_";

const WINDOW_WAIT_TIMEOUT: Duration = Duration::from_secs(10);

const WINDOW_WAIT_POLL: Duration = Duration::from_millis(150);

#[derive(Debug, Clone)]
pub struct RunningWindow {
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

#[derive(Debug, Clone)]
pub struct Launched {
    pub pid: u32,
    pub log_file: String,
    pub action: String,
}

#[derive(Debug)]
pub enum WindowWait {
    Appeared {
        window: RunningWindow,
        waited_ms: u64,
    },
    ProcessExited {
        waited_ms: u64,
    },
    TimedOut {
        waited_ms: u64,
    },
}

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
    launched: Mutex<HashSet<u32>>,
}

#[indicators]
impl ProgramsService {
    #[readiness]
    async fn window_manager(&self) -> anyhow::Result<()> {
        self.windows.windows().await.map(|_| ())
    }
}

impl ProgramsService {
    pub fn list(&self) -> Vec<DesktopApp> {
        self.catalog.apps()
    }

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

    pub async fn wait_for_window(
        &self,
        pid: u32,
        seen_before: &HashSet<i64>,
    ) -> Result<WindowWait> {
        self.wait_for_window_within(pid, seen_before, WINDOW_WAIT_TIMEOUT)
            .await
    }

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

        let Some(program) = self.catalog.resolve(&name) else {
            return Err(ProgramsError::Unknown(name));
        };

        std::fs::create_dir_all(LOG_DIR).map_err(ProgramsError::Log)?;

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
            .process_group(0);

        let child = match command_builder.spawn() {
            Ok(child) => child,
            Err(source) => {
                std::fs::remove_file(&staging).ok();
                return Err(if source.kind() == std::io::ErrorKind::NotFound {
                    ProgramsError::Missing(name)
                } else {
                    ProgramsError::Spawn { program, source }
                });
            }
        };

        let pid = child.id().ok_or(ProgramsError::Vanished)?;

        let final_path = PathBuf::from(LOG_DIR).join(format!("proc-{pid}.log"));
        std::fs::rename(&staging, &final_path).map_err(ProgramsError::Log)?;

        self.launched
            .lock()
            .expect("launched-pid registry poisoned")
            .insert(pid);

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

fn launch_env() -> Vec<(String, String)> {
    let active = format!("{}_", EnvPrefix::current());
    std::env::vars()
        .filter(|(key, _)| !key.starts_with(SCRUBBED_PREFIX) && !key.starts_with(&active))
        .map(|(key, value)| {
            if key == "PATH" {
                (key, format!("{value}:{EXTRA_PATH}"))
            } else {
                (key, value)
            }
        })
        .collect()
}

fn is_running(pid: u32) -> bool {
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
        match service()
            .wait_for_window_within(
                std::process::id(),
                &HashSet::new(),
                Duration::from_millis(50),
            )
            .await
            .unwrap()
        {
            WindowWait::TimedOut { waited_ms } => assert!(waited_ms >= 50),
            other => panic!("expected TimedOut, got {other:?}"),
        }
    }

    #[test]
    fn a_renamed_prefix_still_loses_the_token() {
        unsafe {
            std::env::set_var("NESTRS_ENV_PREFIX", "ACME");
            std::env::set_var("ACME_AUTH__TOKEN", "super-secret");
        }
        assert_eq!(EnvPrefix::current(), "ACME", "the rename took effect");

        let keys: Vec<String> = launch_env().into_iter().map(|(k, _)| k).collect();
        assert!(
            !keys.iter().any(|key| key == "ACME_AUTH__TOKEN"),
            "the desk's own secret must not reach a launched program: {keys:?}",
        );
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
