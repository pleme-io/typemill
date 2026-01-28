//! Prune Handler for Magnificent Seven API
//!
//! Implements the `prune` tool, which provides delete operations with cleanup.
//! This handler wraps the existing DeleteHandler functionality but uses the new
//! API shape with WriteResponse envelope.
//!
//! Supports:
//! - Symbol deletion (AST-based with import cleanup)
//! - File deletion (with reference cleanup)
//! - Directory deletion (with reference cleanup)

use crate::handlers::delete_handler::DeleteHandler;
use crate::handlers::tool_definitions::{Diagnostic, DiagnosticSeverity, WriteResponse};
use crate::handlers::tools::ToolHandler;
use async_trait::async_trait;
use mill_foundation::core::model::mcp::ToolCall;
use mill_foundation::errors::{MillError as ServerError, MillResult as ServerResult};
use mill_foundation::planning::RefactorPlan;
use serde::Deserialize;
use serde_json::Value;
use tracing::{debug, info};

/// Handler for prune operations (delete with cleanup)
pub struct PruneHandler {
    delete_handler: DeleteHandler,
}

impl PruneHandler {
    pub fn new() -> Self {
        Self {
            delete_handler: DeleteHandler::new(),
        }
    }
}

impl Default for PruneHandler {
    fn default() -> Self {
        Self::new()
    }
}

/// Parameters for the prune tool (Magnificent Seven API)
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PruneParams {
    target: PruneTarget,
    #[serde(default)]
    options: PruneOptions,
}

/// Target specification for prune operation
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PruneTarget {
    /// Type of target: "symbol", "file", or "directory"
    kind: String,
    /// Path to the file or directory (or file containing the symbol)
    file_path: String,
    /// Line number for symbol deletion (1-based)
    #[serde(default)]
    line: Option<u32>,
    /// Character offset for symbol deletion (0-based)
    #[serde(default)]
    character: Option<u32>,
}

/// Options for prune operation
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PruneOptions {
    /// Preview mode - don't actually apply changes (default: true for safety)
    #[serde(default = "crate::default_true")]
    dry_run: bool,
    /// Remove orphaned imports after deletion (default: true)
    #[serde(default = "default_true_option")]
    cleanup_imports: Option<bool>,
    /// Force deletion even if target has dependents (default: false)
    #[serde(default)]
    force: Option<bool>,
    /// Also remove associated test files/functions (default: false)
    #[serde(default)]
    remove_tests: Option<bool>,
}

impl Default for PruneOptions {
    fn default() -> Self {
        Self {
            dry_run: true,
            cleanup_imports: Some(true),
            force: None,
            remove_tests: None,
        }
    }
}

/// Helper for default true option values
fn default_true_option() -> Option<bool> {
    Some(true)
}

/// Parameters for the legacy delete handler
#[derive(Debug, serde::Serialize)]
struct DeleteHandlerParams {
    target: DeleteHandlerTarget,
    #[serde(default)]
    options: DeleteHandlerOptions,
}

#[derive(Debug, serde::Serialize)]
struct DeleteHandlerTarget {
    kind: String,
    path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    selector: Option<DeleteHandlerSelector>,
}

#[derive(Debug, serde::Serialize)]
struct DeleteHandlerSelector {
    line: u32,
    character: u32,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct DeleteHandlerOptions {
    dry_run: bool,
    cleanup_imports: Option<bool>,
    force: Option<bool>,
    remove_tests: Option<bool>,
}

#[async_trait]
impl ToolHandler for PruneHandler {
    fn tool_names(&self) -> &[&str] {
        &["prune"]
    }

    fn is_internal(&self) -> bool {
        false // Public tool (Magnificent Seven)
    }

