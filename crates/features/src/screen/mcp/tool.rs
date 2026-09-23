use std::sync::Arc;

use nest_rs::mcp::{CallToolResult, ContentBlock, McpError, Parameters, Valid, mcp, tools};

use crate::blame::Answered;
use crate::screen::dtos::{CaptureDto, RegionDto, ScreenShotDto};
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
            subsequent mouse call are offsets in the image this returned — \
            the moment anything changes on screen, those coordinates are \
            stale and must be recomputed from a fresh capture.\n\n\
            region captures one rectangle instead of the whole screen, and \
            costs proportionally less. What you read off it is measured from \
            that rectangle's own corner, so every mouse call using those \
            coordinates must repeat the same region. Omit region on both, or \
            pass it on both — the two halves are one statement, and half a \
            statement points somewhere real but wrong.\n\n\
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

        let image = CaptureDto::from(&capture).block();

        // A whole-screen capture needs no origin line: it is the origin every
        // coordinate already assumes.
        let Some(region) = params.region else {
            return Ok(CallToolResult::success(vec![image]));
        };

        Ok(CallToolResult::success(vec![
            ContentBlock::text(provenance(&region)),
            image,
        ]))
    }
}

/// What a region capture says about itself, beside its pixels.
///
/// Named rather than inlined so a test can read it without booting a service:
/// it is the half of the region contract that travels with the image, and the
/// half a tool description cannot carry, since the agent read that description
/// many turns ago.
fn provenance(region: &RegionDto) -> String {
    format!(
        "This image is the region x={} y={} width={} height={} of the screen. \
         Coordinates you read here start at that corner — pass the same region \
         to the mouse call that uses them.",
        region.x, region.y, region.width, region.height,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_region_capture_says_where_its_coordinates_start() {
        let said = provenance(&RegionDto {
            x: 40,
            y: 500,
            width: 600,
            height: 200,
        });

        assert!(said.contains("x=40 y=500 width=600 height=200"), "{said}");
        assert!(
            said.contains("pass the same region"),
            "a region capture whose act cannot name it clicks an origin away: \
             {said}",
        );
    }

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
