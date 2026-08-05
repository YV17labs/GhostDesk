//! The MCP edge — one host serving every GhostDesk capability.
//!
//! The MCP spec namespaces tools per endpoint, and every shipped client
//! config points at a single URL, so all fourteen tools and both resources
//! live on one `#[mcp]` struct. It plays the part a `#[controller]` plays over
//! HTTP: inject the domain services, translate wire DTOs into domain calls,
//! and translate the results back. No desktop logic lives here.
//!
//! Coordinates arriving from the agent are in whatever space the caller's
//! `GhostDesk-Model-Space` header declared. Converting them is this layer's
//! job — `GhostdeskToolContext` installed the space, and every `x`/`y` below
//! goes through `coords::to_pixels` before a service sees it.

use std::sync::{Arc, LazyLock};

use nest_rs::mcp::ToolRouter;
use nest_rs::mcp::model::{
    Implementation, ListResourcesResult, PaginatedRequestParams, ReadResourceRequestParams,
    ReadResourceResponse, ReadResourceResult, Resource, ResourceContents, ServerCapabilities,
    ServerInfo,
};
use nest_rs::mcp::rmcp;
use nest_rs::mcp::service::{RequestContext, RoleServer};
use nest_rs::mcp::{
    CallToolResult, ContentBlock, Json, McpError, Parameters, ServerHandler, mcp, tool,
    tool_handler, tool_router,
};
use platform::coords;

use super::dto::*;
use super::icons::icons;
use super::instructions::instructions;
use crate::apps::{AppStatus, AppsService, Launched, RunningApp, WINDOW_WAIT_TIMEOUT, WindowWait};
use crate::clipboard::ClipboardService;
use crate::input::{Feedback, InputService};
use crate::screen::{Capture, ScreenService};

const APPS_URI: &str = "ghostdesk://apps";
const APPS_MIME: &str = "application/json";
const CLIPBOARD_URI: &str = "ghostdesk://clipboard";
const CLIPBOARD_MIME: &str = "text/plain";

#[mcp(path = "/mcp")]
#[derive(Clone)]
pub struct GhostdeskMcp {
    #[inject]
    screen: Arc<ScreenService>,
    #[inject]
    input: Arc<InputService>,
    #[inject]
    apps: Arc<AppsService>,
    #[inject]
    clipboard: Arc<ClipboardService>,
}

impl GhostdeskMcp {
    /// Domain failures reach the model as a structured protocol error, never
    /// as prose smuggled into a successful result — the model can branch on
    /// an error, but it will happily act on a "result" that is really a
    /// stack trace.
    fn failed(err: impl std::fmt::Display) -> McpError {
        let message = err.to_string();
        tracing::error!(target: "ghostdesk::mcp", error = %message, "tool failed");
        McpError::internal_error(message, None)
    }

    /// The caller asked for something the server will not do — bad key name,
    /// arguments where none are allowed, a PID this session did not launch.
    /// A structured `invalid_params` is what lets the model correct itself
    /// and retry, where an `internal_error` reads as "stop trying".
    fn invalid(err: impl std::fmt::Display) -> McpError {
        McpError::invalid_params(err.to_string(), None)
    }

    /// A capture in the form the agent receives it.
    ///
    /// Both tools that hand the model a picture — `screen_shot` and
    /// `app_launch`'s settled frame — come through here, so how a capture is
    /// encoded and what mime it is announced under is decided in one place.
    fn image_block(capture: &Capture) -> ContentBlock {
        use base64::Engine as _;
        let encoded = base64::engine::general_purpose::STANDARD.encode(&capture.bytes);
        ContentBlock::image(encoded, capture.format.mime())
    }
}

