//! Workspace operations tool handlers
//!
//! Handles: rename_directory, analyze_imports, find_dead_code, update_dependencies, extract_module_to_package

use super::{ToolHandler, ToolHandlerContext};
use crate::handlers::compat::{ToolContext, ToolHandler as LegacyToolHandler};
use crate::handlers::file_operation_handler::FileOperationHandler as LegacyFileHandler;
use crate::handlers::refactoring_handler::RefactoringHandler as LegacyRefactoringHandler;
use crate::handlers::system_handler::SystemHandler as LegacySystemHandler;
use cb_protocol::ApiResult as ServerResult;
use async_trait::async_trait;
use cb_core::model::mcp::ToolCall;
use serde_json::Value;

pub struct WorkspaceHandler {
    file_handler: LegacyFileHandler,
    system_handler: LegacySystemHandler,
    refactoring_handler: LegacyRefactoringHandler,
}

impl WorkspaceHandler {
    pub fn new() -> Self {
        Self {
            file_handler: LegacyFileHandler::new(),
            system_handler: LegacySystemHandler::new(),
            refactoring_handler: LegacyRefactoringHandler::new(),
        }
    }
}

#[async_trait]
impl ToolHandler for WorkspaceHandler {
    fn tool_names(&self) -> &[&str] {
        &[
            "rename_directory",
            "analyze_imports",
            "find_dead_code",
            "update_dependencies",
            "extract_module_to_package",
        ]
    }

    async fn handle_tool_call(
        &self,
        context: &ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        // Convert new context to legacy context
        let legacy_context = ToolContext {
            app_state: context.app_state.clone(),
            plugin_manager: context.plugin_manager.clone(),
            lsp_adapter: context.lsp_adapter.clone(),
        };

        // Route to appropriate legacy handler
        match tool_call.name.as_str() {
            "rename_directory" => {
                self.file_handler
                    .handle_tool(tool_call.clone(), &legacy_context)
                    .await
            }
            "analyze_imports" | "find_dead_code" | "update_dependencies" => {
                self.system_handler
                    .handle_tool(tool_call.clone(), &legacy_context)
                    .await
            }
            "extract_module_to_package" => {
                self.refactoring_handler
                    .handle_tool(tool_call.clone(), &legacy_context)
                    .await
            }
            _ => Err(cb_protocol::ApiError::InvalidRequest(format!(
                "Unknown workspace tool: {}",
                tool_call.name
            ))),
        }
    }
}
