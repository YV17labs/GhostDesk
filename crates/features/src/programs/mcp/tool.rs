use std::sync::Arc;

use nest_rs::mcp::{CallToolResult, Json, McpError, Opaque, Parameters, mcp, tools};

use crate::blame::Answered;
use crate::programs::dtos::{
    LaunchDto, LaunchedDto, ListedDto, ProgramDto, ProgramStatusDto, StatusDto, WindowDto,
};
use crate::programs::service::ProgramsService;
use crate::programs::window_wait::WindowWait;
// The one cross-domain reach in the tree, and it is `app_launch`'s whole
// point: the settled frame rides back with the launch so the agent needs no
// follow-up `screen_shot()`. Owned in AGENTS.md, *Known deviations*.
use crate::screen::{CaptureDto, ScreenService};

#[mcp]
#[derive(Clone)]
pub struct ProgramsTool {
    #[inject]
    programs_svc: Arc<ProgramsService>,
    #[inject]
    screen_svc: Arc<ScreenService>,
}

#[tools]
impl ProgramsTool {
    #[tool(
        description = "Return the catalogue of installed GUI applications, \
            as this desktop registers them.\n\n\
            This catalogue is the strict whitelist — app_launch refuses any \
            executable not in it. Call this the first time a task names an \
            application, and again after installing software during the \
            session. Each entry has a name and an exec; exec is the string to \
            pass to app_launch().",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    #[public]
    async fn app_list(&self) -> Result<Json<ListedDto<ProgramDto>>, McpError> {
        Ok(Json(ListedDto::new(
            self.programs_svc.list().into_iter().map(ProgramDto::from),
        )))
    }

    #[tool(
        description = "List the application windows currently open on the \
            desktop — one entry per real client window. Workspaces, outputs \
            and the bar are not included.\n\n\
            Call this before app_launch(): if the app is already in the list, \
            switch to its window instead of launching a second instance and \
            doubling memory use.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    #[public]
    async fn app_running(&self) -> Result<Json<ListedDto<WindowDto>>, McpError> {
        let windows = self.programs_svc.running().await.answered()?;
        Ok(Json(ListedDto::new(
            windows.into_iter().map(WindowDto::from),
        )))
    }

