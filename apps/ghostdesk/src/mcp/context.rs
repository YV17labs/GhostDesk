use std::sync::Arc;

use nest_rs::core::injectable;
use nest_rs::http::poem::Request;
use nest_rs::mcp::{BoxFuture, Captured, McpToolContext, OperationOutcome};
use platform::coords;

use features::idle::IdleService;
use features::telemetry::TelemetryService;

const MODEL_SPACE_HEADER: &str = "ghostdesk-model-space";

const SESSION_HEADER: &str = "mcp-session-id";

struct Ambient {
    space: i64,
    session: String,
}

#[injectable]
pub struct DesktopContext {
    #[inject]
    idle_svc: Arc<IdleService>,
    #[inject]
    telemetry_svc: Arc<TelemetryService>,
}

impl McpToolContext for DesktopContext {
    fn capture(&self, req: &Request) -> Captured {
        let header = |name: &str| {
            req.headers()
                .get(name)
                .and_then(|value| value.to_str().ok())
                .map(str::trim)
        };

        let space: i64 = header(MODEL_SPACE_HEADER)
            .and_then(|value| value.parse().ok())
            .filter(|value| *value > 0)
            .unwrap_or(0);

        Arc::new(Ambient {
            space,
            session: header(SESSION_HEADER).unwrap_or_default().to_string(),
        })
    }

    fn around<'a>(
        &'a self,
        captured: &'a Captured,
        inner: BoxFuture<'a, OperationOutcome>,
    ) -> BoxFuture<'a, OperationOutcome> {
        self.idle_svc.mark_activity();

        let ambient = captured.downcast_ref::<Ambient>();
        let space = ambient.map_or(0, |ambient| ambient.space);
        let session = ambient.map_or("", |ambient| ambient.session.as_str());

        Box::pin(
            self.telemetry_svc
                .with_session(session, coords::with_model_space(space, inner)),
        )
    }
}
