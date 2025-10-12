//! Rename handler for Unified Refactoring API
//!
//! Implements `rename.plan` command for:
//! - Symbol renaming (via LSP)
//! - File renaming (via FileService)
//! - Directory renaming (via FileService)

use crate::handlers::tools::{ToolHandler, ToolHandlerContext};
use async_trait::async_trait;
use cb_core::model::mcp::ToolCall;
use cb_protocol::{
    refactor_plan::{PlanMetadata, PlanSummary, PlanWarning, RenamePlan},
    ApiError as ServerError, ApiResult as ServerResult, RefactorPlan,
};
use lsp_types::{Position, WorkspaceEdit};
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use tracing::{debug, error, info};

/// Handler for rename.plan operations
pub struct RenameHandler;

impl RenameHandler {
    pub fn new() -> Self {
        Self
    }
}

impl Default for RenameHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Deserialize)]
struct RenamePlanParams {
    target: RenameTarget,
    new_name: String,
    #[serde(default)]
    options: RenameOptions,
}

#[derive(Debug, Deserialize)]
struct RenameTarget {
    kind: String, // "symbol" | "file" | "directory"
    path: String,
    #[serde(default)]
    selector: Option<SymbolSelector>,
}

#[derive(Debug, Deserialize)]
struct SymbolSelector {
    position: Position,
}

#[derive(Debug, Deserialize, Default)]
struct RenameOptions {
    #[serde(default)]
    strict: Option<bool>,
    #[serde(default)]
    validate_scope: Option<bool>,
    #[serde(default)]
    update_imports: Option<bool>,
}

#[async_trait]
impl ToolHandler for RenameHandler {
    fn tool_names(&self) -> &[&str] {
        &["rename.plan"]
    }

    fn is_internal(&self) -> bool {
        false // Public tool
    }

    async fn handle_tool_call(
        &self,
        context: &ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        info!(tool_name = %tool_call.name, "Handling rename.plan");

        // Parse parameters
        let args = tool_call.arguments.clone().ok_or_else(|| {
            ServerError::InvalidRequest("Missing arguments for rename.plan".into())
        })?;

        let params: RenamePlanParams = serde_json::from_value(args).map_err(|e| {
            ServerError::InvalidRequest(format!("Invalid rename.plan parameters: {}", e))
        })?;

        debug!(
            kind = %params.target.kind,
            path = %params.target.path,
            new_name = %params.new_name,
            "Generating rename plan"
        );

        // Dispatch based on target kind
        let plan = match params.target.kind.as_str() {
            "symbol" => self.plan_symbol_rename(&params, context).await?,
            "file" => self.plan_file_rename(&params, context).await?,
            "directory" => self.plan_directory_rename(&params, context).await?,
            kind => {
                return Err(ServerError::InvalidRequest(format!(
                    "Unsupported rename kind: {}. Must be one of: symbol, file, directory",
                    kind
                )));
            }
        };

        // Wrap in RefactorPlan enum for discriminant, then serialize for MCP protocol
        let refactor_plan = RefactorPlan::RenamePlan(plan);
        let plan_json = serde_json::to_value(&refactor_plan).map_err(|e| {
            ServerError::Internal(format!("Failed to serialize rename plan: {}", e))
        })?;

        Ok(json!({
            "content": plan_json
        }))
    }
}

