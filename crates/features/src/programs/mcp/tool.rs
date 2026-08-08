//! The four program tools, plus the read-only counterpart of `app_list`.
//!
//! It plays the part a `#[controller]` plays over HTTP: inject the domain
//! services, translate wire DTOs into domain calls, and translate the results
//! back. No desktop logic lives here.

use std::sync::{Arc, LazyLock};

use nest_rs::mcp::ToolRouter;
use nest_rs::mcp::model::{
    CallToolRequestParams, CallToolResponse, ListResourcesResult, PaginatedRequestParams,
    ReadResourceRequestParams, ReadResourceResponse, ReadResourceResult, Resource,
    ResourceContents, ServerCapabilities, ServerInfo,
};
use nest_rs::mcp::rmcp;
use nest_rs::mcp::service::{RequestContext, RoleServer};
use nest_rs::mcp::{
    CallToolResult, Json, McpError, Parameters, ServerHandler, mcp, tool, tool_handler, tool_router,
};

use super::super::dtos::{
    LaunchDto, LaunchedDto, ListedDto, ProgramDto, ProgramStatusDto, StatusDto, WindowDto,
};
use super::super::error::ProgramsError;
use super::super::service::{ProgramsService, WindowWait};
use crate::screen::{CaptureDto, ScreenService};
use crate::telemetry::CallJournal;

/// The catalogue, fetchable without costing the agent a turn.
const CATALOGUE_URI: &str = "ghostdesk://apps";
const CATALOGUE_MIME: &str = "application/json";

/// Whose mistake it was, spelled once.
///
/// The match is exhaustive on purpose: a new variant is a compile error here,
/// not a refusal silently reported as a server crash. `invalid_params` is
/// what lets the model correct itself and retry, where `internal_error` reads
/// as "stop trying".
impl From<ProgramsError> for McpError {
    fn from(err: ProgramsError) -> Self {
        let message = err.to_string();
        match err {
            ProgramsError::Syntax(_)
            | ProgramsError::Empty
            | ProgramsError::Arguments(_)
            | ProgramsError::Unknown(_)
            | ProgramsError::Untracked(_)
            | ProgramsError::Spawn { .. } => Self::invalid_params(message, None),
            ProgramsError::Vanished | ProgramsError::Log(_) | ProgramsError::Desktop(_) => {
                tracing::error!(target: "features::programs", error = %message, "tool failed");
                Self::internal_error(message, None)
            }
        }
    }
}

#[mcp(path = "/mcp")]
#[derive(Clone)]
pub struct ProgramsTool {
    #[inject]
    programs: Arc<ProgramsService>,
    /// The settled frame `app_launch` hands back is a screen capture, so this
    /// tool composes the two domains the way the agent would otherwise have
    /// to over two calls.
    #[inject]
    screen: Arc<ScreenService>,
    #[inject]
    journal: Arc<CallJournal>,
}

