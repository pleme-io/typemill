//! Editing and refactoring tool handlers
//!
//! Handles: rename_symbol, rename_symbol_strict, rename_symbol_with_imports,
//! organize_imports, fix_imports, get_code_actions, format_document,
//! extract_function, extract_variable, inline_variable

use super::{ToolHandler, ToolHandlerContext};
use crate::handlers::refactoring_handler::RefactoringHandler as LegacyRefactoringHandler;
use crate::handlers::tool_handler::{ToolContext, ToolHandler as LegacyToolHandler};
use crate::ServerResult;
use async_trait::async_trait;
use cb_core::model::mcp::ToolCall;
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
    fn supported_tools(&self) -> &[&'static str] {
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

        // Delegate to legacy handler
        self.legacy_handler
            .handle_tool(tool_call, &legacy_context)
            .await
    }
}
