use super::FileService;
use crate::services::git_service::GitService;
use cb_core::dry_run::DryRunnable;
use cb_protocol::{ApiError as ServerError, ApiResult as ServerResult, EditPlan};
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

    /// Generates an EditPlan for a file rename operation, including import updates.
    /// This is a dry-run only operation.
    pub async fn plan_rename_file_with_imports(
        &self,
        old_path: &Path,
        new_path: &Path,
        scan_scope: Option<cb_plugin_api::ScanScope>,
    ) -> ServerResult<EditPlan> {
        info!(old_path = ?old_path, new_path = ?new_path, "Planning file rename with imports");

        // Delegate to MoveService which contains all the planning logic
        self.move_service().plan_file_move(old_path, new_path, scan_scope).await
    }

    /// Generates an EditPlan for a directory rename operation, including import updates.
    /// This is a dry-run only operation.
    pub async fn plan_rename_directory_with_imports(
        &self,
        old_dir_path: &Path,
        new_dir_path: &Path,
        scan_scope: Option<cb_plugin_api::ScanScope>,
    ) -> ServerResult<EditPlan> {
        info!(old_dir_path = ?old_dir_path, new_dir_path = ?new_dir_path, "Planning directory rename with imports");

        // Delegate to MoveService which contains all the Cargo package handling logic
        self.move_service().plan_directory_move(old_dir_path, new_dir_path, scan_scope).await
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

            // Allow case-only renames on case-insensitive filesystems
            // Only error if new path exists AND points to a different file
            if new_abs.exists() && !self.is_same_file(&old_abs, &new_abs).await? {
                return Err(ServerError::AlreadyExists(format!(
                    "Destination file already exists: {:?}",
                    new_abs
                )));
            }

            // Use MoveService for planning (includes all import update logic)
            let edit_plan = self.move_service()
                .plan_file_move(&old_abs, &new_abs, scan_scope.clone())
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
        } else {
            // Execution mode - perform rename and update imports
            if !old_abs.exists() {
                return Err(ServerError::NotFound(format!(
                    "Source file does not exist: {:?}",
                    old_abs
                )));
            }

            // Allow case-only renames on case-insensitive filesystems
            // Only error if new path exists AND points to a different file
            if new_abs.exists() && !self.is_same_file(&old_abs, &new_abs).await? {
                return Err(ServerError::AlreadyExists(format!(
                    "Destination file already exists: {:?}",
                    new_abs
                )));
            }

            // IMPORTANT: Find affected files BEFORE renaming!
            // The old file must still exist on disk for the import resolver to work correctly.
            info!("Finding affected files before rename");
            let mut edit_plan = self.move_service()
                .plan_file_move(&old_abs, &new_abs, scan_scope)
                .await
                .map_err(|e| {
                    warn!(error = %e, "Failed to find affected files");
                    ServerError::Internal(format!("Failed to find affected files: {}", e))
                })?;

            info!(
                edits_count = edit_plan.edits.len(),
                "Found affected files, now performing rename"
            );

            // Now perform the actual rename
            self.perform_rename(&old_abs, &new_abs).await?;

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
    }

    /// Rename a directory and update all imports pointing to files within it
    pub async fn rename_directory_with_imports(
        &self,
        old_dir_path: &Path,
        new_dir_path: &Path,
        dry_run: bool,
        consolidate: bool,
        scan_scope: Option<cb_plugin_api::ScanScope>,
        details: bool,
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
                        .strip_prefix(&old_abs_dir)
                        .unwrap_or(p)
                        .to_string_lossy()
                        .to_string())
                    .collect::<Vec<_>>());
            }

            Ok(DryRunnable::new(true, response))
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

            // IMPORTANT: Find affected files BEFORE renaming the directory!
            // The old directory must still exist on disk for the import resolver to work correctly.
            let mut total_edits_applied = 0;
            let mut total_files_updated = std::collections::HashSet::new();
            let mut all_errors = Vec::new();

            info!("Finding affected files before directory rename");

            // Use MoveService for planning - it handles all Cargo package logic internally
            let edit_plan_result = self.move_service()
                .plan_directory_move(&old_abs_dir, &new_abs_dir, scan_scope)
                .await;

            // Now perform the actual directory rename
            info!(
                edits_planned = edit_plan_result.as_ref().map(|p| p.edits.len()).unwrap_or(0),
                "Found affected files, now performing directory rename"
            );
            self.perform_rename(&old_abs_dir, &new_abs_dir).await?;
            info!("Directory renamed successfully");

            // Apply the edit plan to update imports
            match edit_plan_result {
                Ok(edit_plan) => match self.apply_edit_plan(&edit_plan).await {
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
                        let error_msg =
                            format!("Failed to apply import edits for directory rename: {}", e);
                        warn!(error = %e, "Import update failed for directory");
                        all_errors.push(error_msg);
                    }
                },
                Err(e) => {
                    warn!(error = %e, old_dir = ?old_abs_dir, "Failed to update imports for directory");
                    all_errors.push(format!("Failed to update imports for directory: {}", e));
                }
            }

            // NOTE: Cargo manifest updates (workspace members, package name, dependent crates)
            // are now handled automatically by MoveService within the EditPlan.
            // The edits are included in edit_plan.edits and have already been applied above.

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

            // Extract manifest file updates from the edit plan
            // (MoveService includes Cargo.toml edits in the EditPlan for Cargo packages)
            let manifest_updates = if is_cargo_pkg {
                // Count Cargo.toml files that were updated (from the applied edits)
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
