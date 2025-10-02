//! Advanced operations tool handlers
//!
//! Handles: apply_edits, achieve_intent, batch_execute

use super::{ToolHandler, ToolHandlerContext};
use crate::handlers::tool_handler::{ToolContext, ToolHandler as LegacyToolHandler};
use crate::handlers::workflow_handler::WorkflowHandler as LegacyWorkflowHandler;
use crate::ServerResult;
use async_trait::async_trait;
use cb_core::model::mcp::ToolCall;
use serde_json::Value;

pub struct AdvancedHandler {
    workflow_handler: LegacyWorkflowHandler,
}

impl AdvancedHandler {
    pub fn new() -> Self {
        Self {
            workflow_handler: LegacyWorkflowHandler::new(),
        }
    }
}

#[async_trait]
impl ToolHandler for AdvancedHandler {
    fn supported_tools(&self) -> &[&'static str] {
        &["apply_edits", "achieve_intent", "batch_execute"]
    }

    async fn handle(
        &self,
        tool_name: &str,
        params: Value,
        context: &ToolHandlerContext,
    ) -> ServerResult<Value> {
        // Convert to ToolCall for legacy handler
        let tool_call = ToolCall {
            name: tool_name.to_string(),
            arguments: Some(params),
        };

        // Convert new context to legacy context
        let legacy_context = ToolContext {
            app_state: context.app_state.clone(),
            plugin_manager: context.plugin_manager.clone(),
            lsp_adapter: context.lsp_adapter.clone(),
        };

        match tool_name {
            "apply_edits" | "achieve_intent" => {
                self.workflow_handler
                    .handle_tool(tool_call, &legacy_context)
                    .await
            }
            "batch_execute" => {
                // batch_execute is not yet implemented
                Err(crate::ServerError::Unsupported(
                    "batch_execute not yet implemented".to_string(),
                ))
            }
            _ => Err(crate::ServerError::InvalidRequest(format!(
                "Unknown advanced tool: {}",
                tool_name
            ))),
        }
    }
}
