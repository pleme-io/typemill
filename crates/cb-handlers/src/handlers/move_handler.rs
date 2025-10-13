//! Move handler for Unified Refactoring API
//!
//! Implements `move.plan` command for:
//! - Symbol moving (via LSP if available, else AST fallback)
//! - File moving (via FileService)
//! - Module moving (via AST)

use crate::handlers::tools::{ToolHandler, ToolHandlerContext};
use async_trait::async_trait;
use cb_core::model::mcp::ToolCall;
use cb_protocol::{
    refactor_plan::{MovePlan, PlanMetadata, PlanSummary, PlanWarning},
    ApiError as ServerError, ApiResult as ServerResult, RefactorPlan,
};
use lsp_types::{Position, WorkspaceEdit};
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use tracing::{debug, error, info};

/// Handler for move.plan operations
pub struct MoveHandler;

impl MoveHandler {
    pub fn new() -> Self {
        Self
    }
}

impl Default for MoveHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)] // Reserved for future options support
struct MovePlanParams {
    target: MoveTarget,
    destination: String,
    #[serde(default)]
    options: MoveOptions,
}

#[derive(Debug, Deserialize)]
struct MoveTarget {
    kind: String, // "symbol" | "file" | "module"
    path: String,
    #[serde(default)]
    selector: Option<SymbolSelector>,
}

#[derive(Debug, Deserialize)]
struct SymbolSelector {
    position: Position,
}

#[derive(Debug, Deserialize, Default)]
#[allow(dead_code)] // Reserved for future configuration
struct MoveOptions {
    #[serde(default)]
    update_imports: Option<bool>,
    #[serde(default)]
    preserve_formatting: Option<bool>,
}

#[async_trait]
impl ToolHandler for MoveHandler {
    fn tool_names(&self) -> &[&str] {
        &["move.plan"]
    }

    fn is_internal(&self) -> bool {
        false // Public tool
    }

    async fn handle_tool_call(
        &self,
        context: &ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        info!(tool_name = %tool_call.name, "Handling move.plan");

        // Parse parameters
        let args = tool_call
            .arguments
            .clone()
            .ok_or_else(|| ServerError::InvalidRequest("Missing arguments for move.plan".into()))?;

        let params: MovePlanParams = serde_json::from_value(args).map_err(|e| {
            ServerError::InvalidRequest(format!("Invalid move.plan parameters: {}", e))
        })?;

        debug!(
            kind = %params.target.kind,
            path = %params.target.path,
            destination = %params.destination,
            "Generating move plan"
        );

        // Dispatch based on target kind
        let plan = match params.target.kind.as_str() {
            "symbol" => self.plan_symbol_move(&params, context).await?,
            "file" => self.plan_file_move(&params, context).await?,
            "module" => self.plan_module_move(&params, context).await?,
            kind => {
                return Err(ServerError::InvalidRequest(format!(
                    "Unsupported move kind: {}. Must be one of: symbol, file, module",
                    kind
                )));
            }
        };

        // Wrap in RefactorPlan enum for discriminant, then serialize for MCP protocol
        let refactor_plan = RefactorPlan::MovePlan(plan);
        let plan_json = serde_json::to_value(&refactor_plan)
            .map_err(|e| ServerError::Internal(format!("Failed to serialize move plan: {}", e)))?;

        Ok(json!({
            "content": plan_json
        }))
    }
}

impl MoveHandler {
    /// Generate plan for symbol move using LSP or AST fallback
    async fn plan_symbol_move(
        &self,
        params: &MovePlanParams,
        context: &ToolHandlerContext,
    ) -> ServerResult<MovePlan> {
        debug!(path = %params.target.path, "Planning symbol move");

        // Extract position from selector
        let position = params
            .target
            .selector
            .as_ref()
            .ok_or_else(|| {
                ServerError::InvalidRequest("Symbol move requires selector.position".into())
            })?
            .position;

        // Get file extension to determine LSP client
        let path = Path::new(&params.target.path);
        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .ok_or_else(|| {
                ServerError::InvalidRequest(format!(
                    "File has no extension: {}",
                    params.target.path
                ))
            })?;

        // Try LSP approach first
        let lsp_result = self
            .try_lsp_symbol_move(params, context, extension, position)
            .await;

        match lsp_result {
            Ok(plan) => Ok(plan),
            Err(e) => {
                // LSP failed, try AST fallback
                debug!(error = %e, "LSP symbol move failed, attempting AST fallback");
                self.ast_symbol_move_fallback(params, context).await
            }
        }
    }

