//! Delete handler for Unified Refactoring API
//!
//! Implements `delete` command with dryRun option for:
//! - Symbol deletion (AST-based - placeholder)
//! - File deletion (via FileService)
//! - Directory deletion (via FileService)

use crate::handlers::common::calculate_checksum;
use crate::handlers::tools::ToolHandler;
use async_trait::async_trait;
use futures::stream::StreamExt;
use mill_foundation::core::model::mcp::ToolCall;
use mill_foundation::errors::{MillError as ServerError, MillResult as ServerResult};
use mill_foundation::planning::{
    DeletePlan, DeletionTarget, PlanMetadata, PlanSummary, PlanWarning, RefactorPlan,
};
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{debug, error, info};

/// Handler for delete operations
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
    kind: String, // "symbol" | "file" | "directory"
    path: String,
    #[serde(default)]
    selector: Option<DeleteSelector>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)] // Reserved for future symbol deletion implementation
struct DeleteSelector {
    line: u32,
    character: u32,
    #[serde(default)]
    symbol_name: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeleteOptions {
    /// Preview mode - don't actually apply changes (default: true for safety)
    #[serde(default = "crate::default_true")]
    dry_run: bool,
    #[serde(default)]
    cleanup_imports: Option<bool>,
    #[serde(default)]
    #[allow(dead_code)] // Reserved for future test cleanup implementation
    remove_tests: Option<bool>,
    #[serde(default)]
    force: Option<bool>,
}

impl Default for DeleteOptions {
    fn default() -> Self {
        Self {
            dry_run: true, // Safe default: preview mode
            cleanup_imports: None,
            remove_tests: None,
            force: None,
        }
    }
}

#[async_trait]
impl ToolHandler for DeleteHandler {
    fn tool_names(&self) -> &[&str] {
        &["delete"]
    }

    fn is_internal(&self) -> bool {
        // Legacy - use prune instead
        true
    }

    async fn handle_tool_call(
        &self,
        context: &mill_handler_api::ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        info!(tool_name = %tool_call.name, "Handling delete");

        // Parse parameters
        let args = tool_call
            .arguments
            .clone()
            .ok_or_else(|| ServerError::invalid_request("Missing arguments for delete"))?;

        let params: DeletePlanParams = serde_json::from_value(args).map_err(|e| {
            ServerError::invalid_request(format!("Invalid delete parameters: {}", e))
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
            kind => {
                return Err(ServerError::invalid_request(format!(
                    "Unsupported delete kind: {}. Must be one of: symbol, file, directory",
                    kind
                )));
            }
        };

        // Wrap in RefactorPlan enum for discriminant
        let refactor_plan = RefactorPlan::DeletePlan(plan);

        // Check if we should execute or just return plan
        if params.options.dry_run {
            // Return plan only (preview mode)
            let plan_json = serde_json::to_value(&refactor_plan).map_err(|e| {
                ServerError::internal(format!("Failed to serialize delete plan: {}", e))
            })?;

            info!(
                operation = "delete",
                dry_run = true,
                "Returning delete plan (preview mode)"
            );

            Ok(serde_json::json!({"content": plan_json}))
        } else {
            // Execute the plan
            info!(
                operation = "delete",
                dry_run = false,
                "Executing delete plan"
            );

            let result =
                crate::handlers::common::execute_refactor_plan(context, refactor_plan).await?;

            let result_json = serde_json::to_value(&result).map_err(|e| {
                ServerError::internal(format!("Failed to serialize execution result: {}", e))
            })?;

            info!(
                operation = "delete",
                success = result.success,
                applied_files = result.applied_files.len(),
                "Delete execution completed"
            );

            Ok(serde_json::json!({"content": result_json}))
        }
    }
}

impl DeleteHandler {
    /// Helper to remove a specific identifier from an import statement
    /// Returns Some(new_line) if we can keep the import with other identifiers
    /// Returns None if the entire line should be deleted
    fn remove_import_identifier(&self, line: &str, identifier: &str) -> Option<String> {
        // Check if this is an import statement with curly braces
        if !line.trim_start().starts_with("import") || !line.contains('{') || !line.contains('}') {
            return None; // Not a destructured import, delete entire line
        }

        // Extract the part between curly braces
        let start_brace = line.find('{')?;
        let end_brace = line.find('}')?;
        let imports_section = &line[start_brace + 1..end_brace];

        // Split by comma and filter out the identifier to remove
        let identifiers: Vec<&str> = imports_section
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty() && *s != identifier)
            .collect();

