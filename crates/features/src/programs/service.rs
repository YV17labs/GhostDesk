use std::collections::HashSet;
use std::path::Path;
use std::process::Stdio;
use std::sync::Arc;
use std::time::{Duration, Instant};

use nest_rs::core::{EnvPrefix, injectable};
use nest_rs::health::indicators;
use platform::desktop::{AppCatalog, DesktopApp};
use platform::window::WindowManager;
use tokio::process::Command;

use super::error::ProgramsError;
use super::launched::Launched;
use super::program_status::ProgramStatus;
use super::registry::{LaunchRegistry, tail};
use super::running_window::RunningWindow;
use super::window_wait::WindowWait;

type Result<T> = std::result::Result<T, ProgramsError>;

const EXTRA_PATH: &str = "/usr/games:/usr/local/games";

const SCRUBBED_PREFIX: &str = "GHOSTDESK_";

const WINDOW_WAIT_TIMEOUT: Duration = Duration::from_secs(10);

const WINDOW_WAIT_POLL: Duration = Duration::from_millis(150);

#[injectable]
pub struct ProgramsService {
    #[inject]
    windows: Arc<dyn WindowManager>,
    #[inject]
    catalog: Arc<dyn AppCatalog>,
    #[inject]
    registry: Arc<LaunchRegistry>,
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
                return Ok(if self.registry.is_running(pid) {
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

        let (staged, out, errors) = self.registry.stage_log()?;

        let mut command_builder = Command::new(&program);
        command_builder
            .stdin(Stdio::null())
            .stdout(out)
            .stderr(errors)
            .env_clear()
            .envs(launch_env())
            .process_group(0);

        let child = match command_builder.spawn() {
            Ok(child) => child,
            Err(source) => {
                self.registry.discard(&staged);
                return Err(if source.kind() == std::io::ErrorKind::NotFound {
                    ProgramsError::Missing(name)
                } else {
                    ProgramsError::Spawn { program, source }
                });
            }
        };

        let pid = child.id().ok_or(ProgramsError::Vanished)?;
        let log_file = self.registry.adopt(pid, &staged)?;

        tokio::spawn(async move {
            let mut child = child;
            let _ = child.wait().await;
        });

        tracing::info!(
            target: "features::programs",
            program = %name,
            pid,
            log = %log_file.display(),
            "launched",
        );

        Ok(Launched {
            pid,
            log_file: log_file.to_string_lossy().into_owned(),
            action: format!("Launched: {name}"),
        })
    }

    pub fn status(&self, pid: u32, lines: usize) -> Result<ProgramStatus> {
        if !self.registry.tracked(pid) {
            return Err(ProgramsError::Untracked(pid));
        }

        let log_file = self.registry.log_path(pid);

        Ok(ProgramStatus {
            pid,
            running: self.registry.is_running(pid),
            tail: tail(&log_file, lines),
            log_file: log_file.to_string_lossy().into_owned(),
        })
    }
}

fn launch_env() -> Vec<(String, String)> {
    scrub(std::env::vars(), EnvPrefix::current())
}

/// The scrub itself, over the variables it is handed rather than over the
/// process that runs it.
///
/// A secret this iterator never carried cannot be shown to have been removed,
/// so the boundary is only really guarded by a caller that supplies the
/// secret — which is what the tests do, and what reading the live environment
/// would have made impossible to guarantee.
fn scrub(vars: impl Iterator<Item = (String, String)>, prefix: &str) -> Vec<(String, String)> {
    let active = format!("{prefix}_");
    vars.filter(|(key, _)| !key.starts_with(SCRUBBED_PREFIX) && !key.starts_with(&active))
        .map(|(key, value)| {
            if key == "PATH" {
                (key, format!("{value}:{EXTRA_PATH}"))
            } else {
                (key, value)
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{EmptyCatalog, NoWindows, OneWindow};

    fn service() -> ProgramsService {
        ProgramsService {
            windows: Arc::new(NoWindows),
            catalog: Arc::new(EmptyCatalog),
            registry: Arc::new(LaunchRegistry::default()),
        }
    }

    fn service_showing(pid: i64) -> ProgramsService {
        ProgramsService {
            windows: Arc::new(OneWindow(pid)),
            catalog: Arc::new(EmptyCatalog),
            registry: Arc::new(LaunchRegistry::default()),
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

    /// The environment a launched program is handed, spelled out here rather
    /// than read from the process: these three tests are the one place the
    /// scrub is proved, and a secret that was never in the input proves
    /// nothing about the filter that did not remove it.
    fn desk_environment() -> Vec<(String, String)> {
        [
            ("GHOSTDESK_AUTH__TOKEN", "super-secret"),
            ("GHOSTDESK_VNC_PASSWORD", "hunter2"),
            ("GHOSTDESK_SCREEN__WIDTH", "1280"),
            ("XDG_RUNTIME_DIR", "/run/user/1000"),
            ("PATH", "/usr/bin"),
        ]
        .into_iter()
        .map(|(key, value)| (key.to_owned(), value.to_owned()))
        .collect()
    }

    #[test]
    fn a_renamed_prefix_still_loses_the_token() {
        // `NESTRS_ENV_PREFIX` renames every framework variable at once, and the
        // literal `GHOSTDESK_` scrub cannot follow it — so the active prefix is
        // swept as well, and this is what proves the second half runs.
        let renamed = [
            ("ACME_AUTH__TOKEN", "super-secret"),
            // The container's own knobs are set straight on the process, so
            // they do not follow the framework's rename — which is why the
            // scrub carries a literal prefix as well as the active one.
            ("GHOSTDESK_VNC_PASSWORD", "hunter2"),
            ("XDG_RUNTIME_DIR", "/run/user/1000"),
        ]
        .map(|(key, value)| (key.to_owned(), value.to_owned()));
        let keys: Vec<String> = scrub(renamed.into_iter(), "ACME")
            .into_iter()
            .map(|(key, _)| key)
            .collect();

        assert!(
            !keys.iter().any(|key| key == "ACME_AUTH__TOKEN"),
            "the desk's own secret must not reach a launched program: {keys:?}",
        );
        assert!(
            !keys.iter().any(|key| key == "GHOSTDESK_VNC_PASSWORD"),
            "a rename must not strand the container's own knobs: {keys:?}",
        );
        assert!(
            keys.iter().any(|key| key == "XDG_RUNTIME_DIR"),
            "only the desk's namespace goes: {keys:?}",
        );
    }

    #[test]
    fn server_secrets_never_reach_a_launched_program() {
        let env = scrub(desk_environment().into_iter(), "GHOSTDESK");
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
        let env = scrub(desk_environment().into_iter(), "GHOSTDESK");
        let path = env
            .iter()
            .find(|(k, _)| k == "PATH")
            .map(|(_, v)| v.as_str())
            .expect("PATH survives the scrub");

        assert!(
            path.starts_with("/usr/bin"),
            "the inherited PATH leads: {path}"
        );
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

    #[tokio::test]
    async fn a_refused_launch_stages_no_log() {
        // The catalogue is consulted before the log file exists, so a rejected
        // command cannot litter the log directory.
        let service = service();
        service
            .launch("definitely-not-installed")
            .await
            .unwrap_err();
        assert!(!service.registry.tracked(std::process::id()));
    }

    #[test]
    fn status_refuses_a_pid_this_session_did_not_launch() {
        let err = service().status(1, 50).unwrap_err();
        assert!(matches!(err, ProgramsError::Untracked(1)), "got: {err}");
    }
}
