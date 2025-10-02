//! Workspace operations tool handlers
//!
//! Handles: rename_directory, analyze_imports, find_dead_code, update_dependencies

use super::{ToolHandler, ToolHandlerContext};
use crate::handlers::file_operation_handler::FileOperationHandler as LegacyFileHandler;
use crate::handlers::system_handler::SystemHandler as LegacySystemHandler;
use crate::handlers::tool_handler::{ToolContext, ToolHandler as LegacyToolHandler};
use crate::ServerResult;
use async_trait::async_trait;
use cb_core::model::mcp::ToolCall;
use serde_json::Value;

pub struct WorkspaceHandler {
    file_handler: LegacyFileHandler,
    system_handler: LegacySystemHandler,
}

impl WorkspaceHandler {
    pub fn new() -> Self {
        Self {
            file_handler: LegacyFileHandler::new(),
            system_handler: LegacySystemHandler::new(),
        }
    }
}

#[async_trait]
impl ToolHandler for WorkspaceHandler {
    fn supported_tools(&self) -> &[&'static str] {
        &[
            "rename_directory",
            "analyze_imports",
            "find_dead_code",
            "update_dependencies",
        ]
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

        // Route to appropriate legacy handler
        match tool_name {
            "rename_directory" => {
                self.file_handler
                    .handle_tool(tool_call, &legacy_context)
                    .await
            }
            "analyze_imports" | "find_dead_code" | "update_dependencies" => {
                self.system_handler
                    .handle_tool(tool_call, &legacy_context)
                    .await
            }
            _ => Err(crate::ServerError::InvalidRequest(format!(
                "Unknown workspace tool: {}",
                tool_name
            ))),
        }
    }
}