#[tool_router]
impl ProgramsTool {
    #[tool(
        description = "Return the catalogue of installed GUI applications, \
            as this desktop registers them.\n\n\
            This catalogue is the strict whitelist — app_launch refuses any \
            executable not in it. Call this the first time a task names an \
            application, and again after installing software during the \
            session. Each entry has a name and an exec; exec is the string to \
            pass to app_launch().",
        annotations(read_only_hint = true, idempotent_hint = true)
    )]
    async fn app_list(&self) -> Result<Json<ListedDto<ProgramDto>>, McpError> {
        Ok(Json(ListedDto::new(
            self.programs.list().into_iter().map(ProgramDto::from),
        )))
    }

    #[tool(
        description = "List the application windows currently open on the \
            desktop — one entry per real client window. Workspaces, outputs \
            and the bar are not included.\n\n\
            Call this before app_launch(): if the app is already in the list, \
            switch to its window instead of launching a second instance and \
            doubling memory use.",
        annotations(read_only_hint = true)
    )]
    async fn app_running(&self) -> Result<Json<ListedDto<WindowDto>>, McpError> {
        let windows = self.programs.running().await?;
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
        annotations(destructive_hint = false),
        output_schema = rmcp::handler::server::tool::schema_for_output::<LaunchedDto>()
    )]
    async fn app_launch(
        &self,
        Parameters(params): Parameters<LaunchDto>,
    ) -> Result<CallToolResult, McpError> {
        // The pre-launch window set — what "a window appeared" is measured
        // against. Taken before the spawn so the launch's own window can
        // never be in it. Failing *here* refuses the launch outright, which
        // beats launching and then erroring: the agent would read the error
        // as "not launched" and spawn a double.
        let seen_before = if params.wait_for_window {
            Some(
                self.programs
                    .running()
                    .await?
                    .into_iter()
                    .map(|window| window.pid)
                    .collect(),
            )
        } else {
            None
        };

        let launched = self.programs.launch(&params.command).await?;
        let mut answer = LaunchedDto::from(launched);

        let mut frame = None;
        if let Some(seen_before) = seen_before {
            match self
                .programs
                .wait_for_window(answer.pid, &seen_before)
                .await?
            {
                WindowWait::Appeared { window, waited_ms } => {
                    answer.action = format!(
                        "{}; window \"{}\" appeared after {waited_ms} ms",
                        answer.action, window.title,
                    );
                    answer.window = Some(WindowDto::from(window));
                    answer.window_wait_ms = Some(waited_ms);

                    // The settled frame replaces the follow-up screen_shot
                    // the instructions would otherwise demand. A capture
                    // failure must not fail the tool — the launch already
                    // happened, and an error would provoke a second one.
                    match self.screen.capture_settled().await {
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

        // `structured` mirrors what `Json<LaunchedDto>` would produce — JSON
        // text block plus structuredContent — so clients that read either keep
        // working; the frame rides along as an extra image block.
        let structured = serde_json::to_value(&answer)
            .map_err(|err| McpError::internal_error(err.to_string(), None))?;
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
        annotations(read_only_hint = true)
    )]
    async fn app_status(
        &self,
        Parameters(params): Parameters<StatusDto>,
    ) -> Result<Json<ProgramStatusDto>, McpError> {
        let status = self.programs.status(params.pid, params.lines)?;
        Ok(Json(ProgramStatusDto::from(status)))
    }
}

/// The tool table, built once for the process.
///
/// `#[tool_handler]` defaults to `router = Self::tool_router()`, and that
/// expression is inlined into the generated `call_tool` *and* `list_tools` —
/// so every tool invocation would otherwise rebuild every `Tool` struct and
/// re-run the name validator. The table never changes, so it is built once
/// and borrowed.
static ROUTER: LazyLock<ToolRouter<ProgramsTool>> = LazyLock::new(ProgramsTool::tool_router);

// Parenthesised on purpose: the macro splices this straight into
// `#router.call(…)`, and unparenthesised the `&*` would bind to the call's
// result rather than to `ROUTER`.
#[tool_handler(router = (&*ROUTER))]
impl ServerHandler for ProgramsTool {
    /// Capabilities only. The endpoint's identity and its session brief are
    /// the app's to declare — this host serves one feature of several and
    /// cannot speak for the whole surface.
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(
            ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .build(),
        )
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, McpError> {
        self.journal.dispatch(&ROUTER, self, request, context).await
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, McpError> {
        Ok(ListResourcesResult {
            resources: vec![
                Resource::new(CATALOGUE_URI, "apps")
                    .with_description(
                        "Installed GUI applications as a JSON array of {name, exec}. \
                         Same data as the app_list tool.",
                    )
                    .with_mime_type(CATALOGUE_MIME),
            ],
            ..ListResourcesResult::default()
        })
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResponse, McpError> {
        if request.uri != CATALOGUE_URI {
            // Not this host's URI. The endpoint offers the read to each host
            // in turn until one does not answer not-found, so this is how a
            // sibling's resource gets its chance.
            return Err(McpError::resource_not_found(
                format!("unknown resource `{}`", request.uri),
                None,
            ));
        }

        // A plain array here, not the `ListedDto` wrapper: a resource is a
        // document, not a `structuredContent` object, and the published
        // description promises "a JSON array of {name, exec}".
        let body = serde_json::to_string(
            &self
                .programs
                .list()
                .into_iter()
                .map(ProgramDto::from)
                .collect::<Vec<_>>(),
        )
        .map_err(|err| McpError::internal_error(err.to_string(), None))?;

        // `ResourceContents::text` defaults to text/plain; without this the
        // contents would contradict the `application/json` the listing
        // advertises for this URI.
        let contents = ResourceContents::text(body, &request.uri).with_mime_type(CATALOGUE_MIME);
        Ok(ReadResourceResult::new(vec![contents]).into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tool_table_builds_and_app_launch_keeps_its_output_schema() {
        // `output_schema = …` on app_launch runs at table-build time, not at
        // compile time — this is the only place that executes it before a
        // client does. The schema must survive the switch away from
        // `Json<LaunchedDto>`, or the published contract silently loses a
        // shape it has always had.
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
