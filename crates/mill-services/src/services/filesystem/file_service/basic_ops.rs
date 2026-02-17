use super::FileService;
use crate::services::coordination::operation_queue::{
    FileOperation, OperationTransaction, OperationType,
};
use crate::services::reference_updater::find_project_files;
use mill_foundation::core::dry_run::DryRunnable;
use mill_foundation::errors::MillError as ServerError;

type ServerResult<T> = Result<T, ServerError>;
use serde_json::{json, Value};
use std::path::Path;
use tokio::fs;
use tracing::{info, warn};

impl FileService {
    /// Create a new file with content
    pub async fn create_file(
        &self,
        path: &Path,
        content: Option<&str>,
        overwrite: bool,
        dry_run: bool,
    ) -> ServerResult<DryRunnable<Value>> {
        let abs_path = self.to_absolute_path_checked(path)?;
        let content = content.unwrap_or("").to_string();

        let exists = fs::try_exists(&abs_path).await.unwrap_or(false);

        if dry_run {
            // Preview mode - just return what would happen
            if exists && !overwrite {
                return Err(ServerError::invalid_request(format!(
                    "Resource already exists: File already exists: {:?}",
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
            if exists && !overwrite {
                return Err(ServerError::invalid_request(format!(
                    "Resource already exists: File already exists: {:?}",
                    abs_path
                )));
            }

            // Queue the operations for execution by the background worker
            let mut transaction = OperationTransaction::new(self.operation_queue.clone());

            if let Some(parent) = abs_path.parent() {
                let parent_exists = fs::try_exists(parent).await.unwrap_or(false);
                if !parent_exists {
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
                .map_err(|e| ServerError::internal(e.to_string()))?;

            info!(path = ?abs_path, "Queued create_file operation");

            // Wait for the operation to complete before returning
            self.operation_queue.wait_until_idle().await;

            // Verify the file was created
            let created = fs::try_exists(&abs_path).await.unwrap_or(false);
            if !created {
                return Err(ServerError::internal(format!(
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
        let abs_path = self.to_absolute_path_checked(path)?;
        let exists = fs::try_exists(&abs_path).await.unwrap_or(false);
        let perf_enabled = std::env::var("TYPEMILL_PERF")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);

        if dry_run {
            // Preview mode - just return what would happen
            if !exists {
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
                    return Err(ServerError::not_found(format!(
                        "File does not exist: {:?}",
                        abs_path
                    )));
                }
            }

            let affected_files_count = if !force {
                if self.reference_updater.cached_importers_count(&abs_path) == Some(0) {
                    0
                } else {
                    let detect_start = std::time::Instant::now();
                    let plugins = &self.plugin_registry.all();
                    let discover_start = std::time::Instant::now();
                    let project_files =
                        find_project_files(&self.project_root, plugins, None).await?;
                    let discover_ms = discover_start.elapsed().as_millis();
                    let refs_start = std::time::Instant::now();
                    let affected = self
                        .reference_updater
                        .find_affected_files(&abs_path, &project_files, plugins)
                        .await?;
                    let refs_ms = refs_start.elapsed().as_millis();

                    if perf_enabled {
                        info!(
                            path = %abs_path.display(),
                            discover_ms,
                            detect_refs_ms = refs_ms,
                            detect_total_ms = detect_start.elapsed().as_millis(),
                            affected_files = affected.len(),
                            dry_run,
                            "perf: delete_file_reference_detection"
                        );
                    }

                    if !affected.is_empty() {
                        return Err(ServerError::invalid_request(format!(
                            "File is imported by {} other files",
                            affected.len()
                        )));
                    }
                    affected.len()
                }
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
            if !exists {
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
                    return Err(ServerError::not_found(format!(
                        "File does not exist: {:?}",
                        abs_path
                    )));
                }
            }

            if !force && self.reference_updater.cached_importers_count(&abs_path) != Some(0) {
                let detect_start = std::time::Instant::now();
                let plugins = &self.plugin_registry.all();
                let discover_start = std::time::Instant::now();
                let project_files = find_project_files(&self.project_root, plugins, None).await?;
                let discover_ms = discover_start.elapsed().as_millis();
                let refs_start = std::time::Instant::now();
                let affected = self
                    .reference_updater
                    .find_affected_files(&abs_path, &project_files, plugins)
                    .await?;
                let refs_ms = refs_start.elapsed().as_millis();

                if perf_enabled {
                    info!(
                        path = %abs_path.display(),
                        discover_ms,
                        detect_refs_ms = refs_ms,
                        detect_total_ms = detect_start.elapsed().as_millis(),
                        affected_files = affected.len(),
                        dry_run,
                        "perf: delete_file_reference_detection"
                    );
                }

                if !affected.is_empty() {
                    warn!(
                        affected_files_count = affected.len(),
                        "File is imported by other files. Use force=true to delete anyway"
                    );
                    return Err(ServerError::invalid_request(format!(
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
                .map_err(|e| ServerError::internal(e.to_string()))?;

            info!(path = ?abs_path, "Queued delete_file operation");

            // Wait for the operation to complete before returning
            self.operation_queue.wait_until_idle().await;

            // Verify the file was deleted
            let still_exists = fs::try_exists(&abs_path).await.unwrap_or(false);
            if still_exists {
                return Err(ServerError::internal(format!(
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
        let abs_path = self.to_absolute_path_checked(path)?;

        if !fs::try_exists(&abs_path).await.unwrap_or(false) {
            return Err(ServerError::not_found(format!(
                "File does not exist: {:?}",
                abs_path
            )));
        }

        let content = fs::read_to_string(&abs_path)
            .await
            .map_err(|e| ServerError::internal(format!("Failed to read file: {}", e)))?;

        Ok(content)
    }

    /// Write content to file
    pub async fn write_file(
        &self,
        path: &Path,
        content: &str,
        dry_run: bool,
    ) -> ServerResult<DryRunnable<Value>> {
        let abs_path = self.to_absolute_path_checked(path)?;
        let content = content.to_string();

        let exists = fs::try_exists(&abs_path).await.unwrap_or(false);

        if dry_run {
            // Preview mode - just return what would happen
            Ok(DryRunnable::new(
                true,
                json!({
                    "operation": "write_file",
                    "path": abs_path.to_string_lossy(),
                    "content_size": content.len(),
                    "exists": exists,
                }),
            ))
        } else {
            // Execution mode - queue the operation
            let mut transaction = OperationTransaction::new(self.operation_queue.clone());

            if let Some(parent) = abs_path.parent() {
                let parent_exists = fs::try_exists(parent).await.unwrap_or(false);
                if !parent_exists {
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
                .map_err(|e| ServerError::internal(e.to_string()))?;

            info!(path = ?abs_path, "Queued write_file operation");

            // Wait for the operation to complete before returning
            self.operation_queue.wait_until_idle().await;

            // Verify the file was written
            let written = fs::try_exists(&abs_path).await.unwrap_or(false);
            if !written {
                return Err(ServerError::internal(format!(
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
        let abs_path = self.to_absolute_path_checked(path)?;

        if !fs::try_exists(&abs_path).await.unwrap_or(false) {
            return Err(ServerError::not_found(format!(
                "Directory not found: {}",
                abs_path.display()
            )));
        }

        if !abs_path.is_dir() {
            return Err(ServerError::invalid_request(format!(
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
                .map_err(|e| ServerError::internal(format!("Failed to read directory: {}", e)))?;

            while let Some(entry) = entries.next_entry().await.map_err(|e| {
                ServerError::internal(format!("Failed to read directory entry: {}", e))
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
            ServerError::invalid_request(format!("Invalid glob pattern '{}': {}", pattern, e))
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
            .map_err(|e| ServerError::internal(format!("Failed to read directory: {}", e)))?;

        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| ServerError::internal(format!("Failed to read directory entry: {}", e)))?
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
}
