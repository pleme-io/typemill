//! Navigation and intelligence tool handlers
//!
//! Handles: find_definition, find_references, find_implementations, find_type_definition,
//! get_document_symbols, search_workspace_symbols, get_hover, get_completions,
//! get_signature_help, get_diagnostics, prepare_call_hierarchy,
//! get_call_hierarchy_incoming_calls, get_call_hierarchy_outgoing_calls
//!
//! These tools are delegated to the LSP plugin system.

use super::{ToolHandler, ToolHandlerContext};
use async_trait::async_trait;
use cb_core::model::mcp::ToolCall;
use cb_plugins::PluginRequest;
use cb_protocol::ApiResult as ServerResult;
use serde_json::{json, Value};
use std::path::PathBuf;

pub struct NavigationHandler;

impl NavigationHandler {
    pub fn new() -> Self {
        Self
    }

    fn convert_tool_call_to_plugin_request(
        &self,
        tool_call: &ToolCall,
    ) -> Result<PluginRequest, cb_protocol::ApiError> {
        let args = tool_call.arguments.clone().unwrap_or(json!({}));

        // Handle workspace-level operations that don't require a file path
        let file_path = match tool_call.name.as_str() {
            "search_workspace_symbols" => {
                // Use a dummy file path for workspace symbols
                PathBuf::from(".")
            }
            _ => {
                // Extract file path for file-specific operations
                let file_path_str =
                    args.get("file_path")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| {
                            cb_protocol::ApiError::InvalidRequest(
                                "Missing file_path parameter".into(),
                            )
                        })?;
                PathBuf::from(file_path_str)
            }
        };

        let mut request = PluginRequest::new(tool_call.name.clone(), file_path);

        // Extract position if available
        if let (Some(line), Some(character)) = (
            args.get("line").and_then(|v| v.as_u64()),
            args.get("character").and_then(|v| v.as_u64()),
        ) {
            request = request.with_position(line as u32 - 1, character as u32);
        }

        // Extract range if available
        if let (Some(start_line), Some(start_char), Some(end_line), Some(end_char)) = (
            args.get("start_line").and_then(|v| v.as_u64()),
            args.get("start_character").and_then(|v| v.as_u64()),
            args.get("end_line").and_then(|v| v.as_u64()),
            args.get("end_character").and_then(|v| v.as_u64()),
        ) {
            request = request.with_range(
                start_line as u32 - 1,
                start_char as u32,
                end_line as u32 - 1,
                end_char as u32,
            );
        }

        // Set parameters
        request = request.with_params(args);

        Ok(request)
    }
}

#[async_trait]
impl ToolHandler for NavigationHandler {
    fn tool_names(&self) -> &[&str] {
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

    async fn handle_tool_call(
        &self,
        context: &ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        // Convert to plugin request and delegate to plugin system
        let plugin_request = self.convert_tool_call_to_plugin_request(tool_call)?;

        match context.plugin_manager.handle_request(plugin_request).await {
            Ok(response) => Ok(json!({
                "content": response.data.unwrap_or(json!(null)),
                "plugin": response.metadata.plugin_name,
                "processing_time_ms": response.metadata.processing_time_ms,
                "cached": response.metadata.cached
            })),
            Err(err) => Err(cb_protocol::ApiError::Internal(format!(
                "Plugin request failed: {}",
                err
            ))),
        }
    }
}
