//! Prune Handler for Magnificent Seven API
//!
//! Implements the `prune` tool, which provides delete operations with cleanup.
//! Calls prune planning methods directly for clean service-oriented architecture.
//!
//! Supports:
//! - Symbol deletion (AST-based with import cleanup)
//! - File deletion (with reference cleanup)
//! - Directory deletion (with reference cleanup)

use crate::handlers::prune_ops::{
    PruneOptions, PrunePlanParams, PrunePlanner, PruneSelector, PruneTarget,
};
use crate::handlers::tool_definitions::{Diagnostic, DiagnosticSeverity, WriteResponse};
use crate::handlers::tools::ToolHandler;
use async_trait::async_trait;
use mill_foundation::core::model::mcp::ToolCall;
use mill_foundation::errors::{MillError as ServerError, MillResult as ServerResult};
use mill_foundation::planning::RefactorPlan;
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashSet;
use tracing::{debug, info};

/// Handler for prune operations (delete with cleanup) - calls services directly
pub struct PruneHandler {
    prune_planner: PrunePlanner,
}

impl PruneHandler {
    pub fn new() -> Self {
        Self {
            prune_planner: PrunePlanner::new(),
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
    target: PruneTargetInput,
    #[serde(default)]
    options: PruneOptionsInput,
}

/// Target specification for prune operation
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PruneTargetInput {
    kind: String,
    file_path: String,
    #[serde(default)]
    line: Option<u32>,
    #[serde(default)]
    character: Option<u32>,
}

/// Options for prune operation
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PruneOptionsInput {
    #[serde(default = "crate::default_true")]
    dry_run: bool,
    #[serde(default = "default_true_option")]
    cleanup_imports: Option<bool>,
    #[serde(default)]
    force: Option<bool>,
    #[serde(default)]
    remove_tests: Option<bool>,
}

impl Default for PruneOptionsInput {
    fn default() -> Self {
        Self {
            dry_run: true,
            cleanup_imports: Some(true),
            force: None,
            remove_tests: None,
        }
    }
}

fn default_true_option() -> Option<bool> {
    Some(true)
}

#[async_trait]
impl ToolHandler for PruneHandler {
    fn tool_names(&self) -> &[&str] {
        &["prune"]
    }

    async fn handle_tool_call(
        &self,
        context: &mill_handler_api::ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        info!(tool_name = %tool_call.name, "Handling prune (delete with cleanup)");

        // Parse parameters
        let args = tool_call
            .arguments
            .as_ref()
            .ok_or_else(|| ServerError::invalid_request("Missing arguments for prune"))?;

        let params: PruneParams = PruneParams::deserialize(args).map_err(|e| {
            ServerError::invalid_request(format!("Invalid prune parameters: {}", e))
        })?;

        debug!(
            kind = %params.target.kind,
            file_path = %params.target.file_path,
            dry_run = params.options.dry_run,
            "Processing prune request"
        );

        // Validate symbol parameters
        if params.target.kind == "symbol"
            && (params.target.line.is_none() || params.target.character.is_none())
        {
            return Err(ServerError::invalid_request(
                "Symbol deletion requires line and character parameters",
            ));
        }

        // Convert to prune planning params
        let delete_params = PrunePlanParams {
            target: PruneTarget {
                kind: params.target.kind.clone(),
                path: params.target.file_path.clone(),
                selector: match (params.target.line, params.target.character) {
                    (Some(line), Some(character)) => Some(PruneSelector {
                        line,
                        character,
                        symbol_name: None,
                    }),
                    _ => None,
                },
            },
            options: PruneOptions {
                dry_run: params.options.dry_run,
                cleanup_imports: params.options.cleanup_imports,
                force: params.options.force,
                remove_tests: params.options.remove_tests,
            },
        };

        // Call the appropriate planning method directly
        let planning_start = std::time::Instant::now();
        let plan = match params.target.kind.as_str() {
            "symbol" => {
                self.prune_planner
                    .plan_symbol_delete(&delete_params, context)
                    .await?
            }
            "file" => {
                self.prune_planner
                    .plan_file_delete(&delete_params, context)
                    .await?
            }
            "directory" => {
                self.prune_planner
                    .plan_directory_delete(&delete_params, context)
                    .await?
            }
            _ => {
                return Err(ServerError::invalid_request(format!(
                    "Unknown target kind: '{}'. Expected 'file', 'directory', or 'symbol'",
                    params.target.kind
                )));
            }
        };

        if Self::perf_enabled() {
            tracing::info!(
                target_kind = %params.target.kind,
                target_path = %params.target.file_path,
                planning_ms = planning_start.elapsed().as_millis(),
                "perf: prune_planning"
            );
        }

        // Wrap in RefactorPlan
        let refactor_plan = RefactorPlan::DeletePlan(plan.clone());

        // Handle dry run vs execution
        if params.options.dry_run {
            self.build_preview_response(&plan, &params)
        } else {
            self.execute_and_build_response(context, refactor_plan, &params)
                .await
        }
    }
}

impl PruneHandler {
    fn perf_enabled() -> bool {
        std::env::var("TYPEMILL_PERF")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false)
    }

    /// Build preview response from DeletePlan
    fn build_preview_response(
        &self,
        plan: &mill_foundation::planning::DeletePlan,
        params: &PruneParams,
    ) -> ServerResult<Value> {
        // Collect affected files
        let mut files_changed = HashSet::new();

        for deletion in &plan.deletions {
            files_changed.insert(deletion.path.clone());
        }

        if let Some(edits) = &plan.edits {
            if let Some(changes) = &edits.changes {
                for file_path in changes.keys() {
                    let path_str = file_path.to_string();
                    files_changed.insert(path_str);
                }
            }
            if let Some(doc_changes) = &edits.document_changes {
                Self::extract_files_from_doc_changes(doc_changes, &mut files_changed);
            }
        }

        // Convert to sorted list for deterministic output
        let mut files_list: Vec<String> = files_changed.into_iter().collect();
        files_list.sort();

        let summary = format!(
            "Preview: {} {} deletion affecting {} file(s)",
            params.target.kind,
            params.target.file_path,
            files_list.len()
        );

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

        let plan_json = serde_json::to_value(RefactorPlan::DeletePlan(plan.clone()))
            .map_err(|e| ServerError::internal(format!("Failed to serialize plan: {}", e)))?;

        let mut response = WriteResponse::preview(summary, files_list, plan_json);
        response.diagnostics = diagnostics;

        serde_json::to_value(&response)
            .map(|v| serde_json::json!({ "content": v }))
            .map_err(|e| ServerError::internal(format!("Failed to serialize response: {}", e)))
    }

    /// Execute plan and build response
    async fn execute_and_build_response(
        &self,
        context: &mill_handler_api::ToolHandlerContext,
        plan: RefactorPlan,
        params: &PruneParams,
    ) -> ServerResult<Value> {
        let execute_start = std::time::Instant::now();
        let result = crate::handlers::common::execute_refactor_plan(context, plan).await?;
        let execute_ms = execute_start.elapsed().as_millis();

        if Self::perf_enabled() {
            tracing::info!(
                target_kind = %params.target.kind,
                target_path = %params.target.file_path,
                execute_ms,
                applied_files = result.applied_files.len(),
                "perf: prune_execute"
            );
        }

        let summary = if result.success {
            format!(
                "Successfully deleted {} {} and updated {} file(s)",
                params.target.kind,
                params.target.file_path,
                result.applied_files.len()
            )
        } else {
            format!(
                "Failed to delete {} {}",
                params.target.kind, params.target.file_path
            )
        };

        let diagnostics: Vec<Diagnostic> = result
            .warnings
            .iter()
            .map(|w| Diagnostic {
                severity: DiagnosticSeverity::Warning,
                message: w.clone(),
                file_path: None,
                line: None,
            })
            .collect();

        let response = if result.success {
            let mut r = WriteResponse::success(summary, result.applied_files);
            r.diagnostics = diagnostics;
            r
        } else {
            WriteResponse::error(summary, diagnostics)
        };

        serde_json::to_value(&response)
            .map(|v| serde_json::json!({ "content": v }))
            .map_err(|e| ServerError::internal(format!("Failed to serialize response: {}", e)))
    }

    /// Extract file paths from DocumentChanges
    fn extract_files_from_doc_changes(
        doc_changes: &lsp_types::DocumentChanges,
        files: &mut HashSet<String>,
    ) {
        match doc_changes {
            lsp_types::DocumentChanges::Edits(edits_list) => {
                for edit in edits_list {
                    let uri = edit.text_document.uri.to_string();
                    files.insert(uri);
                }
            }
            lsp_types::DocumentChanges::Operations(ops) => {
                for op in ops {
                    match op {
                        lsp_types::DocumentChangeOperation::Edit(edit) => {
                            let uri = edit.text_document.uri.to_string();
                            files.insert(uri);
                        }
                        lsp_types::DocumentChangeOperation::Op(resource_op) => match resource_op {
                            lsp_types::ResourceOp::Create(c) => {
                                let uri = c.uri.to_string();
                                files.insert(uri);
                            }
                            lsp_types::ResourceOp::Rename(r) => {
                                let old = r.old_uri.to_string();
                                let new = r.new_uri.to_string();
                                files.insert(old);
                                files.insert(new);
                            }
                            lsp_types::ResourceOp::Delete(d) => {
                                let uri = d.uri.to_string();
                                files.insert(uri);
                            }
                        },
                    }
                }
            }
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
    fn test_prune_options_default() {
        let options = PruneOptionsInput::default();
        assert!(options.dry_run);
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
        assert!(params.options.dry_run);
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
        assert!(!params.options.dry_run);
        assert_eq!(params.options.force, Some(true));
    }

    #[test]
    fn test_extract_files_from_doc_changes_duplicates() {
        use lsp_types::{
            DocumentChangeOperation, DocumentChanges, OneOf,
            OptionalVersionedTextDocumentIdentifier, TextDocumentEdit, Uri,
        };
        use std::str::FromStr;

        let uri1 = Uri::from_str("file:///tmp/file1.rs").unwrap();
        let uri2 = Uri::from_str("file:///tmp/file2.rs").unwrap();

        // Create edits for file1 twice
        let edit1 = TextDocumentEdit {
            text_document: OptionalVersionedTextDocumentIdentifier {
                uri: uri1.clone(),
                version: None,
            },
            edits: vec![OneOf::Left(lsp_types::TextEdit {
                range: lsp_types::Range::default(),
                new_text: "change1".to_string(),
            })],
        };

        let edit2 = TextDocumentEdit {
            text_document: OptionalVersionedTextDocumentIdentifier {
                uri: uri1.clone(), // Duplicate URI
                version: None,
            },
            edits: vec![OneOf::Left(lsp_types::TextEdit {
                range: lsp_types::Range::default(),
                new_text: "change2".to_string(),
            })],
        };

        // Create edit for file2
        let edit3 = TextDocumentEdit {
            text_document: OptionalVersionedTextDocumentIdentifier {
                uri: uri2.clone(),
                version: None,
            },
            edits: vec![OneOf::Left(lsp_types::TextEdit {
                range: lsp_types::Range::default(),
                new_text: "change3".to_string(),
            })],
        };

        // Construct DocumentChanges
        let doc_changes = DocumentChanges::Operations(vec![
            DocumentChangeOperation::Edit(edit1),
            DocumentChangeOperation::Edit(edit2),
            DocumentChangeOperation::Edit(edit3),
        ]);

        let mut files = HashSet::new();
        PruneHandler::extract_files_from_doc_changes(&doc_changes, &mut files);

        assert_eq!(files.len(), 2);
        assert!(files.contains(&uri1.to_string()));
        assert!(files.contains(&uri2.to_string()));
    }
}
