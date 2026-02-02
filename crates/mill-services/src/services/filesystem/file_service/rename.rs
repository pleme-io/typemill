use super::FileService;
use crate::services::filesystem::git_service::GitService;
use mill_foundation::core::dry_run::DryRunnable;
use mill_foundation::errors::MillError as ServerError;

type ServerResult<T> = Result<T, ServerError>;
use serde_json::{json, Value};
use std::path::Path;
use tokio::fs;
use tracing::{debug, info, warn};

impl FileService {
    /// Check if two paths point to the same file (handles case-only renames on case-insensitive filesystems)
    async fn is_same_file(&self, path1: &Path, path2: &Path) -> ServerResult<bool> {
        if !path1.exists() || !path2.exists() {
            return Ok(false);
        }

        match (fs::metadata(path1).await, fs::metadata(path2).await) {
            (Ok(meta1), Ok(meta2)) => {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::MetadataExt;
                    Ok(meta1.ino() == meta2.ino())
                }
                #[cfg(not(unix))]
                {
                    Ok(path1.canonicalize().ok() == path2.canonicalize().ok())
                }
            }
            _ => Ok(false),
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
                .map_err(|e| ServerError::internal(format!("Task join error: {}", e)))?
                .map_err(|e| ServerError::internal(format!("git mv failed: {}", e)))?;
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
                    ServerError::internal(format!("Failed to create parent directory: {}", e))
                })?;
            }

            fs::rename(old, new)
                .await
                .map_err(|e| ServerError::internal(format!("Failed to rename file: {}", e)))?;
        }

        Ok(())
    }

    /// Rename a file and update all imports
    pub async fn rename_file_with_imports(
        &self,
        old_path: &Path,
        new_path: &Path,
        dry_run: bool,
        scan_scope: Option<mill_plugin_api::ScanScope>,
    ) -> ServerResult<DryRunnable<Value>> {
        info!(old_path = ?old_path, new_path = ?new_path, dry_run, "Renaming file");

        let old_abs = self.to_absolute_path_checked(old_path)?;
        let new_abs = self.to_absolute_path_checked(new_path)?;

        if dry_run {
            return self
                .preview_rename_file(&old_abs, &new_abs, scan_scope)
                .await;
        }

        self.execute_rename_file(&old_abs, &new_abs, scan_scope)
            .await
    }

    async fn preview_rename_file(
        &self,
        old_abs: &Path,
        new_abs: &Path,
        scan_scope: Option<mill_plugin_api::ScanScope>,
    ) -> ServerResult<DryRunnable<Value>> {
        // Preview mode - just return what would happen
        if !old_abs.exists() {
            return Err(ServerError::not_found(format!(
                "Source file does not exist: {:?}",
                old_abs
            )));
        }

        // Allow case-only renames on case-insensitive filesystems
        // Only error if new path exists AND points to a different file
        if new_abs.exists() && !self.is_same_file(old_abs, new_abs).await? {
            return Err(ServerError::invalid_request(format!(
                "Resource already exists: Destination file already exists: {:?}",
                new_abs
            )));
        }

        // Use MoveService for planning (includes all import update logic)
        let edit_plan = self
            .move_service()
            .plan_file_move(old_abs, new_abs, scan_scope, None)
            .await?;

        Ok(DryRunnable::new(
            true,
            json!({
                "operation": "move_file",
                "old_path": old_abs.to_string_lossy(),
                "new_path": new_abs.to_string_lossy(),
                "import_updates": {
                    "edits_planned": edit_plan.edits.len(),
                    "files_to_modify": edit_plan.edits.iter()
                        .filter_map(|e| e.file_path.as_ref())
                        .collect::<std::collections::HashSet<_>>()
                        .len(),
                },
            }),
        ))
    }

    async fn execute_rename_file(
        &self,
        old_abs: &Path,
        new_abs: &Path,
        scan_scope: Option<mill_plugin_api::ScanScope>,
    ) -> ServerResult<DryRunnable<Value>> {
        // Execution mode - perform rename and update imports
        if !old_abs.exists() {
            return Err(ServerError::not_found(format!(
                "Source file does not exist: {:?}",
                old_abs
            )));
        }

        // Allow case-only renames on case-insensitive filesystems
        // Only error if new path exists AND points to a different file
        if new_abs.exists() && !self.is_same_file(old_abs, new_abs).await? {
            return Err(ServerError::invalid_request(format!(
                "Resource already exists: Destination file already exists: {:?}",
                new_abs
            )));
        }

        // IMPORTANT: Find affected files BEFORE renaming!
        // The old file must still exist on disk for the import resolver to work correctly.
        info!("Finding affected files before rename");
        let mut edit_plan = self
            .move_service()
            .plan_file_move(old_abs, new_abs, scan_scope, None)
            .await
            .map_err(|e| {
                warn!(error = %e, "Failed to find affected files");
                ServerError::internal(format!("Failed to find affected files: {}", e))
            })?;

        info!(
            edits_count = edit_plan.edits.len(),
            "Found affected files, now performing rename"
        );

        // Now perform the actual rename
        self.perform_rename(old_abs, new_abs).await?;

        info!("File renamed successfully");

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

        // Apply the edit plan to update imports
        let edit_result = self.apply_edit_plan(&edit_plan).await.map_err(|e| {
            warn!(error = %e, "Failed to apply import update edits");
            ServerError::internal(format!("Failed to apply import updates: {}", e))
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
                "operation": "move_file",
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

    /// Rename a directory and update all imports pointing to files within it
    ///
    /// NOTE: This is a legacy internal tool. New code should use the Unified Refactoring API
    /// (rename with dryRun option) which handles consolidation through the plugin system.
    pub async fn rename_directory_with_imports(
        &self,
        old_dir_path: &Path,
        new_dir_path: &Path,
        dry_run: bool,
        scan_scope: Option<mill_plugin_api::ScanScope>,
        details: bool,
    ) -> ServerResult<DryRunnable<Value>> {
        info!(old_path = ?old_dir_path, new_path = ?new_dir_path, dry_run, "Renaming directory (legacy internal tool)");

        let old_abs_dir = self.to_absolute_path_checked(old_dir_path)?;
        let new_abs_dir = self.to_absolute_path_checked(new_dir_path)?;

        if dry_run {
            return self
                .preview_rename_directory(&old_abs_dir, &new_abs_dir, details)
                .await;
        }

        self.execute_rename_directory(&old_abs_dir, &new_abs_dir, scan_scope)
            .await
    }

    async fn preview_rename_directory(
        &self,
        old_abs_dir: &Path,
        new_abs_dir: &Path,
        details: bool,
    ) -> ServerResult<DryRunnable<Value>> {
        // Preview mode - just return what would happen
        if !old_abs_dir.exists() {
            return Err(ServerError::not_found(format!(
                "Source directory does not exist: {:?}",
                old_abs_dir
            )));
        }

        if new_abs_dir.exists() {
            return Err(ServerError::invalid_request(format!(
                "Resource already exists: Destination directory already exists: {:?}",
                new_abs_dir
            )));
        }

        let files_to_move = self.collect_files_in_dir(old_abs_dir);
        let is_cargo_pkg = old_abs_dir.join("Cargo.toml").exists();

        // Build response with optional details
        let mut response = json!({
            "operation": "move_directory",
            "old_path": old_abs_dir.to_string_lossy(),
            "new_path": new_abs_dir.to_string_lossy(),
            "files_to_move": files_to_move.len(),
            "is_cargo_package": is_cargo_pkg,
        });

        // Include detailed file list if requested
        if details {
            response["files"] = json!(files_to_move
                .iter()
                .map(|p| p
                    .strip_prefix(old_abs_dir)
                    .unwrap_or(p)
                    .to_string_lossy()
                    .to_string())
                .collect::<Vec<_>>());
        }

        Ok(DryRunnable::new(true, response))
    }

    async fn execute_rename_directory(
        &self,
        old_abs_dir: &Path,
        new_abs_dir: &Path,
        scan_scope: Option<mill_plugin_api::ScanScope>,
    ) -> ServerResult<DryRunnable<Value>> {
        // Execution mode - perform directory rename and update imports
        if !old_abs_dir.exists() {
            return Err(ServerError::not_found(format!(
                "Source directory does not exist: {:?}",
                old_abs_dir
            )));
        }

        if new_abs_dir.exists() {
            return Err(ServerError::invalid_request(format!(
                "Resource already exists: Destination directory already exists: {:?}",
                new_abs_dir
            )));
        }

        let files_to_move = self.collect_files_in_dir(old_abs_dir);
        let is_cargo_pkg = old_abs_dir.join("Cargo.toml").exists();

        // IMPORTANT: Find affected files BEFORE renaming the directory!
        // The old directory must still exist on disk for the import resolver to work correctly.
        let mut total_edits_applied = 0;
        let mut total_files_updated = std::collections::HashSet::new();
        let mut all_errors = Vec::new();

        info!("Planning directory move and import updates");

        // CRITICAL: Plan FIRST before making any filesystem changes
        // Use MoveService for planning - it handles all Cargo package logic internally
        let edit_plan = self
            .move_service()
            .plan_directory_move(old_abs_dir, new_abs_dir, scan_scope, None)
            .await?; // Fail fast if planning fails

        info!(
            edits_planned = edit_plan.edits.len(),
            "Plan generated successfully, now performing directory rename"
        );

        // Now perform the actual directory rename
        // Only execute if planning succeeded
        self.perform_rename(old_abs_dir, new_abs_dir).await?;
        info!("Directory renamed successfully");

        // Apply the edit plan to update imports
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
                warn!(error = %e, "Import update failed for directory rename, attempting rollback");
                // ROLLBACK: Attempt to move directory back to original location
                if let Err(rollback_err) = self.perform_rename(new_abs_dir, old_abs_dir).await {
                    let error_msg = format!(
                        "CRITICAL: Failed to apply import edits AND failed to rollback directory rename. Manual intervention required. Import Error: {}. Rollback Error: {}",
                        e, rollback_err
                    );
                    all_errors.push(error_msg);
                    // Return immediately with error as we are in a bad state
                    return Err(ServerError::internal(format!(
                        "Directory rename failed and rollback failed. System is in inconsistent state: {}",
                        e
                    )));
                } else {
                    info!("Successfully rolled back directory rename");
                    return Err(ServerError::internal(format!(
                        "Failed to apply import updates, directory rename rolled back: {}",
                        e
                    )));
                }
            }
        }

        // NOTE: Cargo manifest updates (workspace members, package name, dependent crates)
        // are now handled automatically by MoveService within the EditPlan.
        // The edits are included in edit_plan.edits and have already been applied above.

        let doc_updates = self
            .update_documentation_references(old_abs_dir, new_abs_dir, false)
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

        // Extract manifest file updates from the edit plan
        let manifest_updates = if is_cargo_pkg {
            let manifest_files: std::collections::HashSet<_> = total_files_updated
                .iter()
                .filter(|f| f.ends_with("Cargo.toml"))
                .cloned()
                .collect();

            if !manifest_files.is_empty() {
                let mut sorted_manifests: Vec<_> = manifest_files.into_iter().collect();
                sorted_manifests.sort();
                Some(json!({
                    "files_updated": sorted_manifests.len(),
                    "updated_files": sorted_manifests,
                    "note": "Manifest updates handled by MoveService"
                }))
            } else {
                Some(json!({
                    "files_updated": 0,
                    "note": "No manifest updates required"
                }))
            }
        } else {
            None
        };

        let mut result = json!({
            "operation": "move_directory",
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

    fn collect_files_in_dir(&self, dir: &Path) -> Vec<std::path::PathBuf> {
        let mut files = Vec::new();
        let walker = ignore::WalkBuilder::new(dir).hidden(false).build();
        for entry in walker.flatten() {
            if entry.path().is_file() {
                files.push(entry.path().to_path_buf());
            }
        }
        files
    }

    /// Perform the actual file rename operation
    pub(super) async fn perform_rename(
        &self,
        old_path: &Path,
        new_path: &Path,
    ) -> ServerResult<()> {
        // Use our git-aware rename helper directly
        self.rename_file_internal(old_path, new_path).await
    }
}
