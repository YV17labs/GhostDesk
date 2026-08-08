//! The one screen tool.
//!
//! Coordinates arriving from the agent are in whatever space the caller's
//! model-space header declared. Converting them is this layer's job — the
//! app's per-call context installed the space, and the region below goes
//! through `coords` before the service sees it.

use std::sync::{Arc, LazyLock};

use nest_rs::mcp::ToolRouter;
use nest_rs::mcp::model::{
    CallToolRequestParams, CallToolResponse, ServerCapabilities, ServerInfo,
};
// `#[tool]` and `#[tool_handler]` expand to bare `rmcp::` paths.
use nest_rs::mcp::rmcp;
use nest_rs::mcp::service::{RequestContext, RoleServer};
use nest_rs::mcp::{
    CallToolResult, McpError, Parameters, ServerHandler, mcp, tool, tool_handler, tool_router,
};
use platform::coords;

use super::super::dtos::{CaptureDto, ScreenShotDto};
use super::super::error::ScreenError;
use super::super::service::ScreenService;
use crate::telemetry::CallJournal;

/// Every screen failure is the server's, so every one of them is an
/// `internal_error`. The match is exhaustive so a caller-facing variant added
/// later cannot be reported as a crash by default.
impl From<ScreenError> for McpError {
    fn from(err: ScreenError) -> Self {
        let message = err.to_string();
        match err {
            ScreenError::Capture(_) | ScreenError::Decode(_) => {
                tracing::error!(target: "features::screen", error = %message, "tool failed");
                Self::internal_error(message, None)
            }
        }
    }
}

#[mcp(path = "/mcp")]
#[derive(Clone)]
pub struct ScreenTool {
    #[inject]
    screen: Arc<ScreenService>,
    #[inject]
    journal: Arc<CallJournal>,
}

#[tool_router]
impl ScreenTool {
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
        annotations(read_only_hint = true, idempotent_hint = true)
    )]
    async fn screen_shot(
        &self,
        Parameters(params): Parameters<ScreenShotDto>,
    ) -> Result<CallToolResult, McpError> {
        use nest_rs::core::validator::Validate;
        params
            .validate()
            .map_err(|err| McpError::invalid_params(err.to_string(), None))?;

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
            .await?;

        Ok(CallToolResult::success(vec![
            CaptureDto::from(&capture).block(),
        ]))
    }
}

/// The tool table, built once for the process. See the note on the programs
/// host: `#[tool_handler]` would otherwise rebuild it on every call.
static ROUTER: LazyLock<ToolRouter<ScreenTool>> = LazyLock::new(ScreenTool::tool_router);

#[tool_handler(router = (&*ROUTER))]
impl ServerHandler for ScreenTool {
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
