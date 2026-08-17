//! What one MCP tool call costs, measured where the call is visible *as* a
//! call — with its name, its arguments and its result together.
//!
//! Every tool host delegates its `call_tool` here rather than instrumenting
//! its own bodies: four hosts times a dozen tools is fifty copies of this,
//! each free to drift. The measurement is MCP-shaped, so it lives in
//! telemetry's MCP adapter and not in the service, which knows nothing about
//! a transport.

use std::sync::Arc;

use nest_rs::core::injectable;
use nest_rs::mcp::model::{CallToolRequestParams, CallToolResponse};
use nest_rs::mcp::rmcp::handler::server::tool::ToolCallContext;
use nest_rs::mcp::service::{RequestContext, RoleServer};
use nest_rs::mcp::{ContentBlock, McpError, ToolRouter};
use tracing::Instrument;

use crate::telemetry::service::{self, CallOutcome, TelemetryService};
use crate::telemetry::session::Outcome;

#[injectable]
pub struct CallJournal {
    #[inject]
    svc: Arc<TelemetryService>,
}

impl CallJournal {
    /// Run one tool call through `router`, journalled.
    ///
    /// This *is* every host's `call_tool`: the whole body is measurement, and
    /// a host that wrote its own would be one more copy free to drift. The
    /// span carries the call's identity to every event emitted underneath it
    /// — a failed click names the tool and the sequence number that produced
    /// it — and is deliberately the shape `nest-rs-opentelemetry` exports.
    ///
    /// The guard is held in this future's own frame, so a client that hangs
    /// up mid-call drops it unfinished and its `Drop` writes the line no
    /// `return` here would ever reach.
    pub async fn dispatch<H: Send + Sync + 'static>(
        &self,
        router: &ToolRouter<H>,
        host: &H,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, McpError> {
        // Read before `request` is moved into the dispatch.
        let call = self.svc.begin(&request.name, request.arguments.as_ref());
        let span = tracing::info_span!("mcp.tool", tool = %request.name, seq = call.seq());

        let dispatch = router.call(ToolCallContext::new(host, request, context));
        let result = call.scope(dispatch).instrument(span).await;

        call.finish(cost_of(&result));
        result
    }

    /// Report what an input action did.
    ///
    /// The verdict is logged where it is produced; this hands the same two
    /// facts to the session accumulator, which is the only thing that can see
    /// a *second* identical failure and call it a pattern. A no-op outside a
    /// tool call.
    pub fn note_action(&self, action: &str, screen_changed: bool) {
        service::note_action(action, screen_changed);
    }
}

/// What one call's result cost the client, and whether it succeeded.
///
/// Sizes are summed from the content blocks rather than by serialising the
/// whole response a second time: a screenshot is tens of kilobytes of base64,
/// and measuring it by re-encoding it would double the most expensive thing
/// the server does, on every single call.
fn cost_of(result: &Result<CallToolResponse, McpError>) -> CallOutcome {
    let complete = match result {
        Ok(CallToolResponse::Complete(complete)) => complete,
        // A tool that materialised a task or asked the client for input has
        // not produced a payload yet. GhostDesk never takes either path
        // today; recording them as zero-byte successes keeps the arithmetic
        // honest if one ever does.
        Ok(_) => return CallOutcome::empty(Outcome::Ok),
        Err(err) => return CallOutcome::failed(&err.message),
    };

    // `structured_content` is deliberately not counted: rmcp's
    // `CallToolResult::structured` — which every `Json<T>` tool goes through —
    // stores the same JSON *both* as a text block and as the structured
    // value. Measuring both would report double what the client receives, and
    // would pay for a second serialisation to do it.
    let mut image_bytes = 0;
    let mut other_bytes = 0;

    for block in &complete.content {
        match block {
            ContentBlock::Text(text) => other_bytes += text.text.len(),
            ContentBlock::Image(image) => image_bytes += image.data.len(),
            // Never produced by this server. Serialising the block is exact
            // and costs nothing while the branch is unreachable, and it stops
            // a future content kind from being silently counted as free.
            other => {
                other_bytes += serde_json::to_string(other).map_or(0, |json| json.len());
            }
        }
    }

    CallOutcome {
        // A handler may also report failure *inside* a successful envelope.
        // Reading only the transport's `Result` would score those as wins.
        outcome: match complete.is_error {
            Some(true) => Outcome::Error,
            _ => Outcome::Ok,
        },
        error: None,
        result_bytes: other_bytes + image_bytes,
        image_bytes,
    }
}

#[cfg(test)]
mod tests {
    use nest_rs::mcp::CallToolResult;
    use serde_json::json;

    use super::*;

    fn complete(result: CallToolResult) -> Result<CallToolResponse, McpError> {
        Ok(CallToolResponse::Complete(result))
    }

    #[test]
    fn a_structured_result_is_counted_once_not_twice() {
        // `CallToolResult::structured` — the path every `Json<T>` tool takes —
        // stores the same JSON as a text block *and* as structured content.
        // Counting both reported double what the client actually receives.
        let value = json!({ "action": "Clicked left at (200, 830)", "screen_changed": false });
        let on_the_wire = value.to_string().len();

        let cost = cost_of(&complete(CallToolResult::structured(value)));
        assert_eq!(cost.result_bytes, on_the_wire);
        assert_eq!(cost.image_bytes, 0);
        assert_eq!(cost.outcome, Outcome::Ok);
    }

    #[test]
    fn an_image_result_reports_its_payload_as_image_bytes() {
        let cost = cost_of(&complete(CallToolResult::success(vec![
            ContentBlock::image("0123456789", "image/webp"),
        ])));
        assert_eq!(cost.image_bytes, 10, "the base64 the model is charged for");
        assert_eq!(cost.result_bytes, 10);
    }

    #[test]
    fn a_launch_counts_its_settled_frame_on_top_of_its_structured_result() {
        // app_launch is the one tool that returns both shapes at once.
        let value = json!({ "pid": 2737 });
        let structured = value.to_string().len();
        let mut result = CallToolResult::structured(value);
        result
            .content
            .push(ContentBlock::image("01234567", "image/webp"));

        let cost = cost_of(&complete(result));
        assert_eq!(cost.image_bytes, 8);
        assert_eq!(cost.result_bytes, structured + 8);
    }

    #[test]
    fn a_refusal_is_reported_as_an_error_carrying_its_message() {
        // `invalid_params` is the one failure path that logs nowhere else, so
        // the message has to survive onto the call line.
        let cost = cost_of(&Err(McpError::invalid_params("unknown key `flurb`", None)));
        assert_eq!(cost.outcome, Outcome::Error);
        assert_eq!(cost.error.as_deref(), Some("unknown key `flurb`"));
        assert_eq!(cost.result_bytes, 0);
    }

    #[test]
    fn a_handler_reporting_failure_inside_a_successful_envelope_is_not_a_win() {
        let mut result = CallToolResult::success(vec![ContentBlock::text("nope")]);
        result.is_error = Some(true);
        assert_eq!(cost_of(&complete(result)).outcome, Outcome::Error);
    }
}
