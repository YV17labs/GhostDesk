//! Per-operation ambient state.
//!
//! rmcp dispatches each operation on its own spawned task, so anything
//! installed around the poem endpoint is gone by the time a tool body runs.
//! [`McpToolContext`] is the seam that carries state across that spawn:
//! `capture` reads the request while it still exists, `around` installs what
//! it read inside the dispatch.
//!
//! Two things ride it. The **model space** from `GhostDesk-Model-Space`,
//! which decides whether the coordinates a tool receives are native pixels or
//! a normalised space; and the **idle clock**, reset on every operation so a
//! working session never has its windows closed underneath it.

use std::sync::Arc;

use nest_rs::core::injectable;
use nest_rs::http::poem::Request;
use nest_rs::mcp::{BoxFuture, Captured, McpToolContext, OperationOutcome};
use platform::coords;

use crate::session::IdleService;

/// The header a client sets to say "my coordinates are normalised to N".
const MODEL_SPACE_HEADER: &str = "ghostdesk-model-space";

#[injectable]
pub struct GhostdeskToolContext {
    #[inject]
    idle: Arc<IdleService>,
}

impl McpToolContext for GhostdeskToolContext {
    fn capture(&self, req: &Request) -> Captured {
        // Missing, malformed or non-positive all mean the same thing:
        // pass-through native pixels, the frontier-model default.
        let space: i64 = req
            .headers()
            .get(MODEL_SPACE_HEADER)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.trim().parse().ok())
            .filter(|value| *value > 0)
            .unwrap_or(0);

        Arc::new(space)
    }

    fn around<'a>(
        &'a self,
        captured: &'a Captured,
        inner: BoxFuture<'a, OperationOutcome>,
    ) -> BoxFuture<'a, OperationOutcome> {
        // Any MCP traffic at all counts as a live session, not just tool
        // calls — a client polling resources is still someone at the desk.
        self.idle.mark_activity();

        let space = captured.downcast_ref::<i64>().copied().unwrap_or(0);
        Box::pin(coords::with_model_space(space, inner))
    }
}
