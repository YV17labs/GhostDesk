//! The seven input tools.
//!
//! Coordinates arriving from the agent are in whatever space the caller's
//! model-space header declared. Converting them is this layer's job — the
//! app's per-call context installed the space, and every `x`/`y` below goes
//! through `coords::to_pixels` before a service sees it.

use std::sync::{Arc, LazyLock};

use nest_rs::mcp::ToolRouter;
use nest_rs::mcp::model::{
    CallToolRequestParams, CallToolResponse, ServerCapabilities, ServerInfo,
};
// `#[tool]` and `#[tool_handler]` expand to bare `rmcp::` paths.
use nest_rs::mcp::rmcp;
use nest_rs::mcp::service::{RequestContext, RoleServer};
use nest_rs::mcp::{
    Json, McpError, Parameters, ServerHandler, mcp, tool, tool_handler, tool_router,
};
use platform::coords;

use super::super::dtos::{ClickDto, DragDto, FeedbackDto, MoveDto, PressDto, ScrollDto, TypeDto};
use super::super::error::InputError;
use super::super::services::{Feedback, InputService};
use crate::telemetry::CallJournal;

/// An unresolvable chord is the caller's to fix; a backend that will not
/// press anything is not. Reporting the second as `invalid_params` would tell
/// the model to rewrite a chord that was already correct.
impl From<InputError> for McpError {
    fn from(err: InputError) -> Self {
        let message = err.to_string();
        match err {
            InputError::Chord(_) => Self::invalid_params(message, None),
            InputError::Backend(_) | InputError::Feedback(_) => {
                tracing::error!(target: "features::input", error = %message, "tool failed");
                Self::internal_error(message, None)
            }
        }
    }
}

#[mcp]
#[derive(Clone)]
pub struct InputTool {
    #[inject]
    input: Arc<InputService>,
    #[inject]
    journal: Arc<CallJournal>,
}

impl InputTool {
    /// Record what the feedback loop saw, then hand the verdict to the
    /// client.
    ///
    /// Here rather than inside `FeedbackService`: a domain service has no
    /// business naming the telemetry module, and this is the layer that
    /// already owns the call the observation belongs to. It goes through the
    /// journal the host is already holding, so measurement has one door.
    fn observed(&self, feedback: Feedback) -> Json<FeedbackDto> {
        self.journal
            .note_action(&feedback.action, feedback.screen_changed);
        Json(FeedbackDto::from(feedback))
    }
}

#[tool_router]
impl InputTool {
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
        annotations(destructive_hint = false)
    )]
    async fn mouse_move(
        &self,
        Parameters(params): Parameters<MoveDto>,
    ) -> Result<Json<FeedbackDto>, McpError> {
        let (x, y) = coords::to_pixels(params.x, params.y);
        Ok(self.observed(self.input.mouse_move(x, y).await?))
    }

    #[tool(
        description = "Click once at screen coordinates (pixels from the last \
            screen_shot()).\n\n\
            A screen_changed of false means the click had no visible effect \
            anywhere on screen. Do not retry the same coordinates — the target \
            probably moved (page scrolled, dialog opened) or was never where \
            you thought. Take a new screen_shot() and recompute.",
        annotations(destructive_hint = true)
    )]
    async fn mouse_click(
        &self,
        Parameters(params): Parameters<ClickDto>,
    ) -> Result<Json<FeedbackDto>, McpError> {
        let (x, y) = coords::to_pixels(params.x, params.y);
        Ok(self.observed(self.input.mouse_click(x, y, params.button.into()).await?))
    }

    #[tool(
        description = "Double-click at screen coordinates. Standard use cases: \
            open a file or folder in a file manager, select an entire word in \
            editable text.",
        annotations(destructive_hint = true)
    )]
    async fn mouse_double_click(
        &self,
        Parameters(params): Parameters<ClickDto>,
    ) -> Result<Json<FeedbackDto>, McpError> {
        let (x, y) = coords::to_pixels(params.x, params.y);
        Ok(self.observed(
            self.input
                .mouse_double_click(x, y, params.button.into())
                .await?,
        ))
    }

    #[tool(
        description = "Drag from one point to another while holding a button. \
            Standard use cases: select a range of text, move a window or item, \
            resize from a corner, draw in a canvas.\n\n\
            For selecting text, mouse_click(start) plus key_press(\"shift+end\") \
            (or any shift+navigation) is often more reliable than a \
            pixel-precise drag.",
        annotations(destructive_hint = true)
    )]
    async fn mouse_drag(
        &self,
        Parameters(params): Parameters<DragDto>,
    ) -> Result<Json<FeedbackDto>, McpError> {
        let from = coords::to_pixels(params.from_x, params.from_y);
        let to = coords::to_pixels(params.to_x, params.to_y);
        Ok(self.observed(
            self.input
                .mouse_drag(from, to, params.button.into())
                .await?,
        ))
    }

    #[tool(
        description = "Scroll the region under (x, y). direction is \
            up/down/left/right, amount is the number of wheel notches (clamped \
            to 1-5 per call — chain multiple calls for long pages, with a \
            screenshot between each).\n\n\
            A screen_changed of false typically means the page is already at \
            the scroll boundary — there is nothing more to reveal in that \
            direction.",
        annotations(destructive_hint = false)
    )]
    async fn mouse_scroll(
        &self,
        Parameters(params): Parameters<ScrollDto>,
    ) -> Result<Json<FeedbackDto>, McpError> {
        let (x, y) = coords::to_pixels(params.x, params.y);
        Ok(self.observed(
            self.input
                .mouse_scroll(x, y, params.direction.into(), params.amount)
                .await?,
        ))
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
        annotations(destructive_hint = true)
    )]
    async fn key_type(
        &self,
        Parameters(params): Parameters<TypeDto>,
    ) -> Result<Json<FeedbackDto>, McpError> {
        Ok(self.observed(self.input.key_type(&params.text).await?))
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
        annotations(destructive_hint = true)
    )]
    async fn key_press(
        &self,
        Parameters(params): Parameters<PressDto>,
    ) -> Result<Json<FeedbackDto>, McpError> {
        Ok(self.observed(self.input.key_press(&params.keys).await?))
    }
}

/// The tool table, built once for the process. See the note on the programs
/// host: `#[tool_handler]` would otherwise rebuild all seven `Tool` structs on
/// every single call.
static ROUTER: LazyLock<ToolRouter<InputTool>> = LazyLock::new(InputTool::tool_router);

#[tool_handler(router = (&*ROUTER))]
impl ServerHandler for InputTool {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, McpError> {
        self.journal.dispatch(&ROUTER, self, request, context).await
    }
}