    #[tool(
        description = "Start a GUI application and wait for its window.\n\n\
            Only bare executable names listed by app_list() are accepted (e.g. \
            \"firefox\"). Command-line arguments are not allowed — pass the \
            exec field from app_list() verbatim.\n\n\
            By default the call waits (up to 10 s) for the app's first window \
            and returns the settled screen as an image — interact with that \
            directly, no follow-up screen_shot() needed. If the result says \
            the process exited without a window, tail its log with \
            app_status(pid); if it says no window appeared in time, the app \
            is slow to start or has no UI — set wait_for_window: false for \
            the latter kind.\n\n\
            The process runs detached; its stdout and stderr are captured to \
            /tmp/ghostdesk/proc-<pid>.log and can be tailed with \
            app_status(pid). Check app_running() first — the target may \
            already be open.",
        annotations(destructive_hint = false, open_world_hint = false),
        output_schema = rmcp::handler::server::tool::schema_for_output::<LaunchedDto>()
    )]
    #[public]
    async fn app_launch(
        &self,
        Parameters(params): Parameters<LaunchDto>,
    ) -> Result<CallToolResult, McpError> {
        let seen_before = if params.wait_for_window {
            Some(
                self.programs_svc
                    .running()
                    .await
                    .answered()?
                    .into_iter()
                    .map(|window| window.pid)
                    .collect(),
            )
        } else {
            None
        };

        let launched = self.programs_svc.launch(&params.command).await.answered()?;
        let mut answer = LaunchedDto::from(launched);

        let mut frame = None;
        if let Some(seen_before) = seen_before {
            match self
                .programs_svc
                .wait_for_window(answer.pid, &seen_before)
                .await
                .answered()?
            {
                WindowWait::Appeared { window, waited_ms } => {
                    answer.action = format!(
                        "{}; window \"{}\" appeared after {waited_ms} ms",
                        answer.action, window.title,
                    );
                    answer.window = Some(WindowDto::from(window));
                    answer.window_wait_ms = Some(waited_ms);

                    match self.screen_svc.capture_settled().await {
                        Ok(capture) => frame = Some(capture),
                        Err(err) => tracing::warn!(
                            target: "features::programs",
                            error = %err,
                            "window appeared but the settled frame could not be captured",
                        ),
                    }
                }
                WindowWait::ProcessExited { waited_ms } => {
                    answer.action = format!(
                        "{}; the process exited and no window appeared within {} s — it \
                         crashed (check app_status({})) or handed off to an \
                         already-running instance (check app_running())",
                        answer.action,
                        waited_ms / 1000,
                        answer.pid,
                    );
                }
                WindowWait::TimedOut { waited_ms } => {
                    answer.action = format!(
                        "{}; still running but no window after {} s — slow to start \
                         (check app_running() shortly) or running without a UI",
                        answer.action,
                        waited_ms / 1000,
                    );
                }
            }
        }

        let structured = serde_json::to_value(&answer).opaque()?;
        let mut result = CallToolResult::structured(structured);
        if let Some(capture) = frame {
            result.content.push(CaptureDto::from(&capture).block());
        }
        Ok(result)
    }

    #[tool(
        description = "Check on an app started by app_launch() in this \
            session. Reports whether the process is still alive and returns \
            the tail of its captured stdout/stderr.\n\n\
            Only PIDs returned by app_launch() are accepted. Use this to \
            confirm an app crashed (tail the log for a traceback) or to watch \
            a long-running app write progress to its output.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    #[public]
    async fn app_status(
        &self,
        Parameters(params): Parameters<StatusDto>,
    ) -> Result<Json<ProgramStatusDto>, McpError> {
        let status = self
            .programs_svc
            .status(params.pid, params.lines)
            .answered()?;
        Ok(Json(ProgramStatusDto::from(status)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::programs::error::ProgramsError;

    #[test]
    fn a_server_failure_leaves_as_the_shared_opaque_message() {
        let err = Err::<(), _>(ProgramsError::Desktop(anyhow::anyhow!(
            "wayland socket /run/user/1000/wayland-1 refused"
        )))
        .answered()
        .unwrap_err();

        assert_eq!(err.message, nest_rs::core::OPAQUE_CLIENT_MESSAGE);
    }

    #[test]
    fn a_permission_denied_spawn_leaves_as_the_shared_opaque_message() {
        let err = Err::<(), _>(ProgramsError::Spawn {
            program: "/usr/lib/firefox-esr/firefox".into(),
            source: std::io::Error::from(std::io::ErrorKind::PermissionDenied),
        })
        .answered()
        .unwrap_err();

        assert_eq!(err.message, nest_rs::core::OPAQUE_CLIENT_MESSAGE);
    }

    #[test]
    fn a_missing_executable_names_what_the_caller_asked_for() {
        let err = Err::<(), _>(ProgramsError::Missing("firefox".into()))
            .answered()
            .unwrap_err();

        assert!(err.message.contains("firefox"), "{}", err.message);
        assert!(
            !err.message.contains('/'),
            "no host path reaches the model: {}",
            err.message,
        );
    }

    #[test]
    fn a_refusal_the_caller_can_act_on_names_what_to_change() {
        let err = Err::<(), _>(ProgramsError::Unknown("gimp".into()))
            .answered()
            .unwrap_err();

        assert!(err.message.contains("gimp"), "{}", err.message);
        assert!(err.message.contains("app_list()"), "{}", err.message);
    }

    #[test]
    fn every_tool_closes_its_world() {
        for tool in ProgramsTool::tool_router().list_all() {
            assert_eq!(
                tool.annotations.and_then(|hints| hints.open_world_hint),
                Some(false),
                "{} declares a closed world",
                tool.name,
            );
        }
    }

    #[test]
    fn the_tool_table_builds_and_app_launch_keeps_its_output_schema() {
        let router = ProgramsTool::tool_router();
        let launch = router
            .list_all()
            .into_iter()
            .find(|tool| tool.name == "app_launch")
            .expect("app_launch is in the tool table");
        let schema = launch
            .output_schema
            .expect("app_launch publishes an output schema");
        assert!(
            serde_json::to_string(&*schema)
                .unwrap()
                .contains("log_file"),
            "the schema still describes LaunchedDto",
        );
    }
}
