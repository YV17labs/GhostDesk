use std::sync::Arc;

use nest_rs::core::operation_log::Unit;
use nest_rs::core::{
    AmbientValue, Interceptor, Layer, Next, Outcome, UnitContext, async_trait, interceptor,
    with_ambient,
};
use nest_rs::http::{self, HttpView};
use nest_rs::mcp::{self, McpOperationKind, McpView};
use platform::coords;

use features::idle::IdleService;

const MODEL_SPACE_HEADER: &str = "ghostdesk-model-space";

/// The coordinate space a connection declared on its `/mcp` request, carried
/// from that request into every operation it holds — on the task the SDK runs
/// each one on, which no task-local of the request's reaches.
struct ModelSpace(i64);

impl AmbientValue for ModelSpace {}

/// What a call on the desk runs inside: the space its coordinates are read in,
/// and the activity the idle watchdog counts.
///
/// At the unit stage because the space is a header of the `/mcp` request, and
/// only a layer around that request sees it: the operations inside are
/// JSON-RPC, and carry no head of their own. It answers that HTTP request and
/// the MCP operations it carries, two edges, so it sits at the app's root
/// rather than under `mcp/`.
#[interceptor(stage = unit)]
pub struct DesktopInterceptor {
    #[inject]
    svc: Arc<IdleService>,
}

impl Layer for DesktopInterceptor {
    fn serves(&self, unit: Unit) -> bool {
        unit == http::unit::REQUEST || unit == mcp::unit::OPERATION
    }
}

#[async_trait]
impl Interceptor for DesktopInterceptor {
    async fn intercept(&self, cx: &mut UnitContext<'_>, next: Next<'_>) -> Outcome {
        if let Ok(view) = HttpView::of(cx) {
            let space = ModelSpace(declared_space(view.request().header(MODEL_SPACE_HEADER)));
            return with_ambient(Arc::new(space), next.run(cx)).await;
        }

        // A tool call is what uses the desk; a listing or a handshake is a
        // client looking at it, which keeps no desktop alive.
        if cx
            .switch_to::<McpView>()
            .is_some_and(|view| view.kind() == Some(McpOperationKind::Tool))
        {
            self.svc.mark_activity();
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
