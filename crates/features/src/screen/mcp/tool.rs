use std::sync::Arc;

use nest_rs::mcp::{CallToolResult, McpError, Parameters, Valid, mcp, tools};

use crate::blame::Answered;
use crate::screen::dtos::{CaptureDto, ScreenShotDto};
use crate::screen::service::ScreenService;

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
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    #[public]
    async fn screen_shot(
        &self,
        Parameters(Valid(params)): Parameters<Valid<ScreenShotDto>>,
    ) -> Result<CallToolResult, McpError> {
        let capture = self
            .svc
            .capture(
                params.region.map(Into::into),
                params.format.into(),
                params.stabilize,
                params.quality,
            )
            .await
            .answered()?;

        Ok(CallToolResult::success(vec![
            CaptureDto::from(&capture).block(),
        ]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_tool_closes_its_world() {
        for tool in ScreenTool::tool_router().list_all() {
            assert_eq!(
                tool.annotations.and_then(|hints| hints.open_world_hint),
                Some(false),
                "{} declares a closed world",
                tool.name,
            );
        }
    }
}
