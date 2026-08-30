use std::sync::Arc;

use nest_rs::mcp::{CallToolResult, ContentBlock, McpError, Parameters, mcp, tools};

use crate::blame::Answered;
use crate::clipboard::dto::ClipboardSetDto;
use crate::clipboard::service::ClipboardService;

#[mcp]
#[derive(Clone)]
pub struct ClipboardTool {
    #[inject]
    svc: Arc<ClipboardService>,
}

#[tools]
impl ClipboardTool {
    #[tool(
        description = "Read the current system clipboard as text. Returns an \
            empty string if the clipboard is empty or holds non-text \
            content.\n\n\
            Standard use cases: grab text the user just copied, fetch output a \
            previous CLI action piped into the clipboard, or transfer a \
            selection between two apps without traversing the filesystem.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    #[public]
    async fn clipboard_get(&self) -> Result<CallToolResult, McpError> {
        Ok(CallToolResult::success(vec![ContentBlock::text(
            self.svc.get().await,
        )]))
    }

    #[tool(
        description = "Write text to the system clipboard.\n\n\
            The canonical pattern is clipboard_set(text) followed by the \
            paste shortcut in the target app — the fastest and most \
            reliable way to place more than a sentence or two into any \
            editable field. Bypasses autocomplete and autocorrect, and does \
            not race with the app's own key handlers the way key_type can.",
        annotations(
            destructive_hint = true,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    #[public]
    async fn clipboard_set(
        &self,
        Parameters(params): Parameters<ClipboardSetDto>,
    ) -> Result<CallToolResult, McpError> {
        let message = self.svc.set(&params.text).await.answered()?;
        Ok(CallToolResult::success(vec![ContentBlock::text(message)]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_tool_closes_its_world() {
        for tool in ClipboardTool::tool_router().list_all() {
            assert_eq!(
                tool.annotations.and_then(|hints| hints.open_world_hint),
                Some(false),
                "{} declares a closed world",
                tool.name,
            );
        }
    }
}
