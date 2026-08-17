//! The two clipboard tools, plus the read-only counterpart of
//! `clipboard_get`.

use std::sync::{Arc, LazyLock};

use nest_rs::mcp::ToolRouter;
use nest_rs::mcp::model::{
    CallToolRequestParams, CallToolResponse, ListResourcesResult, PaginatedRequestParams,
    ReadResourceRequestParams, ReadResourceResponse, ReadResourceResult, Resource,
    ResourceContents, ServerCapabilities, ServerInfo,
};
// `#[tool]` and `#[tool_handler]` expand to bare `rmcp::` paths.
use nest_rs::mcp::rmcp;
use nest_rs::mcp::service::{RequestContext, RoleServer};
use nest_rs::mcp::{
    CallToolResult, ContentBlock, McpError, Parameters, ServerHandler, mcp, tool, tool_handler,
    tool_router,
};

use super::super::dtos::ClipboardSetDto;
use super::super::error::ClipboardError;
use super::super::service::ClipboardService;
use crate::telemetry::CallJournal;

const CLIPBOARD_URI: &str = "ghostdesk://clipboard";
const CLIPBOARD_MIME: &str = "text/plain";

impl From<ClipboardError> for McpError {
    fn from(err: ClipboardError) -> Self {
        let message = err.to_string();
        match err {
            ClipboardError::Write(_) => {
                tracing::error!(target: "features::clipboard", error = %message, "tool failed");
                Self::internal_error(message, None)
            }
        }
    }
}

#[mcp]
#[derive(Clone)]
pub struct ClipboardTool {
    #[inject]
    svc: Arc<ClipboardService>,
    #[inject]
    journal: Arc<CallJournal>,
}

#[tool_router]
impl ClipboardTool {
    #[tool(
        description = "Read the current system clipboard as text. Returns an \
            empty string if the clipboard is empty or holds non-text \
            content.\n\n\
            Standard use cases: grab text the user just copied, fetch output a \
            previous CLI action piped into the clipboard, or transfer a \
            selection between two apps without traversing the filesystem.",
        annotations(read_only_hint = true, idempotent_hint = true)
    )]
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
        annotations(destructive_hint = false, idempotent_hint = true)
    )]
    async fn clipboard_set(
        &self,
        Parameters(params): Parameters<ClipboardSetDto>,
    ) -> Result<CallToolResult, McpError> {
        let message = self.svc.set(&params.text).await?;
        Ok(CallToolResult::success(vec![ContentBlock::text(message)]))
    }
}

/// The tool table, built once for the process.
static ROUTER: LazyLock<ToolRouter<ClipboardTool>> = LazyLock::new(ClipboardTool::tool_router);

#[tool_handler(router = (&*ROUTER))]
impl ServerHandler for ClipboardTool {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(
            ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .build(),
        )
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, McpError> {
        self.journal.dispatch(&ROUTER, self, request, context).await
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, McpError> {
        Ok(ListResourcesResult {
            resources: vec![
                Resource::new(CLIPBOARD_URI, "clipboard")
                    .with_description(
                        "Current system clipboard text. Same data as the \
                         clipboard_get tool.",
                    )
                    .with_mime_type(CLIPBOARD_MIME),
            ],
            ..ListResourcesResult::default()
        })
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResponse, McpError> {
        if request.uri != CLIPBOARD_URI {
            // Not this host's URI — the endpoint offers the read to each host
            // in turn until one does not answer not-found.
            return Err(McpError::resource_not_found(
                format!("unknown resource `{}`", request.uri),
                None,
            ));
        }

        let contents = ResourceContents::text(self.svc.get().await, &request.uri)
            .with_mime_type(CLIPBOARD_MIME);
        Ok(ReadResourceResult::new(vec![contents]).into())
    }
}
