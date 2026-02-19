//! Relocate handler for Magnificent Seven refactoring API
//!
//! Implements the `relocate` tool that moves symbols, files, or directories
//! with automatic import and reference updates. Calls services directly.
//!
//! # API Format
//!
//! ```json
//! {
//!   "target": {
//!     "kind": "symbol" | "file" | "directory",
//!     "filePath": "path/to/file",
//!     "line": 10,        // Required for symbol moves
//!     "character": 5     // Required for symbol moves
//!   },
//!   "destination": "path/to/destination",
//!   "options": {
//!     "dryRun": true  // Default: true (preview mode)
//!   }
//! }
//! ```

use crate::handlers::relocate_ops::{directory_move, file_move, symbol_move};
use crate::handlers::tool_definitions::{Diagnostic, DiagnosticSeverity, WriteResponse};
use crate::handlers::tools::ToolHandler;
use async_trait::async_trait;
use mill_foundation::core::model::mcp::ToolCall;
use mill_foundation::errors::{MillError as ServerError, MillResult as ServerResult};
use mill_foundation::planning::RefactorPlan;
use serde::Deserialize;
use serde_json::Value;
use std::path::Path;
use std::time::Instant;
use tracing::{debug, error, info};
use uuid::Uuid;

/// Handler for the `relocate` tool - calls services directly
pub struct RelocateHandler;

impl RelocateHandler {
    pub fn new() -> Self {
        Self
    }
}

impl Default for RelocateHandler {
    fn default() -> Self {
        Self::new()
    }
}

/// Input parameters for the relocate tool
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RelocateParams {
    target: RelocateTarget,
    destination: String,
    #[serde(default)]
    options: RelocateOptions,
}

/// Target specification for relocation
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RelocateTarget {
    kind: String,
    file_path: String,
    #[serde(default)]
    line: Option<u32>,
    #[serde(default)]
    character: Option<u32>,
}

/// Options for relocation operations
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RelocateOptions {
    #[serde(default = "crate::default_true")]
    dry_run: bool,
}

impl Default for RelocateOptions {
    fn default() -> Self {
        Self { dry_run: true }
    }
}

#[async_trait]
impl ToolHandler for RelocateHandler {
    fn tool_names(&self) -> &[&str] {
        &["relocate"]
    }

    async fn handle_tool_call(
        &self,
        context: &mill_handler_api::ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        let operation_id = Uuid::new_v4().to_string();

        info!(
            operation_id = %operation_id,
            tool_name = %tool_call.name,
            "Starting relocate operation"
        );

        // Parse parameters
        let args = tool_call
            .arguments
            .clone()
            .ok_or_else(|| ServerError::invalid_request("Missing arguments for relocate"))?;

        let params: RelocateParams = serde_json::from_value(args).map_err(|e| {
            ServerError::invalid_request(format!("Invalid relocate parameters: {}", e))
        })?;

        info!(
            operation_id = %operation_id,
            kind = %params.target.kind,
            source_path = %params.target.file_path,
            destination = %params.destination,
            dry_run = params.options.dry_run,
            "Parsed relocate parameters"
        );

        // Dispatch to appropriate planning function
        let plan = match params.target.kind.as_str() {
            "file" => {
                let old_path = Path::new(&params.target.file_path);
                let new_path = Path::new(&params.destination);
                let move_plan =
                    file_move::plan_file_move(old_path, new_path, context, &operation_id).await?;
                RefactorPlan::MovePlan(move_plan)
            }
            "directory" => {
                let old_path = Path::new(&params.target.file_path);
                let new_path = Path::new(&params.destination);
                let move_plan = directory_move::plan_directory_move(
                    old_path,
                    new_path,
                    None,
                    context,
                    &operation_id,
                )
                .await?;
                RefactorPlan::MovePlan(move_plan)
            }
            "symbol" => {
                let line = params.target.line.ok_or_else(|| {
                    ServerError::invalid_request("Symbol move requires 'line' field")
                })?;
                let character = params.target.character.ok_or_else(|| {
                    ServerError::invalid_request("Symbol move requires 'character' field")
                })?;
                let position = lsp_types::Position { line, character };
                let move_plan = symbol_move::plan_symbol_move(
                    &params.target.file_path,
                    &params.destination,
                    position,
                    context,
                    &operation_id,
                )
                .await?;
                RefactorPlan::MovePlan(move_plan)
            }
            _ => {
                return Err(ServerError::invalid_request(format!(
                    "Unknown target kind: '{}'. Expected 'file', 'directory', or 'symbol'",
                    params.target.kind
                )));
            }
        };

        // Handle dry run vs execution
        if params.options.dry_run {
            self.build_preview_response(&plan, &params, &operation_id)
        } else {
            self.execute_and_build_response(context, plan, &params, &operation_id)
                .await
        }
    }
}

