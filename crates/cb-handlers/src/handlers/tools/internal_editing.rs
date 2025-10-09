//! Internal editing tool handlers
//!
//! Handles tools that are used by workflows/backend but should not be
//! exposed to AI agents via MCP.
//!
//! Tools: rename_symbol_with_imports

use super::{ToolHandler, ToolHandlerContext};
use crate::handlers::refactoring_handler::RefactoringHandler as LegacyRefactoringHandler;
use async_trait::async_trait;
use cb_core::model::mcp::ToolCall;
use cb_protocol::ApiResult as ServerResult;
use serde_json::Value;

pub struct InternalEditingHandler {
    legacy_handler: LegacyRefactoringHandler,
}

impl InternalEditingHandler {
    pub fn new() -> Self {
        Self {
            legacy_handler: LegacyRefactoringHandler::new(),
        }
    }
}

#[async_trait]
impl ToolHandler for InternalEditingHandler {
    fn tool_names(&self) -> &[&str] {
        &["rename_symbol_with_imports"]
    }

    fn is_internal(&self) -> bool {
        // These tools are internal - used by workflows but not for direct AI agent use.
        // AI agents should use rename_symbol + optimize_imports explicitly for clarity.
        true
    }

    async fn handle_tool_call(
        &self,
        context: &ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        // RefactoringHandler now uses the new trait, so delegate directly
        self.legacy_handler.handle_tool_call(context, tool_call).await
    }
}
