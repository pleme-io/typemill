//! Internal editing tool handlers
//!
//! Handles tools that are used by workflows/backend but should not be
//! exposed to AI agents via MCP.
//!
//! Tools: rename_symbol_with_imports

use super::{ToolHandler, ToolHandlerContext};
use crate::handlers::refactoring_handler::RefactoringHandler;
use async_trait::async_trait;
use mill_foundation::core::model::mcp::ToolCall;
use mill_foundation::protocol::ApiResult as ServerResult;
use serde_json::Value;

pub struct InternalEditingToolsHandler {
    refactoring_handler: RefactoringHandler,
}

impl InternalEditingToolsHandler {
    pub fn new() -> Self {
        Self {
            refactoring_handler: RefactoringHandler::new(),
        }
    }
}

#[async_trait]
impl ToolHandler for InternalEditingToolsHandler {
    fn tool_names(&self) -> &[&str] {
        &["rename_symbol_with_imports"]
    }

    fn is_internal(&self) -> bool {
        // These tools are internal - used by workflows but not for direct AI agent use.
        // AI agents should use the unified refactoring API (rename with dryRun option) for clarity.
        true
    }

    async fn handle_tool_call(
        &self,
        context: &ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        // RefactoringHandler now uses the new trait, so delegate directly
        self.refactoring_handler
            .handle_tool_call(context, tool_call)
            .await
    }
}
