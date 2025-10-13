use super::FileService;
use crate::services::git_service::GitService;
use cb_core::dry_run::DryRunnable;
use cb_protocol::{ApiError as ServerError, ApiResult as ServerResult, EditPlan};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::{debug, info, warn};

impl FileService {
    /// Generates an EditPlan for a file rename operation, including import updates.
    /// This is a dry-run only operation.
    pub async fn plan_rename_file_with_imports(
        &self,
        old_path: &Path,
        new_path: &Path,
        scan_scope: Option<cb_plugin_api::ScanScope>,
    ) -> ServerResult<EditPlan> {
        info!(old_path = ?old_path, new_path = ?new_path, "Planning file rename with imports");

        let old_abs = self.to_absolute_path(old_path);
        let new_abs = self.to_absolute_path(new_path);

        if !old_abs.exists() {
            return Err(ServerError::NotFound(format!(
                "Source file does not exist: {:?}",
                old_abs
            )));
        }

        // The `true` flag indicates a dry run.
        self.reference_updater
            .update_references(&old_abs, &new_abs, &self.plugin_registry.all(), None, true, scan_scope)
            .await
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

        let old_abs = self.to_absolute_path(old_dir_path);
        let new_abs = self.to_absolute_path(new_dir_path);

        if !old_abs.exists() {
            return Err(ServerError::NotFound(format!(
                "Source directory does not exist: {:?}",
                old_abs
            )));
        }

        // Extract rename_info if this is a Cargo package (needed for Rust use statement updates)
        let is_cargo_pkg = self.is_cargo_package(&old_abs).await?;
        let rename_info = if is_cargo_pkg {
            self.extract_cargo_rename_info(&old_abs, &new_abs).await.ok()
        } else {
            None
        };

        // If this is a cargo package, force workspace-wide import scan
        let effective_scan_scope = if is_cargo_pkg {
            info!("Cargo package rename detected in planning phase, forcing workspace-wide import scan");
            Some(cb_plugin_api::ScanScope::AllUseStatements)
        } else {
            scan_scope
        };

        // For directory renames, we need to update imports that reference files inside the directory
        // The `true` flag indicates a dry run.
        self.reference_updater
            .update_references(&old_abs, &new_abs, &self.plugin_registry.all(), rename_info.as_ref(), true, effective_scan_scope)
            .await
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
            if new_abs.exists() {
                // Compare metadata (inode on Unix) to check if same file
                let same_file = match (fs::metadata(&old_abs).await, fs::metadata(&new_abs).await) {
                    (Ok(old_meta), Ok(new_meta)) => {
                        // On Unix, compare inodes; on other platforms, compare file paths
                        #[cfg(unix)]
                        {
                            use std::os::unix::fs::MetadataExt;
                            old_meta.ino() == new_meta.ino()
                        }
                        #[cfg(not(unix))]
                        {
                            // On non-Unix, compare canonicalized paths
                            old_abs.canonicalize().ok() == new_abs.canonicalize().ok()
                        }
                    }
                    _ => false,
                };

                // If they don't point to the same file, it's a conflict
                if !same_file {
                    return Err(ServerError::AlreadyExists(format!(
                        "Destination file already exists: {:?}",
                        new_abs
                    )));
                }
            }

            // let affected_files = self.reference_updater.find_affected_files(&old_abs).await?;

            let edit_plan = self
                .reference_updater
                .update_references(&old_abs, &new_abs, &self.plugin_registry.all(), None, true, scan_scope.clone())
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
            if new_abs.exists() {
                // Compare metadata (inode on Unix) to check if same file
                let same_file = match (fs::metadata(&old_abs).await, fs::metadata(&new_abs).await) {
                    (Ok(old_meta), Ok(new_meta)) => {
                        // On Unix, compare inodes; on other platforms, compare file paths
                        #[cfg(unix)]
                        {
                            use std::os::unix::fs::MetadataExt;
                            old_meta.ino() == new_meta.ino()
                        }
                        #[cfg(not(unix))]
                        {
                            // On non-Unix, compare canonicalized paths
                            old_abs.canonicalize().ok() == new_abs.canonicalize().ok()
                        }
                    }
                    _ => false,
                };

                // If they don't point to the same file, it's a conflict
                if !same_file {
                    return Err(ServerError::AlreadyExists(format!(
                        "Destination file already exists: {:?}",
                        new_abs
                    )));
                }
            }

            self.perform_rename(&old_abs, &new_abs).await?;

            info!("File renamed successfully");

            let mut edit_plan = self
                .reference_updater
                .update_references(&old_abs, &new_abs, &self.plugin_registry.all(), None, false, scan_scope)
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
                    "operation": "move_directory",
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

            // If this is a cargo package, we need to scan the entire workspace for
            // `use` statements and fully-qualified paths. Otherwise, respect the provided scope.
            let effective_scan_scope = if is_cargo_pkg {
                info!("Cargo package rename detected, forcing workspace-wide import scan.");
                Some(cb_plugin_api::ScanScope::AllUseStatements)
            } else {
                scan_scope
            };

            // Call update_imports_for_rename ONCE for the entire directory rename
            // This prevents creating duplicate edits for the same affected files
            match self
                .reference_updater
                .update_references(
                    &old_abs_dir, // Use directory paths instead of individual files
                    &new_abs_dir,
                    &self.plugin_registry.all(),
                    rename_info.as_ref(),
                    false,
                    effective_scan_scope,
                )
                .await
            {
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
                    if let (Some(old_package_name), Some(new_package_name)) = (
                        info.get("old_package_name").and_then(|v| v.as_str()),
                        info.get("new_package_name").and_then(|v| v.as_str()),
                    ) {
                        match self
                            .update_dependent_crate_paths(
                                old_package_name,
                                new_package_name,
                                &new_abs_dir,
                            )
                            .await
                        {
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
                                let error_msg =
                                    format!("Failed to update dependent crate paths: {}", e);
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
