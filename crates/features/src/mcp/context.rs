//! Per-operation ambient state.
//!
//! rmcp dispatches each operation on its own spawned task, so anything
//! installed around the poem endpoint is gone by the time a tool body runs.
//! [`McpToolContext`] is the seam that carries state across that spawn:
//! `capture` reads the request while it still exists, `around` installs what
//! it read inside the dispatch.
//!
//! Three things ride it. The **model space** from `GhostDesk-Model-Space`,
//! which decides whether the coordinates a tool receives are native pixels or
//! a normalised space; the **idle clock**, reset on every operation so a
//! working session never has its windows closed underneath it; and the
//! **session id** from `Mcp-Session-Id`, which is what lets the journal
//! reassemble one agent's calls into a trajectory.

use std::sync::Arc;

use nest_rs::core::injectable;
use nest_rs::http::poem::Request;
use nest_rs::mcp::{BoxFuture, Captured, McpToolContext, OperationOutcome};
use platform::coords;

use crate::session::IdleService;
use crate::telemetry::TelemetryService;

/// The header a client sets to say "my coordinates are normalised to N".
const MODEL_SPACE_HEADER: &str = "ghostdesk-model-space";

/// The streamable-HTTP transport's session id, as the MCP spec spells it.
const SESSION_HEADER: &str = "mcp-session-id";

/// What one request carries into its dispatch.
struct Ambient {
    space: i64,
    /// Empty when the transport carries no session — stdio, or a client on
    /// the spec's sessionless path. The journal has its own name for that
    /// case; the header is not the place to invent one.
    session: String,
}

#[injectable]
pub struct GhostdeskToolContext {
    #[inject]
    idle: Arc<IdleService>,
    #[inject]
    telemetry: Arc<TelemetryService>,
}

impl McpToolContext for GhostdeskToolContext {
    fn capture(&self, req: &Request) -> Captured {
        let header = |name: &str| {
            req.headers()
                .get(name)
                .and_then(|value| value.to_str().ok())
                .map(str::trim)
        };

        // Missing, malformed or non-positive all mean the same thing:
        // pass-through native pixels, the frontier-model default.
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
        // Any MCP traffic at all counts as a live session, not just tool
        // calls — a client polling resources is still someone at the desk.
        self.idle.mark_activity();

        let ambient = captured.downcast_ref::<Ambient>();
        let space = ambient.map_or(0, |ambient| ambient.space);
        let session = ambient.map_or("", |ambient| ambient.session.as_str());

        // Session outside model space, so that a tool body reading either one
        // sees both: the journal's `with_session` is also what keeps an idle
        // session from being summarised while a call of its own is in flight.
        Box::pin(
            self.telemetry
                .with_session(session, coords::with_model_space(space, inner)),
        )
    }
}