        if identifiers.is_empty() {
            // No identifiers left, delete entire line
            None
        } else if identifiers.len()
            == imports_section
                .split(',')
                .filter(|s| !s.trim().is_empty())
                .count()
        {
            // Identifier wasn't found, delete entire line
            None
        } else {
            // Reconstruct the import with remaining identifiers
            let prefix = &line[..start_brace + 1];
            let suffix = &line[end_brace..];
            let new_imports = identifiers.join(", ");
            Some(format!("{} {} {}", prefix, new_imports, suffix))
        }
    }

    /// Generate plan for symbol deletion using AST-based analysis
    async fn plan_symbol_delete(
        &self,
        params: &DeletePlanParams,
        context: &mill_handler_api::ToolHandlerContext,
    ) -> ServerResult<DeletePlan> {
        debug!(path = %params.target.path, "Planning symbol delete");

        let file_path = Path::new(&params.target.path);

        // Validate selector is provided
        let selector = params.target.selector.as_ref().ok_or_else(|| {
            ServerError::invalid_request("Symbol delete requires selector with line/character")
        })?;

        // Get file extension to find the right language plugin
        let extension = file_path
            .extension()
            .and_then(|ext| ext.to_str())
            .ok_or_else(|| {
                error!(
                    path = %params.target.path,
                    "File has no extension for plugin lookup"
                );
                ServerError::invalid_request(format!(
                    "File has no extension: {}",
                    params.target.path
                ))
            })?;

        debug!(
            extension = %extension,
            "Looking up language plugin for extension"
        );

        // Get the language plugin for this file extension
        let plugin = context
            .app_state
            .language_plugins
            .get_plugin(extension)
            .ok_or_else(|| {
                error!(
                    extension = %extension,
                    "No language plugin found for extension"
                );
                ServerError::not_supported(format!(
                    "No language plugin available for .{} files",
                    extension
                ))
            })?;

        debug!(
            plugin = %plugin.metadata().name,
            "Found language plugin, getting refactoring provider capability"
        );

        // Get the refactoring provider capability from the plugin
        let refactoring_provider = plugin.refactoring_provider().ok_or_else(|| {
            error!(
                plugin = %plugin.metadata().name,
                "Plugin does not support refactoring operations"
            );
            ServerError::not_supported(format!(
                "{} plugin does not support symbol deletion refactoring",
                plugin.metadata().name
            ))
        })?;

        // Read file content
        let content = context
            .app_state
            .file_service
            .read_file(file_path)
            .await
            .map_err(|e| {
                error!(error = %e, file_path = %params.target.path, "Failed to read file");
                ServerError::internal(format!("Failed to read file: {}", e))
            })?;

        debug!("File read successfully, calling plan_symbol_delete on RefactoringProvider");

        // Call the plugin's symbol delete planning
        // Note: This is a new method that needs to be added to the RefactoringProvider trait
        let edit_plan = refactoring_provider
            .plan_symbol_delete(
                &content,
                selector.line,
                selector.character,
                &params.target.path,
            )
            .await
            .map_err(|e| {
                error!(
                    error = %e,
                    "Plugin symbol delete failed"
                );
                ServerError::internal(format!("Symbol delete failed: {}", e))
            })?;

        // Convert EditPlan to WorkspaceEdit using the converter utility from move module
        let workspace_edit =
            crate::handlers::r#move::converter::convert_edit_plan_to_workspace_edit(&edit_plan)?;

        // Calculate file checksums
        let mut file_checksums = HashMap::new();
        file_checksums.insert(
            file_path.to_string_lossy().to_string(),
            calculate_checksum(&content),
        );

        // Build summary
        let summary = PlanSummary {
            affected_files: 1,
            created_files: 0,
            deleted_files: 0,
        };

        // Determine language from extension via plugin registry
        let language = plugin.metadata().name.to_string();

        // Build metadata
        let metadata = PlanMetadata {
            plan_version: "1.0".to_string(),
            kind: "delete".to_string(),
            language,
            estimated_impact: "low".to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
        };

        info!(
            file_path = %params.target.path,
            line = selector.line,
            "Created AST-based delete plan for symbol"
        );

        Ok(DeletePlan {
            deletions: Vec::new(), // No file deletions, using edits instead
            edits: Some(workspace_edit),
            summary,
            warnings: Vec::new(),
            metadata,
            file_checksums,
        })
    }

