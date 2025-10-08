//! File operations service with import awareness

use crate::services::git_service::GitService;
use crate::services::import_service::ImportService;
use crate::services::lock_manager::LockManager;
use crate::services::operation_queue::{FileOperation, OperationTransaction, OperationType};
use cb_ast::AstCache;
use cb_core::config::AppConfig;
use cb_core::dry_run::DryRunnable;
use cb_protocol::{ApiError as ServerError, ApiResult as ServerResult};
use cb_protocol::{DependencyUpdate, EditPlan, EditPlanMetadata, TextEdit};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;
use tracing::{debug, error, info, warn};

/// Service for file operations with import update capabilities
pub struct FileService {
    /// Import service for handling import updates
    import_service: ImportService,
    /// Project root directory
    project_root: PathBuf,
    /// AST cache for invalidation after edits
    ast_cache: Arc<AstCache>,
    /// Lock manager for atomic operations
    lock_manager: Arc<LockManager>,
    /// Operation queue for serializing file operations
    operation_queue: Arc<super::operation_queue::OperationQueue>,
    /// Git service for git-aware file operations
    #[allow(dead_code)]
    git_service: GitService,
    /// Whether to use git for file operations
    use_git: bool,
    /// Validation configuration
    validation_config: cb_core::config::ValidationConfig,
}

impl FileService {
    /// Create a new file service
    pub fn new(
        project_root: impl AsRef<Path>,
        ast_cache: Arc<AstCache>,
        lock_manager: Arc<LockManager>,
        operation_queue: Arc<super::operation_queue::OperationQueue>,
        config: &AppConfig,
        plugin_registry: Arc<cb_plugin_api::PluginRegistry>,
    ) -> Self {
        let project_root = project_root.as_ref().to_path_buf();

        // Determine if we should use git based on:
        // 1. Configuration git.enabled flag
        // 2. Whether the project is actually a git repository
        let is_git_repo = GitService::is_git_repo(&project_root);
        let use_git = config.git.enabled && is_git_repo;

        debug!(
            project_root = %project_root.display(),
            git_enabled_in_config = config.git.enabled,
            is_git_repo,
            use_git,
            "Initializing FileService with git support and injected plugin registry"
        );

        Self {
            import_service: ImportService::new(&project_root, plugin_registry),
            project_root,
            ast_cache,
            lock_manager,
            operation_queue,
            git_service: GitService::new(),
            use_git,
            validation_config: config.validation.clone(),
        }
    }

    /// Run post-operation validation if configured
    /// Returns validation results to be included in the operation response
    async fn run_validation(&self) -> Option<Value> {
        use std::process::Command;

        if !self.validation_config.enabled {
            return None;
        }

        info!(
            command = %self.validation_config.command,
            "Running post-operation validation"
        );

        // Run validation command in the project root
        let output = match Command::new("sh")
            .arg("-c")
            .arg(&self.validation_config.command)
            .current_dir(&self.project_root)
            .output()
        {
            Ok(output) => output,
            Err(e) => {
                error!(error = %e, "Failed to execute validation command");
                return Some(json!({
                    "validation_status": "error",
                    "validation_error": format!("Failed to execute command: {}", e)
                }));
            }
        };

        let success = output.status.success();
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if success {
            info!("Validation passed");
            Some(json!({
                "validation_status": "passed",
                "validation_command": self.validation_config.command
            }))
        } else {
            warn!(
                stderr = %stderr,
                "Validation failed"
            );

            // For Report action, just include the errors in the response
            match self.validation_config.on_failure {
                cb_core::config::ValidationFailureAction::Report => Some(json!({
                    "validation_status": "failed",
                    "validation_command": self.validation_config.command,
                    "validation_errors": stderr,
                    "validation_stdout": stdout,
                    "suggestion": format!(
                        "Validation failed. Run '{}' to see details. Consider reviewing changes before committing.",
                        self.validation_config.command
                    )
                })),
                cb_core::config::ValidationFailureAction::Rollback => {
                    warn!(
                        stderr = %stderr,
                        "Validation failed. Executing automatic rollback via 'git reset --hard HEAD'"
                    );

                    let rollback_output = Command::new("git")
                        .args(["reset", "--hard", "HEAD"])
                        .current_dir(&self.project_root)
                        .output();

                    let (rollback_status, rollback_error) = match rollback_output {
                        Ok(out) if out.status.success() => {
                            info!("Rollback completed successfully");
                            ("rollback_succeeded", None)
                        }
                        Ok(out) => {
                            let error_msg = String::from_utf8_lossy(&out.stderr).to_string();
                            error!(error = %error_msg, "Rollback command failed");
                            ("rollback_failed", Some(error_msg))
                        }
                        Err(e) => {
                            error!(error = %e, "Failed to execute rollback command");
                            ("rollback_failed", Some(e.to_string()))
                        }
                    };

                    Some(json!({
                        "validation_status": "failed",
                        "validation_action": rollback_status,
                        "validation_command": self.validation_config.command,
                        "validation_errors": stderr,
                        "rollback_error": rollback_error,
                        "suggestion": if rollback_status == "rollback_succeeded" {
                            "Validation failed and changes were automatically rolled back using git."
                        } else {
                            "Validation failed and automatic rollback failed. Please manually revert changes."
                        }
                    }))
                }
                cb_core::config::ValidationFailureAction::Interactive => Some(json!({
                    "validation_status": "failed",
                    "validation_action": "interactive_prompt",
                    "validation_command": self.validation_config.command,
                    "validation_errors": stderr,
                    "validation_stdout": stdout,
                    "rollback_available": true,
                    "suggestion": "Validation failed. Please review the errors and decide whether to keep or revert the changes. Run 'git reset --hard HEAD' to rollback."
                })),
            }
        }
    }

    /// Perform a git-aware file rename
    ///
    /// Uses `git mv` if the file is tracked and git is available, otherwise falls back to filesystem rename.
    async fn rename_file_internal(&self, old: &Path, new: &Path) -> ServerResult<()> {
        if self.use_git && GitService::is_file_tracked(old) {
            // Use git mv for tracked files
            debug!(
                old = %old.display(),
                new = %new.display(),
                "Using git mv for tracked file"
            );

            // git mv is synchronous, run in blocking task
            let old_clone = old.to_path_buf();
            let new_clone = new.to_path_buf();

            tokio::task::spawn_blocking(move || GitService::git_mv(&old_clone, &new_clone))
                .await
                .map_err(|e| ServerError::Internal(format!("Task join error: {}", e)))?
                .map_err(|e| ServerError::Internal(format!("git mv failed: {}", e)))?;
        } else {
            // Fallback to filesystem rename
            debug!(
                old = %old.display(),
                new = %new.display(),
                use_git = self.use_git,
                "Using filesystem rename"
            );

            // Ensure parent directory exists
            if let Some(parent) = new.parent() {
                fs::create_dir_all(parent).await.map_err(|e| {
                    ServerError::Internal(format!("Failed to create parent directory: {}", e))
                })?;
            }

            fs::rename(old, new)
                .await
                .map_err(|e| ServerError::Internal(format!("Failed to rename file: {}", e)))?;
        }

        Ok(())
    }

    /// Rename a file and update all imports
    pub async fn rename_file_with_imports(
        &self,
        old_path: &Path,
        new_path: &Path,
        dry_run: bool,
        scan_scope: Option<cb_plugin_api::ScanScope>,
    ) -> ServerResult<DryRunnable<Value>> {
        info!(old_path = ?old_path, new_path = ?new_path, dry_run, "Renaming file");

        let old_abs = self.to_absolute_path(old_path);
        let new_abs = self.to_absolute_path(new_path);

        if dry_run {
            // Preview mode - just return what would happen
            if !old_abs.exists() {
                return Err(ServerError::NotFound(format!(
                    "Source file does not exist: {:?}",
                    old_abs
                )));
            }

            if new_abs.exists() {
                return Err(ServerError::AlreadyExists(format!(
                    "Destination file already exists: {:?}",
                    new_abs
                )));
            }

            let affected_files = self.import_service.find_affected_files(&old_abs).await?;

            let edit_plan = self
                .import_service
                .update_imports_for_rename(&old_abs, &new_abs, None, true, scan_scope.clone())
                .await?;

            Ok(DryRunnable::new(
                true,
                json!({
                    "operation": "rename_file",
                    "old_path": old_abs.to_string_lossy(),
                    "new_path": new_abs.to_string_lossy(),
                    "affected_files": affected_files.len(),
                    "import_updates": {
                        "edits_planned": edit_plan.edits.len(),
                        "files_to_modify": edit_plan.edits.iter()
                            .filter_map(|e| e.file_path.as_ref())
                            .collect::<std::collections::HashSet<_>>()
                            .len(),
                    },
                }),
            ))
        } else {
            // Execution mode - perform rename and update imports
            if !old_abs.exists() {
                return Err(ServerError::NotFound(format!(
                    "Source file does not exist: {:?}",
                    old_abs
                )));
            }

            if new_abs.exists() {
                return Err(ServerError::AlreadyExists(format!(
                    "Destination file already exists: {:?}",
                    new_abs
                )));
            }

            self.perform_rename(&old_abs, &new_abs).await?;

            info!("File renamed successfully");

            let mut edit_plan = self
                .import_service
                .update_imports_for_rename(&old_abs, &new_abs, None, false, scan_scope)
                .await
                .map_err(|e| {
                    warn!(error = %e, "File renamed but import updates failed");
                    ServerError::Internal(format!("Import updates failed: {}", e))
                })?;

            // Update the source_file in the edit plan to the new path
            // since the file has been renamed
            if edit_plan.source_file == old_abs.to_string_lossy() {
                edit_plan.source_file = new_abs.to_string_lossy().to_string();
            }

            debug!(
                edits_count = edit_plan.edits.len(),
                dependency_updates_count = edit_plan.dependency_updates.len(),
                source_file = %edit_plan.source_file,
                "EditPlan before applying"
            );

            // Log dependency updates for debugging
            for (i, dep_update) in edit_plan.dependency_updates.iter().enumerate() {
                debug!(
                    index = i,
                    target_file = %dep_update.target_file,
                    update_type = ?dep_update.update_type,
                    "Dependency update"
                );
            }

            // Apply the edit plan to update imports
            let edit_result = self.apply_edit_plan(&edit_plan).await.map_err(|e| {
                warn!(error = %e, "Failed to apply import update edits");
                ServerError::Internal(format!("Failed to apply import updates: {}", e))
            })?;

            info!(
                edits_applied = edit_plan.edits.len(),
                files_modified = edit_result.modified_files.len(),
                success = edit_result.success,
                "Successfully updated imports via EditPlan"
            );

            Ok(DryRunnable::new(
                false,
                json!({
                    "operation": "rename_file",
                    "old_path": old_abs.to_string_lossy(),
                    "new_path": new_abs.to_string_lossy(),
                    "success": true,
                    "import_updates": {
                        "edits_applied": edit_plan.edits.len(),
                        "files_modified": edit_result.modified_files,
                        "success": edit_result.success,
                    },
                }),
            ))
        }
    }

