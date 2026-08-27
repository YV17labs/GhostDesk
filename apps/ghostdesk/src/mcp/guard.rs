use nest_rs::core::{Layer, injectable};
use nest_rs::guards::{Denial, Guard, McpGuard, async_trait};
use nest_rs::mcp::McpOperationContext;

/// Names the tool a caller asked for, on the way in.
///
/// Neither of the lines already filed says which one it was. The endpoint's own
/// line names the JSON-RPC method a client called — one word for all fourteen
/// tools — and the trail the services keep records what the desktop *did*, which
/// only exists once something happened. `app_list` and a call rejected by
/// validation touch nothing, so a session reconstructed from effects alone
/// silently omits everything that failed before reaching one.
///
/// This is a guard that guards nothing, and that is a deliberate trade.
/// `host` / `kind` / `name` reach an application in exactly one place the
/// framework offers — the per-operation guard chain — and no observation seam
/// carries them yet. It only ever answers `Ok`: the endpoint's bearer check is
/// what decides admission, and a guard that observed *and* refused would be two
/// policies in one file.
#[injectable]
#[derive(Default)]
pub struct CallTrail;

impl Layer for CallTrail {}

#[async_trait]
impl Guard for CallTrail {
    async fn check_mcp(&self, ctx: &McpOperationContext<'_>) -> Result<(), Denial> {
        // Three fields rather than one joined label: the question asked of this
        // trail is "what was called on this host", and a `ProgramsTool.app_list`
        // string cannot answer either half on its own.
        //
        // `name` rather than `operation`: the endpoint's own line already spells
        // `operation`, and it means the JSON-RPC method there. One field name
        // with two vocabularies in it is a facet nobody can read.
        tracing::info!(
            target: "ghostdesk::mcp",
            host = host_name(ctx.host()),
            kind = ctx.kind().as_str(),
            name = ctx.name(),
            "mcp operation",
        );
        Ok(())
    }
}

impl McpGuard for CallTrail {}

/// The host's bare type name. The framework hands over the full path, which
/// spells the module tree that mounted the host — true, and a repetition of
/// what `operation` already locates.
fn host_name(path: &'static str) -> &'static str {
    path.rsplit("::").next().unwrap_or(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_host_is_named_not_located() {
        assert_eq!(
            host_name("features::input::mcp::tool::InputTool"),
            "InputTool",
        );
        assert_eq!(host_name("InputTool"), "InputTool");
    }
}
