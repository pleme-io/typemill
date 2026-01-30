//! Rename All Handler - Magnificent Seven API
//!
//! Implements the `rename_all` tool which provides a unified interface for
//! renaming symbols, files, and directories with automatic reference updates.
//!
//! This handler uses the rename planning service and exposes it through the
//! Magnificent Seven API with the WriteResponse envelope.

use super::rename_ops::{RenameOptions, RenameService, RenameTarget, SymbolSelector};
use super::tool_definitions::WriteResponse;
use crate::handlers::tools::ToolHandler;
use async_trait::async_trait;
use lsp_types::Position;
use mill_foundation::core::model::mcp::ToolCall;
use mill_foundation::errors::{MillError as ServerError, MillResult as ServerResult};
use mill_foundation::planning::RefactorPlan;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashSet;
use tracing::{debug, info};

/// Handler for the `rename_all` tool (Magnificent Seven API)
pub struct RenameAllHandler {
    rename_service: RenameService,
}

impl RenameAllHandler {
    pub fn new() -> Self {
        Self {
            rename_service: RenameService::new(),
        }
    }
}

impl Default for RenameAllHandler {
    fn default() -> Self {
        Self::new()
    }
}

/// Parameters for the rename_all tool
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RenameAllParams {
    /// The target to rename
    target: RenameAllTarget,
    /// New name for the target
    new_name: String,
    /// Optional configuration
    #[serde(default)]
    options: RenameAllOptions,
}

/// Target specification for rename_all
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RenameAllTarget {
    /// Kind of target: "symbol", "file", or "directory"
    kind: String,
    /// Path to the file/directory, or file containing the symbol
    file_path: String,
    /// Line number for symbol rename (0-based)
    #[serde(default)]
    line: Option<u32>,
    /// Character offset for symbol rename (0-based)
    #[serde(default)]
    character: Option<u32>,
}

/// Options for rename_all
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RenameAllOptions {
    /// Preview changes without applying (default: true for safety)
    #[serde(default = "crate::default_true")]
    dry_run: bool,
    /// Scope configuration: "code", "standard", "comments", "everything"
    #[serde(default)]
    scope: Option<String>,
    /// Consolidate source package into target (for directory renames only)
    /// When true, merges Cargo.toml dependencies and updates all imports.
    /// When false, disables consolidation even if auto-detection would enable it.
    /// When None, auto-detects based on path patterns (moving crate into another crate's src/).
    #[serde(default)]
    consolidate: Option<bool>,
}

impl Default for RenameAllOptions {
    fn default() -> Self {
        Self {
            dry_run: true, // Safe default - preview mode
            scope: None,
            consolidate: None,
        }
    }
}

impl RenameAllHandler {
    /// Convert RenameAllParams to the internal RenameTarget format
    fn convert_to_rename_target(target: &RenameAllTarget) -> ServerResult<RenameTarget> {
        // For symbol renames, position is required
        let selector = if target.kind == "symbol" {
            let line = target.line.ok_or_else(|| {
                ServerError::invalid_request("line is required for symbol rename")
            })?;
            let character = target.character.ok_or_else(|| {
                ServerError::invalid_request("character is required for symbol rename")
            })?;

            Some(SymbolSelector {
                position: Position { line, character },
            })
        } else {
            None
        };

        Ok(RenameTarget {
            kind: target.kind.clone(),
            path: target.file_path.clone(),
            new_name: None, // Not used in single target mode
            selector,
        })
    }

    /// Convert RenameAllOptions to internal RenameOptions
    fn convert_to_rename_options(options: &RenameAllOptions) -> RenameOptions {
        RenameOptions {
            dry_run: options.dry_run,
            scope: options.scope.clone(),
            strict: None,
            validate_scope: None,
            update_imports: None,
            custom_scope: None,
            consolidate: options.consolidate,
        }
    }

    /// Convert plan or execution result to WriteResponse
    fn convert_to_write_response(content: Value, dry_run: bool) -> ServerResult<WriteResponse> {
        if dry_run {
            // Preview mode - extract plan details
            // RefactorPlan uses internally tagged serialization: {"planType": "renamePlan", ...}
            // So fields are directly on content, not nested under "RenamePlan"
            let plan_type = content
                .get("planType")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    ServerError::internal("Expected planType in preview mode".to_string())
                })?;

            // Verify it's a rename plan (camelCase due to serde rename_all)
            if plan_type != "renamePlan" {
                return Err(ServerError::internal(format!(
                    "Expected renamePlan, got: {}",
                    plan_type
                )));
            }

            // Access summary directly on content (not nested)
            let summary_obj = content.get("summary").ok_or_else(|| {
                ServerError::internal("Missing summary in RenamePlan".to_string())
            })?;

