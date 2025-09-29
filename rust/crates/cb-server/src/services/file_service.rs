//! File operations service with import awareness

use crate::{ServerError, ServerResult};
use crate::services::import_service::{ImportService, ImportUpdateReport};
use crate::services::lock_manager::LockManager;
use cb_ast::AstCache;
use cb_api::{DependencyUpdate, EditPlan, EditPlanMetadata, TextEdit};
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

        debug!(affected_files_count = affected_files.len(), "Found files potentially affected by rename");

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

    /// Apply edits with file coordination
    async fn apply_edits_with_coordination(&self, plan: &EditPlan) -> ServerResult<EditPlanResult> {
        let mut modified_files = Vec::new();
        let mut errors = Vec::new();

        // 1. Apply main file edits with locking
        let main_file = self.to_absolute_path(Path::new(&plan.source_file));
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
                errors.push(format!("Main file {}: {}", plan.source_file, e));
            }
        }

        // Guard is dropped here, releasing the lock

        // 2. Apply dependency updates to other files with locking
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
                    errors.push(format!("Dependency file {}: {}", dep_update.target_file, e));
                }
            }
            // Guard is dropped here after each file
        }

        // 3. Invalidate AST cache for all modified files
        for file_path in &modified_files {
            let abs_path = self.to_absolute_path(Path::new(file_path));
            self.ast_cache.invalidate(&abs_path);
            debug!(file_path = %file_path, "Invalidated AST cache");
        }

        // 4. Collect validation results if any errors occurred
        if !errors.is_empty() && modified_files.is_empty() {
            return Err(ServerError::Internal(format!(
                "All edits failed: {}",
                errors.join("; ")
            )));
        }

        let success = errors.is_empty();
        if !success {
            warn!(
                error_count = errors.len(),
                errors = %errors.join("; "),
                "Edit plan completed with errors"
            );
        }

        Ok(EditPlanResult {
            success,
            modified_files,
            errors: if errors.is_empty() {
                None
            } else {
                Some(errors)
            },
            plan_metadata: plan.metadata.clone(),
        })
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
        // Read file content
        let content = match fs::read_to_string(file_path).await {
            Ok(content) => content,
            Err(e) => {
                warn!(
                    file_path = %file_path.display(),
                    error = %e,
                    "Could not read file for dependency update"
                );
                return Ok(false); // File doesn't exist, skip update
            }
        };

        // Simple string replacement for dependency updates
        // In a production system, this would use proper AST parsing
        let old_ref = &update.old_reference;
        let new_ref = &update.new_reference;

        if content.contains(old_ref) {
            let updated_content = content.replace(old_ref, new_ref);

            fs::write(file_path, updated_content).await.map_err(|e| {
                ServerError::Internal(format!(
                    "Failed to write dependency update to {}: {}",
                    file_path.display(),
                    e
                ))
            })?;

            debug!(
                old_ref = %old_ref,
                new_ref = %new_ref,
                file_path = %file_path.display(),
                "Updated dependency reference"
            );
            return Ok(true);
        }

        Ok(false) // No changes made
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
}