    /// Generate plan for file deletion using FileService
    async fn plan_file_delete(
        &self,
        params: &DeletePlanParams,
        context: &mill_handler_api::ToolHandlerContext,
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

        // Extract the inner value from DryRunnable
        let result_value = dry_run_result.result;

        // Read file content for checksum before deletion
        let content = context
            .app_state
            .file_service
            .read_file(file_path)
            .await
            .map_err(|e| {
                error!(error = %e, file_path = %params.target.path, "Failed to read file");
                ServerError::internal(format!("Failed to read file for checksum: {}", e))
            })?;

        // Calculate checksum
        let mut file_checksums = HashMap::new();
        file_checksums.insert(
            file_path.to_string_lossy().to_string(),
            calculate_checksum(&content),
        );

        // Canonicalize path to ensure proper path handling
        let abs_file_path = tokio::fs::canonicalize(file_path)
            .await
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
        let affected_files_count = result_value
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
        let language = crate::handlers::common::detect_language(&params.target.path).to_string();

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
            edits: None, // Symbol deletion will use this field
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
        context: &mill_handler_api::ToolHandlerContext,
    ) -> ServerResult<DeletePlan> {
        debug!(
            path = %params.target.path,
            "Planning directory delete"
        );

        let dir_path = Path::new(&params.target.path);

        // Walk directory to collect files and checksums
        // Move directory walking and checking to a blocking task to avoid blocking the async runtime
        let dir_path_buf = dir_path.to_path_buf();
        let target_path_str = params.target.path.clone();

        let (files, abs_dir): (Vec<PathBuf>, PathBuf) = tokio::task::spawn_blocking(move || {
            // Verify it's a directory inside the blocking task using synchronous fs
            if !dir_path_buf.is_dir() {
                return Err(ServerError::invalid_request(format!(
                    "Path is not a directory: {}",
                    target_path_str
                )));
            }

            let abs_dir = std::fs::canonicalize(&dir_path_buf).unwrap_or(dir_path_buf);
            let walker = ignore::WalkBuilder::new(&abs_dir).hidden(false).build();
            let files = walker
                .flatten()
                .filter(|entry| entry.path().is_file())
                .map(|entry| entry.path().to_path_buf())
                .collect();
            Ok((files, abs_dir))
        })
        .await
        .map_err(|e| ServerError::internal(format!("Task failed: {}", e)))??;

        let mut file_checksums = HashMap::new();
        let mut file_count = 0;

        // Process file reads and checksums concurrently
        let results: Vec<_> = futures::stream::iter(files)
            .map(|path| {
                let fs = context.app_state.file_service.clone();
                async move {
                    match fs.read_file(&path).await {
                        Ok(content) => Some((
                            path.to_string_lossy().to_string(),
                            calculate_checksum(&content),
                        )),
                        Err(_) => None,
                    }
                }
            })
            .buffer_unordered(50) // Process 50 files concurrently
            .collect()
            .await;

        for (path_str, checksum) in results.into_iter().flatten() {
            file_checksums.insert(path_str, checksum);
            file_count += 1;
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
        if tokio::fs::try_exists(&cargo_toml).await.unwrap_or(false) {
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
            edits: None, // Symbol deletion will use this field
            summary,
            warnings,
            metadata,
            file_checksums,
        })
    }
}