#[tool_router]
impl GhostdeskMcp {
    #[tool(
        description = "Capture the current screen as an image.\n\n\
            This is your only source of truth. Coordinates used in any \
            subsequent mouse call are pixel offsets in the image this \
            returned — the moment anything changes on screen, those \
            coordinates are stale and must be recomputed from a fresh \
            capture.\n\n\
            Cheap. Use liberally: before a click to locate the target, after \
            an action to verify the effect, and once more before reporting a \
            mission complete.",
        annotations(read_only_hint = true, idempotent_hint = true),
        icons = icons()
    )]
    async fn screen_shot(
        &self,
        Parameters(params): Parameters<ScreenShotParams>,
    ) -> Result<CallToolResult, McpError> {
        use nest_rs::core::validator::Validate;
        params.validate().map_err(Self::invalid)?;

        let region = params
            .region
            .map(|region| coords::region_to_pixels(region.into()));

        let capture = self
            .screen
            .capture(
                region,
                params.format.into(),
                params.stabilize,
                params.quality,
            )
            .await
            .map_err(Self::failed)?;

        Ok(CallToolResult::success(vec![Self::image_block(&capture)]))
    }

    #[tool(
        description = "Move the cursor to (x, y) without pressing any button.\n\n\
            Use this to trigger hover-only UI reactions: dropdown menus that \
            appear on mouse-over, tooltips, CSS :hover states, or any element \
            that reveals itself when the pointer enters its bounds. Take a \
            screen_shot() first to get the coordinates.\n\n\
            A screen_changed of false is expected for elements that have no \
            hover effect — it is not an error. If the menu or tooltip you \
            wanted does not appear, the element probably needs a click \
            instead: fall back to mouse_click.",
        annotations(destructive_hint = false),
        icons = icons()
    )]
    async fn mouse_move(
        &self,
        Parameters(params): Parameters<MoveParams>,
    ) -> Result<Json<Feedback>, McpError> {
        let (x, y) = coords::to_pixels(params.x, params.y);
        self.input
            .mouse_move(x, y)
            .await
            .map(Json)
            .map_err(Self::failed)
    }

    #[tool(
        description = "Click once at screen coordinates (pixels from the last \
            screen_shot()).\n\n\
            A screen_changed of false means the click had no visible effect \
            anywhere on screen. Do not retry the same coordinates — the target \
            probably moved (page scrolled, dialog opened) or was never where \
            you thought. Take a new screen_shot() and recompute.",
        annotations(destructive_hint = true),
        icons = icons()
    )]
    async fn mouse_click(
        &self,
        Parameters(params): Parameters<ClickParams>,
    ) -> Result<Json<Feedback>, McpError> {
        let (x, y) = coords::to_pixels(params.x, params.y);
        self.input
            .mouse_click(x, y, params.button.into())
            .await
            .map(Json)
            .map_err(Self::failed)
    }

    #[tool(
        description = "Double-click at screen coordinates. Standard use cases: \
            open a file or folder in a file manager, select an entire word in \
            editable text.",
        annotations(destructive_hint = true),
        icons = icons()
    )]
    async fn mouse_double_click(
        &self,
        Parameters(params): Parameters<ClickParams>,
    ) -> Result<Json<Feedback>, McpError> {
        let (x, y) = coords::to_pixels(params.x, params.y);
        self.input
            .mouse_double_click(x, y, params.button.into())
            .await
            .map(Json)
            .map_err(Self::failed)
    }

    #[tool(
        description = "Drag from one point to another while holding a button. \
            Standard use cases: select a range of text, move a window or item, \
            resize from a corner, draw in a canvas.\n\n\
            For selecting text, mouse_click(start) plus key_press(\"shift+end\") \
            (or any shift+navigation) is often more reliable than a \
            pixel-precise drag.",
        annotations(destructive_hint = true),
        icons = icons()
    )]
    async fn mouse_drag(
        &self,
        Parameters(params): Parameters<DragParams>,
    ) -> Result<Json<Feedback>, McpError> {
        let from = coords::to_pixels(params.from_x, params.from_y);
        let to = coords::to_pixels(params.to_x, params.to_y);
        self.input
            .mouse_drag(from, to, params.button.into())
            .await
            .map(Json)
            .map_err(Self::failed)
    }

    #[tool(
        description = "Scroll the region under (x, y). direction is \
            up/down/left/right, amount is the number of wheel notches (clamped \
            to 1-5 per call — chain multiple calls for long pages, with a \
            screenshot between each).\n\n\
            A screen_changed of false typically means the page is already at \
            the scroll boundary — there is nothing more to reveal in that \
            direction.",
        annotations(destructive_hint = false),
        icons = icons()
    )]
    async fn mouse_scroll(
        &self,
        Parameters(params): Parameters<ScrollParams>,
    ) -> Result<Json<Feedback>, McpError> {
        let (x, y) = coords::to_pixels(params.x, params.y);
        self.input
            .mouse_scroll(x, y, params.direction.into(), params.amount)
            .await
            .map(Json)
            .map_err(Self::failed)
    }

    #[tool(
        description = "Type text at the current keyboard focus. Handles \
            Unicode, newlines and tabs. Layout-independent — a French AZERTY \
            host produces the same output as US QWERTY.\n\n\
            For more than a sentence or two, prefer clipboard_set(text) plus \
            the paste shortcut: it is instant, immune to autocomplete and \
            autocorrect, and does not race with the app's own key handlers. \
            The server instructions name the modifier this desktop uses — it \
            is not the same on every OS.\n\n\
            A screen_changed of false almost always means the field did not \
            have focus. Click into it first and retry.",
        annotations(destructive_hint = true),
        icons = icons()
    )]
    async fn key_type(
        &self,
        Parameters(params): Parameters<TypeParams>,
    ) -> Result<Json<Feedback>, McpError> {
        self.input
            .key_type(&params.text)
            .await
            .map(Json)
            .map_err(Self::failed)
    }

    #[tool(
        description = "Press a key or a chord (modifiers plus key), using + as \
            separator.\n\n\
            Modifier tokens: ctrl/control, alt/option, shift, \
            super/meta/win/cmd. Which of them carries the standard shortcuts \
            differs by OS — the server instructions say which, and sending \
            the wrong one types a stray character instead of failing.\n\n\
            Non-printable tokens: return/enter, escape/esc, backspace, delete, \
            tab, space, home/end, pageup/pagedown, left/right/up/down, \
            f1..f12.\n\n\
            A screen_changed of false usually means the keystroke went to a \
            window or field that did not care about it — check focus with a \
            screenshot.",
        annotations(destructive_hint = true),
        icons = icons()
    )]
    async fn key_press(
        &self,
        Parameters(params): Parameters<PressParams>,
    ) -> Result<Json<Feedback>, McpError> {
        // An unknown key name is the caller's mistake, not a server failure.
        self.input
            .key_press(&params.keys)
            .await
            .map(Json)
            .map_err(Self::invalid)
    }

    #[tool(
        description = "Return the catalogue of installed GUI applications, \
            as this desktop registers them.\n\n\
            This catalogue is the strict whitelist — app_launch refuses any \
            executable not in it. Call this the first time a task names an \
            application, and again after installing software during the \
            session. Each entry has a name and an exec; exec is the string to \
            pass to app_launch().",
        annotations(read_only_hint = true, idempotent_hint = true),
        icons = icons()
    )]
    async fn app_list(&self) -> Result<Json<Listed<AppEntry>>, McpError> {
        Ok(Json(Listed::new(
            self.apps.list().into_iter().map(AppEntry::from),
        )))
    }

    #[tool(
        description = "List the application windows currently open on the \
            desktop — one entry per real client window. Workspaces, outputs \
            and the bar are not included.\n\n\
            Call this before app_launch(): if the app is already in the list, \
            switch to its window instead of launching a second instance and \
            doubling memory use.",
        annotations(read_only_hint = true),
        icons = icons()
    )]
    async fn app_running(&self) -> Result<Json<Listed<RunningApp>>, McpError> {
        self.apps
            .running()
            .await
            .map(|apps| Json(Listed::new(apps)))
            .map_err(Self::failed)
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
        icons = icons(),
        output_schema = rmcp::handler::server::tool::schema_for_output::<Launched>()
    )]
    async fn app_launch(
        &self,
        Parameters(params): Parameters<LaunchParams>,
    ) -> Result<CallToolResult, McpError> {
        // The pre-launch window set — what "a window appeared" is measured
        // against. Taken before the spawn so the launch's own window can
        // never be in it. Failing *here* is a server failure and refuses the
        // launch outright, which beats launching and then erroring: the
        // agent would read the error as "not launched" and spawn a double.
        let seen_before = if params.wait_for_window {
            Some(
                self.apps
                    .running()
                    .await
                    .map_err(Self::failed)?
                    .into_iter()
                    .map(|window| window.pid)
                    .collect(),
            )
        } else {
            None
        };

        // Refusals here are all "you asked for the wrong thing" — arguments
        // supplied, unknown executable, bad quoting.
        let mut launched = self
            .apps
            .launch(&params.command)
            .await
            .map_err(Self::invalid)?;

        let mut frame = None;
        if let Some(seen_before) = seen_before {
            match self
                .apps
                .wait_for_window(launched.pid, &seen_before)
                .await
                .map_err(Self::failed)?
            {
                WindowWait::Appeared { window, waited_ms } => {
                    launched.action = format!(
                        "{}; window \"{}\" appeared after {waited_ms} ms",
                        launched.action, window.title,
                    );
                    launched.window = Some(window);
                    launched.window_wait_ms = Some(waited_ms);

                    // The settled frame replaces the follow-up screen_shot
                    // the instructions would otherwise demand. A capture
                    // failure must not fail the tool — the launch already
                    // happened, and an error would provoke a second one.
                    match self
                        .screen
                        .capture(
                            None,
                            platform::screen::ImageFormat::Webp,
                            true,
                            platform::screen::DEFAULT_WEBP_QUALITY,
                        )
                        .await
                    {
                        Ok(capture) => frame = Some(capture),
                        Err(err) => tracing::warn!(
                            target: "ghostdesk::mcp",
                            error = %err,
                            "window appeared but the settled frame could not be captured",
                        ),
                    }
                }
                WindowWait::ProcessExited => {
                    launched.action = format!(
                        "{}; the process exited and no window appeared within {} s — it \
                         crashed (check app_status({})) or handed off to an \
                         already-running instance (check app_running())",
                        launched.action,
                        WINDOW_WAIT_TIMEOUT.as_secs(),
                        launched.pid,
                    );
                }
                WindowWait::TimedOut => {
                    launched.action = format!(
                        "{}; still running but no window after {} s — slow to start \
                         (check app_running() shortly) or running without a UI",
                        launched.action,
                        WINDOW_WAIT_TIMEOUT.as_secs(),
                    );
                }
            }
        }

        // `structured` mirrors what `Json<Launched>` produced — JSON text
        // block plus structuredContent — so clients that read either keep
        // working; the frame rides along as an extra image block.
        let structured = serde_json::to_value(&launched).map_err(Self::failed)?;
        let mut result = CallToolResult::structured(structured);
        if let Some(capture) = frame {
            result.content.push(Self::image_block(&capture));
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
        annotations(read_only_hint = true),
        icons = icons()
    )]
    async fn app_status(
        &self,
        Parameters(params): Parameters<StatusParams>,
    ) -> Result<Json<AppStatus>, McpError> {
        self.apps
            .status(params.pid, params.lines)
            .map(Json)
            .map_err(Self::invalid)
    }

    #[tool(
        description = "Read the current system clipboard as text. Returns an \
            empty string if the clipboard is empty or holds non-text \
            content.\n\n\
            Standard use cases: grab text the user just copied, fetch output a \
            previous CLI action piped into the clipboard, or transfer a \
            selection between two apps without traversing the filesystem.",
        annotations(read_only_hint = true, idempotent_hint = true),
        icons = icons()
    )]
    async fn clipboard_get(&self) -> Result<CallToolResult, McpError> {
        Ok(CallToolResult::success(vec![ContentBlock::text(
            self.clipboard.get().await,
        )]))
    }

    #[tool(
        description = "Write text to the system clipboard.\n\n\
            The canonical pattern is clipboard_set(text) followed by the \
            paste shortcut in the target app — the fastest and most \
            reliable way to place more than a sentence or two into any \
            editable field. Bypasses autocomplete and autocorrect, and does \
            not race with the app's own key handlers the way key_type can.",
        annotations(destructive_hint = false, idempotent_hint = true),
        icons = icons()
    )]
    async fn clipboard_set(
        &self,
        Parameters(params): Parameters<ClipboardSetParams>,
    ) -> Result<CallToolResult, McpError> {
        let message = self
            .clipboard
            .set(&params.text)
            .await
            .map_err(Self::failed)?;
        Ok(CallToolResult::success(vec![ContentBlock::text(message)]))
    }
}

