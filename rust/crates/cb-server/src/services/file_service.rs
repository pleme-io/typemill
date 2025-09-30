//! File operations service with import awareness

use crate::services::import_service::{ImportService, ImportUpdateReport};
use crate::services::lock_manager::LockManager;
use crate::{ServerError, ServerResult};
use cb_api::{DependencyUpdate, EditPlan, EditPlanMetadata, TextEdit};
use cb_ast::AstCache;
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
}

impl FileService {
    /// Create a new file service
    pub fn new(
        project_root: impl AsRef<Path>,
        ast_cache: Arc<AstCache>,
        lock_manager: Arc<LockManager>,
    ) -> Self {
        let project_root = project_root.as_ref().to_path_buf();
        Self {
            import_service: ImportService::new(&project_root),
            project_root,
            ast_cache,
            lock_manager,
        }
    }

    /// Rename a file and update all imports
    pub async fn rename_file_with_imports(
        &self,
        old_path: &Path,
        new_path: &Path,
        dry_run: bool,
    ) -> ServerResult<FileRenameResult> {
        info!(old_path = ?old_path, new_path = ?new_path, dry_run, "Renaming file");

        // Convert to absolute paths
        let old_abs = self.to_absolute_path(old_path);
        let new_abs = self.to_absolute_path(new_path);

        // Check if source file exists
        if !old_abs.exists() {
            return Err(ServerError::NotFound(format!(
                "Source file does not exist: {:?}",
                old_abs
            )));
        }

        // Check if destination already exists
        if new_abs.exists() && !dry_run {
            return Err(ServerError::AlreadyExists(format!(
                "Destination file already exists: {:?}",
                new_abs
            )));
        }

        // Find files that need import updates before renaming
        let affected_files = self.import_service.find_affected_files(&old_abs).await?;

        debug!(
            affected_files_count = affected_files.len(),
            "Found files potentially affected by rename"
        );

        let mut result = FileRenameResult {
            old_path: old_abs.to_string_lossy().to_string(),
            new_path: new_abs.to_string_lossy().to_string(),
            success: false,
            import_updates: None,
            error: None,
        };

        if dry_run {
            // Dry run - don't actually rename, but simulate import updates
            let import_report = self
                .import_service
                .update_imports_for_rename(&old_abs, &new_abs, true)
                .await?;

            result.success = true;
            result.import_updates = Some(import_report);
            info!("Dry run complete - no actual changes made");
        } else {
            // Perform the actual rename
            match self.perform_rename(&old_abs, &new_abs).await {
                Ok(_) => {
                    info!("File renamed successfully");

                    // Update imports in affected files
                    match self
                        .import_service
                        .update_imports_for_rename(&old_abs, &new_abs, false)
                        .await
                    {
                        Ok(import_report) => {
                            result.success = true;
                            info!(
                                imports_updated = import_report.imports_updated,
                                files_updated = import_report.files_updated,
                                "Successfully updated imports"
                            );
                            result.import_updates = Some(import_report);
                        }
                        Err(e) => {
                            // File was renamed but imports failed to update
                            warn!(error = %e, "File renamed but import updates failed");
                            result.success = true; // Partial success
                            result.error = Some(format!("Import updates failed: {}", e));
                        }
                    }
                }
                Err(e) => {
                    error!(error = %e, "Failed to rename file");
                    result.error = Some(e.to_string());
                    return Err(e);
                }
            }
        }

        Ok(result)
    }

    /// Perform the actual file rename operation
    async fn perform_rename(&self, old_path: &Path, new_path: &Path) -> ServerResult<()> {
        // Ensure parent directory exists
        if let Some(parent) = new_path.parent() {
            fs::create_dir_all(parent).await.map_err(|e| {
                ServerError::Internal(format!("Failed to create parent directory: {}", e))
            })?;
        }

        // Rename the file
        fs::rename(old_path, new_path)
            .await
            .map_err(|e| ServerError::Internal(format!("Failed to rename file: {}", e)))?;

        Ok(())
    }

    /// Create a new file with content
    pub async fn create_file(
        &self,
        path: &Path,
        content: Option<&str>,
        overwrite: bool,
    ) -> ServerResult<()> {
        let abs_path = self.to_absolute_path(path);

        // Check if file already exists
        if abs_path.exists() && !overwrite {
            return Err(ServerError::AlreadyExists(format!(
                "File already exists: {:?}",
                abs_path
            )));
        }

        // Ensure parent directory exists
        if let Some(parent) = abs_path.parent() {
            fs::create_dir_all(parent).await.map_err(|e| {
                ServerError::Internal(format!("Failed to create parent directory: {}", e))
            })?;
        }

        // Write content to file
        let content = content.unwrap_or("");
        fs::write(&abs_path, content)
            .await
            .map_err(|e| ServerError::Internal(format!("Failed to write file: {}", e)))?;

        info!(path = ?abs_path, "Created file");
        Ok(())
    }

