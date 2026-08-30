use nest_rs::core::{Layer, injectable};
use nest_rs::guards::{Denial, Guard, McpGuard, async_trait};
use nest_rs::mcp::McpOperationContext;

#[injectable]
#[derive(Default)]
pub struct CallTrail;

impl Layer for CallTrail {}

#[async_trait]
impl Guard for CallTrail {
    async fn check_mcp(&self, ctx: &McpOperationContext<'_>) -> Result<(), Denial> {
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