/// The tool table, built once for the process.
///
/// `#[tool_handler]` defaults to `router = Self::tool_router()`, and that
/// expression is inlined into the generated `call_tool` *and* `list_tools` —
/// so every tool invocation would otherwise rebuild all fourteen `Tool`
/// structs, re-run the name validator, and clone the icon data URI fourteen
/// times. The table never changes, so it is built once and borrowed.
static ROUTER: LazyLock<ToolRouter<GhostdeskMcp>> = LazyLock::new(GhostdeskMcp::tool_router);

// Parenthesised on purpose: the macro splices this straight into
// `#router.call(…)`, and unparenthesised the `&*` would bind to the call's
// result rather than to `ROUTER`.
#[tool_handler(router = (&*ROUTER))]
impl ServerHandler for GhostdeskMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(
            ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .build(),
        )
        .with_instructions(instructions(&self.input.conventions()))
        .with_server_info(
            // Not `from_build_env()`: its `env!` expands inside rmcp, so it
            // would announce the SDK's name and version instead of ours.
            Implementation::new("ghostdesk", env!("CARGO_PKG_VERSION"))
                .with_title("GhostDesk")
                .with_description("MCP server to control a virtual desktop")
                .with_icons(icons()),
        )
    }

    /// The read-only counterparts of two tools.
    ///
    /// Same payloads, fetchable through `resources/read` — which does not
    /// cost the agent a turn the way a tool call does.
    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, McpError> {
        Ok(ListResourcesResult {
            resources: vec![
                Resource::new(APPS_URI, "apps")
                    .with_description(
                        "Installed GUI applications as a JSON array of {name, exec}. \
                         Same data as the app_list tool.",
                    )
                    .with_mime_type(APPS_MIME)
                    .with_icons(icons()),
                Resource::new(CLIPBOARD_URI, "clipboard")
                    .with_description(
                        "Current system clipboard text. Same data as the \
                         clipboard_get tool.",
                    )
                    .with_mime_type(CLIPBOARD_MIME)
                    .with_icons(icons()),
            ],
            ..ListResourcesResult::default()
        })
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResponse, McpError> {
        let contents = match request.uri.as_str() {
            APPS_URI => ResourceContents::text(
                // A plain array here, not the `Listed` wrapper: a resource is
                // a document, not a `structuredContent` object, and the
                // published description promises "a JSON array of
                // {name, exec}".
                serde_json::to_string(
                    &self
                        .apps
                        .list()
                        .into_iter()
                        .map(AppEntry::from)
                        .collect::<Vec<_>>(),
                )
                .map_err(Self::failed)?,
                &request.uri,
            )
            // `ResourceContents::text` defaults to text/plain; without this
            // the contents would contradict the `application/json` the
            // listing advertises for this URI.
            .with_mime_type(APPS_MIME),
            CLIPBOARD_URI => ResourceContents::text(self.clipboard.get().await, &request.uri)
                .with_mime_type(CLIPBOARD_MIME),
            unknown => {
                return Err(McpError::resource_not_found(
                    format!("unknown resource `{unknown}`"),
                    None,
                ));
            }
        };

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
        // `Json<Launched>`, or the published contract silently loses a shape
        // it has always had.
        let router = GhostdeskMcp::tool_router();
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
            "the schema still describes Launched",
        );
    }
}