    /// Try to move symbol using LSP
    async fn try_lsp_symbol_move(
        &self,
        params: &MovePlanParams,
        context: &ToolHandlerContext,
        extension: &str,
        position: Position,
    ) -> ServerResult<MovePlan> {
        // Get LSP adapter
        let lsp_adapter = context.lsp_adapter.lock().await;
        let adapter = lsp_adapter
            .as_ref()
            .ok_or_else(|| ServerError::Internal("LSP adapter not initialized".into()))?;

        // Get or create LSP client for this extension
        let client = adapter.get_or_create_client(extension).await.map_err(|e| {
            ServerError::Unsupported(format!(
                "No LSP server configured for extension {}: {}",
                extension, e
            ))
        })?;

        // Convert path to absolute and create file URI
        let path = Path::new(&params.target.path);
        let abs_path = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
        let file_uri = url::Url::from_file_path(&abs_path)
            .map_err(|_| {
                ServerError::Internal(format!("Invalid file path: {}", abs_path.display()))
            })?
            .to_string();

        // Build LSP code action request for move refactoring
        let lsp_params = json!({
            "textDocument": {
                "uri": file_uri
            },
            "range": {
                "start": position,
                "end": position
            },
            "context": {
                "diagnostics": [],
                "only": ["refactor.move"]
            }
        });

        // Send textDocument/codeAction request to LSP
        debug!(method = "textDocument/codeAction", "Sending LSP request");
        let lsp_result = client
            .send_request("textDocument/codeAction", lsp_params)
            .await
            .map_err(|e| {
                error!(error = %e, "LSP move request failed");
                ServerError::Internal(format!("LSP move failed: {}", e))
            })?;

        // Parse code actions from response
        let code_actions: Vec<Value> = serde_json::from_value(lsp_result).map_err(|e| {
            ServerError::Internal(format!("Failed to parse LSP code actions: {}", e))
        })?;

        // Find the appropriate move action
        let move_action = code_actions
            .into_iter()
            .find(|action| {
                action
                    .get("kind")
                    .and_then(|k| k.as_str())
                    .map(|k| k.starts_with("refactor.move"))
                    .unwrap_or(false)
            })
            .ok_or_else(|| {
                ServerError::Unsupported("No move code action available from LSP".into())
            })?;

        // Extract WorkspaceEdit from code action
        let workspace_edit: WorkspaceEdit = serde_json::from_value(
            move_action
                .get("edit")
                .cloned()
                .ok_or_else(|| ServerError::Internal("Code action missing edit field".into()))?,
        )
        .map_err(|e| ServerError::Internal(format!("Failed to parse WorkspaceEdit: {}", e)))?;

        // Calculate file checksums and summary
        let (file_checksums, summary, warnings) = self
            .analyze_workspace_edit(&workspace_edit, context)
            .await?;

        // Determine language from extension
        let language = self.extension_to_language(extension);

        // Build metadata
        let metadata = PlanMetadata {
            plan_version: "1.0".to_string(),
            kind: "move".to_string(),
            language,
            estimated_impact: self.estimate_impact(summary.affected_files),
            created_at: chrono::Utc::now().to_rfc3339(),
        };

        Ok(MovePlan {
            edits: workspace_edit,
            summary,
            warnings,
            metadata,
            file_checksums,
        })
    }

    /// AST-based fallback for symbol move
    async fn ast_symbol_move_fallback(
        &self,
        _params: &MovePlanParams,
        _context: &ToolHandlerContext,
    ) -> ServerResult<MovePlan> {
        // For now, return unsupported error
        // Full AST-based symbol move would require extensive analysis
        Err(ServerError::Unsupported(
            "AST-based symbol move not yet implemented. LSP server required.".into(),
        ))
    }

    /// Generate plan for file move using FileService
    async fn plan_file_move(
        &self,
        params: &MovePlanParams,
        context: &ToolHandlerContext,
    ) -> ServerResult<MovePlan> {
        debug!(
            old_path = %params.target.path,
            new_path = %params.destination,
            "Planning file move"
        );

        let old_path = Path::new(&params.target.path);
        let new_path = Path::new(&params.destination);

        // Use FileService to generate dry-run plan for file move
        // Note: rename_file_with_imports handles import updates
        let _dry_run_result = context
            .app_state
            .file_service
            .rename_file_with_imports(old_path, new_path, true, None)
            .await?;

        // Read file content for checksum
        let abs_old = std::fs::canonicalize(old_path).unwrap_or_else(|_| old_path.to_path_buf());

        let content = context
            .app_state
            .file_service
            .read_file(&abs_old)
            .await
            .map_err(|e| {
                ServerError::Internal(format!("Failed to read file for checksum: {}", e))
            })?;

        // Calculate checksum
        let mut file_checksums = HashMap::new();
        file_checksums.insert(
            abs_old.to_string_lossy().to_string(),
            calculate_checksum(&content),
        );

        // Create WorkspaceEdit representing file move using LSP ResourceOp::Rename
        use lsp_types::{DocumentChangeOperation, DocumentChanges, RenameFile, ResourceOp, Uri};

        let abs_new = std::fs::canonicalize(new_path.parent().unwrap_or(Path::new(".")))
            .unwrap_or_else(|_| new_path.parent().unwrap_or(Path::new(".")).to_path_buf())
            .join(new_path.file_name().unwrap_or(std::ffi::OsStr::new("file")));

        let old_uri = url::Url::from_file_path(&abs_old)
            .map_err(|_| ServerError::InvalidRequest("Invalid source file path".into()))?;
        let new_uri = url::Url::from_file_path(&abs_new)
            .map_err(|_| ServerError::InvalidRequest("Invalid destination file path".into()))?;

        let rename_op =
            ResourceOp::Rename(RenameFile {
                old_uri: old_uri.as_str().parse().map_err(|e| {
                    ServerError::Internal(format!("Failed to parse old URI: {}", e))
                })?,
                new_uri: new_uri.as_str().parse().map_err(|e| {
                    ServerError::Internal(format!("Failed to parse new URI: {}", e))
                })?,
                options: None,
                annotation_id: None,
            });

        let workspace_edit = WorkspaceEdit {
            changes: None,
            document_changes: Some(DocumentChanges::Operations(vec![
                DocumentChangeOperation::Op(rename_op),
            ])),
            change_annotations: None,
        };

        // Build summary
        let summary = PlanSummary {
            affected_files: 1,
            created_files: 1,
            deleted_files: 1,
        };

        // No warnings for simple file move
        let warnings = Vec::new();

        // Determine language from extension
        let extension = old_path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("unknown");
        let language = self.extension_to_language(extension);

        // Build metadata
        let metadata = PlanMetadata {
            plan_version: "1.0".to_string(),
            kind: "move".to_string(),
            language,
            estimated_impact: "low".to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
        };

        Ok(MovePlan {
            edits: workspace_edit,
            summary,
            warnings,
            metadata,
            file_checksums,
        })
    }