    /// Delete a file
    pub async fn delete_file(&self, path: &Path, force: bool) -> ServerResult<()> {
        let abs_path = self.to_absolute_path(path);

        if !abs_path.exists() {
            if force {
                // Force mode - don't error if file doesn't exist
                return Ok(());
            } else {
                return Err(ServerError::NotFound(format!(
                    "File does not exist: {:?}",
                    abs_path
                )));
            }
        }

        // Check if any files import this file
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

        // Delete the file
        fs::remove_file(&abs_path)
            .await
            .map_err(|e| ServerError::Internal(format!("Failed to delete file: {}", e)))?;

        info!(path = ?abs_path, "Deleted file");
        Ok(())
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
    pub async fn write_file(&self, path: &Path, content: &str) -> ServerResult<()> {
        let abs_path = self.to_absolute_path(path);

        // Ensure parent directory exists
        if let Some(parent) = abs_path.parent() {
            fs::create_dir_all(parent).await.map_err(|e| {
                ServerError::Internal(format!("Failed to create parent directory: {}", e))
            })?;
        }

        fs::write(&abs_path, content)
            .await
            .map_err(|e| ServerError::Internal(format!("Failed to write file: {}", e)))?;

        info!(path = ?abs_path, "Wrote to file");
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
        // Step 1: Identify all files that will be affected
        let mut affected_files = std::collections::HashSet::new();

        // Main source file
        let main_file = self.to_absolute_path(Path::new(&plan.source_file));
        affected_files.insert(main_file.clone());

        // Files affected by dependency updates
        for dep_update in &plan.dependency_updates {
            let target_file = self.to_absolute_path(Path::new(&dep_update.target_file));
            affected_files.insert(target_file);
        }

        // Step 2: Create snapshots of all affected files before any modifications
        let snapshots = self.create_file_snapshots(&affected_files).await?;
        debug!(
            snapshot_count = snapshots.len(),
            "Created file snapshots for atomic operation"
        );

        let mut modified_files = Vec::new();

        // Step 3: Apply main file edits with locking
        let file_lock = self.lock_manager.get_lock(&main_file).await;
        let _guard = file_lock.write().await;

        match self.apply_file_edits(&main_file, &plan.edits).await {
            Ok(_) => {
                modified_files.push(plan.source_file.clone());
                info!(
                    edits_count = plan.edits.len(),
                    source_file = %plan.source_file,
                    "Successfully applied edits"
                );
            }
            Err(e) => {
                error!(
                    source_file = %plan.source_file,
                    error = %e,
                    "Failed to apply edits to main file"
                );
                // Rollback all changes and return error
                self.rollback_from_snapshots(&snapshots).await?;
                return Err(ServerError::Internal(format!(
                    "Failed to apply edits to main file {}: {}. All changes have been rolled back.",
                    plan.source_file, e
                )));
            }
        }

        // Guard is dropped here, releasing the lock

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
            match fs::read_to_string(file_path).await {
                Ok(content) => {
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

        // Sort edits by position (highest line/column first) to avoid offset issues
        let mut sorted_edits = edits.to_vec();
        sorted_edits.sort_by(|a, b| {
            let line_cmp = b.location.start_line.cmp(&a.location.start_line);
            if line_cmp == std::cmp::Ordering::Equal {
                b.location.start_column.cmp(&a.location.start_column)
            } else {
                line_cmp
            }
        });

        // Apply edits from end to beginning to preserve positions
        let mut modified_content = content;
        for edit in sorted_edits {
            modified_content = self.apply_single_edit(&modified_content, &edit)?;
        }

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
    fn apply_single_edit(&self, content: &str, edit: &TextEdit) -> ServerResult<String> {
        let lines: Vec<&str> = content.lines().collect();

        if edit.location.start_line as usize >= lines.len() {
            return Err(ServerError::InvalidRequest(format!(
                "Edit location line {} is beyond file length {}",
                edit.location.start_line,
                lines.len()
            )));
        }

        let mut result = Vec::new();

        // Copy lines before the edit
        for (i, line) in lines.iter().enumerate() {
            if i < edit.location.start_line as usize {
                result.push(line.to_string());
            } else if i == edit.location.start_line as usize {
                // Apply the edit to this line
                let line_chars: Vec<char> = line.chars().collect();
                let start_col = edit.location.start_column as usize;
                let end_col = if edit.location.end_line == edit.location.start_line {
                    edit.location.end_column as usize
                } else {
                    line_chars.len()
                };

                if start_col > line_chars.len() {
                    return Err(ServerError::InvalidRequest(format!(
                        "Edit start column {} is beyond line length {}",
                        start_col,
                        line_chars.len()
                    )));
                }

                let mut new_line = String::new();
                new_line.push_str(&line_chars[..start_col].iter().collect::<String>());
                new_line.push_str(&edit.new_text);

                if edit.location.end_line == edit.location.start_line {
                    // Single line edit
                    if end_col <= line_chars.len() {
                        new_line.push_str(&line_chars[end_col..].iter().collect::<String>());
                    }
                    result.push(new_line);
                } else {
                    // Multi-line edit - this line becomes the new line
                    result.push(new_line);
                    // Skip lines until end_line
                    break;
                }
            } else if i > edit.location.end_line as usize {
                // Copy lines after the edit
                result.push(line.to_string());
            }
            // Skip lines that are being replaced (between start_line and end_line)
        }

        Ok(result.join("\n"))
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
}

/// Result of a file rename operation
#[derive(Debug, Clone, serde::Serialize)]
pub struct FileRenameResult {
    /// Original file path
    pub old_path: String,
    /// New file path
    pub new_path: String,
    /// Whether the rename was successful
    pub success: bool,
    /// Import update report if applicable
    pub import_updates: Option<ImportUpdateReport>,
    /// Error message if operation failed
    pub error: Option<String>,
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

    #[tokio::test]
    async fn test_create_and_read_file() {
        let temp_dir = TempDir::new().unwrap();
        let ast_cache = Arc::new(AstCache::new());
        let lock_manager = Arc::new(LockManager::new());
        let service = FileService::new(temp_dir.path(), ast_cache, lock_manager);

        let file_path = Path::new("test.txt");
        let content = "Hello, World!";

        // Create file
        service
            .create_file(file_path, Some(content), false)
            .await
            .unwrap();

        // Read file
        let read_content = service.read_file(file_path).await.unwrap();
        assert_eq!(read_content, content);
    }

    #[tokio::test]
    async fn test_rename_file() {
        let temp_dir = TempDir::new().unwrap();
        let ast_cache = Arc::new(AstCache::new());
        let lock_manager = Arc::new(LockManager::new());
        let service = FileService::new(temp_dir.path(), ast_cache, lock_manager);

        // Create initial file
        let old_path = Path::new("old.txt");
        let new_path = Path::new("new.txt");
        service
            .create_file(old_path, Some("content"), false)
            .await
            .unwrap();

        // Rename file
        let result = service
            .rename_file_with_imports(old_path, new_path, false)
            .await
            .unwrap();
        assert!(result.success);

        // Verify old file doesn't exist and new file does
        assert!(!temp_dir.path().join(old_path).exists());
        assert!(temp_dir.path().join(new_path).exists());
    }

    #[tokio::test]
    async fn test_delete_file() {
        let temp_dir = TempDir::new().unwrap();
        let ast_cache = Arc::new(AstCache::new());
        let lock_manager = Arc::new(LockManager::new());
        let service = FileService::new(temp_dir.path(), ast_cache, lock_manager);

        let file_path = Path::new("to_delete.txt");

        // Create and then delete file
        service
            .create_file(file_path, Some("temporary"), false)
            .await
            .unwrap();
        assert!(temp_dir.path().join(file_path).exists());

        service.delete_file(file_path, false).await.unwrap();
        assert!(!temp_dir.path().join(file_path).exists());
    }

    #[tokio::test]
    async fn test_atomic_edit_plan_success() {
        use cb_api::{DependencyUpdateType, EditLocation, EditType};

        let temp_dir = TempDir::new().unwrap();
        let ast_cache = Arc::new(AstCache::new());
        let lock_manager = Arc::new(LockManager::new());
        let service = FileService::new(temp_dir.path(), ast_cache, lock_manager);

        // Create test files
        let main_file = "main.ts";
        let dep_file = "dependency.ts";

        service
            .create_file(Path::new(main_file), Some("import { foo } from './old';\nconst x = 1;"), false)
            .await
            .unwrap();
        service
            .create_file(Path::new(dep_file), Some("import './old';\nconst y = 2;"), false)
            .await
            .unwrap();

        // Create edit plan
        let plan = EditPlan {
            source_file: main_file.to_string(),
            edits: vec![TextEdit {
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
        use cb_api::{DependencyUpdateType, EditLocation, EditType};

        let temp_dir = TempDir::new().unwrap();
        let ast_cache = Arc::new(AstCache::new());
        let lock_manager = Arc::new(LockManager::new());
        let service = FileService::new(temp_dir.path(), ast_cache, lock_manager);

        // Create test files with specific content
        let main_file = "main.ts";
        let dep_file = "dependency.ts";

        let main_original = "import { foo } from './old';\nconst x = 1;";
        let dep_original = "import './old';\nconst y = 2;";

        service
            .create_file(Path::new(main_file), Some(main_original), false)
            .await
            .unwrap();
        service
            .create_file(Path::new(dep_file), Some(dep_original), false)
            .await
            .unwrap();

        // Create edit plan with invalid edit location that will fail
        let plan = EditPlan {
            source_file: main_file.to_string(),
            edits: vec![TextEdit {
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
        assert_eq!(main_content, main_original, "Main file should be rolled back");

        let dep_content = service.read_file(Path::new(dep_file)).await.unwrap();
        assert_eq!(dep_content, dep_original, "Dependency file should be rolled back");
    }

    #[tokio::test]
    async fn test_atomic_rollback_on_dependency_failure() {
        use cb_api::{DependencyUpdateType, EditLocation, EditType};

        let temp_dir = TempDir::new().unwrap();
        let ast_cache = Arc::new(AstCache::new());
        let lock_manager = Arc::new(LockManager::new());
        let service = FileService::new(temp_dir.path(), ast_cache, lock_manager);

        // Create main file
        let main_file = "main.ts";
        let main_original = "const x = 1;";

        service
            .create_file(Path::new(main_file), Some(main_original), false)
            .await
            .unwrap();

        // Create a dependency file with unparseable content that will cause AST failure
        let dep_file = "bad_syntax.ts";
        let dep_original = "<<<< this is invalid typescript syntax >>>>";

        service
            .create_file(Path::new(dep_file), Some(dep_original), false)
            .await
            .unwrap();

        // Create edit plan that will fail when trying to parse the bad dependency file
        let plan = EditPlan {
            source_file: main_file.to_string(),
            edits: vec![TextEdit {
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
        assert_eq!(main_content, main_original, "Main file should be rolled back after dependency failure");

        // Verify bad dependency file was also rolled back
        let dep_content = service.read_file(Path::new(dep_file)).await.unwrap();
        assert_eq!(dep_content, dep_original, "Dependency file should be rolled back");
    }

    #[tokio::test]
    async fn test_atomic_rollback_multiple_files() {
        use cb_api::{DependencyUpdateType, EditLocation, EditType};

        let temp_dir = TempDir::new().unwrap();
        let ast_cache = Arc::new(AstCache::new());
        let lock_manager = Arc::new(LockManager::new());
        let service = FileService::new(temp_dir.path(), ast_cache, lock_manager);

        // Create multiple files
        let main_file = "main.ts";
        let dep_file1 = "dep1.ts";
        let dep_file2 = "dep2.ts";
        let dep_file3 = "dep3.ts";

        let main_original = "const x = 1;";
        let dep1_original = "import './old1';";
        let dep2_original = "import './old2';";
        let dep3_original = "import 'this_will_cause_parse_error'; <<<< invalid syntax >>>>";

        service.create_file(Path::new(main_file), Some(main_original), false).await.unwrap();
        service.create_file(Path::new(dep_file1), Some(dep1_original), false).await.unwrap();
        service.create_file(Path::new(dep_file2), Some(dep2_original), false).await.unwrap();
        service.create_file(Path::new(dep_file3), Some(dep3_original), false).await.unwrap();

        // Create edit plan that will fail on the last dependency due to parse error
        let plan = EditPlan {
            source_file: main_file.to_string(),
            edits: vec![TextEdit {
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
        assert_eq!(main_content, main_original, "Main file should be rolled back");

        let dep1_content = service.read_file(Path::new(dep_file1)).await.unwrap();
        assert_eq!(dep1_content, dep1_original, "First dependency file should be rolled back");

        let dep2_content = service.read_file(Path::new(dep_file2)).await.unwrap();
        assert_eq!(dep2_content, dep2_original, "Second dependency file should be rolled back");

        let dep3_content = service.read_file(Path::new(dep_file3)).await.unwrap();
        assert_eq!(dep3_content, dep3_original, "Third dependency file should remain unchanged");
    }
}