    async fn handle_tool_call(
        &self,
        context: &mill_handler_api::ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        info!(tool_name = %tool_call.name, "Handling prune (delete with cleanup)");

        // Parse parameters from Magnificent Seven API shape
        let args = tool_call
            .arguments
            .clone()
            .ok_or_else(|| ServerError::invalid_request("Missing arguments for prune"))?;

        let params: PruneParams = serde_json::from_value(args.clone()).map_err(|e| {
            ServerError::invalid_request(format!("Invalid prune parameters: {}", e))
        })?;

        debug!(
            kind = %params.target.kind,
            file_path = %params.target.file_path,
            dry_run = params.options.dry_run,
            "Processing prune request"
        );

        // Validate target parameters based on kind
        if params.target.kind == "symbol" {
            if params.target.line.is_none() || params.target.character.is_none() {
                return Err(ServerError::invalid_request(
                    "Symbol deletion requires line and character parameters",
                ));
            }
        }

        // Convert to legacy DeleteHandler format
        let delete_params = DeleteHandlerParams {
            target: DeleteHandlerTarget {
                kind: params.target.kind.clone(),
                path: params.target.file_path.clone(),
                selector: match (params.target.line, params.target.character) {
                    (Some(line), Some(character)) => {
                        Some(DeleteHandlerSelector { line, character })
                    }
                    _ => None,
                },
            },
            options: DeleteHandlerOptions {
                dry_run: params.options.dry_run,
                cleanup_imports: params.options.cleanup_imports,
                force: params.options.force,
                remove_tests: params.options.remove_tests,
            },
        };

        // Create a new tool call for the delete handler
        let delete_tool_call = ToolCall {
            name: "delete".to_string(),
            arguments: Some(serde_json::to_value(delete_params).map_err(|e| {
                ServerError::internal(format!("Failed to serialize delete params: {}", e))
            })?),
        };

        // Call the legacy delete handler
        let delete_result = self
            .delete_handler
            .handle_tool_call(context, &delete_tool_call)
            .await?;

        // Convert the delete handler response to WriteResponse format
        let write_response = convert_delete_response_to_write_response(
            delete_result,
            &params.target.kind,
            &params.target.file_path,
            params.options.dry_run,
        )?;

        // Serialize and wrap in content envelope
        let response_json = serde_json::to_value(&write_response).map_err(|e| {
            ServerError::internal(format!("Failed to serialize WriteResponse: {}", e))
        })?;

        info!(
            status = ?write_response.status,
            files_changed = write_response.files_changed.len(),
            "Prune operation completed"
        );

        Ok(serde_json::json!({ "content": response_json }))
    }
}

/// Convert delete handler response to WriteResponse format
fn convert_delete_response_to_write_response(
    delete_result: Value,
    target_kind: &str,
    target_path: &str,
    dry_run: bool,
) -> ServerResult<WriteResponse> {
    // Extract the inner content from the delete response
    let content = delete_result
        .get("content")
        .ok_or_else(|| ServerError::internal("Delete response missing 'content' field"))?;

    // Check if this is a DeletePlan (preview) or ExecutionResult (applied)
    if dry_run {
        // Parse as RefactorPlan (which wraps DeletePlan)
        let refactor_plan: RefactorPlan = serde_json::from_value(content.clone())
            .map_err(|e| ServerError::internal(format!("Failed to parse RefactorPlan: {}", e)))?;

        // Extract the inner DeletePlan
        let plan = match refactor_plan {
            RefactorPlan::DeletePlan(p) => p,
            _ => {
                return Err(ServerError::internal(
                    "Expected DeletePlan, got different RefactorPlan variant",
                ))
            }
        };

        // Collect affected files from deletions and edits
        let mut files_changed = Vec::new();

        // Add deletion targets
        for deletion in &plan.deletions {
            files_changed.push(deletion.path.clone());
        }

        // Add files from workspace edits if present
        if let Some(edits) = &plan.edits {
            if let Some(changes) = &edits.changes {
                for file_path in changes.keys() {
                    let path_str = file_path.to_string();
                    if !files_changed.contains(&path_str) {
                        files_changed.push(path_str);
                    }
                }
            }
            // Handle document_changes if present (LSP WorkspaceEdit structure)
            if let Some(doc_changes) = &edits.document_changes {
                match doc_changes {
                    lsp_types::DocumentChanges::Edits(edits_list) => {
                        for edit in edits_list {
                            let uri = edit.text_document.uri.to_string();
                            if !files_changed.contains(&uri) {
                                files_changed.push(uri);
                            }
                        }
                    }
                    lsp_types::DocumentChanges::Operations(ops) => {
                        for op in ops {
                            match op {
                                lsp_types::DocumentChangeOperation::Edit(edit) => {
                                    let uri = edit.text_document.uri.to_string();
                                    if !files_changed.contains(&uri) {
                                        files_changed.push(uri);
                                    }
                                }
                                lsp_types::DocumentChangeOperation::Op(resource_op) => {
                                    // Handle resource operations (create, rename, delete)
                                    match resource_op {
                                        lsp_types::ResourceOp::Create(create) => {
                                            let uri = create.uri.to_string();
                                            if !files_changed.contains(&uri) {
                                                files_changed.push(uri);
                                            }
                                        }
                                        lsp_types::ResourceOp::Rename(rename) => {
                                            let old_uri = rename.old_uri.to_string();
                                            let new_uri = rename.new_uri.to_string();
                                            if !files_changed.contains(&old_uri) {
                                                files_changed.push(old_uri);
                                            }
                                            if !files_changed.contains(&new_uri) {
                                                files_changed.push(new_uri);
                                            }
                                        }
                                        lsp_types::ResourceOp::Delete(delete) => {
                                            let uri = delete.uri.to_string();
                                            if !files_changed.contains(&uri) {
                                                files_changed.push(uri);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Build summary message
        let summary = format!(
            "Preview: {} {} deletion affecting {} file(s)",
            target_kind,
            target_path,
            files_changed.len()
        );

        // Convert warnings to diagnostics
        let diagnostics: Vec<Diagnostic> = plan
            .warnings
            .iter()
            .map(|w| Diagnostic {
                severity: DiagnosticSeverity::Warning,
                message: w.message.clone(),
                file_path: None,
                line: None,
            })
            .collect();

        // Return preview response with plan as changes
        let mut response = WriteResponse::preview(summary, files_changed, content.clone());
        response.diagnostics = diagnostics;
        Ok(response)
    } else {
        // Work with ExecutionResult as JSON (it's only Serialize, not Deserialize)
        let success = content
            .get("success")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let applied_files: Vec<String> = content
            .get("appliedFiles")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        let warnings: Vec<String> = content
            .get("warnings")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        // Build summary message
        let summary = if success {
            format!(
                "Successfully deleted {} {} and updated {} file(s)",
                target_kind, target_path, applied_files.len()
            )
        } else {
            format!("Failed to delete {} {}", target_kind, target_path)
        };

        // Convert warnings to diagnostics
        let mut diagnostics: Vec<Diagnostic> = warnings
            .iter()
            .map(|w| Diagnostic {
                severity: DiagnosticSeverity::Warning,
                message: w.clone(),
                file_path: None,
                line: None,
            })
            .collect();

        // Convert validation result to diagnostics if present
        if let Some(validation) = content.get("validation") {
            let passed = validation
                .get("passed")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);

            if !passed {
                let command = validation
                    .get("command")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                let exit_code = validation
                    .get("exitCode")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(-1);
                let stderr = validation
                    .get("stderr")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                diagnostics.push(Diagnostic {
                    severity: DiagnosticSeverity::Error,
                    message: format!("Validation failed: {} (exit code {})", command, exit_code),
                    file_path: None,
                    line: None,
                });

                if !stderr.is_empty() {
                    diagnostics.push(Diagnostic {
                        severity: DiagnosticSeverity::Error,
                        message: stderr.to_string(),
                        file_path: None,
                        line: None,
                    });
                }
            }
        }

        // Return success or error response
        if success {
            let mut response = WriteResponse::success(summary, applied_files);
            response.diagnostics = diagnostics;
            Ok(response)
        } else {
            Ok(WriteResponse::error(summary, diagnostics))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prune_handler_tool_name() {
        let handler = PruneHandler::new();
        assert_eq!(handler.tool_names(), &["prune"]);
    }

    #[test]
    fn test_prune_handler_is_public() {
        let handler = PruneHandler::new();
        assert!(!handler.is_internal());
    }

    #[test]
    fn test_prune_options_default() {
        let options = PruneOptions::default();
        assert_eq!(options.dry_run, true);
        assert_eq!(options.cleanup_imports, Some(true));
        assert_eq!(options.force, None);
        assert_eq!(options.remove_tests, None);
    }

    #[test]
    fn test_prune_params_deserialization_symbol() {
        let json = serde_json::json!({
            "target": {
                "kind": "symbol",
                "filePath": "src/main.rs",
                "line": 10,
                "character": 5
            }
        });

        let params: PruneParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.target.kind, "symbol");
        assert_eq!(params.target.file_path, "src/main.rs");
        assert_eq!(params.target.line, Some(10));
        assert_eq!(params.target.character, Some(5));
        assert_eq!(params.options.dry_run, true); // default
    }

    #[test]
    fn test_prune_params_deserialization_file() {
        let json = serde_json::json!({
            "target": {
                "kind": "file",
                "filePath": "src/utils.rs"
            },
            "options": {
                "dryRun": false,
                "force": true
            }
        });

        let params: PruneParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.target.kind, "file");
        assert_eq!(params.target.file_path, "src/utils.rs");
        assert_eq!(params.target.line, None);
        assert_eq!(params.target.character, None);
        assert_eq!(params.options.dry_run, false);
        assert_eq!(params.options.force, Some(true));
    }

    #[test]
    fn test_prune_params_deserialization_directory() {
        let json = serde_json::json!({
            "target": {
                "kind": "directory",
                "filePath": "src/deprecated"
            },
            "options": {
                "dryRun": false,
                "cleanupImports": true,
                "removeTests": true
            }
        });

        let params: PruneParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.target.kind, "directory");
        assert_eq!(params.target.file_path, "src/deprecated");
        assert_eq!(params.options.dry_run, false);
        assert_eq!(params.options.cleanup_imports, Some(true));
        assert_eq!(params.options.remove_tests, Some(true));
    }
}