    /// Generate plan for module move
    async fn plan_module_move(
        &self,
        _params: &MovePlanParams,
        _context: &ToolHandlerContext,
    ) -> ServerResult<MovePlan> {
        // Module move is complex and requires language-specific support
        // Would use extract_module_to_package or similar AST functions
        Err(ServerError::Unsupported(
            "Module move not yet implemented. Requires language plugin support.".into(),
        ))
    }

    /// Analyze WorkspaceEdit to calculate checksums and summary
    async fn analyze_workspace_edit(
        &self,
        edit: &WorkspaceEdit,
        context: &ToolHandlerContext,
    ) -> ServerResult<(HashMap<String, String>, PlanSummary, Vec<PlanWarning>)> {
        let mut file_checksums = HashMap::new();
        let mut affected_files: HashSet<std::path::PathBuf> = HashSet::new();

        // Extract file paths from WorkspaceEdit
        if let Some(ref changes) = edit.changes {
            for (uri, _edits) in changes {
                let path = std::path::PathBuf::from(uri.path().as_str());
                affected_files.insert(path);
            }
        }

        if let Some(ref document_changes) = edit.document_changes {
            match document_changes {
                lsp_types::DocumentChanges::Edits(edits) => {
                    for edit in edits {
                        let path = std::path::PathBuf::from(edit.text_document.uri.path().as_str());
                        affected_files.insert(path);
                    }
                }
                lsp_types::DocumentChanges::Operations(ops) => {
                    for op in ops {
                        match op {
                            lsp_types::DocumentChangeOperation::Edit(edit) => {
                                let path = std::path::PathBuf::from(
                                    edit.text_document.uri.path().as_str(),
                                );
                                affected_files.insert(path);
                            }
                            lsp_types::DocumentChangeOperation::Op(resource_op) => {
                                match resource_op {
                                    lsp_types::ResourceOp::Create(create) => {
                                        let path =
                                            std::path::PathBuf::from(create.uri.path().as_str());
                                        affected_files.insert(path);
                                    }
                                    lsp_types::ResourceOp::Rename(rename) => {
                                        let path = std::path::PathBuf::from(
                                            rename.old_uri.path().as_str(),
                                        );
                                        affected_files.insert(path);
                                        let path = std::path::PathBuf::from(
                                            rename.new_uri.path().as_str(),
                                        );
                                        affected_files.insert(path);
                                    }
                                    lsp_types::ResourceOp::Delete(delete) => {
                                        let path =
                                            std::path::PathBuf::from(delete.uri.path().as_str());
                                        affected_files.insert(path);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Calculate checksums for all affected files
        for file_path in &affected_files {
            if file_path.exists() {
                if let Ok(content) = context.app_state.file_service.read_file(file_path).await {
                    file_checksums.insert(
                        file_path.to_string_lossy().to_string(),
                        calculate_checksum(&content),
                    );
                }
            }
        }

        let summary = PlanSummary {
            affected_files: affected_files.len(),
            created_files: 0,
            deleted_files: 0,
        };

        let warnings = Vec::new();

        Ok((file_checksums, summary, warnings))
    }

    /// Map file extension to language name
    fn extension_to_language(&self, extension: &str) -> String {
        match extension {
            "rs" => "rust",
            "ts" | "tsx" => "typescript",
            "js" | "jsx" => "javascript",
            "py" | "pyi" => "python",
            "go" => "go",
            "java" => "java",
            "swift" => "swift",
            "cs" => "csharp",
            _ => "unknown",
        }
        .to_string()
    }

    /// Estimate impact based on number of affected files
    fn estimate_impact(&self, affected_files: usize) -> String {
        if affected_files <= 3 {
            "low"
        } else if affected_files <= 10 {
            "medium"
        } else {
            "high"
        }
        .to_string()
    }
}

/// Calculate SHA-256 checksum of file content
fn calculate_checksum(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}
