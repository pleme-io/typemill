//! Editing and refactoring tool handlers
//!
//! Handles: rename_symbol, rename_symbol_strict, rename_symbol_with_imports,
//! organize_imports, fix_imports, get_code_actions, format_document,
//! extract_function, extract_variable, inline_variable

use super::{ToolHandler, ToolHandlerContext};
use crate::handlers::compat::ToolHandler as LegacyToolHandler;
use crate::handlers::refactoring_handler::RefactoringHandler as LegacyRefactoringHandler;
use async_trait::async_trait;
use cb_core::model::mcp::ToolCall;
use cb_plugins::PluginRequest;
use cb_protocol::ApiResult as ServerResult;
use serde_json::{json, Value};
use std::path::PathBuf;

pub struct EditingHandler {
    legacy_handler: LegacyRefactoringHandler,
}

impl EditingHandler {
    pub fn new() -> Self {
        Self {
            legacy_handler: LegacyRefactoringHandler::new(),
        }
    }

    async fn handle_format_document(
        &self,
        context: &ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        let args = tool_call.arguments.clone().unwrap_or(json!({}));

        let file_path_str = args
            .get("file_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                cb_protocol::ApiError::InvalidRequest("Missing file_path parameter".into())
            })?
            .to_string();

        let file_path = PathBuf::from(&file_path_str);
        let mut request = PluginRequest::new("format_document".to_string(), file_path.clone());

        // Set parameters (including options if provided)
        request = request.with_params(args);

        match context.plugin_manager.handle_request(request).await {
            Ok(response) => {
                // Check if formatting was applied
                let formatted = response
                    .data
                    .as_ref()
                    .and_then(|d| d.as_bool())
                    .unwrap_or(false);

                Ok(json!({
                    "formatted": formatted,
                    "file_path": file_path_str,
                    "plugin": response.metadata.plugin_name,
                    "processing_time_ms": response.metadata.processing_time_ms,
                }))
            }
            Err(err) => Err(cb_protocol::ApiError::Internal(format!(
                "Format document failed: {}",
                err
            ))),
        }
    }

    async fn handle_get_code_actions(
        &self,
        context: &ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        let args = tool_call.arguments.clone().unwrap_or(json!({}));

        let file_path_str = args
            .get("file_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                cb_protocol::ApiError::InvalidRequest("Missing file_path parameter".into())
            })?
            .to_string();

        let file_path = PathBuf::from(&file_path_str);
        let mut request = PluginRequest::new("get_code_actions".to_string(), file_path.clone());

        // Extract range if available
        if let Some(range) = args.get("range") {
            if let (Some(start), Some(end)) = (range.get("start"), range.get("end")) {
                if let (Some(start_line), Some(start_char), Some(end_line), Some(end_char)) = (
                    start.get("line").and_then(|v| v.as_u64()),
                    start.get("character").and_then(|v| v.as_u64()),
                    end.get("line").and_then(|v| v.as_u64()),
                    end.get("character").and_then(|v| v.as_u64()),
                ) {
                    request = request.with_range(
                        start_line as u32,
                        start_char as u32,
                        end_line as u32,
                        end_char as u32,
                    );
                }
            }
        }

        // Set parameters
        request = request.with_params(args);

        match context.plugin_manager.handle_request(request).await {
            Ok(response) => {
                let actions = response.data.unwrap_or(json!([]));

                Ok(json!({
                    "actions": actions,
                    "file_path": file_path_str,
                    "plugin": response.metadata.plugin_name,
                    "processing_time_ms": response.metadata.processing_time_ms,
                }))
            }
            Err(err) => Err(cb_protocol::ApiError::Internal(format!(
                "Get code actions failed: {}",
                err
            ))),
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
        match tool_call.name.as_str() {
            "format_document" => self.handle_format_document(context, tool_call).await,
            "get_code_actions" => self.handle_get_code_actions(context, tool_call).await,
            _ => crate::delegate_to_legacy!(self, context, tool_call),
        }
    }
}
