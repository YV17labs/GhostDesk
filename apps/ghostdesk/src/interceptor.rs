use std::sync::Arc;

use nest_rs::core::operation_log::Unit;
use nest_rs::core::{
    AmbientValue, Interceptor, Layer, Next, Outcome, UnitContext, async_trait, interceptor,
    with_ambient,
};
use nest_rs::http::{self, HttpView};
use nest_rs::mcp;
use platform::coords;

const MODEL_SPACE_HEADER: &str = "ghostdesk-model-space";

/// The coordinate space a connection declared on its `/mcp` request, carried
/// from that request into every operation it holds — on the task the SDK runs
/// each one on, which no task-local of the request's reaches.
struct ModelSpace(i64);

impl AmbientValue for ModelSpace {}

/// The space a call's coordinates are read in, from the request that carried
/// it to the operation that reads it.
///
/// At the unit stage because the space is a header of the `/mcp` request, and
/// only a layer around that request sees it: the operations inside are
/// JSON-RPC, and carry no head of their own. It answers that HTTP request and
/// the MCP operations it carries, two edges, so it sits at the app's root
/// rather than under `mcp/`.
#[interceptor(stage = unit)]
pub struct ModelSpaceInterceptor;

impl Layer for ModelSpaceInterceptor {
    fn serves(&self, unit: Unit) -> bool {
        unit == http::unit::REQUEST || unit == mcp::unit::OPERATION
    }
}

#[async_trait]
impl Interceptor for ModelSpaceInterceptor {
    async fn intercept(&self, cx: &mut UnitContext<'_>, next: Next<'_>) -> Outcome {
        if let Some(view) = cx.switch_to::<HttpView>() {
            let space = ModelSpace(declared_space(view.request().header(MODEL_SPACE_HEADER)));
            return with_ambient(Arc::new(space), next.run(cx)).await;
        }

        let space = cx.ambient::<ModelSpace>().map_or(0, |space| space.0);
        coords::with_model_space(space, next.run(cx)).await
    }
}

/// The space a request declared, `0` — the screen's own pixels — when it
/// declared none.
fn declared_space(header: Option<&str>) -> i64 {
    header
        .map(str::trim)
        .and_then(|value| value.parse().ok())
        .filter(|value| *value > 0)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_declared_space_is_read_and_anything_else_is_the_screens_own() {
        assert_eq!(declared_space(Some(" 1000 ")), 1000);
        assert_eq!(declared_space(None), 0);
        assert_eq!(declared_space(Some("0")), 0);
        assert_eq!(declared_space(Some("-5")), 0);
        assert_eq!(declared_space(Some("wide")), 0);
    }
}