            // Field is camelCase: affectedFiles
            let affected_files = summary_obj
                .get("affectedFiles")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);

            // Extract list of affected file paths from edits
            let files_changed = Self::extract_affected_files(&content);

            let summary_text = format!(
                "Preview: {} file(s) will be affected by this rename",
                affected_files
            );

            // Extract warnings
            let diagnostics =
                if let Some(warnings_arr) = content.get("warnings").and_then(|w| w.as_array()) {
                    warnings_arr
                        .iter()
                        .filter_map(|w| {
                            let message = w.get("message")?.as_str()?;
                            Some(super::tool_definitions::Diagnostic {
                                severity: super::tool_definitions::DiagnosticSeverity::Warning,
                                message: message.to_string(),
                                file_path: None,
                                line: None,
                            })
                        })
                        .collect()
                } else {
                    Vec::new()
                };

            Ok(WriteResponse {
                status: super::tool_definitions::WriteStatus::Preview,
                summary: summary_text,
                files_changed,
                diagnostics,
                changes: Some(content.clone()),
            })
        } else {
            // Execution mode - extract result details
            let result = content.get("content").ok_or_else(|| {
                ServerError::internal("Expected content in execution result".to_string())
            })?;

            let success = result
                .get("success")
                .and_then(|s| s.as_bool())
                .unwrap_or(false);

            let applied_files = result
                .get("applied_files")
                .or_else(|| result.get("appliedFiles"))
                .and_then(|a| a.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();

            let summary_text = if success {
                format!("Successfully renamed {} file(s)", applied_files.len())
            } else {
                "Rename operation failed".to_string()
            };

            // Extract warnings from execution result
            let diagnostics =
                if let Some(warnings_arr) = result.get("warnings").and_then(|w| w.as_array()) {
                    warnings_arr
                        .iter()
                        .filter_map(|w| {
                            let message = w.as_str()?;
                            Some(super::tool_definitions::Diagnostic {
                                severity: super::tool_definitions::DiagnosticSeverity::Warning,
                                message: message.to_string(),
                                file_path: None,
                                line: None,
                            })
                        })
                        .collect()
                } else {
                    Vec::new()
                };

            let status = if success {
                super::tool_definitions::WriteStatus::Success
            } else {
                super::tool_definitions::WriteStatus::Error
            };

            Ok(WriteResponse {
                status,
                summary: summary_text,
                files_changed: applied_files,
                diagnostics,
                changes: Some(result.clone()),
            })
        }
    }

    /// Extract list of affected file paths from a RenamePlan
    fn extract_affected_files(plan: &Value) -> Vec<String> {
        let mut files = HashSet::new();

        // Extract from edits.changes
        if let Some(changes) = plan.get("edits").and_then(|e| e.get("changes")) {
            if let Some(changes_obj) = changes.as_object() {
                for (uri, _) in changes_obj {
                    if let Ok(url) = url::Url::parse(uri) {
                        if let Ok(path) = url.to_file_path() {
                            files.insert(path.to_string_lossy().to_string());
                        }
                    }
                }
            }
        }

        // Extract from edits.documentChanges
        if let Some(doc_changes) = plan.get("edits").and_then(|e| e.get("documentChanges")) {
            if let Some(operations) = doc_changes.get("Operations").and_then(|o| o.as_array()) {
                for op in operations {
                    // Handle Edit operations
                    if let Some(edit) = op.get("Edit") {
                        if let Some(uri) = edit
                            .get("textDocument")
                            .and_then(|td| td.get("uri"))
                            .and_then(|u| u.as_str())
                        {
                            if let Ok(url) = url::Url::parse(uri) {
                                if let Ok(path) = url.to_file_path() {
                                    files.insert(path.to_string_lossy().to_string());
                                }
                            }
                        }
                    }

                    // Handle resource operations (Create, Rename, Delete)
                    if let Some(resource_op) = op.get("Op") {
                        // Create
                        if let Some(create) = resource_op.get("Create") {
                            if let Some(uri) = create.get("uri").and_then(|u| u.as_str()) {
                                if let Ok(url) = url::Url::parse(uri) {
                                    if let Ok(path) = url.to_file_path() {
                                        files.insert(path.to_string_lossy().to_string());
                                    }
                                }
                            }
                        }

                        // Rename
                        if let Some(rename) = resource_op.get("Rename") {
                            if let Some(old_uri) = rename.get("oldUri").and_then(|u| u.as_str()) {
                                if let Ok(url) = url::Url::parse(old_uri) {
                                    if let Ok(path) = url.to_file_path() {
                                        files.insert(path.to_string_lossy().to_string());
                                    }
                                }
                            }
                            if let Some(new_uri) = rename.get("newUri").and_then(|u| u.as_str()) {
                                if let Ok(url) = url::Url::parse(new_uri) {
                                    if let Ok(path) = url.to_file_path() {
                                        files.insert(path.to_string_lossy().to_string());
                                    }
                                }
                            }
                        }

                        // Delete
                        if let Some(delete) = resource_op.get("Delete") {
                            if let Some(uri) = delete.get("uri").and_then(|u| u.as_str()) {
                                if let Ok(url) = url::Url::parse(uri) {
                                    if let Ok(path) = url.to_file_path() {
                                        files.insert(path.to_string_lossy().to_string());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        files.into_iter().collect()
    }
}

#[async_trait]
impl ToolHandler for RenameAllHandler {
    fn tool_names(&self) -> &[&str] {
        &["rename_all"]
    }

    async fn handle_tool_call(
        &self,
        context: &mill_handler_api::ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        info!(tool_name = %tool_call.name, "Handling rename_all");

        // Parse parameters
        let args = tool_call
            .arguments
            .clone()
            .ok_or_else(|| ServerError::invalid_request("Missing arguments for rename_all"))?;

        let params: RenameAllParams = serde_json::from_value(args).map_err(|e| {
            ServerError::invalid_request(format!("Invalid rename_all parameters: {}", e))
        })?;

        debug!(
            kind = %params.target.kind,
            file_path = %params.target.file_path,
            new_name = %params.new_name,
            dry_run = params.options.dry_run,
            "Processing rename_all request"
        );

        // Validate target kind
        if !["symbol", "file", "directory"].contains(&params.target.kind.as_str()) {
            return Err(ServerError::invalid_request(format!(
                "Unsupported target kind: '{}'. Must be one of: symbol, file, directory",
                params.target.kind
            )));
        }

        // Convert to internal format
        let rename_target = Self::convert_to_rename_target(&params.target)?;
        let rename_options = Self::convert_to_rename_options(&params.options);

        // Call the appropriate rename handler method based on target kind
        let plan = match params.target.kind.as_str() {
            "symbol" => {
                self.rename_service
                    .plan_symbol_rename(&rename_target, &params.new_name, &rename_options, context)
                    .await?
            }
            "file" => {
                self.rename_service
                    .plan_file_rename(&rename_target, &params.new_name, &rename_options, context)
                    .await?
            }
            "directory" => {
                self.rename_service
                    .plan_directory_rename(
                        &rename_target,
                        &params.new_name,
                        &rename_options,
                        context,
                    )
                    .await?
            }
            _ => unreachable!("Target kind already validated"),
        };

        // Wrap in RefactorPlan enum
        let refactor_plan = RefactorPlan::RenamePlan(plan);

        // Check if we should execute or just return plan
        let result_json = if params.options.dry_run {
            // Preview mode - return plan
            let plan_json = serde_json::to_value(&refactor_plan).map_err(|e| {
                ServerError::internal(format!("Failed to serialize rename plan: {}", e))
            })?;

            info!(
                operation = "rename_all",
                dry_run = true,
                "Returning rename plan (preview mode)"
            );

            plan_json
        } else {
            // Execution mode - execute the plan
            info!(
                operation = "rename_all",
                dry_run = false,
                "Executing rename plan"
            );

            let result =
                crate::handlers::common::execute_refactor_plan(context, refactor_plan).await?;

            let result_json = serde_json::to_value(&result).map_err(|e| {
                ServerError::internal(format!("Failed to serialize execution result: {}", e))
            })?;

            info!(
                operation = "rename_all",
                success = result.success,
                applied_files = result.applied_files.len(),
                "Rename execution completed"
            );

            json!({
                "content": result_json
            })
        };

        // Convert to WriteResponse envelope
        let write_response = Self::convert_to_write_response(result_json, params.options.dry_run)?;

        // Wrap in MCP content envelope
        Ok(json!({
            "content": write_response
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handler_tool_names() {
        let handler = RenameAllHandler::new();
        assert_eq!(handler.tool_names(), &["rename_all"]);
    }

    #[test]
    fn test_convert_to_rename_target_file() {
        let target = RenameAllTarget {
            kind: "file".to_string(),
            file_path: "src/main.rs".to_string(),
            line: None,
            character: None,
        };

        let result = RenameAllHandler::convert_to_rename_target(&target).unwrap();
        assert_eq!(result.kind, "file");
        assert_eq!(result.path, "src/main.rs");
        assert!(result.selector.is_none());
    }

    #[test]
    fn test_convert_to_rename_target_symbol() {
        let target = RenameAllTarget {
            kind: "symbol".to_string(),
            file_path: "src/lib.rs".to_string(),
            line: Some(10),
            character: Some(5),
        };

        let result = RenameAllHandler::convert_to_rename_target(&target).unwrap();
        assert_eq!(result.kind, "symbol");
        assert_eq!(result.path, "src/lib.rs");
        assert!(result.selector.is_some());
        let selector = result.selector.unwrap();
        assert_eq!(selector.position.line, 10);
        assert_eq!(selector.position.character, 5);
    }

    #[test]
    fn test_convert_to_rename_target_symbol_missing_position() {
        let target = RenameAllTarget {
            kind: "symbol".to_string(),
            file_path: "src/lib.rs".to_string(),
            line: None,
            character: Some(5),
        };

        let result = RenameAllHandler::convert_to_rename_target(&target);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("line is required"));
    }

    #[test]
    fn test_default_options() {
        let options = RenameAllOptions::default();
        assert!(options.dry_run); // Default is true for safety
        assert!(options.scope.is_none());
    }
}