    /// Rename a directory and update all imports pointing to files within it
    pub async fn rename_directory_with_imports(
        &self,
        old_dir_path: &Path,
        new_dir_path: &Path,
        dry_run: bool,
        consolidate: bool,
        scan_scope: Option<cb_plugin_api::ScanScope>,
    ) -> ServerResult<DryRunnable<Value>> {
        info!(old_path = ?old_dir_path, new_path = ?new_dir_path, dry_run, consolidate, "Renaming directory");

        // If consolidate flag is set, use consolidation logic instead
        if consolidate {
            return self
                .consolidate_rust_package(old_dir_path, new_dir_path, dry_run)
                .await;
        }

        let old_abs_dir = self.to_absolute_path(old_dir_path);
        let new_abs_dir = self.to_absolute_path(new_dir_path);

        if dry_run {
            // Preview mode - just return what would happen
            if !old_abs_dir.exists() {
                return Err(ServerError::NotFound(format!(
                    "Source directory does not exist: {:?}",
                    old_abs_dir
                )));
            }

            if new_abs_dir.exists() {
                return Err(ServerError::AlreadyExists(format!(
                    "Destination directory already exists: {:?}",
                    new_abs_dir
                )));
            }

            let mut files_to_move = Vec::new();
            let walker = ignore::WalkBuilder::new(&old_abs_dir).hidden(false).build();
            for entry in walker.flatten() {
                if entry.path().is_file() {
                    files_to_move.push(entry.path().to_path_buf());
                }
            }

            let is_cargo_pkg = self.is_cargo_package(&old_abs_dir).await?;

            Ok(DryRunnable::new(
                true,
                json!({
                    "operation": "rename_directory",
                    "old_path": old_abs_dir.to_string_lossy(),
                    "new_path": new_abs_dir.to_string_lossy(),
                    "files_to_move": files_to_move.len(),
                    "is_cargo_package": is_cargo_pkg,
                }),
            ))
        } else {
            // Execution mode - perform directory rename and update imports
            if !old_abs_dir.exists() {
                return Err(ServerError::NotFound(format!(
                    "Source directory does not exist: {:?}",
                    old_abs_dir
                )));
            }

            if new_abs_dir.exists() {
                return Err(ServerError::AlreadyExists(format!(
                    "Destination directory already exists: {:?}",
                    new_abs_dir
                )));
            }

            let mut files_to_move = Vec::new();
            let walker = ignore::WalkBuilder::new(&old_abs_dir).hidden(false).build();
            for entry in walker.flatten() {
                if entry.path().is_file() {
                    files_to_move.push(entry.path().to_path_buf());
                }
            }

            let is_cargo_pkg = self.is_cargo_package(&old_abs_dir).await?;

            // Build rename_info for Cargo packages
            let rename_info = if is_cargo_pkg {
                self.extract_cargo_rename_info(&old_abs_dir, &new_abs_dir)
                    .await
                    .ok()
            } else {
                None
            };

            self.perform_rename(&old_abs_dir, &new_abs_dir).await?;

            info!("Directory renamed successfully");

            let mut total_edits_applied = 0;
            let mut total_files_updated = std::collections::HashSet::new();
            let mut all_errors = Vec::new();

            // Call update_imports_for_rename ONCE for the entire directory rename
            // This prevents creating duplicate edits for the same affected files
            match self
                .import_service
                .update_imports_for_rename(
                    &old_abs_dir,  // Use directory paths instead of individual files
                    &new_abs_dir,
                    rename_info.as_ref(),
                    false,
                    scan_scope,
                )
                .await
            {
                Ok(edit_plan) => {
                    match self.apply_edit_plan(&edit_plan).await {
                        Ok(result) => {
                            total_edits_applied += edit_plan.edits.len();
                            let files_modified_count = result.modified_files.len();
                            for file in result.modified_files {
                                total_files_updated.insert(file);
                            }
                            if let Some(errors) = result.errors {
                                all_errors.extend(errors);
                            }
                            info!(
                                edits_applied = edit_plan.edits.len(),
                                files_modified = files_modified_count,
                                "Successfully updated imports for directory rename"
                            );
                        }
                        Err(e) => {
                            let error_msg = format!(
                                "Failed to apply import edits for directory rename: {}",
                                e
                            );
                            warn!(error = %e, "Import update failed for directory");
                            all_errors.push(error_msg);
                        }
                    }
                }
                Err(e) => {
                    warn!(error = %e, old_dir = ?old_abs_dir, "Failed to update imports for directory");
                    all_errors.push(format!("Failed to update imports for directory: {}", e));
                }
            }

            // Track manifest updates (Cargo.toml files)
            let mut manifest_updated_files: Vec<PathBuf> = Vec::new();
            let mut manifest_errors: Vec<String> = Vec::new();

            if is_cargo_pkg {
                // Update workspace members array
                match self
                    .update_workspace_manifests(&old_abs_dir, &new_abs_dir)
                    .await
                {
                    Ok(updated_files) => {
                        manifest_updated_files.extend(updated_files);
                    }
                    Err(e) => {
                        warn!(error = %e, "Failed to update workspace manifest");
                        let error_msg = format!("Failed to update workspace manifest: {}", e);
                        all_errors.push(error_msg.clone());
                        manifest_errors.push(error_msg);
                    }
                }

                // Update path dependencies in other crates that depend on this one
                if let Some(ref info) = rename_info {
                    // Use old_package_name (with hyphens) for Cargo.toml dependency lookups
                    if let Some(old_package_name) = info.get("old_package_name").and_then(|v| v.as_str()) {
                        match self.update_dependent_crate_paths(old_package_name, &new_abs_dir).await {
                            Ok(updated_files) => {
                                if !updated_files.is_empty() {
                                    info!(
                                        files_updated = updated_files.len(),
                                        "Updated Cargo.toml path dependencies in dependent crates"
                                    );
                                    manifest_updated_files.extend(updated_files);
                                }
                            }
                            Err(e) => {
                                warn!(error = %e, "Failed to update dependent crate paths");
                                let error_msg = format!("Failed to update dependent crate paths: {}", e);
                                all_errors.push(error_msg.clone());
                                manifest_errors.push(error_msg);
                            }
                        }
                    }
                }
            }

            let doc_updates = self
                .update_documentation_references(&old_abs_dir, &new_abs_dir, false)
                .await
                .ok();

            info!(
                files_moved = files_to_move.len(),
                edits_applied = total_edits_applied,
                files_updated = total_files_updated.len(),
                "Directory rename complete"
            );

            // Run post-operation validation if configured
            let validation_result = self.run_validation().await;

            // Build manifest updates report (consistent format with other reports)
            // Deduplicate manifest files (same manifest can be touched in multiple phases)
            // Sort for deterministic output (important for snapshot testing and stable API)
            let manifest_updates = if is_cargo_pkg {
                let unique_manifests: std::collections::HashSet<_> =
                    manifest_updated_files.into_iter().collect();
                let mut sorted_manifests: Vec<_> = unique_manifests.into_iter().collect();
                sorted_manifests.sort();
                Some(json!({
                    "files_updated": sorted_manifests.len(),
                    "updated_files": sorted_manifests.iter()
                        .map(|p| p.to_string_lossy().to_string())
                        .collect::<Vec<_>>(),
                    "errors": manifest_errors,
                }))
            } else {
                None
            };

            let mut result = json!({
                "operation": "rename_directory",
                "old_path": old_abs_dir.to_string_lossy(),
                "new_path": new_abs_dir.to_string_lossy(),
                "files_moved": files_to_move.len(),
                "import_updates": {
                    "files_updated": total_files_updated.len(),
                    "edits_applied": total_edits_applied,
                    "errors": all_errors,
                },
                "documentation_updates": doc_updates,
                "manifest_updates": manifest_updates,
                "success": all_errors.is_empty(),
            });

            // Add validation results if available
            if let Some(validation) = validation_result {
                result["validation"] = validation;
            }

            Ok(DryRunnable::new(false, result))
        }
    }

    /// Perform the actual file rename operation
    async fn perform_rename(&self, old_path: &Path, new_path: &Path) -> ServerResult<()> {
        // Use our git-aware rename helper directly
        self.rename_file_internal(old_path, new_path).await
    }

    /// Create a new file with content
    pub async fn create_file(
        &self,
        path: &Path,
        content: Option<&str>,
        overwrite: bool,
        dry_run: bool,
    ) -> ServerResult<DryRunnable<Value>> {
        let abs_path = self.to_absolute_path(path);
        let content = content.unwrap_or("").to_string();

        if dry_run {
            // Preview mode - just return what would happen
            if abs_path.exists() && !overwrite {
                return Err(ServerError::AlreadyExists(format!(
                    "File already exists: {:?}",
                    abs_path
                )));
            }

            Ok(DryRunnable::new(
                true,
                json!({
                    "operation": "create_file",
                    "path": abs_path.to_string_lossy(),
                    "overwrite": overwrite,
                    "content_size": content.len(),
                }),
            ))
        } else {
            // Execution mode - queue the operation
            if abs_path.exists() && !overwrite {
                return Err(ServerError::AlreadyExists(format!(
                    "File already exists: {:?}",
                    abs_path
                )));
            }

            // Queue the operations for execution by the background worker
            let mut transaction = OperationTransaction::new(self.operation_queue.clone());

            if let Some(parent) = abs_path.parent() {
                if !parent.exists() {
                    transaction.add_operation(FileOperation::new(
                        "system".to_string(),
                        OperationType::CreateDir,
                        parent.to_path_buf(),
                        json!({ "recursive": true }),
                    ));
                }
            }

            transaction.add_operation(FileOperation::new(
                "system".to_string(),
                OperationType::CreateFile,
                abs_path.clone(),
                json!({ "content": content }),
            ));

            transaction
                .commit()
                .await
                .map_err(|e| ServerError::Internal(e.to_string()))?;

            info!(path = ?abs_path, "Queued create_file operation");

            // Wait for the operation to complete before returning
            self.operation_queue.wait_until_idle().await;

            // Verify the file was created
            if !abs_path.exists() {
                return Err(ServerError::Internal(format!(
                    "File creation failed: {:?}",
                    abs_path
                )));
            }

            Ok(DryRunnable::new(
                false,
                json!({
                    "success": true,
                    "path": abs_path.to_string_lossy()
                }),
            ))
        }
    }

    /// Delete a file
    pub async fn delete_file(
        &self,
        path: &Path,
        force: bool,
        dry_run: bool,
    ) -> ServerResult<DryRunnable<Value>> {
        let abs_path = self.to_absolute_path(path);

        if dry_run {
            // Preview mode - just return what would happen
            if !abs_path.exists() {
                if force {
                    return Ok(DryRunnable::new(
                        true,
                        json!({
                            "operation": "delete_file",
                            "path": abs_path.to_string_lossy(),
                            "force": force,
                            "status": "not_exists",
                        }),
                    ));
                } else {
                    return Err(ServerError::NotFound(format!(
                        "File does not exist: {:?}",
                        abs_path
                    )));
                }
            }

            let affected_files_count = if !force {
                let affected = self.import_service.find_affected_files(&abs_path).await?;
                if !affected.is_empty() {
                    return Err(ServerError::InvalidRequest(format!(
                        "File is imported by {} other files",
                        affected.len()
                    )));
                }
                affected.len()
            } else {
                0
            };

            Ok(DryRunnable::new(
                true,
                json!({
                    "operation": "delete_file",
                    "path": abs_path.to_string_lossy(),
                    "force": force,
                    "affected_files": affected_files_count,
                }),
            ))
        } else {
            // Execution mode - queue the operation
            if !abs_path.exists() {
                if force {
                    return Ok(DryRunnable::new(
                        false,
                        json!({
                            "operation": "delete_file",
                            "path": abs_path.to_string_lossy(),
                            "deleted": false,
                            "reason": "not_exists",
                        }),
                    ));
                } else {
                    return Err(ServerError::NotFound(format!(
                        "File does not exist: {:?}",
                        abs_path
                    )));
                }
            }

            if !force {
                let affected = self.import_service.find_affected_files(&abs_path).await?;
                if !affected.is_empty() {
                    warn!(
                        affected_files_count = affected.len(),
                        "File is imported by other files. Use force=true to delete anyway"
                    );
                    return Err(ServerError::InvalidRequest(format!(
                        "File is imported by {} other files",
                        affected.len()
                    )));
                }
            }

            // Queue the operation for execution by the background worker
            let mut transaction = OperationTransaction::new(self.operation_queue.clone());
            transaction.add_operation(FileOperation::new(
                "system".to_string(),
                OperationType::Delete,
                abs_path.clone(),
                json!({ "force": force }),
            ));
            transaction
                .commit()
                .await
                .map_err(|e| ServerError::Internal(e.to_string()))?;

            info!(path = ?abs_path, "Queued delete_file operation");

            // Wait for the operation to complete before returning
            self.operation_queue.wait_until_idle().await;

            // Verify the file was deleted
            if abs_path.exists() {
                return Err(ServerError::Internal(format!(
                    "File deletion failed: {:?}",
                    abs_path
                )));
            }

            Ok(DryRunnable::new(
                false,
                json!({
                    "success": true,
                    "path": abs_path.to_string_lossy()
                }),
            ))
        }
    }

    /// Read file contents
    pub async fn read_file(&self, path: &Path) -> ServerResult<String> {
        let abs_path = self.to_absolute_path(path);

        if !abs_path.exists() {
            return Err(ServerError::NotFound(format!(
                "File does not exist: {:?}",
                abs_path
            )));
        }

        let content = fs::read_to_string(&abs_path)
            .await
            .map_err(|e| ServerError::Internal(format!("Failed to read file: {}", e)))?;

        Ok(content)
    }

    /// Write content to file
    pub async fn write_file(
        &self,
        path: &Path,
        content: &str,
        dry_run: bool,
    ) -> ServerResult<DryRunnable<Value>> {
        let abs_path = self.to_absolute_path(path);
        let content = content.to_string();

        if dry_run {
            // Preview mode - just return what would happen
            Ok(DryRunnable::new(
                true,
                json!({
                    "operation": "write_file",
                    "path": abs_path.to_string_lossy(),
                    "content_size": content.len(),
                    "exists": abs_path.exists(),
                }),
            ))
        } else {
            // Execution mode - queue the operation
            let mut transaction = OperationTransaction::new(self.operation_queue.clone());

            if let Some(parent) = abs_path.parent() {
                if !parent.exists() {
                    transaction.add_operation(FileOperation::new(
                        "system".to_string(),
                        OperationType::CreateDir,
                        parent.to_path_buf(),
                        json!({ "recursive": true }),
                    ));
                }
            }

            transaction.add_operation(FileOperation::new(
                "system".to_string(),
                OperationType::Write,
                abs_path.clone(),
                json!({ "content": content }),
            ));

            transaction
                .commit()
                .await
                .map_err(|e| ServerError::Internal(e.to_string()))?;

            info!(path = ?abs_path, "Queued write_file operation");

            // Wait for the operation to complete before returning
            self.operation_queue.wait_until_idle().await;

            // Verify the file was written
            if !abs_path.exists() {
                return Err(ServerError::Internal(format!(
                    "File write failed: {:?}",
                    abs_path
                )));
            }

            Ok(DryRunnable::new(
                false,
                json!({
                    "success": true,
                    "path": abs_path.to_string_lossy()
                }),
            ))
        }
    }

    /// List files in a directory
    pub async fn list_files(&self, path: &Path, recursive: bool) -> ServerResult<Vec<String>> {
        self.list_files_with_pattern(path, recursive, None).await
    }

    /// List files in a directory with optional glob pattern filtering
    pub async fn list_files_with_pattern(
        &self,
        path: &Path,
        recursive: bool,
        pattern: Option<&str>,
    ) -> ServerResult<Vec<String>> {
        let abs_path = self.to_absolute_path(path);

        if !abs_path.exists() {
            return Err(ServerError::NotFound(format!(
                "Directory not found: {}",
                abs_path.display()
            )));
        }

        if !abs_path.is_dir() {
            return Err(ServerError::InvalidRequest(format!(
                "Path is not a directory: {}",
                abs_path.display()
            )));
        }

        let mut files = Vec::new();

        if recursive {
            self.list_files_recursive(&abs_path, &abs_path, &mut files)
                .await?;
        } else {
            let mut entries = fs::read_dir(&abs_path)
                .await
                .map_err(|e| ServerError::Internal(format!("Failed to read directory: {}", e)))?;

            while let Some(entry) = entries.next_entry().await.map_err(|e| {
                ServerError::Internal(format!("Failed to read directory entry: {}", e))
            })? {
                let path = entry.path();
                if let Some(file_name) = path.file_name() {
                    files.push(file_name.to_string_lossy().to_string());
                }
            }
        }

        // Apply pattern filtering if provided
        if let Some(pattern) = pattern {
            files = Self::filter_by_pattern(files, pattern)?;
        }

        files.sort();
        Ok(files)
    }

    /// Filter files by glob pattern
    fn filter_by_pattern(files: Vec<String>, pattern: &str) -> ServerResult<Vec<String>> {
        use globset::{Glob, GlobMatcher};

        let glob = Glob::new(pattern).map_err(|e| {
            ServerError::InvalidRequest(format!("Invalid glob pattern '{}': {}", pattern, e))
        })?;
        let matcher: GlobMatcher = glob.compile_matcher();

        Ok(files
            .into_iter()
            .filter(|file| matcher.is_match(file))
            .collect())
    }

    /// Recursively list files in a directory
    async fn list_files_recursive(
        &self,
        base_path: &Path,
        current_path: &Path,
        files: &mut Vec<String>,
    ) -> ServerResult<()> {
        let mut entries = fs::read_dir(current_path)
            .await
            .map_err(|e| ServerError::Internal(format!("Failed to read directory: {}", e)))?;

        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| ServerError::Internal(format!("Failed to read directory entry: {}", e)))?
        {
            let path = entry.path();

            if path.is_dir() {
                Box::pin(self.list_files_recursive(base_path, &path, files)).await?;
            } else {
                // Get relative path from base directory
                if let Ok(relative) = path.strip_prefix(base_path) {
                    files.push(relative.to_string_lossy().to_string());
                }
            }
        }

