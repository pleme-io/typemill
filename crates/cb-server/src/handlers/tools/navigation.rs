//! Navigation and intelligence tool handlers
//!
//! Handles: find_definition, find_references, find_implementations, find_type_definition,
//! get_document_symbols, search_workspace_symbols, get_hover, get_completions,
//! get_signature_help, get_diagnostics, prepare_call_hierarchy,
//! get_call_hierarchy_incoming_calls, get_call_hierarchy_outgoing_calls
//!
//! NOTE: These tools are handled by the plugin system directly via LSP adapters.
//! This handler exists for completeness and potential future direct handling.

use super::{ToolHandler, ToolHandlerContext};
use crate::ServerResult;
use async_trait::async_trait;
use serde_json::Value;

pub struct NavigationHandler;

impl NavigationHandler {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ToolHandler for NavigationHandler {
    fn supported_tools(&self) -> &[&'static str] {
        &[
            "find_definition",
            "find_references",
            "find_implementations",
            "find_type_definition",
            "get_document_symbols",
            "search_workspace_symbols",
            "get_hover",
            "get_completions",
            "get_signature_help",
            "get_diagnostics",
            "prepare_call_hierarchy",
            "get_call_hierarchy_incoming_calls",
            "get_call_hierarchy_outgoing_calls",
        ]
    }

    async fn handle(
        &self,
        tool_name: &str,
        _params: Value,
        _context: &ToolHandlerContext,
    ) -> ServerResult<Value> {
        // These tools are handled by the plugin system
        Err(crate::ServerError::Unsupported(format!(
            "Tool {} is handled by plugin system",
            tool_name
        )))
    }
}
