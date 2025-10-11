//! Delete handler for Unified Refactoring API
//!
//! Implements `delete.plan` command for:
//! - Symbol deletion (AST-based - placeholder)
//! - File deletion (via FileService)
//! - Directory deletion (via FileService)
//! - Dead code deletion (batch operation - placeholder)

use crate::handlers::tools::{ToolHandler, ToolHandlerContext};
use async_trait::async_trait;
use cb_core::model::mcp::ToolCall;
use cb_protocol::{
    refactor_plan::{DeletePlan, DeletionTarget, PlanMetadata, PlanSummary, PlanWarning},
    ApiError as ServerError, ApiResult as ServerResult, RefactorPlan,
};
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::Path;
use tracing::{debug, error, info};

/// Handler for delete.plan operations
pub struct DeleteHandler;

impl DeleteHandler {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DeleteHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Deserialize)]
struct DeletePlanParams {
    target: DeleteTarget,
    #[serde(default)]
    options: DeleteOptions,
}

#[derive(Debug, Deserialize)]
struct DeleteTarget {
    kind: String, // "symbol" | "file" | "directory" | "dead_code"
    path: String,
    #[serde(default)]
    selector: Option<DeleteSelector>,
}

#[derive(Debug, Deserialize)]
struct DeleteSelector {
    line: u32,
    character: u32,
    #[serde(default)]
    symbol_name: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct DeleteOptions {
    #[serde(default)]
    cleanup_imports: Option<bool>,
    #[serde(default)]
    remove_tests: Option<bool>,
    #[serde(default)]
    force: Option<bool>,
}

#[async_trait]
impl ToolHandler for DeleteHandler {
    fn tool_names(&self) -> &[&str] {
        &["delete.plan"]
    }

    fn is_internal(&self) -> bool {
        false // Public tool
    }

    async fn handle_tool_call(
        &self,
        context: &ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        info!(tool_name = %tool_call.name, "Handling delete.plan");

        // Parse parameters
        let args = tool_call.arguments.clone().ok_or_else(|| {
            ServerError::InvalidRequest("Missing arguments for delete.plan".into())
        })?;

        let params: DeletePlanParams = serde_json::from_value(args).map_err(|e| {
            ServerError::InvalidRequest(format!("Invalid delete.plan parameters: {}", e))
        })?;

        debug!(
            kind = %params.target.kind,
            path = %params.target.path,
            "Generating delete plan"
        );

        // Dispatch based on target kind
        let plan = match params.target.kind.as_str() {
            "symbol" => self.plan_symbol_delete(&params, context).await?,
            "file" => self.plan_file_delete(&params, context).await?,
            "directory" => self.plan_directory_delete(&params, context).await?,
            "dead_code" => self.plan_dead_code_delete(&params, context).await?,
            kind => {
                return Err(ServerError::InvalidRequest(format!(
                    "Unsupported delete kind: {}. Must be one of: symbol, file, directory, dead_code",
                    kind
                )));
            }
        };

        // Wrap in RefactorPlan enum for discriminant, then serialize for MCP protocol
        let refactor_plan = RefactorPlan::DeletePlan(plan);
        let plan_json = serde_json::to_value(&refactor_plan).map_err(|e| {
            ServerError::Internal(format!("Failed to serialize delete plan: {}", e))
        })?;

        Ok(serde_json::json!({
            "content": plan_json
        }))
    }
}

impl DeleteHandler {
    /// Generate plan for symbol deletion using AST (placeholder)
    async fn plan_symbol_delete(
        &self,
        params: &DeletePlanParams,
        context: &ToolHandlerContext,
    ) -> ServerResult<DeletePlan> {
        debug!(path = %params.target.path, "Planning symbol delete (placeholder)");

        let file_path = Path::new(&params.target.path);

        // Validate selector is provided
        let _selector = params.target.selector.as_ref().ok_or_else(|| {
            ServerError::InvalidRequest("Symbol delete requires selector with line/character".into())
        })?;

        // Read file content for checksum
        let content = context
            .app_state
            .file_service
            .read_file(file_path)
            .await
            .map_err(|e| {
                error!(error = %e, file_path = %params.target.path, "Failed to read file");
                ServerError::Internal(format!("Failed to read file for checksum: {}", e))
            })?;

        // Calculate checksum
        let mut file_checksums = HashMap::new();
        file_checksums.insert(
            file_path.to_string_lossy().to_string(),
            calculate_checksum(&content),
        );

        // Create empty deletions list (placeholder - AST-based symbol deletion not implemented)
        let deletions = Vec::new();

        // Build summary
        let summary = PlanSummary {
            affected_files: 1,
            created_files: 0,
            deleted_files: 0,
        };

        // Add placeholder warning
        let warnings = vec![PlanWarning {
            code: "SYMBOL_DELETE_NOT_IMPLEMENTED".to_string(),
            message: "Symbol deletion requires AST-based implementation (not yet available)"
                .to_string(),
            candidates: None,
        }];

        // Determine language from extension
        let language = self.detect_language(file_path);

        // Build metadata
        let metadata = PlanMetadata {
            plan_version: "1.0".to_string(),
            kind: "delete".to_string(),
            language,
            estimated_impact: "low".to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
        };

        Ok(DeletePlan {
            deletions,
            summary,
            warnings,
            metadata,
            file_checksums,
        })
    }

