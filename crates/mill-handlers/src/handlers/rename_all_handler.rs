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
use lsp_types::{DocumentChangeOperation, DocumentChanges, Position, ResourceOp};
use mill_foundation::core::model::mcp::ToolCall;
use mill_foundation::errors::{MillError as ServerError, MillResult as ServerResult};
use mill_foundation::planning::RefactorPlan;
use mill_foundation::protocol::RefactorPlanExt;
use mill_services::services::ExecutionResult;
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
    /// The target to rename (single mode)
    target: Option<RenameAllTarget>,
    /// Multiple targets (batch mode)
    #[serde(default)]
    targets: Option<Vec<RenameAllTarget>>,
    /// New name for the target (single mode)
    new_name: Option<String>,
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
    /// New name (required for batch mode, optional for single mode)
    #[serde(default)]
    new_name: Option<String>,
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
            new_name: target.new_name.clone(),
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

    /// Convert RefactorPlan to WriteResponse (Preview Mode)
    fn convert_plan_to_write_response(plan: &RefactorPlan) -> ServerResult<WriteResponse> {
        // Extract summary directly from struct
        let summary = match plan {
            RefactorPlan::RenamePlan(p) => &p.summary,
            RefactorPlan::ExtractPlan(p) => &p.summary,
            RefactorPlan::InlinePlan(p) => &p.summary,
            RefactorPlan::MovePlan(p) => &p.summary,
            RefactorPlan::ReorderPlan(p) => &p.summary,
            RefactorPlan::TransformPlan(p) => &p.summary,
            RefactorPlan::DeletePlan(p) => &p.summary,
        };

        let affected_files_count = summary.affected_files;
        let summary_text = format!(
            "Preview: {} file(s) will be affected by this rename",
            affected_files_count
        );

        // Extract affected files using RefactorPlanExt and manual extraction
        let files_changed = Self::extract_affected_files_from_plan(plan);

        // Extract warnings via RefactorPlanExt
        let diagnostics = plan
            .warnings()
            .iter()
            .map(|w| super::tool_definitions::Diagnostic {
                severity: super::tool_definitions::DiagnosticSeverity::Warning,
                message: w.message.clone(),
                file_path: None,
                line: None,
            })
            .collect();

        // Convert plan to JSON for the changes field
        let changes = serde_json::to_value(plan)
            .map_err(|e| ServerError::internal(format!("Failed to serialize plan: {}", e)))?;

        Ok(WriteResponse {
            status: super::tool_definitions::WriteStatus::Preview,
            summary: summary_text,
            files_changed,
            diagnostics,
            changes: Some(changes),
        })
    }

    /// Convert ExecutionResult to WriteResponse (Execution Mode)
    fn convert_result_to_write_response(result: &ExecutionResult) -> ServerResult<WriteResponse> {
        let summary_text = if result.success {
            format!("Successfully renamed {} file(s)", result.applied_files.len())
        } else {
            "Rename operation failed".to_string()
        };

        let status = if result.success {
            super::tool_definitions::WriteStatus::Success
        } else {
            super::tool_definitions::WriteStatus::Error
        };

        let diagnostics = result
            .warnings
            .iter()
            .map(|w| super::tool_definitions::Diagnostic {
                severity: super::tool_definitions::DiagnosticSeverity::Warning,
                message: w.clone(),
                file_path: None,
                line: None,
            })
            .collect();

        // Convert result to JSON for the changes field
        let changes = serde_json::to_value(result)
            .map_err(|e| ServerError::internal(format!("Failed to serialize result: {}", e)))?;

        Ok(WriteResponse {
            status,
            summary: summary_text,
            files_changed: result.applied_files.clone(),
            diagnostics,
            changes: Some(changes),
        })
    }

    /// Extract list of affected file paths from a RefactorPlan
    fn extract_affected_files_from_plan(plan: &RefactorPlan) -> Vec<String> {
        let mut files = HashSet::new();

        // Handle DeletePlan specifically
        if let RefactorPlan::DeletePlan(delete_plan) = plan {
            for target in &delete_plan.deletions {
                files.insert(target.path.clone());
            }
        }

        // Handle WorkspaceEdit for all plans
        let edit = plan.workspace_edit();

        // Extract from edits.changes
        if let Some(changes) = &edit.changes {
            for uri in changes.keys() {
                if let Some(path) = Self::uri_to_path(uri) {
                    files.insert(path);
                }
            }
        }

        // Extract from edits.documentChanges
        if let Some(doc_changes) = &edit.document_changes {
            match doc_changes {
                DocumentChanges::Edits(edits) => {
                    for text_doc_edit in edits {
                        if let Some(path) = Self::uri_to_path(&text_doc_edit.text_document.uri) {
                            files.insert(path);
                        }
                    }
                }
                DocumentChanges::Operations(ops) => {
                    for op in ops {
                        match op {
                            DocumentChangeOperation::Edit(edit) => {
                                if let Some(path) = Self::uri_to_path(&edit.text_document.uri) {
                                    files.insert(path);
                                }
                            }
                            DocumentChangeOperation::Op(op) => match op {
                                ResourceOp::Create(create) => {
                                    if let Some(path) = Self::uri_to_path(&create.uri) {
                                        files.insert(path);
                                    }
                                }
                                ResourceOp::Rename(rename) => {
                                    if let Some(path) = Self::uri_to_path(&rename.old_uri) {
                                        files.insert(path);
                                    }
                                    if let Some(path) = Self::uri_to_path(&rename.new_uri) {
                                        files.insert(path);
                                    }
                                }
                                ResourceOp::Delete(delete) => {
                                    if let Some(path) = Self::uri_to_path(&delete.uri) {
                                        files.insert(path);
                                    }
                                }
                            },
                        }
                    }
                }
            }
        }

        files.into_iter().collect()
    }

    /// Helper to convert LSP Uri to file path string
    fn uri_to_path(uri: &lsp_types::Uri) -> Option<String> {
        url::Url::parse(uri.as_str()).ok().and_then(|url| {
            url.to_file_path()
                .ok()
                .map(|p| p.to_string_lossy().to_string())
        })
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
            .as_ref()
            .ok_or_else(|| ServerError::invalid_request("Missing arguments for rename_all"))?;

        let params: RenameAllParams = RenameAllParams::deserialize(args).map_err(|e| {
            ServerError::invalid_request(format!("Invalid rename_all parameters: {}", e))
        })?;

        let rename_options = Self::convert_to_rename_options(&params.options);

        // Determine mode: Batch or Single
        let plan = if let Some(targets) = &params.targets {
            // Batch mode
            debug!(
                targets_count = targets.len(),
                dry_run = params.options.dry_run,
                "Processing batch rename_all request"
            );

            // Convert all targets
            let mut rename_targets = Vec::new();
            for t in targets {
                if !["symbol", "file", "directory"].contains(&t.kind.as_str()) {
                    return Err(ServerError::invalid_request(format!(
                        "Unsupported target kind: '{}'. Must be one of: symbol, file, directory",
                        t.kind
                    )));
                }
                rename_targets.push(Self::convert_to_rename_target(t)?);
            }

            self.rename_service
                .plan_batch_rename(&rename_targets, &rename_options, context)
                .await?
        } else if let Some(target) = &params.target {
            // Single mode
            debug!(
                kind = %target.kind,
                file_path = %target.file_path,
                new_name = ?params.new_name,
                dry_run = params.options.dry_run,
                "Processing single rename_all request"
            );

            // Validate target kind
            if !["symbol", "file", "directory"].contains(&target.kind.as_str()) {
                return Err(ServerError::invalid_request(format!(
                    "Unsupported target kind: '{}'. Must be one of: symbol, file, directory",
                    target.kind
                )));
            }

            // For single mode, new_name is required in params if not in target (but we'll check later or in handler)
            // Ideally it should be provided.
            let new_name = params
                .new_name
                .as_ref()
                .or(target.new_name.as_ref())
                .ok_or_else(|| {
                    ServerError::invalid_request("new_name is required for rename operation")
                })?;

            // Convert to internal format
            let rename_target = Self::convert_to_rename_target(target)?;

            // Call the appropriate rename handler method based on target kind
            match target.kind.as_str() {
                "symbol" => {
                    self.rename_service
                        .plan_symbol_rename(&rename_target, new_name, &rename_options, context)
                        .await?
                }
                "file" => {
                    self.rename_service
                        .plan_file_rename(&rename_target, new_name, &rename_options, context)
                        .await?
                }
                "directory" => {
                    self.rename_service
                        .plan_directory_rename(&rename_target, new_name, &rename_options, context)
                        .await?
                }
                _ => unreachable!("Target kind already validated"),
            }
        } else {
            return Err(ServerError::invalid_request(
                "Either 'target' (single) or 'targets' (batch) must be provided",
            ));
        };

        // Wrap in RefactorPlan enum
        let refactor_plan = RefactorPlan::RenamePlan(plan);

        // Check if we should execute or just return plan
        let write_response = if params.options.dry_run {
            info!(
                operation = "rename_all",
                dry_run = true,
                "Returning rename plan (preview mode)"
            );

            // Preview mode - return plan
            Self::convert_plan_to_write_response(&refactor_plan)?
        } else {
            // Execution mode - execute the plan
            info!(
                operation = "rename_all",
                dry_run = false,
                "Executing rename plan"
            );

            let result =
                crate::handlers::common::execute_refactor_plan(context, refactor_plan).await?;

            info!(
                operation = "rename_all",
                success = result.success,
                applied_files = result.applied_files.len(),
                "Rename execution completed"
            );

            Self::convert_result_to_write_response(&result)?
        };

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
            new_name: None,
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
            new_name: None,
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
            new_name: None,
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