        Ok(())
    }

    /// Apply an edit plan to the filesystem atomically
    pub async fn apply_edit_plan(&self, plan: &EditPlan) -> ServerResult<EditPlanResult> {
        info!(source_file = %plan.source_file, "Applying edit plan");
        debug!(
            edits_count = plan.edits.len(),
            dependency_updates_count = plan.dependency_updates.len(),
            "Edit plan contents"
        );

        // For simplicity, we'll apply edits sequentially with individual locks
        // In a production system, you might want more sophisticated coordination
        self.apply_edits_with_coordination(plan).await
    }

    /// Apply edits with file coordination and atomic rollback on failure
    async fn apply_edits_with_coordination(&self, plan: &EditPlan) -> ServerResult<EditPlanResult> {
        // Ensure all pending file operations are complete before creating snapshots
        // This is critical for cross-process cache coherency
        self.operation_queue.wait_until_idle().await;
        debug!("Operation queue idle before creating snapshots");

        // Step 1: Identify all files that will be affected
        let mut affected_files = std::collections::HashSet::new();

        // Main source file (may not have edits if this is a rename operation)
        // Skip empty source_file (used in multi-file workspace edits)
        if !plan.source_file.is_empty() {
            let main_file = self.to_absolute_path(Path::new(&plan.source_file));
            affected_files.insert(main_file.clone());
        }

        // Files affected by text edits (group by file_path)
        use std::collections::HashMap;
        let mut edits_by_file: HashMap<String, Vec<&cb_protocol::TextEdit>> = HashMap::new();

        for edit in &plan.edits {
            if let Some(file_path) = &edit.file_path {
                let abs_path = self.to_absolute_path(Path::new(file_path));
                affected_files.insert(abs_path);
                edits_by_file
                    .entry(file_path.clone())
                    .or_default()
                    .push(edit);
            } else {
                // Edit without explicit file_path goes to source_file
                edits_by_file
                    .entry(plan.source_file.clone())
                    .or_default()
                    .push(edit);
            }
        }

        // Files affected by dependency updates
        for dep_update in &plan.dependency_updates {
            let target_file = self.to_absolute_path(Path::new(&dep_update.target_file));
            affected_files.insert(target_file);
        }

        // Step 2: Create snapshots of all affected files before any modifications
        let snapshots = self.create_file_snapshots(&affected_files).await?;
        debug!(
            snapshot_count = snapshots.len(),
            files_with_edits = edits_by_file.len(),
            "Created file snapshots for atomic operation"
        );

        let mut modified_files = Vec::new();

        // Step 3: Apply text edits grouped by file with locking
        // Use snapshot content to avoid race conditions with file system
        for (file_path, edits) in edits_by_file {
            let abs_file_path = self.to_absolute_path(Path::new(&file_path));
            let file_lock = self.lock_manager.get_lock(&abs_file_path).await;
            let _guard = file_lock.write().await;

            // Convert &TextEdit to TextEdit
            let owned_edits: Vec<cb_protocol::TextEdit> =
                edits.iter().map(|e| (*e).clone()).collect();

            // Get the original content from snapshot (guarantees atomicity)
            let original_content = snapshots.get(&abs_file_path).ok_or_else(|| {
                ServerError::Internal(format!("File {} not found in snapshots", file_path))
            })?;

            // DEBUG: Log snapshot content length
            if original_content.is_empty() {
                error!(
                    file_path = %file_path,
                    "BUG: Snapshot content is EMPTY for file!"
                );
            }

            // Apply edits to the snapshot content (no I/O, fully synchronous)
            match self.apply_edits_to_content(original_content, &owned_edits) {
                Ok(modified_content) => {
                    // Write the final modified content to disk
                    if let Err(e) = fs::write(&abs_file_path, modified_content).await {
                        error!(
                            file_path = %file_path,
                            error = %e,
                            "Failed to write modified file"
                        );
                        self.rollback_from_snapshots(&snapshots).await?;
                        return Err(ServerError::Internal(format!(
                            "Failed to write file {}: {}. All changes have been rolled back.",
                            file_path, e
                        )));
                    }

                    if !modified_files.contains(&file_path) {
                        modified_files.push(file_path.clone());
                    }
                    info!(
                        edits_count = owned_edits.len(),
                        file_path = %file_path,
                        "Successfully applied edits to file"
                    );
                }
                Err(e) => {
                    error!(
                        file_path = %file_path,
                        error = %e,
                        "Failed to apply edits to file content"
                    );
                    // Rollback all changes and return error
                    self.rollback_from_snapshots(&snapshots).await?;
                    return Err(ServerError::Internal(format!(
                        "Failed to apply edits to file {}: {}. All changes have been rolled back.",
                        file_path, e
                    )));
                }
            }
            // Guard is dropped here, releasing the lock
        }

        // Step 4: Apply dependency updates to other files with locking
        for dep_update in &plan.dependency_updates {
            let target_file = self.to_absolute_path(Path::new(&dep_update.target_file));
            let file_lock = self.lock_manager.get_lock(&target_file).await;
            let _guard = file_lock.write().await;

            match self.apply_dependency_update(&target_file, dep_update).await {
                Ok(changed) => {
                    if changed && !modified_files.contains(&dep_update.target_file) {
                        modified_files.push(dep_update.target_file.clone());
                        info!(target_file = %dep_update.target_file, "Applied dependency update");
                    }
                }
                Err(e) => {
                    error!(
                        target_file = %dep_update.target_file,
                        error = %e,
                        "Failed to apply dependency update"
                    );
                    // Rollback all changes and return error
                    self.rollback_from_snapshots(&snapshots).await?;
                    return Err(ServerError::Internal(format!(
                        "Failed to apply dependency update to {}: {}. All changes have been rolled back.",
                        dep_update.target_file, e
                    )));
                }
            }
            // Guard is dropped here after each file
        }

        // Step 5: Invalidate AST cache for all modified files
        for file_path in &modified_files {
            let abs_path = self.to_absolute_path(Path::new(file_path));
            self.ast_cache.invalidate(&abs_path);
            debug!(file_path = %file_path, "Invalidated AST cache");
        }

        // Step 6: All operations successful - snapshots can be dropped
        info!(
            modified_files_count = modified_files.len(),
            "Edit plan completed successfully with atomic guarantees"
        );

        Ok(EditPlanResult {
            success: true,
            modified_files,
            errors: None,
            plan_metadata: plan.metadata.clone(),
        })
    }

    /// Create snapshots of file contents before modification
    async fn create_file_snapshots(
        &self,
        file_paths: &std::collections::HashSet<PathBuf>,
    ) -> ServerResult<HashMap<PathBuf, String>> {
        let mut snapshots = HashMap::new();

        for file_path in file_paths {
            // Acquire a read lock to ensure no other task modifies the file
            // while we are creating the snapshot. This prevents race conditions
            // where concurrent edits could truncate or modify files during snapshot.
            let file_lock = self.lock_manager.get_lock(file_path).await;
            let _guard = file_lock.read().await;

            // Open file with explicit handle and force cache drop
            let read_result = async {
                use tokio::io::AsyncReadExt;
                let mut file = fs::OpenOptions::new().read(true).open(file_path).await?;

                // Force page cache invalidation on Unix systems
                #[cfg(unix)]
                {
                    use std::os::unix::io::AsRawFd;
                    unsafe {
                        // POSIX_FADV_DONTNEED = 4
                        libc::posix_fadvise(file.as_raw_fd(), 0, 0, 4);
                    }
                }

                let mut content = String::new();
                file.read_to_string(&mut content).await?;

                // DEBUG: Log if we read empty content
                if content.is_empty() {
                    eprintln!(
                        "CACHE BUG: Read {} as EMPTY (should have content)!",
                        file_path.display()
                    );
                    // Try ONE more time with explicit sync
                    drop(file);
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                    let mut retry_file = fs::File::open(file_path).await?;
                    let mut retry_content = String::new();
                    retry_file.read_to_string(&mut retry_content).await?;
                    if !retry_content.is_empty() {
                        eprintln!(
                            "CACHE BUG CONFIRMED: Retry read {} bytes!",
                            retry_content.len()
                        );
                        return Ok(retry_content);
                    }
                }

                Ok::<String, std::io::Error>(content)
            }
            .await;

            match read_result {
                Ok(content) => {
                    debug!(
                        file_path = %file_path.display(),
                        content_len = content.len(),
                        "Snapshot created with content"
                    );
                    // DEBUG: Check line structure
                    let lines: Vec<&str> = content.lines().collect();
                    if lines.len() > 1 {
                        eprintln!(
                            "DEBUG SNAPSHOT: {} - line_count={}, line[0].len={}, line[1].len={}",
                            file_path.display(),
                            lines.len(),
                            lines.first().map(|l| l.len()).unwrap_or(0),
                            lines.get(1).map(|l| l.len()).unwrap_or(0)
                        );
                    }
                    snapshots.insert(file_path.clone(), content);
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    // File doesn't exist yet - store empty string to indicate deletion on rollback
                    debug!(
                        file_path = %file_path.display(),
                        "File does not exist yet, will be created during operation"
                    );
                    snapshots.insert(file_path.clone(), String::new());
                }
                Err(e) => {
                    return Err(ServerError::Internal(format!(
                        "Failed to read file {} for snapshot: {}",
                        file_path.display(),
                        e
                    )));
                }
            }
        }

        Ok(snapshots)
    }

    /// Rollback all file modifications using snapshots
    async fn rollback_from_snapshots(
        &self,
        snapshots: &HashMap<PathBuf, String>,
    ) -> ServerResult<()> {
        warn!(
            files_count = snapshots.len(),
            "Rolling back file modifications"
        );

        let mut rollback_errors = Vec::new();

        for (file_path, original_content) in snapshots {
            if original_content.is_empty() {
                // File didn't exist before, so delete it if it was created
                if file_path.exists() {
                    if let Err(e) = fs::remove_file(file_path).await {
                        rollback_errors.push(format!(
                            "Failed to remove file {} during rollback: {}",
                            file_path.display(),
                            e
                        ));
                    } else {
                        debug!(
                            file_path = %file_path.display(),
                            "Removed newly created file during rollback"
                        );
                    }
                }
            } else {
                // Restore original content
                if let Err(e) = fs::write(file_path, original_content).await {
                    rollback_errors.push(format!(
                        "Failed to restore file {} during rollback: {}",
                        file_path.display(),
                        e
                    ));
                } else {
                    debug!(
                        file_path = %file_path.display(),
                        "Restored original content during rollback"
                    );
                }
            }

            // Invalidate AST cache for rolled-back file
            self.ast_cache.invalidate(file_path);
        }

        if !rollback_errors.is_empty() {
            error!(
                error_count = rollback_errors.len(),
                errors = %rollback_errors.join("; "),
                "Encountered errors during rollback"
            );
            return Err(ServerError::Internal(format!(
                "Rollback partially failed: {}",
                rollback_errors.join("; ")
            )));
        }

        info!("Successfully rolled back all file modifications");
        Ok(())
    }

    /// Apply text edits to a single file
    /// Apply edits to file content and return the modified content (synchronous, no I/O)
    fn apply_edits_to_content(
        &self,
        original_content: &str,
        edits: &[TextEdit],
    ) -> ServerResult<String> {
        if edits.is_empty() {
            return Ok(original_content.to_string());
        }

        // DEBUG: Log content length to diagnose empty content issue
        if original_content.is_empty() {
            error!(
                edits_count = edits.len(),
                "BUG: apply_edits_to_content called with EMPTY content! First edit: {:?}",
                edits.first()
            );
        }

        // Sort edits by position (highest line/column first) to avoid offset issues
        // For multi-line edits, we need to consider end_line to ensure proper ordering
        let mut sorted_edits = edits.to_vec();
        sorted_edits.sort_by(|a, b| {
            // Primary sort: by end_line (descending) - apply edits that end later first
            let end_line_cmp = b.location.end_line.cmp(&a.location.end_line);
            if end_line_cmp != std::cmp::Ordering::Equal {
                return end_line_cmp;
            }

            // Secondary sort: by start_line (descending)
            let start_line_cmp = b.location.start_line.cmp(&a.location.start_line);
            if start_line_cmp != std::cmp::Ordering::Equal {
                return start_line_cmp;
            }

            // Tertiary sort: by start_column (descending)
            b.location.start_column.cmp(&a.location.start_column)
        });

        // Apply edits from end to beginning to preserve positions
        let mut modified_content = original_content.to_string();
        for edit in sorted_edits.iter() {
            modified_content = self.apply_single_edit(&modified_content, edit)?;
        }

        Ok(modified_content)
    }

    /// Legacy wrapper for apply_edits_to_content that reads from file and writes back
    /// Used for backward compatibility with existing code
    #[allow(dead_code)]
    async fn apply_file_edits(&self, file_path: &Path, edits: &[TextEdit]) -> ServerResult<()> {
        if edits.is_empty() {
            return Ok(());
        }

        // Read current file content
        let content = match fs::read_to_string(file_path).await {
            Ok(content) => content,
            Err(e) => {
                return Err(ServerError::Internal(format!(
                    "Failed to read file {}: {}",
                    file_path.display(),
                    e
                )));
            }
        };

        // Apply edits using the new function
        let modified_content = self.apply_edits_to_content(&content, edits)?;

        // Write modified content back to file
        fs::write(file_path, modified_content).await.map_err(|e| {
            ServerError::Internal(format!(
                "Failed to write file {}: {}",
                file_path.display(),
                e
            ))
        })?;

        Ok(())
    }

    /// Apply a single text edit to content
    ///
    /// This function correctly handles both single-line and multi-line edits by:
    /// 1. Preserving all lines before the edit region
    /// 2. Constructing the edited line(s) correctly
    /// 3. Preserving all lines after the edit region
    /// 4. Maintaining original file's trailing newline behavior
    fn apply_single_edit(&self, content: &str, edit: &TextEdit) -> ServerResult<String> {
        let original_had_newline = content.ends_with('\n');
        let lines: Vec<&str> = content.lines().collect();

        debug!(
            start_line = edit.location.start_line,
            start_col = edit.location.start_column,
            end_line = edit.location.end_line,
            end_col = edit.location.end_column,
            total_lines = lines.len(),
            new_text_len = edit.new_text.len(),
            new_text_has_newlines = edit.new_text.contains('\n'),
            "Applying single text edit"
        );

        // Validate edit location
        if edit.location.start_line as usize >= lines.len() {
            return Err(ServerError::InvalidRequest(format!(
                "Edit location line {} is beyond file length {}",
                edit.location.start_line,
                lines.len()
            )));
        }

        // Special case: Full-file replacement
        // When an edit replaces the entire file (start=0,0 and end=last_line,last_col),
        // and new_text contains the complete file content with embedded newlines,
        // we should use new_text directly instead of trying to splice it line-by-line.
        if edit.location.start_line == 0
            && edit.location.start_column == 0
            && edit.location.end_line as usize == lines.len().saturating_sub(1)
        {
            let last_line_len = lines.last().map(|l| l.chars().count()).unwrap_or(0);
            if edit.location.end_column as usize >= last_line_len {
                debug!(
                    original_lines = lines.len(),
                    new_text_lines = edit.new_text.lines().count(),
                    "Detected full-file replacement, using new_text directly"
                );

                let mut final_content = edit.new_text.clone();

                // Preserve original file's trailing newline behavior
                if original_had_newline && !final_content.ends_with('\n') {
                    final_content.push('\n');
                } else if !original_had_newline && final_content.ends_with('\n') {
                    // Remove trailing newline if original didn't have one
                    final_content = final_content.trim_end_matches('\n').to_string();
                }

                debug!(
                    final_lines = final_content.lines().count(),
                    has_trailing_newline = final_content.ends_with('\n'),
                    "Full-file replacement applied successfully"
                );

                return Ok(final_content);
            }
        }

        if edit.location.end_line as usize >= lines.len() {
            return Err(ServerError::InvalidRequest(format!(
                "Edit end line {} is beyond file length {}",
                edit.location.end_line,
                lines.len()
            )));
        }

        let mut result = Vec::new();

        // Step 1: Copy all lines BEFORE the edit region (unchanged)
        for i in 0..(edit.location.start_line as usize) {
            result.push(lines[i].to_string());
        }

        // Step 2: Construct the edited line(s)
        let start_line_idx = edit.location.start_line as usize;
        let end_line_idx = edit.location.end_line as usize;
        let start_line = lines[start_line_idx];
        let start_line_chars: Vec<char> = start_line.chars().collect();
        let start_col = edit.location.start_column as usize;

        // Validate start column
        if start_col > start_line_chars.len() {
            return Err(ServerError::InvalidRequest(format!(
                "Edit start column {} is beyond line length {}",
                start_col,
                start_line_chars.len()
            )));
        }

        if edit.location.start_line == edit.location.end_line {
            // CASE A: Single-line edit
            let end_col = edit.location.end_column as usize;

            // Validate end column
            if end_col > start_line_chars.len() {
                return Err(ServerError::InvalidRequest(format!(
                    "Edit end column {} is beyond line length {}",
                    end_col,
                    start_line_chars.len()
                )));
            }

            // Build: [before edit] + [new text] + [after edit]
            let mut edited_line = String::new();
            edited_line.push_str(&start_line_chars[..start_col].iter().collect::<String>());
            edited_line.push_str(&edit.new_text);
            if end_col <= start_line_chars.len() {
                edited_line.push_str(&start_line_chars[end_col..].iter().collect::<String>());
            }
            result.push(edited_line);
        } else {
            // CASE B: Multi-line edit (spans multiple lines)
            let end_line = lines[end_line_idx];
            let end_line_chars: Vec<char> = end_line.chars().collect();
            let end_col = edit.location.end_column as usize;

            // Validate end column
            if end_col > end_line_chars.len() {
                return Err(ServerError::InvalidRequest(format!(
                    "Edit end column {} is beyond end line length {}",
                    end_col,
                    end_line_chars.len()
                )));
            }

            // Build: [prefix from start line] + [new text] + [suffix from end line]
            let mut edited_line = String::new();
            edited_line.push_str(&start_line_chars[..start_col].iter().collect::<String>());
            edited_line.push_str(&edit.new_text);
            if end_col <= end_line_chars.len() {
                edited_line.push_str(&end_line_chars[end_col..].iter().collect::<String>());
            }
            result.push(edited_line);
        }

        // Step 3: Copy all lines AFTER the edit region (unchanged)
        for i in (end_line_idx + 1)..lines.len() {
            result.push(lines[i].to_string());
        }

        // Step 4: Reconstruct final content with proper newline handling
        let mut final_content = result.join("\n");

        // Preserve original file's trailing newline behavior
        if original_had_newline && !final_content.is_empty() && !final_content.ends_with('\n') {
            final_content.push('\n');
        }

        debug!(
            result_lines = result.len(),
            has_trailing_newline = final_content.ends_with('\n'),
            "Text edit applied successfully"
        );

        Ok(final_content)
    }

    /// Apply a dependency update (import/export change) to a file
    async fn apply_dependency_update(
        &self,
        file_path: &Path,
        update: &DependencyUpdate,
    ) -> ServerResult<bool> {
        // Delegate the dependency update to the import service, which handles AST transformations.
        self.import_service
            .update_import_reference(file_path, update)
            .await
            .map_err(|e| {
                error!(
                    file_path = %file_path.display(),
                    error = %e,
                    "AST-based dependency update failed"
                );
                ServerError::Internal(format!("Failed to apply dependency update: {}", e))
            })
    }

    /// Convert a path to absolute path within the project
    fn to_absolute_path(&self, path: &Path) -> PathBuf {
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.project_root.join(path)
        }
    }

    /// Consolidate a Rust package into a target directory
    ///
    /// This function moves source code from old_package_path to new_package_path,
    /// merges Cargo.toml dependencies, removes the old crate from workspace members,
    /// and automatically updates all import statements across the workspace.
    async fn consolidate_rust_package(
        &self,
        old_package_path: &Path,
        new_package_path: &Path,
        dry_run: bool,
    ) -> ServerResult<DryRunnable<Value>> {
        info!(
            old_path = ?old_package_path,
            new_path = ?new_package_path,
            dry_run,
            "Consolidating Rust package"
        );

        let old_abs = self.to_absolute_path(old_package_path);
        let new_abs = self.to_absolute_path(new_package_path);

        // Validate that old_path is a Cargo package
        if !self.is_cargo_package(&old_abs).await? {
            return Err(ServerError::InvalidRequest(format!(
                "Source directory is not a Cargo package: {:?}",
                old_abs
            )));
        }

        let old_src_dir = old_abs.join("src");
        if !old_src_dir.exists() {
            return Err(ServerError::NotFound(format!(
                "Source directory does not have a src/ folder: {:?}",
                old_abs
            )));
        }

        if dry_run {
            // In dry run mode, don't create directories
            // Preview mode - return what would happen
            let old_cargo_toml = old_abs.join("Cargo.toml");
            let new_cargo_toml = new_abs.join("Cargo.toml");

            // Calculate rename info for preview
            let rename_info = self
                .extract_consolidation_rename_info(&old_abs, &new_abs)
                .await?;
            let old_crate_name = rename_info["old_crate_name"].as_str().unwrap_or("unknown");
            let new_import_prefix = rename_info["new_import_prefix"]
                .as_str()
                .unwrap_or("unknown");
            let submodule_name = rename_info["submodule_name"].as_str().unwrap_or("unknown");
            let target_crate_name = rename_info["target_crate_name"]
                .as_str()
                .unwrap_or("unknown");

            return Ok(DryRunnable::new(
                true,
                json!({
                    "operation": "consolidate_rust_package",
                    "old_path": old_abs.to_string_lossy(),
                    "new_path": new_abs.to_string_lossy(),
                    "actions": {
                        "move_src": format!("{} -> {}", old_src_dir.display(), new_abs.display()),
                        "merge_dependencies": format!("{} -> {}", old_cargo_toml.display(), new_cargo_toml.display()),
                        "remove_from_workspace": "Remove old crate from workspace members",
                        "update_imports": format!("use {}::... → use {}::...", old_crate_name, new_import_prefix),
                        "delete_old_crate": format!("Delete {}", old_abs.display())
                    },
                    "import_changes": {
                        "old_crate": old_crate_name,
                        "new_prefix": new_import_prefix,
                        "submodule": submodule_name,
                        "target_crate": target_crate_name
                    },
                    "next_steps": format!("After consolidation, add 'pub mod {};' to {}/src/lib.rs", submodule_name, target_crate_name),
                    "note": "This is a dry run. No changes will be made."
                }),
            ));
        }

        // Execution mode
        // Calculate rename info before moving files
        let rename_info = self
            .extract_consolidation_rename_info(&old_abs, &new_abs)
            .await?;
        let old_crate_name = rename_info["old_crate_name"]
            .as_str()
            .unwrap_or("unknown")
            .to_string();
        let new_import_prefix = rename_info["new_import_prefix"]
            .as_str()
            .unwrap_or("unknown")
            .to_string();
        let submodule_name = rename_info["submodule_name"]
            .as_str()
            .unwrap_or("unknown")
            .to_string();
        let target_crate_name = rename_info["target_crate_name"]
            .as_str()
            .unwrap_or("unknown")
            .to_string();

        info!(
            old_crate = %old_crate_name,
            new_prefix = %new_import_prefix,
            submodule = %submodule_name,
            "Calculated consolidation rename info"
        );

        // Step 1: Move src files to target directory
        let mut moved_files = Vec::new();
        let walker = ignore::WalkBuilder::new(&old_src_dir).hidden(false).build();
        for entry in walker.flatten() {
            let path = entry.path();
            if path.is_file() {
                let relative_path = path.strip_prefix(&old_src_dir).unwrap();
                let target_path = new_abs.join(relative_path);

                // Ensure parent directory exists
                if let Some(parent) = target_path.parent() {
                    fs::create_dir_all(parent).await.map_err(|e| {
                        ServerError::Internal(format!("Failed to create directory: {}", e))
                    })?;
                }

                // Move the file
                fs::rename(path, &target_path)
                    .await
                    .map_err(|e| ServerError::Internal(format!("Failed to move file: {}", e)))?;

                moved_files.push(target_path.to_string_lossy().to_string());
            }
        }

        info!(files_moved = moved_files.len(), "Moved source files");

        // Step 2: Merge Cargo.toml dependencies
        // Find the parent crate's Cargo.toml (traverse up from new_abs)
        let old_cargo_toml = old_abs.join("Cargo.toml");
        let target_cargo_toml = self.find_parent_cargo_toml(&new_abs).await?;

        if let Some(target_toml_path) = target_cargo_toml {
            info!(
                source = ?old_cargo_toml,
                target = ?target_toml_path,
                "Merging dependencies"
            );
            self.merge_cargo_dependencies(&old_cargo_toml, &target_toml_path)
                .await?;
        } else {
            warn!("Could not find target crate's Cargo.toml for dependency merging");
        }

        // Step 3: Remove old crate from workspace members
        if let Err(e) = self.remove_from_workspace_members(&old_abs).await {
            warn!(error = %e, "Failed to update workspace manifest");
        }

        // Step 3.5: Update all workspace Cargo.toml files that depend on the old crate
        // IMPORTANT: Must happen BEFORE deleting the old crate directory
        let old_crate_name_for_deps = old_crate_name.replace('_', "-"); // Cargo.toml uses hyphens
        let target_crate_name_for_deps = target_crate_name.replace('_', "-"); // Cargo.toml uses hyphens

        // Find target crate root by walking up from new_abs to find Cargo.toml with [package]
        let mut target_crate_root = None;
        let mut current = new_abs.as_path();
        while let Some(parent) = current.parent() {
            let cargo_toml = parent.join("Cargo.toml");
            if cargo_toml.exists() {
                if let Ok(content) = fs::read_to_string(&cargo_toml).await {
                    if content.contains("[package]") {
                        target_crate_root = Some(parent.to_path_buf());
                        break;
                    }
                }
            }
            current = parent;
        }

        let target_crate_path = target_crate_root.ok_or_else(|| {
            ServerError::Internal("Could not find target crate root directory".to_string())
        })?;

        match self
            .update_workspace_cargo_dependencies(
                &old_abs,
                &target_crate_path,
                &target_crate_name_for_deps,
                &old_crate_name_for_deps,
            )
            .await
        {
            Ok(updated_count) => {
                info!(
                    updated_files = updated_count,
                    old_crate = %old_crate_name_for_deps,
                    new_crate = %target_crate_name,
                    "Updated workspace Cargo.toml dependencies"
                );
            }
            Err(e) => {
                warn!(
                    error = %e,
                    old_crate = %old_crate_name_for_deps,
                    "Failed to update some workspace Cargo.toml files, but continuing with consolidation"
                );
            }
        }

        // Step 4: Delete the old crate directory
        fs::remove_dir_all(&old_abs).await.map_err(|e| {
            ServerError::Internal(format!("Failed to delete old crate directory: {}", e))
        })?;

        info!("Old crate directory deleted, starting import updates");

        // Step 5: Update all imports across the workspace
        let mut total_imports_updated = 0;
        let mut files_with_import_updates = Vec::new();

        // Use a "virtual" old file path for the import service
        // This represents the old crate's "entry point" for import resolution
        let virtual_old_path = old_abs.join("src/lib.rs");
        let virtual_new_path = new_abs.join("lib.rs");

        match self
            .import_service
            .update_imports_for_rename(
                &virtual_old_path,
                &virtual_new_path,
                Some(&rename_info),
                false,
                Some(cb_plugin_api::ScanScope::AllUseStatements),
            )
            .await
        {
            Ok(edit_plan) => {
                info!(
                    edits_planned = edit_plan.edits.len(),
                    "Created import update plan"
                );

                // Apply the edit plan
                match self.apply_edit_plan(&edit_plan).await {
                    Ok(result) => {
                        total_imports_updated = edit_plan.edits.len();
                        files_with_import_updates = result.modified_files;
                        info!(
                            imports_updated = total_imports_updated,
                            files_modified = files_with_import_updates.len(),
                            "Successfully updated imports"
                        );
                    }
                    Err(e) => {
                        warn!(error = %e, "Failed to apply import updates, but consolidation was successful");
                    }
                }
            }
            Err(e) => {
                warn!(error = %e, "Failed to create import update plan, but consolidation was successful");
            }
        }

        // Step 6: Log lib.rs suggestion
        let lib_rs_path = format!("{}/src/lib.rs", target_crate_name);
        let suggestion = format!(
            "📝 Next step: Add 'pub mod {};' to {} to make the consolidated module public",
            submodule_name, lib_rs_path
        );
        info!(suggestion = %suggestion, "Consolidation complete");

        info!(
            old_path = ?old_abs,
            new_path = ?new_abs,
            files_moved = moved_files.len(),
            imports_updated = total_imports_updated,
            "Consolidation complete"
        );

        Ok(DryRunnable::new(
            false,
            json!({
                "operation": "consolidate_rust_package",
                "success": true,
                "old_path": old_abs.to_string_lossy(),
                "new_path": new_abs.to_string_lossy(),
                "files_moved": moved_files.len(),
                "import_updates": {
                    "old_crate": old_crate_name,
                    "new_prefix": new_import_prefix,
                    "imports_updated": total_imports_updated,
                    "files_modified": files_with_import_updates.len(),
                    "modified_files": files_with_import_updates,
                },
                "next_steps": suggestion,
                "note": format!("Consolidation complete! All imports have been automatically updated from '{}' to '{}'.", old_crate_name, new_import_prefix)
            }),
        ))
    }

    /// Merge Cargo.toml dependencies from source to target
    async fn merge_cargo_dependencies(
        &self,
        source_toml_path: &Path,
        target_toml_path: &Path,
    ) -> ServerResult<()> {
        use toml_edit::DocumentMut;

        // Read both TOML files
        let source_content = fs::read_to_string(source_toml_path).await.map_err(|e| {
            ServerError::Internal(format!("Failed to read source Cargo.toml: {}", e))
        })?;

        let target_content = fs::read_to_string(target_toml_path).await.map_err(|e| {
            ServerError::Internal(format!("Failed to read target Cargo.toml: {}", e))
        })?;

        // Parse both documents
        let source_doc = source_content.parse::<DocumentMut>().map_err(|e| {
            ServerError::Internal(format!("Failed to parse source Cargo.toml: {}", e))
        })?;

        let mut target_doc = target_content.parse::<DocumentMut>().map_err(|e| {
            ServerError::Internal(format!("Failed to parse target Cargo.toml: {}", e))
        })?;

        let mut merged_count = 0;
        let mut conflict_count = 0;

        // Extract target crate name for circular dependency detection (before any mutable borrows)
        let target_crate_name = target_doc
            .get("package")
            .and_then(|p| p.get("name"))
            .and_then(|n| n.as_str())
            .unwrap_or("unknown")
            .to_string(); // Clone the string to avoid borrow conflicts

        // Merge [dependencies], [dev-dependencies], and [build-dependencies]
        for section in ["dependencies", "dev-dependencies", "build-dependencies"] {
            if let Some(source_deps) = source_doc.get(section).and_then(|v| v.as_table()) {
                // Ensure target has this section
                if target_doc.get(section).is_none() {
                    target_doc[section] = toml_edit::Item::Table(toml_edit::Table::new());
                }

                if let Some(target_deps) = target_doc[section].as_table_mut() {

                    for (dep_name, dep_value) in source_deps.iter() {
                        // Check for self-dependency
                        if dep_name == target_crate_name.as_str() {
                            warn!(
                                dependency = %dep_name,
                                section = %section,
                                target_crate = %target_crate_name,
                                "Skipping self-dependency (would create circular dependency)"
                            );
                            conflict_count += 1;
                            continue;
                        }

                        // Check for circular dependency
                        // If source depends on X, and target also depends on X, that's OK
                        // But if source depends on target's parent crate, that would be circular
                        if self.would_create_circular_dependency(dep_name, &target_crate_name) {
                            warn!(
                                dependency = %dep_name,
                                section = %section,
                                target_crate = %target_crate_name,
                                "Skipping dependency to avoid circular dependency"
                            );
                            conflict_count += 1;
                            continue;
                        }

                        if target_deps.contains_key(dep_name) {
                            warn!(
                                dependency = %dep_name,
                                section = %section,
                                "Dependency already exists in target, skipping"
                            );
                            conflict_count += 1;
                        } else {
                            target_deps.insert(dep_name, dep_value.clone());
                            info!(
                                dependency = %dep_name,
                                section = %section,
                                "Merged dependency"
                            );
                            merged_count += 1;
                        }
                    }
                }
            }
        }

        // Write the updated target TOML
        fs::write(target_toml_path, target_doc.to_string())
            .await
            .map_err(|e| {
                ServerError::Internal(format!("Failed to write target Cargo.toml: {}", e))
            })?;

        info!(
            merged = merged_count,
            conflicts = conflict_count,
            "Dependency merge complete"
        );

        Ok(())
    }

    /// Check if adding a dependency would create a circular dependency
    ///
    /// This is a simplified check that detects obvious circular dependencies:
    /// - Source depends on target (self-dependency after merge)
    /// - Source depends on a crate that depends on target (one-level circular)
    ///
    /// # Arguments
    ///
    /// * `dep_name` - Name of the dependency being added
    /// * `target_crate_name` - Name of the crate receiving the dependency
    ///
    /// # Returns
    ///
    /// `true` if adding this dependency would create a circular dependency
    fn would_create_circular_dependency(&self, dep_name: &str, target_crate_name: &str) -> bool {
        // Simple heuristic checks:

        // 1. Direct circular: dependency matches target
        if dep_name == target_crate_name {
            return true;
        }

        // 2. Known parent-child relationships in this codebase
        // For example: cb-core is a base crate that many others depend on
        // If we're merging into cb-types, and source depends on cb-core,
        // we need to check if cb-core depends on cb-types (which it does in this codebase)
        let known_circular_patterns = [
            // (dependency, target) pairs that would create circular deps
            ("cb-core", "cb-types"),     // cb-core -> cb-types -> cb-core
            ("cb-types", "cb-protocol"), // cb-types -> cb-protocol -> cb-types
            ("cb-types", "cb-core"),     // cb-types -> cb-core -> cb-types
        ];

        for (dep, target) in &known_circular_patterns {
            if dep_name == *dep && target_crate_name == *target {
                return true;
            }
        }

        // Could be extended with full dependency graph analysis,
        // but this simple check catches the most common cases
        false
    }

    /// Update all workspace Cargo.toml files that reference the old crate
    ///
    /// This scans all Cargo.toml files in the workspace and replaces dependencies
    /// on the old crate with dependencies on the target crate.
    ///
    /// # Arguments
    ///
    /// * `old_crate_path` - Path to the old crate directory (to derive old crate name)
    /// * `target_crate_path` - Path to the target crate directory
    /// * `target_crate_name` - Name of the target crate to use as replacement
    /// * `old_crate_name` - Name of the old crate (with hyphens, as appears in Cargo.toml)
    ///
    /// # Returns
    ///
    /// Number of Cargo.toml files successfully updated
    async fn update_workspace_cargo_dependencies(
        &self,
        old_crate_path: &Path,
        target_crate_path: &Path,
        target_crate_name: &str,
        old_crate_name: &str,
    ) -> ServerResult<usize> {
        use toml_edit::DocumentMut;

        info!(
            old_crate = %old_crate_name,
            target_crate = %target_crate_name,
            target_crate_path = ?target_crate_path,
            "Scanning workspace for Cargo.toml files with dependencies on old crate"
        );

        let mut updated_count = 0;
        let mut checked_count = 0;

        // Find all Cargo.toml files in the workspace
        let walker = ignore::WalkBuilder::new(&self.project_root)
            .hidden(false)
            .build();

        for entry in walker.flatten() {
            let path = entry.path();

            // Only process Cargo.toml files
            if path.file_name() != Some(std::ffi::OsStr::new("Cargo.toml")) {
                continue;
            }

            // Skip the old crate's Cargo.toml (it's being deleted anyway)
            if path.starts_with(old_crate_path) {
                continue;
            }

            checked_count += 1;

            // Read the Cargo.toml file
            let content = match fs::read_to_string(path).await {
                Ok(c) => c,
                Err(e) => {
                    warn!(
                        file = ?path,
                        error = %e,
                        "Failed to read Cargo.toml"
                    );
                    continue;
                }
            };

            // Check if this file references the old crate
            if !content.contains(old_crate_name) {
                continue;
            }

            // Parse the TOML document
            let mut doc = match content.parse::<DocumentMut>() {
                Ok(d) => d,
                Err(e) => {
                    warn!(
                        file = ?path,
                        error = %e,
                        "Failed to parse Cargo.toml"
                    );
                    continue;
                }
            };

            let mut file_modified = false;

            // Update dependencies in all relevant sections
            for section in ["dependencies", "dev-dependencies", "build-dependencies"] {
                if let Some(deps) = doc.get_mut(section).and_then(|v| v.as_table_mut()) {
                    if deps.contains_key(old_crate_name) {
                        // Remove old dependency
                        let old_dep_value = deps.remove(old_crate_name);

                        // Check if target crate dependency already exists
                        if deps.contains_key(target_crate_name) {
                            info!(
                                file = ?path,
                                section = %section,
                                target_crate = %target_crate_name,
                                old_crate = %old_crate_name,
                                "Target crate already exists, removed old crate"
                            );
                        } else {
                            // Add target crate dependency
                            // Derive the path to the target crate
                            if let Some(old_dep) = old_dep_value {
                                // Clone the dependency spec and update the path
                                let mut new_dep = old_dep.clone();

                                // If it's a table with a path, update the path
                                if let Some(dep_table) = new_dep.as_inline_table_mut() {
                                    if dep_table.contains_key("path") {
                                        // Calculate relative path from this Cargo.toml to target crate
                                        let this_cargo_dir = path.parent().unwrap();
                                        if let Some(rel_path) =
                                            pathdiff::diff_paths(target_crate_path, this_cargo_dir)
                                        {
                                            let path_str = rel_path.to_string_lossy().to_string();
                                            dep_table.insert(
                                                "path",
                                                toml_edit::Value::from(path_str),
                                            );
                                        }
                                    }
                                } else if let Some(dep_table) = new_dep.as_table_mut() {
                                    if dep_table.contains_key("path") {
                                        // Same logic for regular tables
                                        let this_cargo_dir = path.parent().unwrap();
                                        if let Some(rel_path) =
                                            pathdiff::diff_paths(target_crate_path, this_cargo_dir)
                                        {
                                            let path_str = rel_path.to_string_lossy().to_string();
                                            dep_table.insert("path", toml_edit::value(path_str));
                                        }
                                    }
                                }

                                deps.insert(target_crate_name, new_dep);
                            }

                            info!(
                                file = ?path,
                                section = %section,
                                old_crate = %old_crate_name,
                                new_crate = %target_crate_name,
                                "Replaced dependency"
                            );
                        }

                        file_modified = true;
                    }
                }
            }

            // Write back if modified
            if file_modified {
                match fs::write(path, doc.to_string()).await {
                    Ok(_) => {
                        info!(
                            file = ?path,
                            "Updated Cargo.toml dependencies"
                        );
                        updated_count += 1;
                    }
                    Err(e) => {
                        error!(
                            file = ?path,
                            error = %e,
                            "Failed to write updated Cargo.toml"
                        );
                    }
                }
            }
        }

        info!(
            checked = checked_count,
            updated = updated_count,
            "Workspace Cargo.toml dependency scan complete"
        );

        Ok(updated_count)
    }

    /// Find the path to a crate by its name in the workspace
    #[allow(dead_code)]
    async fn find_crate_path_by_name(&self, crate_name: &str) -> ServerResult<Option<PathBuf>> {
        let walker = ignore::WalkBuilder::new(&self.project_root)
            .max_depth(Some(3))
            .hidden(false)
            .build();

        for entry in walker.flatten() {
            let path = entry.path();

            if path.file_name() == Some(std::ffi::OsStr::new("Cargo.toml")) {
                if let Ok(content) = fs::read_to_string(path).await {
                    if let Ok(doc) = content.parse::<toml_edit::DocumentMut>() {
                        if let Some(name) = doc
                            .get("package")
                            .and_then(|p| p.get("name"))
                            .and_then(|n| n.as_str())
                        {
                            if name == crate_name {
                                return Ok(path.parent().map(|p| p.to_path_buf()));
                            }
                        }
                    }
                }
            }
        }

        Ok(None)
    }

    /// Remove a package path from workspace members in the root Cargo.toml
    async fn remove_from_workspace_members(&self, package_path: &Path) -> ServerResult<()> {
        use toml_edit::DocumentMut;

        // Find the workspace root
        let mut current_path = package_path.parent();

        while let Some(path) = current_path {
            let workspace_toml_path = path.join("Cargo.toml");
            if workspace_toml_path.exists() {
                let content = fs::read_to_string(&workspace_toml_path)
                    .await
                    .map_err(|e| {
                        ServerError::Internal(format!("Failed to read workspace Cargo.toml: {}", e))
                    })?;

                if content.contains("[workspace]") {
                    // Parse the workspace manifest
                    let mut doc = content.parse::<DocumentMut>().map_err(|e| {
                        ServerError::Internal(format!(
                            "Failed to parse workspace Cargo.toml: {}",
                            e
                        ))
                    })?;

                    // Calculate relative path from workspace root to package
                    let package_rel_path = package_path.strip_prefix(path).map_err(|_| {
                        ServerError::Internal("Failed to calculate relative path".to_string())
                    })?;

                    let package_rel_str = package_rel_path.to_string_lossy().to_string();

                    // Remove from workspace members
                    let should_write =
                        if let Some(members) = doc["workspace"]["members"].as_array_mut() {
                            let index_opt = members
                                .iter()
                                .position(|m| m.as_str() == Some(&package_rel_str));
                            if let Some(index) = index_opt {
                                members.remove(index);
                                true
                            } else {
                                false
                            }
                        } else {
                            false
                        };

                    if should_write {
                        // Write back
                        fs::write(&workspace_toml_path, doc.to_string())
                            .await
                            .map_err(|e| {
                                ServerError::Internal(format!(
                                    "Failed to write workspace Cargo.toml: {}",
                                    e
                                ))
                            })?;

                        info!(
                            workspace = ?workspace_toml_path,
                            removed_member = %package_rel_str,
                            "Removed package from workspace members"
                        );
                    }

                    return Ok(());
                }
            }

            if path == self.project_root {
                break;
            }
            current_path = path.parent();
        }

        Ok(())
    }

    /// Check if a directory is a Cargo package by looking for a Cargo.toml with a [package] section.
    async fn is_cargo_package(&self, dir: &Path) -> ServerResult<bool> {
        let cargo_toml_path = dir.join("Cargo.toml");
        if !cargo_toml_path.exists() {
            return Ok(false);
        }
        match fs::read_to_string(&cargo_toml_path).await {
            Ok(content) => Ok(content.contains("[package]")),
            Err(_) => Ok(false),
        }
    }

    /// Find the parent crate's Cargo.toml by traversing up from a directory
    ///
    /// When consolidating to `target_crate/src/source`, this finds `target_crate/Cargo.toml`
    async fn find_parent_cargo_toml(&self, start_path: &Path) -> ServerResult<Option<PathBuf>> {
        let mut current = start_path;

        while let Some(parent) = current.parent() {
            let cargo_toml = parent.join("Cargo.toml");
            if cargo_toml.exists() {
                // Check if it's a package (not just a workspace)
                if let Ok(content) = fs::read_to_string(&cargo_toml).await {
                    if content.contains("[package]") {
                        return Ok(Some(cargo_toml));
                    }
                }
            }

            // Stop at project root
            if parent == self.project_root {
                break;
            }

            current = parent;
        }

        Ok(None)
    }

    /// Extract consolidation rename information for import updating
    ///
    /// This calculates:
    /// - old_crate_name: The name from the old Cargo.toml
    /// - new_import_prefix: The new import path (e.g., "target_crate::submodule")
    /// - submodule_name: The name of the subdirectory that will contain the consolidated code
    /// - target_crate_name: The name of the target crate
    async fn extract_consolidation_rename_info(
        &self,
        old_package_path: &Path,
        new_package_path: &Path,
    ) -> ServerResult<serde_json::Value> {
        use serde_json::json;

        // Read the old Cargo.toml to get the old crate name
        let old_cargo_toml = old_package_path.join("Cargo.toml");
        let old_content = fs::read_to_string(&old_cargo_toml)
            .await
            .map_err(|e| ServerError::Internal(format!("Failed to read old Cargo.toml: {}", e)))?;

        let old_doc = old_content
            .parse::<toml_edit::DocumentMut>()
            .map_err(|e| ServerError::Internal(format!("Failed to parse old Cargo.toml: {}", e)))?;

        let old_crate_name = old_doc["package"]["name"]
            .as_str()
            .ok_or_else(|| {
                ServerError::Internal("Missing [package].name in old Cargo.toml".to_string())
            })?
            .replace('-', "_"); // Normalize to underscores for imports

        // Find the target crate by looking for Cargo.toml in parent directories
        let mut target_crate_name = String::new();
        let mut current = new_package_path;

        while let Some(parent) = current.parent() {
            let cargo_toml = parent.join("Cargo.toml");
            if cargo_toml.exists() {
                if let Ok(content) = fs::read_to_string(&cargo_toml).await {
                    if content.contains("[package]") {
                        // Found the target crate
                        if let Ok(doc) = content.parse::<toml_edit::DocumentMut>() {
                            if let Some(name) = doc["package"]["name"].as_str() {
                                target_crate_name = name.replace('-', "_");
                                break;
                            }
                        }
                    }
                }
            }
            current = parent;
        }

        if target_crate_name.is_empty() {
            return Err(ServerError::Internal(
                "Could not find target crate Cargo.toml".to_string(),
            ));
        }

        // Extract submodule name from the new path
        // e.g., "crates/cb-types/src/protocol" -> "protocol"
        let submodule_name = new_package_path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| ServerError::Internal("Invalid new directory path".to_string()))?
            .to_string();

        // Build the new import prefix
        // e.g., "cb_types::protocol"
        let new_import_prefix = format!("{}::{}", target_crate_name, submodule_name);

        info!(
            old_crate_name = %old_crate_name,
            new_import_prefix = %new_import_prefix,
            submodule_name = %submodule_name,
            target_crate_name = %target_crate_name,
            "Extracted consolidation rename information"
        );

        Ok(json!({
            "old_crate_name": old_crate_name,
            "new_crate_name": new_import_prefix.clone(), // For compatibility with update_imports_for_rename
            "new_import_prefix": new_import_prefix,
            "submodule_name": submodule_name,
            "target_crate_name": target_crate_name,
        }))
    }

    /// Extract Cargo package rename information for import rewriting
    async fn extract_cargo_rename_info(
        &self,
        old_package_path: &Path,
        new_package_path: &Path,
    ) -> ServerResult<serde_json::Value> {
        use serde_json::json;

        // Read the old Cargo.toml to get the old crate name
        let old_cargo_toml = old_package_path.join("Cargo.toml");
        let old_content = fs::read_to_string(&old_cargo_toml)
            .await
            .map_err(|e| ServerError::Internal(format!("Failed to read old Cargo.toml: {}", e)))?;

        let old_doc = old_content
            .parse::<toml_edit::DocumentMut>()
            .map_err(|e| ServerError::Internal(format!("Failed to parse old Cargo.toml: {}", e)))?;

        let old_crate_name = old_doc["package"]["name"]
            .as_str()
            .ok_or_else(|| {
                ServerError::Internal("Missing [package].name in Cargo.toml".to_string())
            })?
            .to_string();

        // Derive the new crate name from the new directory path
        // Convert directory name to valid crate name (replace hyphens with underscores for imports)
        let new_dir_name = new_package_path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| ServerError::Internal("Invalid new directory path".to_string()))?;

        // For Rust crate names: keep hyphens in package name but use underscores for imports
        let new_crate_name = new_dir_name.replace('_', "-"); // Normalize to hyphens for package name
        let new_crate_import = new_dir_name.replace('-', "_"); // Use underscores for use statements

        info!(
            old_crate_name = %old_crate_name,
            new_crate_name = %new_crate_name,
            new_crate_import = %new_crate_import,
            "Extracted Cargo rename information"
        );

        Ok(json!({
            "old_crate_name": old_crate_name.replace('-', "_"), // For Rust import updates (use statements)
            "old_package_name": old_crate_name, // For Cargo.toml dependency lookups (keep hyphens)
            "new_crate_name": new_crate_import, // Use underscores for imports
            "new_package_name": new_crate_name, // Keep hyphens for Cargo.toml
        }))
    }

    /// Find the parent Cargo workspace and update the members array to reflect a renamed package.
    ///
    /// Returns the list of Cargo.toml files that were updated (workspace root + moved package)
    async fn update_workspace_manifests(
        &self,
        old_package_path: &Path,
        new_package_path: &Path,
    ) -> ServerResult<Vec<PathBuf>> {
        let mut updated_files = Vec::new();
        let mut current_path = old_package_path.parent();

        while let Some(path) = current_path {
            let workspace_toml_path = path.join("Cargo.toml");
            if workspace_toml_path.exists() {
                let content = fs::read_to_string(&workspace_toml_path)
                    .await
                    .map_err(|e| {
                        ServerError::Internal(format!("Failed to read workspace Cargo.toml: {}", e))
                    })?;

                if content.contains("[workspace]") {
                    // This is the workspace root we need to modify.
                    let mut doc = content.parse::<toml_edit::DocumentMut>().map_err(|e| {
                        ServerError::Internal(format!(
                            "Failed to parse workspace Cargo.toml: {}",
                            e
                        ))
                    })?;

                    let old_rel_path = old_package_path.strip_prefix(path).map_err(|_| {
                        ServerError::Internal("Failed to calculate old relative path".to_string())
                    })?;
                    let new_rel_path = new_package_path.strip_prefix(path).map_err(|_| {
                        ServerError::Internal("Failed to calculate new relative path".to_string())
                    })?;

                    let old_path_str = old_rel_path.to_string_lossy().to_string();
                    let new_path_str = new_rel_path.to_string_lossy().to_string();

                    // Check if we need to update the workspace members
                    let members = doc["workspace"]["members"].as_array_mut().ok_or_else(|| {
                        ServerError::Internal(
                            "`[workspace.members]` is not a valid array".to_string(),
                        )
                    })?;

                    let index_opt = members
                        .iter()
                        .position(|m| m.as_str() == Some(&old_path_str));
                    if let Some(index) = index_opt {
                        members.remove(index);
                        members.push(new_path_str.as_str());

                        info!(
                            workspace = ?workspace_toml_path,
                            old = %old_path_str,
                            new = %new_path_str,
                            "Updated workspace members"
                        );

                        fs::write(&workspace_toml_path, doc.to_string())
                            .await
                            .map_err(|e| {
                                ServerError::Internal(format!(
                                    "Failed to write updated workspace Cargo.toml: {}",
                                    e
                                ))
                            })?;

                        updated_files.push(workspace_toml_path.clone());
                    }

                    // Also update relative path dependencies in the moved package's Cargo.toml
                    let package_cargo_toml = new_package_path.join("Cargo.toml");
                    if package_cargo_toml.exists() {
                        let package_updated = self.update_package_relative_paths(
                            &package_cargo_toml,
                            old_package_path,
                            new_package_path,
                            path,
                        )
                        .await?;

                        if package_updated {
                            updated_files.push(package_cargo_toml);
                        }
                    }

                    // If we found the workspace, we can stop searching.
                    return Ok(updated_files);
                }
            }

            if path == self.project_root {
                break;
            }
            current_path = path.parent();
        }

        Ok(updated_files)
    }

    /// Update path dependencies in other crates that depend on the moved crate
    ///
    /// When a crate moves, other crates with path dependencies need their paths updated.
    /// For example, if cb-lang-common moves from crates/languages/ to crates/, then
    /// cb-lang-go's Cargo.toml needs: path = "../cb-lang-common" → "../../cb-lang-common"
    async fn update_dependent_crate_paths(
        &self,
        moved_crate_name: &str,
        new_crate_path: &Path,
    ) -> ServerResult<Vec<PathBuf>> {
        let mut updated_files = Vec::new();

        // Find all Cargo.toml files in the workspace
        let walker = ignore::WalkBuilder::new(&self.project_root)
            .hidden(false)
            .build();

        for entry in walker.flatten() {
            let path = entry.path();
            if path.file_name() == Some(std::ffi::OsStr::new("Cargo.toml")) {
                // Skip the moved crate's own Cargo.toml
                if path.parent() == Some(new_crate_path) {
                    continue;
                }

                // Try to update this Cargo.toml if it depends on the moved crate
                match self.update_cargo_toml_dependency_path(path, moved_crate_name, new_crate_path).await {
                    Ok(true) => {
                        info!(cargo_toml = %path.display(), "Updated path dependency");
                        updated_files.push(path.to_path_buf());
                    }
                    Ok(false) => {
                        // File doesn't depend on the moved crate, skip
                    }
                    Err(e) => {
                        warn!(
                            error = %e,
                            cargo_toml = %path.display(),
                            "Failed to update dependency path"
                        );
                    }
                }
            }
        }

        Ok(updated_files)
    }

    /// Update a single Cargo.toml's path dependency if it depends on the moved crate
    ///
    /// Handles all dependency sections:
    /// - [dependencies], [dev-dependencies], [build-dependencies]
    /// - [target.'cfg(...)'.dependencies]
    /// - [workspace.dependencies]
    /// - [patch.crates-io], [patch.'...']
    ///
    /// Returns Ok(true) if the file was updated, Ok(false) if no update was needed
    async fn update_cargo_toml_dependency_path(
        &self,
        cargo_toml_path: &Path,
        moved_crate_name: &str,
        new_crate_path: &Path,
    ) -> ServerResult<bool> {
        let content = fs::read_to_string(cargo_toml_path)
            .await
            .map_err(|e| ServerError::Internal(format!("Failed to read Cargo.toml: {}", e)))?;

        let mut doc = content
            .parse::<toml_edit::DocumentMut>()
            .map_err(|e| ServerError::Internal(format!("Failed to parse Cargo.toml: {}", e)))?;

        let mut updated = false;
        let cargo_toml_dir = cargo_toml_path.parent().unwrap();

        // Helper to update a dependency table
        let update_dep_in_table = |dep: &mut toml_edit::Item, updated: &mut bool| -> ServerResult<()> {
            if let Some(dep_table) = dep.as_inline_table_mut() {
                if dep_table.contains_key("path") {
                    let new_rel_path = pathdiff::diff_paths(new_crate_path, cargo_toml_dir)
                        .ok_or_else(|| ServerError::Internal("Failed to calculate relative path".to_string()))?;
                    dep_table.insert("path", toml_edit::Value::from(new_rel_path.to_string_lossy().to_string()));
                    *updated = true;
                }
            } else if let Some(dep_table) = dep.as_table_mut() {
                if dep_table.contains_key("path") {
                    let new_rel_path = pathdiff::diff_paths(new_crate_path, cargo_toml_dir)
                        .ok_or_else(|| ServerError::Internal("Failed to calculate relative path".to_string()))?;
                    dep_table.insert("path", toml_edit::value(new_rel_path.to_string_lossy().to_string()));
                    *updated = true;
                }
            }
            Ok(())
        };

        // Check standard dependency sections
        for section in ["dependencies", "dev-dependencies", "build-dependencies"] {
            if let Some(deps) = doc.get_mut(section).and_then(|d| d.as_table_like_mut()) {
                if let Some(dep) = deps.get_mut(moved_crate_name) {
                    update_dep_in_table(dep, &mut updated)?;
                }
            }
        }

        // Check [workspace.dependencies]
        if let Some(workspace) = doc.get_mut("workspace").and_then(|w| w.as_table_mut()) {
            if let Some(deps) = workspace.get_mut("dependencies").and_then(|d| d.as_table_like_mut()) {
                if let Some(dep) = deps.get_mut(moved_crate_name) {
                    update_dep_in_table(dep, &mut updated)?;
                }
            }
        }

        // Check [target.'cfg(...)'.dependencies] sections
        if let Some(target) = doc.get_mut("target").and_then(|t| t.as_table_mut()) {
            for (_target_name, target_table) in target.iter_mut() {
                if let Some(target_table) = target_table.as_table_mut() {
                    for dep_section in ["dependencies", "dev-dependencies", "build-dependencies"] {
                        if let Some(deps) = target_table.get_mut(dep_section).and_then(|d| d.as_table_like_mut()) {
                            if let Some(dep) = deps.get_mut(moved_crate_name) {
                                update_dep_in_table(dep, &mut updated)?;
                            }
                        }
                    }
                }
            }
        }

        // Check [patch.crates-io] and [patch.'...'] sections
        if let Some(patch) = doc.get_mut("patch").and_then(|p| p.as_table_mut()) {
            for (_registry, registry_table) in patch.iter_mut() {
                if let Some(registry_table) = registry_table.as_table_like_mut() {
                    if let Some(dep) = registry_table.get_mut(moved_crate_name) {
                        update_dep_in_table(dep, &mut updated)?;
                    }
                }
            }
        }

        if updated {
            fs::write(cargo_toml_path, doc.to_string())
                .await
                .map_err(|e| ServerError::Internal(format!("Failed to write Cargo.toml: {}", e)))?;
        }

        Ok(updated)
    }

    /// Update relative `path` dependencies in a package's Cargo.toml after it moves
    async fn update_package_relative_paths(
        &self,
        package_cargo_toml: &Path,
        old_package_path: &Path,
        new_package_path: &Path,
        workspace_root: &Path,
    ) -> ServerResult<bool> {
        let content = fs::read_to_string(package_cargo_toml).await.map_err(|e| {
            ServerError::Internal(format!("Failed to read package Cargo.toml: {}", e))
        })?;

        let mut doc = content.parse::<toml_edit::DocumentMut>().map_err(|e| {
            ServerError::Internal(format!("Failed to parse package Cargo.toml: {}", e))
        })?;

        let mut updated_count = 0;

        // Update [package].name to match the new directory name
        let new_dir_name = new_package_path.file_name().and_then(|n| n.to_str());

        if let Some(new_name) = new_dir_name {
            let new_crate_name = new_name.replace('_', "-"); // Normalize to hyphens for Cargo.toml
            if let Some(package_section) = doc.get_mut("package") {
                if let Some(name_field) = package_section.get_mut("name") {
                    let old_name = name_field.as_str().unwrap_or("");
                    if old_name != new_crate_name {
                        info!(
                            old_name = %old_name,
                            new_name = %new_crate_name,
                            "Updating package name in Cargo.toml"
                        );
                        *name_field = toml_edit::value(new_crate_name);
                        updated_count += 1;
                    }
                }
            }
        }

        // Calculate depth change for relative path updates
        let old_depth = old_package_path
            .strip_prefix(workspace_root)
            .map(|p| p.components().count())
            .unwrap_or(0);
        let new_depth = new_package_path
            .strip_prefix(workspace_root)
            .map(|p| p.components().count())
            .unwrap_or(0);

        // Helper closure to update path dependencies in a dependency table
        let update_deps_in_table = |deps: &mut toml_edit::Table, updated: &mut usize| {
            for (name, value) in deps.iter_mut() {
                if let Some(table) = value.as_inline_table_mut() {
                    if let Some(path_value) = table.get_mut("path") {
                        if let Some(old_path_str) = path_value.as_str() {
                            let new_path_str =
                                self.adjust_relative_path(old_path_str, old_depth, new_depth);
                            if new_path_str != old_path_str {
                                info!(
                                    dependency = %name,
                                    old_path = %old_path_str,
                                    new_path = %new_path_str,
                                    "Updating relative path dependency"
                                );
                                *path_value = new_path_str.as_str().into();
                                *updated += 1;
                            }
                        }
                    }
                } else if let Some(table) = value.as_table_mut() {
                    if let Some(path_value) = table.get_mut("path") {
                        if let Some(old_path_str) = path_value.as_str() {
                            let new_path_str =
                                self.adjust_relative_path(old_path_str, old_depth, new_depth);
                            if new_path_str != old_path_str {
                                info!(
                                    dependency = %name,
                                    old_path = %old_path_str,
                                    new_path = %new_path_str,
                                    "Updating relative path dependency"
                                );
                                *path_value = new_path_str.as_str().into();
                                *updated += 1;
                            }
                        }
                    }
                }
            }
        };

        // Update standard dependency sections
        for section in ["dependencies", "dev-dependencies", "build-dependencies"] {
            if let Some(deps) = doc[section].as_table_mut() {
                update_deps_in_table(deps, &mut updated_count);
            }
        }

        // Update target-specific dependency sections
        if let Some(target) = doc.get_mut("target").and_then(|t| t.as_table_mut()) {
            for (_target_spec, target_value) in target.iter_mut() {
                if let Some(target_table) = target_value.as_table_mut() {
                    for dep_section in ["dependencies", "dev-dependencies", "build-dependencies"] {
                        if let Some(deps) = target_table.get_mut(dep_section).and_then(|d| d.as_table_mut()) {
                            update_deps_in_table(deps, &mut updated_count);
                        }
                    }
                }
            }
        }

        if updated_count > 0 {
            fs::write(package_cargo_toml, doc.to_string())
                .await
                .map_err(|e| {
                    ServerError::Internal(format!(
                        "Failed to write updated package Cargo.toml: {}",
                        e
                    ))
                })?;
            info!(
                package = ?package_cargo_toml,
                updated_count = updated_count,
                "Updated relative path dependencies in package manifest"
            );
        } else {
            debug!("No relative path dependencies needed updating");
        }

        Ok(updated_count > 0)
    }

    /// Adjust a relative path based on depth change
    fn adjust_relative_path(&self, path: &str, old_depth: usize, new_depth: usize) -> String {
        let depth_diff = new_depth as i32 - old_depth as i32;

        if depth_diff > 0 {
            // Moved deeper, add more "../"
            let additional_uplevels = "../".repeat(depth_diff as usize);
            format!("{}{}", additional_uplevels, path)
        } else if depth_diff < 0 {
            // Moved shallower, remove "../"
            let uplevels_to_remove = (-depth_diff) as usize;
            let mut remaining = path;
            for _ in 0..uplevels_to_remove {
                remaining = remaining.strip_prefix("../").unwrap_or(remaining);
            }
            remaining.to_string()
        } else {
            path.to_string()
        }
    }

    /// Update documentation file references after directory rename
    async fn update_documentation_references(
        &self,
        old_dir_path: &Path,
        new_dir_path: &Path,
        dry_run: bool,
    ) -> ServerResult<DocumentationUpdateReport> {
        let old_rel = old_dir_path
            .strip_prefix(&self.project_root)
            .unwrap_or(old_dir_path);
        let new_rel = new_dir_path
            .strip_prefix(&self.project_root)
            .unwrap_or(new_dir_path);

        let old_path_str = old_rel.to_string_lossy();
        let new_path_str = new_rel.to_string_lossy();

        // Documentation file patterns
        let doc_patterns = ["*.md", "*.txt", "README*", "CHANGELOG*", "CONTRIBUTING*"];

        let mut updated_files = Vec::new();
        let mut failed_files = Vec::new();
        let mut total_references = 0;

        // Walk project root for documentation files
        let walker = ignore::WalkBuilder::new(&self.project_root)
            .hidden(false)
            .git_ignore(true)
            .build();

        for entry in walker.flatten() {
            let path = entry.path();

            // Check if matches doc pattern
            if !path.is_file() {
                continue;
            }

            let matches_pattern = doc_patterns.iter().any(|pattern| {
                if pattern.starts_with('*') {
                    path.extension()
                        .and_then(|e| e.to_str())
                        .map(|e| pattern.ends_with(e))
                        .unwrap_or(false)
                } else {
                    path.file_name()
                        .and_then(|n| n.to_str())
                        .map(|n| n.starts_with(pattern.trim_end_matches('*')))
                        .unwrap_or(false)
                }
            });

            if !matches_pattern {
                continue;
            }

            // Read file content
            match fs::read_to_string(&path).await {
                Ok(content) => {
                    // Count and replace references
                    let count = content.matches(old_path_str.as_ref()).count();
                    if count == 0 {
                        continue;
                    }

                    total_references += count;

                    if dry_run {
                        info!(
                            file = %path.display(),
                            references = count,
                            "[DRY RUN] Would update documentation references"
                        );
                        updated_files.push(path.to_string_lossy().to_string());
                    } else {
                        let new_content =
                            content.replace(old_path_str.as_ref(), new_path_str.as_ref());

                        match fs::write(&path, new_content).await {
                            Ok(_) => {
                                info!(
                                    file = %path.display(),
                                    references = count,
                                    old = %old_path_str,
                                    new = %new_path_str,
                                    "Updated documentation references"
                                );
                                updated_files.push(path.to_string_lossy().to_string());
                            }
                            Err(e) => {
                                warn!(
                                    file = %path.display(),
                                    error = %e,
                                    "Failed to update documentation file"
                                );
                                failed_files.push(format!("{}: {}", path.display(), e));
                            }
                        }
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::InvalidData => {
                    // Skip binary files silently
                    debug!(file = %path.display(), "Skipping binary file");
                }
                Err(e) => {
                    warn!(
                        file = %path.display(),
                        error = %e,
                        "Failed to read documentation file"
                    );
                    failed_files.push(format!("{}: {}", path.display(), e));
                }
            }
        }

        Ok(DocumentationUpdateReport {
            files_updated: updated_files.len(),
            references_updated: total_references,
            updated_files,
            failed_files,
        })
    }
}

/// Result of documentation reference updates
#[derive(Debug, Clone, serde::Serialize)]
pub struct DocumentationUpdateReport {
    /// Number of documentation files updated
    pub files_updated: usize,
    /// Number of path references updated
    pub references_updated: usize,
    /// Paths of updated documentation files
    pub updated_files: Vec<String>,
    /// Files that failed to update
    pub failed_files: Vec<String>,
}

/// Result of applying an edit plan
#[derive(Debug, Clone, serde::Serialize)]
pub struct EditPlanResult {
    /// Whether all edits were applied successfully
    pub success: bool,
    /// List of files that were modified
    pub modified_files: Vec<String>,
    /// Error messages if any edits failed
    pub errors: Option<Vec<String>>,
    /// Original plan metadata
    pub plan_metadata: EditPlanMetadata,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    // Helper to start a background worker for tests
    fn spawn_test_worker(queue: Arc<super::super::operation_queue::OperationQueue>) {
        use crate::services::operation_queue::OperationType;
        use cb_protocol::ApiError;
        use tokio::fs;

        tokio::spawn(async move {
            queue
                .process_with(|op, stats| async move {
                    let result: Result<(), ApiError> = match op.operation_type {
                        OperationType::CreateDir => {
                            fs::create_dir_all(&op.file_path).await.map_err(|e| {
                                ApiError::Internal(format!("Failed to create directory: {}", e))
                            })
                        }
                        OperationType::CreateFile | OperationType::Write => {
                            let content = op
                                .params
                                .get("content")
                                .and_then(|v| v.as_str())
                                .unwrap_or("");
                            fs::write(&op.file_path, content).await.map_err(|e| {
                                ApiError::Internal(format!("Failed to write file: {}", e))
                            })
                        }
                        OperationType::Delete => {
                            if op.file_path.exists() {
                                fs::remove_file(&op.file_path).await.map_err(|e| {
                                    ApiError::Internal(format!("Failed to delete file: {}", e))
                                })
                            } else {
                                Ok(())
                            }
                        }
                        OperationType::Rename => {
                            let new_path_str = op
                                .params
                                .get("new_path")
                                .and_then(|v| v.as_str())
                                .ok_or_else(|| {
                                ApiError::Internal("Missing new_path".to_string())
                            })?;
                            fs::rename(&op.file_path, new_path_str).await.map_err(|e| {
                                ApiError::Internal(format!("Failed to rename file: {}", e))
                            })
                        }
                        _ => Ok(()),
                    };

                    // Update stats after operation completes
                    let mut stats_guard = stats.lock().await;
                    match &result {
                        Ok(_) => {
                            stats_guard.completed_operations += 1;
                        }
                        Err(_) => {
                            stats_guard.failed_operations += 1;
                        }
                    }
                    drop(stats_guard);

                    result.map(|_| serde_json::Value::Null)
                })
                .await;
        });
    }

    fn create_test_service(
        temp_dir: &TempDir,
    ) -> (
        FileService,
        Arc<super::super::operation_queue::OperationQueue>,
    ) {
        let ast_cache = Arc::new(AstCache::new());
        let lock_manager = Arc::new(LockManager::new());
        let operation_queue = Arc::new(super::super::operation_queue::OperationQueue::new(
            lock_manager.clone(),
        ));
        let config = cb_core::AppConfig::default();
        let plugin_registry = crate::services::build_language_plugin_registry();
        let service = FileService::new(
            temp_dir.path(),
            ast_cache,
            lock_manager,
            operation_queue.clone(),
            &config,
            plugin_registry,
        );

        // Spawn background worker to process queued operations
        spawn_test_worker(operation_queue.clone());

        (service, operation_queue)
    }

    #[tokio::test]
    async fn test_create_and_read_file() {
        let temp_dir = TempDir::new().unwrap();
        let (service, queue) = create_test_service(&temp_dir);

        let file_path = Path::new("test.txt");
        let content = "Hello, World!";

        // Create file
        service
            .create_file(file_path, Some(content), false, false)
            .await
            .unwrap();

        // Wait for queue to process operations
        queue.wait_until_idle().await;

        // Read file
        let read_content = service.read_file(file_path).await.unwrap();
        assert_eq!(read_content, content);
    }

    #[tokio::test]
    async fn test_rename_file() {
        let temp_dir = TempDir::new().unwrap();
        let (service, queue) = create_test_service(&temp_dir);

        // Create initial file
        let old_path = Path::new("old.txt");
        let new_path = Path::new("new.txt");
        service
            .create_file(old_path, Some("content"), false, false)
            .await
            .unwrap();
        queue.wait_until_idle().await;

        // Rename file
        let result = service
            .rename_file_with_imports(old_path, new_path, false, None)
            .await
            .unwrap();
        assert!(result.result["success"].as_bool().unwrap_or(false));
        queue.wait_until_idle().await;

        // Verify old file doesn't exist and new file does
        assert!(!temp_dir.path().join(old_path).exists());
        assert!(temp_dir.path().join(new_path).exists());
    }

    #[tokio::test]
    async fn test_delete_file() {
        let temp_dir = TempDir::new().unwrap();
        let (service, queue) = create_test_service(&temp_dir);

        let file_path = Path::new("to_delete.txt");

        // Create and then delete file
        service
            .create_file(file_path, Some("temporary"), false, false)
            .await
            .unwrap();
        queue.wait_until_idle().await;
        assert!(temp_dir.path().join(file_path).exists());

        service.delete_file(file_path, false, false).await.unwrap();
        queue.wait_until_idle().await;
        assert!(!temp_dir.path().join(file_path).exists());
    }

    #[tokio::test]
    async fn test_atomic_edit_plan_success() {
        use cb_protocol::{DependencyUpdateType, EditLocation, EditType};

        let temp_dir = TempDir::new().unwrap();
        let (service, queue) = create_test_service(&temp_dir);

        // Create test files
        let main_file = "main.ts";
        let dep_file = "dependency.ts";

        service
            .create_file(
                Path::new(main_file),
                Some("import { foo } from './old';\nconst x = 1;"),
                false,
                false,
            )
            .await
            .unwrap();
        service
            .create_file(
                Path::new(dep_file),
                Some("import './old';\nconst y = 2;"),
                false,
                false,
            )
            .await
            .unwrap();

        // Create edit plan
        let plan = EditPlan {
            source_file: main_file.to_string(),
            edits: vec![TextEdit {
                file_path: None,
                edit_type: EditType::Replace,
                location: EditLocation {
                    start_line: 1,
                    start_column: 0,
                    end_line: 1,
                    end_column: 12,
                },
                original_text: "const x = 1;".to_string(),
                new_text: "const x = 2;".to_string(),
                priority: 1,
                description: "Update value".to_string(),
            }],
            dependency_updates: vec![DependencyUpdate {
                target_file: dep_file.to_string(),
                update_type: DependencyUpdateType::ImportPath,
                old_reference: "./old".to_string(),
                new_reference: "./new".to_string(),
            }],
            validations: vec![],
            metadata: EditPlanMetadata {
                intent_name: "test".to_string(),
                intent_arguments: serde_json::json!({}),
                created_at: chrono::Utc::now(),
                complexity: 1,
                impact_areas: vec!["test".to_string()],
            },
        };

        // Apply edit plan
        let result = service.apply_edit_plan(&plan).await.unwrap();

        // Verify success
        assert!(result.success);
        assert_eq!(result.modified_files.len(), 2);
        assert!(result.errors.is_none());

        // Verify file contents were updated
        let main_content = service.read_file(Path::new(main_file)).await.unwrap();
        assert!(main_content.contains("const x = 2;"));

        let dep_content = service.read_file(Path::new(dep_file)).await.unwrap();
        assert!(dep_content.contains("./new"));
    }

    #[tokio::test]
    async fn test_atomic_rollback_on_main_file_failure() {
        use cb_protocol::{DependencyUpdateType, EditLocation, EditType};

        let temp_dir = TempDir::new().unwrap();
        let (service, queue) = create_test_service(&temp_dir);

        // Create test files with specific content
        let main_file = "main.ts";
        let dep_file = "dependency.ts";

        let main_original = "import { foo } from './old';\nconst x = 1;";
        let dep_original = "import './old';\nconst y = 2;";

        service
            .create_file(Path::new(main_file), Some(main_original), false, false)
            .await
            .unwrap();
        service
            .create_file(Path::new(dep_file), Some(dep_original), false, false)
            .await
            .unwrap();
        queue.wait_until_idle().await;

        // Create edit plan with invalid edit location that will fail
        let plan = EditPlan {
            source_file: main_file.to_string(),
            edits: vec![TextEdit {
                file_path: None,
                edit_type: EditType::Replace,
                location: EditLocation {
                    start_line: 999, // Invalid line - will cause failure
                    start_column: 0,
                    end_line: 999,
                    end_column: 10,
                },
                original_text: "invalid".to_string(),
                new_text: "replacement".to_string(),
                priority: 1,
                description: "This should fail".to_string(),
            }],
            dependency_updates: vec![DependencyUpdate {
                target_file: dep_file.to_string(),
                update_type: DependencyUpdateType::ImportPath,
                old_reference: "./old".to_string(),
                new_reference: "./new".to_string(),
            }],
            validations: vec![],
            metadata: EditPlanMetadata {
                intent_name: "test_failure".to_string(),
                intent_arguments: serde_json::json!({}),
                created_at: chrono::Utc::now(),
                complexity: 1,
                impact_areas: vec!["test".to_string()],
            },
        };

        // Apply edit plan - should fail
        let result = service.apply_edit_plan(&plan).await;
        assert!(result.is_err());

        // Verify files were rolled back to original state
        let main_content = service.read_file(Path::new(main_file)).await.unwrap();
        assert_eq!(
            main_content, main_original,
            "Main file should be rolled back"
        );

        let dep_content = service.read_file(Path::new(dep_file)).await.unwrap();
        assert_eq!(
            dep_content, dep_original,
            "Dependency file should be rolled back"
        );
    }

    #[tokio::test]
    async fn test_atomic_rollback_on_dependency_failure() {
        use cb_protocol::{DependencyUpdateType, EditLocation, EditType};

        let temp_dir = TempDir::new().unwrap();
        let (service, queue) = create_test_service(&temp_dir);

        // Create main file
        let main_file = "main.ts";
        let main_original = "const x = 1;";

        service
            .create_file(Path::new(main_file), Some(main_original), false, false)
            .await
            .unwrap();

        // Create a dependency file with unparseable content that will cause AST failure
        let dep_file = "bad_syntax.ts";
        let dep_original = "<<<< this is invalid typescript syntax >>>>";

        service
            .create_file(Path::new(dep_file), Some(dep_original), false, false)
            .await
            .unwrap();
        queue.wait_until_idle().await;

        // Create edit plan that will fail when trying to parse the bad dependency file
        let plan = EditPlan {
            source_file: main_file.to_string(),
            edits: vec![TextEdit {
                file_path: None,
                edit_type: EditType::Replace,
                location: EditLocation {
                    start_line: 0,
                    start_column: 0,
                    end_line: 0,
                    end_column: 12,
                },
                original_text: "const x = 1;".to_string(),
                new_text: "const x = 2;".to_string(),
                priority: 1,
                description: "Update value".to_string(),
            }],
            dependency_updates: vec![DependencyUpdate {
                target_file: dep_file.to_string(),
                update_type: DependencyUpdateType::ImportPath,
                old_reference: "<<<<".to_string(),
                new_reference: "./new".to_string(),
            }],
            validations: vec![],
            metadata: EditPlanMetadata {
                intent_name: "test_dep_failure".to_string(),
                intent_arguments: serde_json::json!({}),
                created_at: chrono::Utc::now(),
                complexity: 1,
                impact_areas: vec!["test".to_string()],
            },
        };

        // Apply edit plan - should fail on dependency update due to parse error
        let result = service.apply_edit_plan(&plan).await;
        assert!(result.is_err());

        // Verify main file was rolled back to original state
        let main_content = service.read_file(Path::new(main_file)).await.unwrap();
        assert_eq!(
            main_content, main_original,
            "Main file should be rolled back after dependency failure"
        );

        // Verify bad dependency file was also rolled back
        let dep_content = service.read_file(Path::new(dep_file)).await.unwrap();
        assert_eq!(
            dep_content, dep_original,
            "Dependency file should be rolled back"
        );
    }

    #[tokio::test]
    async fn test_atomic_rollback_multiple_files() {
        use cb_protocol::{DependencyUpdateType, EditLocation, EditType};

        let temp_dir = TempDir::new().unwrap();
        let (service, queue) = create_test_service(&temp_dir);

        // Create multiple files
        let main_file = "main.ts";
        let dep_file1 = "dep1.ts";
        let dep_file2 = "dep2.ts";
        let dep_file3 = "dep3.ts";

        let main_original = "const x = 1;";
        let dep1_original = "import './old1';";
        let dep2_original = "import './old2';";
        let dep3_original = "import 'this_will_cause_parse_error'; <<<< invalid syntax >>>>";

        service
            .create_file(Path::new(main_file), Some(main_original), false, false)
            .await
            .unwrap();
        service
            .create_file(Path::new(dep_file1), Some(dep1_original), false, false)
            .await
            .unwrap();
        service
            .create_file(Path::new(dep_file2), Some(dep2_original), false, false)
            .await
            .unwrap();
        service
            .create_file(Path::new(dep_file3), Some(dep3_original), false, false)
            .await
            .unwrap();
        queue.wait_until_idle().await;

        // Create edit plan that will fail on the last dependency due to parse error
        let plan = EditPlan {
            source_file: main_file.to_string(),
            edits: vec![TextEdit {
                file_path: None,
                edit_type: EditType::Replace,
                location: EditLocation {
                    start_line: 0,
                    start_column: 0,
                    end_line: 0,
                    end_column: 12,
                },
                original_text: "const x = 1;".to_string(),
                new_text: "const x = 999;".to_string(),
                priority: 1,
                description: "Update value".to_string(),
            }],
            dependency_updates: vec![
                DependencyUpdate {
                    target_file: dep_file1.to_string(),
                    update_type: DependencyUpdateType::ImportPath,
                    old_reference: "./old1".to_string(),
                    new_reference: "./new1".to_string(),
                },
                DependencyUpdate {
                    target_file: dep_file2.to_string(),
                    update_type: DependencyUpdateType::ImportPath,
                    old_reference: "./old2".to_string(),
                    new_reference: "./new2".to_string(),
                },
                DependencyUpdate {
                    target_file: dep_file3.to_string(),
                    update_type: DependencyUpdateType::ImportPath,
                    old_reference: "this_will_cause_parse_error".to_string(),
                    new_reference: "./new3".to_string(),
                },
            ],
            validations: vec![],
            metadata: EditPlanMetadata {
                intent_name: "test_multi_rollback".to_string(),
                intent_arguments: serde_json::json!({}),
                created_at: chrono::Utc::now(),
                complexity: 3,
                impact_areas: vec!["test".to_string()],
            },
        };

        // Apply edit plan - should fail on third dependency due to parse error
        let result = service.apply_edit_plan(&plan).await;
        assert!(result.is_err());

        // Verify ALL files were rolled back to original state
        let main_content = service.read_file(Path::new(main_file)).await.unwrap();
        assert_eq!(
            main_content, main_original,
            "Main file should be rolled back"
        );

        let dep1_content = service.read_file(Path::new(dep_file1)).await.unwrap();
        assert_eq!(
            dep1_content, dep1_original,
            "First dependency file should be rolled back"
        );

        let dep2_content = service.read_file(Path::new(dep_file2)).await.unwrap();
        assert_eq!(
            dep2_content, dep2_original,
            "Second dependency file should be rolled back"
        );

        let dep3_content = service.read_file(Path::new(dep_file3)).await.unwrap();
        assert_eq!(
            dep3_content, dep3_original,
            "Third dependency file should remain unchanged"
        );
    }
}

#[cfg(test)]
mod workspace_tests {
    use super::*;
    use tempfile::TempDir;

    // Helper to start a background worker for tests
    fn spawn_test_worker(queue: Arc<super::super::operation_queue::OperationQueue>) {
        use crate::services::operation_queue::OperationType;
        use cb_protocol::ApiError;
        use tokio::fs;

        tokio::spawn(async move {
            queue
                .process_with(|op, stats| async move {
                    let result: Result<(), ApiError> = match op.operation_type {
                        OperationType::CreateDir => {
                            fs::create_dir_all(&op.file_path).await.map_err(|e| {
                                ApiError::Internal(format!("Failed to create directory: {}", e))
                            })
                        }
                        OperationType::CreateFile | OperationType::Write => {
                            let content = op
                                .params
                                .get("content")
                                .and_then(|v| v.as_str())
                                .unwrap_or("");
                            fs::write(&op.file_path, content).await.map_err(|e| {
                                ApiError::Internal(format!("Failed to write file: {}", e))
                            })
                        }
                        OperationType::Delete => {
                            if op.file_path.exists() {
                                fs::remove_file(&op.file_path).await.map_err(|e| {
                                    ApiError::Internal(format!("Failed to delete file: {}", e))
                                })
                            } else {
                                Ok(())
                            }
                        }
                        OperationType::Rename => {
                            let new_path_str = op
                                .params
                                .get("new_path")
                                .and_then(|v| v.as_str())
                                .ok_or_else(|| {
                                ApiError::Internal("Missing new_path".to_string())
                            })?;
                            fs::rename(&op.file_path, new_path_str).await.map_err(|e| {
                                ApiError::Internal(format!("Failed to rename file: {}", e))
                            })
                        }
                        _ => Ok(()),
                    };

                    // Update stats after operation completes
                    let mut stats_guard = stats.lock().await;
                    match &result {
                        Ok(_) => {
                            stats_guard.completed_operations += 1;
                        }
                        Err(_) => {
                            stats_guard.failed_operations += 1;
                        }
                    }
                    drop(stats_guard);

                    result.map(|_| serde_json::Value::Null)
                })
                .await;
        });
    }

    fn create_test_service(
        temp_dir: &TempDir,
    ) -> (
        FileService,
        Arc<super::super::operation_queue::OperationQueue>,
    ) {
        let ast_cache = Arc::new(AstCache::new());
        let lock_manager = Arc::new(LockManager::new());
        let operation_queue = Arc::new(super::super::operation_queue::OperationQueue::new(
            lock_manager.clone(),
        ));
        let config = cb_core::AppConfig::default();
        let plugin_registry = crate::services::build_language_plugin_registry();
        let service = FileService::new(
            temp_dir.path(),
            ast_cache,
            lock_manager,
            operation_queue.clone(),
            &config,
            plugin_registry,
        );

        // Spawn background worker to process queued operations
        spawn_test_worker(operation_queue.clone());

        (service, operation_queue)
    }

    #[tokio::test]
    async fn test_update_workspace_manifests_simple_rename() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();

        // Create a workspace Cargo.toml
        let workspace_toml_content = r#"
[workspace]
members = [
    "crates/my-crate",
]
"#;
        fs::write(project_root.join("Cargo.toml"), workspace_toml_content)
            .await
            .unwrap();

        // Create the package directory and its Cargo.toml
        let old_crate_dir = project_root.join("crates/my-crate");
        fs::create_dir_all(&old_crate_dir).await.unwrap();
        fs::write(
            old_crate_dir.join("Cargo.toml"),
            "[package]\nname = \"my-crate\"",
        )
        .await
        .unwrap();

        let new_crate_dir = project_root.join("crates/my-renamed-crate");

        // Setup FileService
        let (service, queue) = create_test_service(&temp_dir);

        // Run the update
        service
            .update_workspace_manifests(&old_crate_dir, &new_crate_dir)
            .await
            .unwrap();

        // Verify the workspace Cargo.toml was updated
        let updated_content = fs::read_to_string(project_root.join("Cargo.toml"))
            .await
            .unwrap();
        let doc = updated_content.parse::<toml_edit::DocumentMut>().unwrap();
        let members = doc["workspace"]["members"].as_array().unwrap();

        assert_eq!(members.len(), 1);
        assert_eq!(
            members.iter().next().unwrap().as_str(),
            Some("crates/my-renamed-crate")
        );
    }

    #[test]
    fn test_adjust_relative_path_logic() {
        let temp_dir = TempDir::new().unwrap();
        // This test doesn't need async operations, so create service directly
        let ast_cache = Arc::new(AstCache::new());
        let lock_manager = Arc::new(LockManager::new());
        let operation_queue = Arc::new(super::super::operation_queue::OperationQueue::new(
            lock_manager.clone(),
        ));
        let config = cb_core::AppConfig::default();
        let plugin_registry = crate::services::build_language_plugin_registry();
        let service = FileService::new(
            temp_dir.path(),
            ast_cache,
            lock_manager,
            operation_queue,
            &config,
            plugin_registry,
        );

        // Moved deeper: 1 level
        assert_eq!(
            service.adjust_relative_path("../sibling", 1, 2),
            "../../sibling"
        );
        // Moved deeper: 2 levels
        assert_eq!(
            service.adjust_relative_path("../sibling", 1, 3),
            "../../../sibling"
        );
        // Moved shallower: 1 level
        assert_eq!(
            service.adjust_relative_path("../../sibling", 2, 1),
            "../sibling"
        );
        // Moved shallower: 2 levels
        assert_eq!(
            service.adjust_relative_path("../../../sibling", 3, 1),
            "../sibling"
        );
        // No change
        assert_eq!(
            service.adjust_relative_path("../sibling", 2, 2),
            "../sibling"
        );
        // Path with no up-levels
        assert_eq!(service.adjust_relative_path("sibling", 2, 1), "sibling");
    }
}