    /// Generate plan for file deletion using FileService
    async fn plan_file_delete(
        &self,
        params: &DeletePlanParams,
        context: &ToolHandlerContext,
    ) -> ServerResult<DeletePlan> {
        debug!(
            path = %params.target.path,
            "Planning file delete"
        );

        let file_path = Path::new(&params.target.path);
        let force = params.options.force.unwrap_or(false);

        // Use FileService to generate dry-run plan for file deletion
        let dry_run_result = context
            .app_state
            .file_service
            .delete_file(file_path, force, true)
            .await?;

        // Read file content for checksum before deletion
        let content = context
            .app_state
            .file_service
            .read_file(file_path)
            .await
            .map_err(|e| {
                error!(error = %e, file_path = %params.target.path, "Failed to read file");
                ServerError::Internal(format!("Failed to read file for checksum: {}", e))
            })?;

        // Calculate checksum
        let mut file_checksums = HashMap::new();
        file_checksums.insert(
            file_path.to_string_lossy().to_string(),
            calculate_checksum(&content),
        );

        // Canonicalize path to ensure proper path handling
        let abs_file_path = std::fs::canonicalize(file_path)
            .unwrap_or_else(|_| file_path.to_path_buf());

        // Create explicit deletion target
        let deletions = vec![DeletionTarget {
            path: abs_file_path.to_string_lossy().to_string(),
            kind: "file".to_string(),
        }];

        // Build summary
        let summary = PlanSummary {
            affected_files: 1,
            created_files: 0,
            deleted_files: 1,
        };

        // Check if there are affected files (imports to clean up)
        let mut warnings = Vec::new();
        let affected_files_count = dry_run_result
            .result
            .get("affected_files")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        if affected_files_count > 0 && params.options.cleanup_imports.unwrap_or(true) {
            warnings.push(PlanWarning {
                code: "IMPORT_CLEANUP_REQUIRED".to_string(),
                message: format!(
                    "File deletion will clean up imports in {} dependent files",
                    affected_files_count
                ),
                candidates: None,
            });
        }

        // Determine language from extension
        let language = self.detect_language(file_path);

        // Build metadata
        let metadata = PlanMetadata {
            plan_version: "1.0".to_string(),
            kind: "delete".to_string(),
            language,
            estimated_impact: if affected_files_count > 5 {
                "high"
            } else if affected_files_count > 0 {
                "medium"
            } else {
                "low"
            }
            .to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
        };

        Ok(DeletePlan {
            deletions,
            summary,
            warnings,
            metadata,
            file_checksums,
        })
    }

