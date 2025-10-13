use super::FileService;
use cb_protocol::{
    ApiError as ServerError, ApiResult as ServerResult, DependencyUpdate, EditPlan,
    EditPlanMetadata, TextEdit,
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::{debug, error, info, warn};

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

impl FileService {
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
        // Skip file operations (Move, Create, Delete) - they're handled separately
        use cb_protocol::EditType;
        use std::collections::HashMap;
        let mut edits_by_file: HashMap<String, Vec<&cb_protocol::TextEdit>> = HashMap::new();

        for edit in &plan.edits {
            // Skip file operations - they're handled in Step 3
            if matches!(
                edit.edit_type,
                EditType::Move | EditType::Create | EditType::Delete
            ) {
                continue;
            }

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
        let mut created_files = Vec::new();
        let mut deleted_files = Vec::new();

        // Step 3: Process file operations (Move, Create, Delete) first
        for edit in &plan.edits {
            match edit.edit_type {
                EditType::Move => {
                    // File rename/move operation
                    if let Some(old_path_str) = &edit.file_path {
                        let new_path_str = &edit.new_text;
                        let old_path = Path::new(old_path_str);
                        let new_path = Path::new(new_path_str);

                        info!(
                            old_path = %old_path_str,
                            new_path = %new_path_str,
                            "Executing file rename operation"
                        );

                        // Perform low-level file rename without import updates
                        // (import updates should be handled separately via dependency_updates in the plan)
                        let abs_old_path = self.to_absolute_path(old_path);
                        let abs_new_path = self.to_absolute_path(new_path);

                        // Create parent directory for new path if needed
                        if let Some(parent) = abs_new_path.parent() {
                            fs::create_dir_all(parent).await.map_err(|e| {
                                ServerError::Internal(format!(
                                    "Failed to create parent directory for {}: {}",
                                    new_path_str, e
                                ))
                            })?;
                        }

                        // Perform the actual file system rename
                        fs::rename(&abs_old_path, &abs_new_path)
                            .await
                            .map_err(|e| {
                                error!(error = %e, "File rename failed");
                                ServerError::Internal(format!(
                                    "Failed to rename {} to {}: {}",
                                    old_path_str, new_path_str, e
                                ))
                            })?;

                        modified_files.push(new_path_str.clone());
                        deleted_files.push(old_path_str.clone());
                    }
                }
                EditType::Create => {
                    // File creation operation
                    if let Some(file_path_str) = &edit.file_path {
                        let file_path = Path::new(file_path_str);

                        info!(file_path = %file_path_str, "Executing file create operation");

                        // Create parent directories if needed
                        if let Some(parent) = file_path.parent() {
                            fs::create_dir_all(parent).await.map_err(|e| {
                                ServerError::Internal(format!(
                                    "Failed to create parent directory for {}: {}",
                                    file_path_str, e
                                ))
                            })?;
                        }

                        // Create empty file or with initial content from new_text
                        fs::write(file_path, &edit.new_text).await.map_err(|e| {
                            ServerError::Internal(format!(
                                "Failed to create file {}: {}",
                                file_path_str, e
                            ))
                        })?;

                        created_files.push(file_path_str.clone());
                        modified_files.push(file_path_str.clone());
                    }
                }
                EditType::Delete => {
                    // File or directory deletion operation
                    if let Some(file_path_str) = &edit.file_path {
                        let file_path = Path::new(file_path_str);

                        info!(file_path = %file_path_str, "Executing delete operation");

                        // Check if it's a file or directory
                        if file_path.is_dir() {
                            // Delete directory recursively
                            fs::remove_dir_all(file_path).await.map_err(|e| {
                                ServerError::Internal(format!(
                                    "Failed to delete directory {}: {}",
                                    file_path_str, e
                                ))
                            })?;
                        } else {
                            // Delete single file
                            fs::remove_file(file_path).await.map_err(|e| {
                                ServerError::Internal(format!(
                                    "Failed to delete file {}: {}",
                                    file_path_str, e
                                ))
                            })?;
                        }

                        deleted_files.push(file_path_str.clone());
                    }
                }
                _ => {
                    // Not a file operation - will be handled in text edit phase
                }
            }
        }

        // Step 4: Apply text edits grouped by file with locking
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

        // Step 5: Apply dependency updates to other files with locking
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

        // Step 6: Invalidate AST cache for all modified files
        for file_path in &modified_files {
            let abs_path = self.to_absolute_path(Path::new(file_path));
            self.ast_cache.invalidate(&abs_path);
            debug!(file_path = %file_path, "Invalidated AST cache");
        }

        // Step 7: All operations successful - snapshots can be dropped
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
        self.reference_updater
            .update_import_reference(file_path, update, &self.plugin_registry.all())
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
}
