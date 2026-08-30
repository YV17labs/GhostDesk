use std::sync::Arc;

use nest_rs::core::injectable;
use nest_rs::http::poem::Request;
use nest_rs::mcp::{BoxFuture, Captured, McpToolContext, OperationOutcome};
use platform::coords;

use features::idle::IdleService;

const MODEL_SPACE_HEADER: &str = "ghostdesk-model-space";

struct Ambient {
    space: i64,
}

#[injectable]
pub struct DesktopContext {
    #[inject]
    svc: Arc<IdleService>,
}

impl McpToolContext for DesktopContext {
    fn capture(&self, req: &Request) -> Captured {
        let space: i64 = req
            .headers()
            .get(MODEL_SPACE_HEADER)
            .and_then(|value| value.to_str().ok())
            .map(str::trim)
            .and_then(|value| value.parse().ok())
            .filter(|value| *value > 0)
            .unwrap_or(0);

        Arc::new(Ambient { space })
    }

    fn around<'a>(
        &'a self,
        captured: &'a Captured,
        inner: BoxFuture<'a, OperationOutcome>,
    ) -> BoxFuture<'a, OperationOutcome> {
        self.svc.mark_activity();

        let space = captured
            .downcast_ref::<Ambient>()
            .map_or(0, |ambient| ambient.space);

        Box::pin(coords::with_model_space(space, inner))
    }
}