impl RelocateHandler {
    /// Build preview response from plan
    fn build_preview_response(
        &self,
        plan: &RefactorPlan,
        params: &RelocateParams,
        operation_id: &str,
    ) -> ServerResult<Value> {
        debug!(operation_id = %operation_id, "Building preview response");

        let (affected_files, warnings) = match plan {
            RefactorPlan::MovePlan(move_plan) => {
                let files = Self::extract_files_from_workspace_edit(&move_plan.edits);
                let warnings: Vec<String> = move_plan
                    .warnings
                    .iter()
                    .map(|w| w.message.clone())
                    .collect();
                (files, warnings)
            }
            _ => (vec![], vec![]),
        };

        let summary = format!(
            "Preview: Would move {} from '{}' to '{}' (affects {} file(s))",
            params.target.kind,
            params.target.file_path,
            params.destination,
            affected_files.len()
        );

        let plan_json = serde_json::to_value(plan)
            .map_err(|e| ServerError::internal(format!("Failed to serialize plan: {}", e)))?;

        let mut response = WriteResponse::preview(summary, affected_files, plan_json);
        for warning in warnings {
            response = response.with_warning(warning);
        }

        serde_json::to_value(&response)
            .map(|v| serde_json::json!({ "content": v }))
            .map_err(|e| ServerError::internal(format!("Failed to serialize response: {}", e)))
    }

    /// Extract file paths from a WorkspaceEdit
    fn extract_files_from_workspace_edit(edit: &lsp_types::WorkspaceEdit) -> Vec<String> {
        use std::collections::HashSet;
        let mut files = HashSet::new();

        // Extract from changes map
        if let Some(ref changes) = edit.changes {
            for uri in changes.keys() {
                files.insert(uri.to_string());
            }
        }

        // Extract from document_changes
        if let Some(ref doc_changes) = edit.document_changes {
            match doc_changes {
                lsp_types::DocumentChanges::Edits(edits) => {
                    for edit in edits {
                        files.insert(edit.text_document.uri.to_string());
                    }
                }
                lsp_types::DocumentChanges::Operations(ops) => {
                    for op in ops {
                        match op {
                            lsp_types::DocumentChangeOperation::Edit(edit) => {
                                files.insert(edit.text_document.uri.to_string());
                            }
                            lsp_types::DocumentChangeOperation::Op(resource_op) => {
                                match resource_op {
                                    lsp_types::ResourceOp::Create(c) => {
                                        files.insert(c.uri.to_string());
                                    }
                                    lsp_types::ResourceOp::Rename(r) => {
                                        files.insert(r.old_uri.to_string());
                                        files.insert(r.new_uri.to_string());
                                    }
                                    lsp_types::ResourceOp::Delete(d) => {
                                        files.insert(d.uri.to_string());
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

    /// Execute plan and build response
    async fn execute_and_build_response(
        &self,
        context: &mill_handler_api::ToolHandlerContext,
        plan: RefactorPlan,
        params: &RelocateParams,
        operation_id: &str,
    ) -> ServerResult<Value> {
        debug!(operation_id = %operation_id, "Executing relocate plan");

        let apply_start = Instant::now();
        let result = crate::handlers::common::execute_refactor_plan(context, plan).await?;
        info!(
            operation_id = %operation_id,
            apply_ms = apply_start.elapsed().as_millis(),
            success = result.success,
            applied_files = result.applied_files.len(),
            "perf: relocate_apply"
        );

        if result.success {
            let summary = format!(
                "Successfully moved {} from '{}' to '{}' ({} file(s) modified)",
                params.target.kind,
                params.target.file_path,
                params.destination,
                result.applied_files.len()
            );

            let response = WriteResponse::success(summary, result.applied_files);
            serde_json::to_value(&response)
                .map(|v| serde_json::json!({ "content": v }))
                .map_err(|e| ServerError::internal(format!("Failed to serialize response: {}", e)))
        } else {
            error!(operation_id = %operation_id, "Relocate execution failed");
            let response = WriteResponse::error(
                format!("Failed to move {}", params.target.kind),
                vec![Diagnostic {
                    severity: DiagnosticSeverity::Error,
                    message: "Move operation failed".to_string(),
                    file_path: Some(params.target.file_path.clone()),
                    line: params.target.line,
                }],
            );
            serde_json::to_value(&response)
                .map(|v| serde_json::json!({ "content": v }))
                .map_err(|e| ServerError::internal(format!("Failed to serialize response: {}", e)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_relocate_handler_tool_names() {
        let handler = RelocateHandler::new();
        assert_eq!(handler.tool_names(), &["relocate"]);
    }

    #[test]
    fn test_relocate_params_deserialization() {
        let json = json!({
            "target": {
                "kind": "file",
                "filePath": "src/old.rs"
            },
            "destination": "src/new.rs"
        });

        let params: RelocateParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.target.kind, "file");
        assert_eq!(params.target.file_path, "src/old.rs");
        assert_eq!(params.destination, "src/new.rs");
        assert!(params.options.dry_run);
    }

    #[test]
    fn test_relocate_params_with_options() {
        let json = json!({
            "target": {
                "kind": "symbol",
                "filePath": "src/app.rs",
                "line": 10,
                "character": 5
            },
            "destination": "src/utils.rs",
            "options": {
                "dryRun": false
            }
        });

        let params: RelocateParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.target.kind, "symbol");
        assert_eq!(params.target.line, Some(10));
        assert_eq!(params.target.character, Some(5));
        assert!(!params.options.dry_run);
    }
}
