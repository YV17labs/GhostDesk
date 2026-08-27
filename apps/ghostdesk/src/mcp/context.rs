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
    idle_svc: Arc<IdleService>,
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

    /// The caller is deliberately **not** captured here.
    ///
    /// This used to read the peer address and `x-forwarded-for` itself and hang
    /// them on a span, because nothing else answered "who did this". Two things
    /// now do, and both answer better: the transport resolves the caller once —
    /// the peer, or a forwarding header only from a proxy the deployment named
    /// — and files it on the request's own line, and the operation every event
    /// below inherits already carries the ids that tie the two together.
    ///
    /// Re-reading it here would be a second resolution to keep in agreement
    /// with the first, which is the disagreement that makes a trail
    /// unauditable. Nor is a span of our own the place to put it: a span names
    /// a unit of work an operator asks about, and neither pushing the watchdog
    /// back nor installing a coordinate space is one.
    ///
    /// What stays is what no layer above can know: the desktop is being
    /// touched, so the idle watchdog is pushed back, and the agent's coordinate
    /// space is installed for the tools that convert against it.
    fn around<'a>(
        &'a self,
        captured: &'a Captured,
        inner: BoxFuture<'a, OperationOutcome>,
    ) -> BoxFuture<'a, OperationOutcome> {
        self.idle_svc.mark_activity();

        let space = captured
            .downcast_ref::<Ambient>()
            .map_or(0, |ambient| ambient.space);

        Box::pin(coords::with_model_space(space, inner))
    }
}
