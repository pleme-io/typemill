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
                // LSP textDocument/formatting returns an array of TextEdit objects
                // Extract them and apply to the file
                let text_edits = response.data.as_ref().and_then(|d| d.as_array());

                if let Some(edits) = text_edits {
                    if !edits.is_empty() {
                        // Read the current file content
                        let content = context.app_state.file_service.read_file(&file_path).await?;

                        // Apply the text edits
                        let formatted_content = Self::apply_text_edits(&content, edits)?;

                        // Write the formatted content back
                        context
                            .app_state
                            .file_service
                            .write_file(&file_path, &formatted_content, false)
                            .await?;

                        Ok(json!({
                            "formatted": true,
                            "file_path": file_path_str,
                            "plugin": response.metadata.plugin_name,
                            "processing_time_ms": response.metadata.processing_time_ms,
                        }))
                    } else {
                        // No formatting changes needed
                        Ok(json!({
                            "formatted": false,
                            "file_path": file_path_str,
                            "plugin": response.metadata.plugin_name,
                            "processing_time_ms": response.metadata.processing_time_ms,
                        }))
                    }
                } else {
                    // No edits returned
                    Ok(json!({
                        "formatted": false,
                        "file_path": file_path_str,
                        "plugin": response.metadata.plugin_name,
                        "processing_time_ms": response.metadata.processing_time_ms,
                    }))
                }
            }
            Err(err) => Err(cb_protocol::ApiError::Internal(format!(
                "Format document failed: {}",
                err
            ))),
        }
    }

    async fn handle_organize_imports(
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

        let dry_run = args
            .get("dry_run")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let file_path = PathBuf::from(&file_path_str);
        let mut request = PluginRequest::new("organize_imports".to_string(), file_path.clone());

        // Set parameters
        request = request.with_params(args);

        match context.plugin_manager.handle_request(request).await {
            Ok(response) => {
                // LSP textDocument/codeAction returns an array of CodeAction objects
                // Extract TextEdit[] from the first CodeAction's edit field
                let code_actions = response.data.as_ref().and_then(|d| d.as_array());

                if let Some(actions) = code_actions {
                    if !actions.is_empty() {
                        // Get first CodeAction with kind "source.organizeImports"
                        let organize_action = actions.iter().find(|action| {
                            action
                                .get("kind")
                                .and_then(|k| k.as_str())
                                .map(|k| k.starts_with("source.organizeImports"))
                                .unwrap_or(false)
                        });

                        if let Some(action) = organize_action {
                            // Extract TextEdit[] from action.edit.changes[file_uri]
                            if let Some(edit) = action.get("edit") {
                                if let Some(changes) = edit.get("changes") {
                                    // Get the first (and usually only) file's edits
                                    if let Some(changes_obj) = changes.as_object() {
                                        for (_uri, edits) in changes_obj {
                                            if let Some(text_edits) = edits.as_array() {
                                                if !text_edits.is_empty() {
                                                    if dry_run {
                                                        // Preview mode - don't apply changes
                                                        return Ok(json!({
                                                            "operation": "organize_imports",
                                                            "dry_run": true,
                                                            "modified": false,
                                                            "status": "preview",
                                                            "file_path": file_path_str,
                                                            "plugin": response.metadata.plugin_name,
                                                            "processing_time_ms": response.metadata.processing_time_ms,
                                                            "preview": {
                                                                "edits_count": text_edits.len(),
                                                                "edits": text_edits
                                                            }
                                                        }));
                                                    }

                                                    // Apply mode - apply the text edits
                                                    let content = context
                                                        .app_state
                                                        .file_service
                                                        .read_file(&file_path)
                                                        .await?;

                                                    let organized_content =
                                                        Self::apply_text_edits(&content, text_edits)?;

                                                    context
                                                        .app_state
                                                        .file_service
                                                        .write_file(&file_path, &organized_content, false)
                                                        .await?;

                                                    return Ok(json!({
                                                        "organized": true,
                                                        "file_path": file_path_str,
                                                        "plugin": response.metadata.plugin_name,
                                                        "processing_time_ms": response.metadata.processing_time_ms,
                                                    }));
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // No organize imports action available or no edits needed
                if dry_run {
                    Ok(json!({
                        "operation": "organize_imports",
                        "dry_run": true,
                        "modified": false,
                        "status": "preview",
                        "file_path": file_path_str,
                        "plugin": response.metadata.plugin_name,
                        "processing_time_ms": response.metadata.processing_time_ms,
                    }))
                } else {
                    Ok(json!({
                        "organized": false,
                        "file_path": file_path_str,
                        "plugin": response.metadata.plugin_name,
                        "processing_time_ms": response.metadata.processing_time_ms,
                    }))
                }
            }
            Err(err) => Err(cb_protocol::ApiError::Internal(format!(
                "Organize imports failed: {}",
                err
            ))),
        }
    }

    /// Apply LSP TextEdit array to content
    /// LSP formatting typically returns a single edit that replaces the entire document
    fn apply_text_edits(content: &str, edits: &[Value]) -> ServerResult<String> {
        // For formatting, LSP usually returns a single TextEdit replacing the entire document
        if edits.len() == 1 {
            let edit = &edits[0];
            if let Some(new_text) = edit["newText"].as_str() {
                return Ok(new_text.to_string());
            }
        }

        // If we have multiple edits, we need to apply them carefully
        // Convert content to owned lines for modification
        let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();

        // Sort edits by reverse position to apply them from end to start
        let mut sorted_edits: Vec<_> = edits.iter().collect();
        sorted_edits.sort_by(|a, b| {
            let a_range = &a["range"];
            let b_range = &b["range"];
            let a_start_line = a_range["start"]["line"].as_u64().unwrap_or(0);
            let b_start_line = b_range["start"]["line"].as_u64().unwrap_or(0);
            b_start_line.cmp(&a_start_line) // Reverse order
        });

        for edit in sorted_edits {
            let range = &edit["range"];
            let new_text = edit["newText"].as_str().unwrap_or("");

            let start_line = range["start"]["line"]
                .as_u64()
                .ok_or_else(|| cb_protocol::ApiError::Internal("Invalid edit range".into()))?
                as usize;
            let start_char = range["start"]["character"]
                .as_u64()
                .ok_or_else(|| cb_protocol::ApiError::Internal("Invalid edit range".into()))?
                as usize;
            let end_line = range["end"]["line"]
                .as_u64()
                .ok_or_else(|| cb_protocol::ApiError::Internal("Invalid edit range".into()))?
                as usize;
            let end_char = range["end"]["character"]
                .as_u64()
                .ok_or_else(|| cb_protocol::ApiError::Internal("Invalid edit range".into()))?
                as usize;

            if start_line == end_line && start_line < lines.len() {
                // Single line edit
                let line = &mut lines[start_line];
                let before = line.chars().take(start_char).collect::<String>();
                let after = line.chars().skip(end_char).collect::<String>();
                lines[start_line] = format!("{}{}{}", before, new_text, after);
            }
        }

        Ok(lines.join("\n"))
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
            // "rename_symbol_with_imports" moved to InternalEditingHandler
            "organize_imports",
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
            "organize_imports" => self.handle_organize_imports(context, tool_call).await,
            _ => crate::delegate_to_legacy!(self, context, tool_call),
        }
    }
}
