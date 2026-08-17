//! Wire translation only: DTO in, service call, DTO out.

use std::sync::Arc;

use nest_rs::mcp::{CallToolResult, McpError, Parameters, Valid, mcp, tools};

use super::super::dtos::{CaptureDto, ScreenShotDto};
use super::super::error::ScreenError;
use super::super::service::ScreenService;

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

#[mcp]
#[derive(Clone)]
pub struct ScreenTool {
    #[inject]
    svc: Arc<ScreenService>,
}

#[tools]
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
    #[public]
    async fn screen_shot(
        &self,
        Parameters(params): Parameters<Valid<ScreenShotDto>>,
    ) -> Result<CallToolResult, McpError> {
        let params = params.into_inner();

        let capture = self
            .svc
            .capture(
                params.region.map(Into::into),
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