    /// Generate plan for directory deletion using FileService
    async fn plan_directory_delete(
        &self,
        params: &DeletePlanParams,
        context: &ToolHandlerContext,
    ) -> ServerResult<DeletePlan> {
        debug!(
            path = %params.target.path,
            "Planning directory delete"
        );

        let dir_path = Path::new(&params.target.path);

        // Verify it's a directory
        if !dir_path.is_dir() {
            return Err(ServerError::InvalidRequest(format!(
                "Path is not a directory: {}",
                params.target.path
            )));
        }

        // Walk directory to collect files and checksums
        let abs_dir = std::fs::canonicalize(dir_path)
            .unwrap_or_else(|_| dir_path.to_path_buf());
        let mut file_checksums = HashMap::new();
        let mut file_count = 0;

        let walker = ignore::WalkBuilder::new(&abs_dir).hidden(false).build();
        for entry in walker.flatten() {
            if entry.path().is_file() {
                if let Ok(content) = context
                    .app_state
                    .file_service
                    .read_file(entry.path())
                    .await
                {
                    file_checksums.insert(
                        entry.path().to_string_lossy().to_string(),
                        calculate_checksum(&content),
                    );
                    file_count += 1;
                }
            }
        }

        // Create explicit deletion target for directory
        let deletions = vec![DeletionTarget {
            path: abs_dir.to_string_lossy().to_string(),
            kind: "directory".to_string(),
        }];

        // Build summary
        let summary = PlanSummary {
            affected_files: file_count,
            created_files: 0,
            deleted_files: file_count,
        };

        // Add warnings
        let mut warnings = Vec::new();
        if params.options.cleanup_imports.unwrap_or(true) {
            warnings.push(PlanWarning {
                code: "IMPORT_CLEANUP_REQUIRED".to_string(),
                message: format!(
                    "Directory deletion will clean up imports for {} files",
                    file_count
                ),
                candidates: None,
            });
        }

        // Check if this is a Cargo package
        let cargo_toml = abs_dir.join("Cargo.toml");
        if cargo_toml.exists() {
            warnings.push(PlanWarning {
                code: "CARGO_PACKAGE_DELETE".to_string(),
                message: "Deleting a Cargo package will remove it from workspace members"
                    .to_string(),
                candidates: None,
            });
        }

        // Build metadata
        let metadata = PlanMetadata {
            plan_version: "1.0".to_string(),
            kind: "delete".to_string(),
            language: "unknown".to_string(),
            estimated_impact: if file_count > 10 {
                "high"
            } else if file_count > 3 {
                "medium"
            } else {
                "low"
            }
            .to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
        };

        Ok(DeletePlan {
            deletions,
            summary,
            warnings,
            metadata,
            file_checksums,
        })
    }

    /// Generate plan for dead code deletion (placeholder)
    async fn plan_dead_code_delete(
        &self,
        params: &DeletePlanParams,
        _context: &ToolHandlerContext,
    ) -> ServerResult<DeletePlan> {
        debug!(
            path = %params.target.path,
            "Planning dead code delete (placeholder)"
        );

        // Create empty deletions list (placeholder - dead code analysis not yet integrated)
        let deletions = Vec::new();

        // Build summary
        let summary = PlanSummary {
            affected_files: 0,
            created_files: 0,
            deleted_files: 0,
        };

        // Add placeholder warning
        let warnings = vec![PlanWarning {
            code: "DEAD_CODE_DELETE_NOT_IMPLEMENTED".to_string(),
            message: "Dead code deletion requires integration with dead code analysis (not yet available)".to_string(),
            candidates: None,
        }];

        // Build metadata
        let metadata = PlanMetadata {
            plan_version: "1.0".to_string(),
            kind: "delete".to_string(),
            language: "unknown".to_string(),
            estimated_impact: "high".to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
        };

        Ok(DeletePlan {
            deletions,
            summary,
            warnings,
            metadata,
            file_checksums: HashMap::new(),
        })
    }

    /// Detect language from file extension
    fn detect_language(&self, path: &Path) -> String {
        match path.extension().and_then(|ext| ext.to_str()) {
            Some("rs") => "rust",
            Some("ts") | Some("tsx") => "typescript",
            Some("js") | Some("jsx") => "javascript",
            Some("py") | Some("pyi") => "python",
            Some("go") => "go",
            Some("java") => "java",
            Some("swift") => "swift",
            Some("cs") => "csharp",
            _ => "unknown",
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
