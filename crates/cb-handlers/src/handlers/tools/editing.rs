//! Editing and refactoring tool handlers
//!
//! Handles: rename_symbol, rename_symbol_strict, rename_symbol_with_imports,
//! organize_imports, fix_imports, get_code_actions, format_document,
//! extract_function, extract_variable, inline_variable

use super::{ToolHandler, ToolHandlerContext};
use crate::handlers::compat::{ToolContext, ToolHandler as LegacyToolHandler};
use crate::handlers::refactoring_handler::RefactoringHandler as LegacyRefactoringHandler;
use async_trait::async_trait;
use cb_core::model::mcp::ToolCall;
use cb_protocol::ApiResult as ServerResult;
use serde_json::Value;

pub struct EditingHandler {
    legacy_handler: LegacyRefactoringHandler,
}

impl EditingHandler {
    pub fn new() -> Self {
        Self {
            legacy_handler: LegacyRefactoringHandler::new(),
        }
    }
}

#[async_trait]
impl ToolHandler for EditingHandler {
    fn tool_names(&self) -> &[&str] {
        &[
            "rename_symbol",
            "rename_symbol_strict",
            "rename_symbol_with_imports",
            "organize_imports",
            "fix_imports",
            "get_code_actions",
            "format_document",
            "extract_function",
            "extract_variable",
            "inline_variable",
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

        // Delegate to legacy handler
        self.legacy_handler
            .handle_tool(tool_call.clone(), &legacy_context)
            .await
    }
}