impl RenameHandler {
    /// Generate plan for symbol rename using LSP
    async fn plan_symbol_rename(
        &self,
        params: &RenamePlanParams,
        context: &ToolHandlerContext,
    ) -> ServerResult<RenamePlan> {
        debug!(path = %params.target.path, "Planning symbol rename via LSP");

        // Extract position from selector
        let position = params
            .target
            .selector
            .as_ref()
            .ok_or_else(|| {
                ServerError::InvalidRequest("Symbol rename requires selector.position".into())
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
        let abs_path = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
        let file_uri = url::Url::from_file_path(&abs_path)
            .map_err(|_| {
                ServerError::Internal(format!("Invalid file path: {}", abs_path.display()))
            })?
            .to_string();

        // Build LSP rename request
        let lsp_params = json!({
            "textDocument": {
                "uri": file_uri
            },
            "position": position,
            "newName": params.new_name
        });

        // Send textDocument/rename request to LSP
        debug!(method = "textDocument/rename", "Sending LSP request");
        let lsp_result = client
            .send_request("textDocument/rename", lsp_params)
            .await
            .map_err(|e| {
                error!(error = %e, "LSP rename request failed");
                ServerError::Internal(format!("LSP rename failed: {}", e))
            })?;

        // Parse WorkspaceEdit from LSP response
        let workspace_edit: WorkspaceEdit = serde_json::from_value(lsp_result).map_err(|e| {
            ServerError::Internal(format!("Failed to parse LSP WorkspaceEdit: {}", e))
        })?;

        // Calculate file checksums and summary
        let (file_checksums, summary, warnings) = self
            .analyze_workspace_edit(&workspace_edit, context)
            .await?;

        // Determine language from extension
        let language = self.extension_to_language(extension);

        // Build metadata
        let metadata = PlanMetadata {
            plan_version: "1.0".to_string(),
            kind: "rename".to_string(),
            language,
            estimated_impact: self.estimate_impact(summary.affected_files),
            created_at: chrono::Utc::now().to_rfc3339(),
        };

        Ok(RenamePlan {
            edits: workspace_edit,
            summary,
            warnings,
            metadata,
            file_checksums,
        })
    }

    /// Generate plan for file rename using FileService
    async fn plan_file_rename(
        &self,
        params: &RenamePlanParams,
        context: &ToolHandlerContext,
    ) -> ServerResult<RenamePlan> {
        debug!(
            old_path = %params.target.path,
            new_path = %params.new_name,
            "Planning file rename"
        );

        let old_path = Path::new(&params.target.path);
        let new_path = Path::new(&params.new_name);

        // Use FileService to generate dry-run plan for file rename
        let _dry_run_result = context
            .app_state
            .file_service
            .rename_file_with_imports(old_path, new_path, true, None)
            .await?;

        // Extract edit plan from dry-run result
        // Note: FileService returns ServerResult<Value> for dry runs
        // For now, we'll create a minimal WorkspaceEdit representing the file move
        let abs_old = std::fs::canonicalize(old_path).unwrap_or_else(|_| old_path.to_path_buf());
        let _abs_new = new_path.to_path_buf();

        // Read file content for checksum
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

        // Create WorkspaceEdit representing file rename using LSP ResourceOp
        use lsp_types::{DocumentChangeOperation, DocumentChanges, RenameFile, ResourceOp, Uri};

        let old_url = url::Url::from_file_path(&abs_old).map_err(|_| {
            ServerError::Internal(format!("Invalid old path: {}", abs_old.display()))
        })?;

        let old_uri: Uri = old_url
            .as_str()
            .parse()
            .map_err(|e| ServerError::Internal(format!("Failed to parse URI: {}", e)))?;

        let abs_new = std::fs::canonicalize(new_path.parent().unwrap_or(Path::new(".")))
            .unwrap_or_else(|_| new_path.parent().unwrap_or(Path::new(".")).to_path_buf())
            .join(new_path.file_name().unwrap_or(new_path.as_os_str()));

        let new_url = url::Url::from_file_path(&abs_new).map_err(|_| {
            ServerError::Internal(format!("Invalid new path: {}", abs_new.display()))
        })?;

        let new_uri: Uri = new_url
            .as_str()
            .parse()
            .map_err(|e| ServerError::Internal(format!("Failed to parse URI: {}", e)))?;

        let rename_op = ResourceOp::Rename(RenameFile {
            old_uri,
            new_uri,
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

        // No warnings for simple file rename
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
            kind: "rename".to_string(),
            language,
            estimated_impact: "low".to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
        };

        Ok(RenamePlan {
            edits: workspace_edit,
            summary,
            warnings,
            metadata,
            file_checksums,
        })
    }

    /// Generate plan for directory rename using FileService
    async fn plan_directory_rename(
        &self,
        params: &RenamePlanParams,
        context: &ToolHandlerContext,
    ) -> ServerResult<RenamePlan> {
        debug!(
            old_path = %params.target.path,
            new_path = %params.new_name,
            "Planning directory rename"
        );

        let old_path = Path::new(&params.target.path);
        let new_path = Path::new(&params.new_name);

        // Use FileService to generate dry-run plan for directory rename
        let dry_run_result = context
            .app_state
            .file_service
            .rename_directory_with_imports(old_path, new_path, true, false, None)
            .await?;

        // Extract metadata from dry-run result
        // Note: dry_run_result is DryRunnable<Value>
        let files_to_move = dry_run_result
            .result
            .get("files_to_move")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize;

        // For directory rename, we need to calculate checksums for all files being moved
        let abs_old = std::fs::canonicalize(old_path).unwrap_or_else(|_| old_path.to_path_buf());
        let mut file_checksums = HashMap::new();

        // Walk directory to collect files and calculate checksums
        let walker = ignore::WalkBuilder::new(&abs_old).hidden(false).build();
        for entry in walker.flatten() {
            if entry.path().is_file() {
                if let Ok(content) = context.app_state.file_service.read_file(entry.path()).await {
                    file_checksums.insert(
                        entry.path().to_string_lossy().to_string(),
                        calculate_checksum(&content),
                    );
                }
            }
        }

        // Create WorkspaceEdit representing directory rename using LSP ResourceOp
        use lsp_types::{DocumentChangeOperation, DocumentChanges, RenameFile, ResourceOp, Uri};

        let old_url = url::Url::from_file_path(&abs_old).map_err(|_| {
            ServerError::Internal(format!("Invalid old path: {}", abs_old.display()))
        })?;

        let old_uri: Uri = old_url
            .as_str()
            .parse()
            .map_err(|e| ServerError::Internal(format!("Failed to parse URI: {}", e)))?;

        let abs_new = std::fs::canonicalize(new_path.parent().unwrap_or(Path::new(".")))
            .unwrap_or_else(|_| new_path.parent().unwrap_or(Path::new(".")).to_path_buf())
            .join(new_path.file_name().unwrap_or(new_path.as_os_str()));

        let new_url = url::Url::from_file_path(&abs_new).map_err(|_| {
            ServerError::Internal(format!("Invalid new path: {}", abs_new.display()))
        })?;

        let new_uri: Uri = new_url
            .as_str()
            .parse()
            .map_err(|e| ServerError::Internal(format!("Failed to parse URI: {}", e)))?;

        let rename_op = ResourceOp::Rename(RenameFile {
            old_uri,
            new_uri,
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
            affected_files: files_to_move,
            created_files: files_to_move,
            deleted_files: files_to_move,
        };

        // Add warning if this is a Cargo package
        let mut warnings = Vec::new();
        if dry_run_result
            .result
            .get("is_cargo_package")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            warnings.push(PlanWarning {
                code: "CARGO_PACKAGE_RENAME".to_string(),
                message: "Renaming a Cargo package will update workspace members and dependencies"
                    .to_string(),
                candidates: None,
            });
        }

        // Build metadata
        let metadata = PlanMetadata {
            plan_version: "1.0".to_string(),
            kind: "rename".to_string(),
            language: "rust".to_string(), // Assume Rust for directory renames with Cargo
            estimated_impact: self.estimate_impact(files_to_move),
            created_at: chrono::Utc::now().to_rfc3339(),
        };

        Ok(RenamePlan {
            edits: workspace_edit,
            summary,
            warnings,
            metadata,
            file_checksums,
        })
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
