use super::FileService;
use cb_protocol::{
    ApiError as ServerError, ApiResult as ServerResult, DependencyUpdate, EditPlan,
    EditPlanMetadata, TextEdit,
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::{debug, error, info, warn};

// Import the transformer for delegating text edit application
use cb_ast::transformer;

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

        // Write debug info to file
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("/tmp/directory_rename_debug.log")
        {
            use std::io::Write;
            let _ = writeln!(file, "\n=== FILE SERVICE: APPLY EDIT PLAN ===");
            let _ = writeln!(file, "Total edits in plan: {}", plan.edits.len());
            for (i, edit) in plan.edits.iter().enumerate() {
                let _ = writeln!(file, "  [{}] edit_type={:?}, file_path={:?}, description={}",
                    i, edit.edit_type, edit.file_path, edit.description);
            }
            let _ = writeln!(file, "======================================\n");
        }

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

        // Step 0: Track all Move operations to handle renamed files correctly
        // When a directory is renamed, snapshots must be created at OLD paths, but text edits
        // reference NEW paths. We need:
        // 1. A map of new_path -> old_path for snapshot lookup during Step 4
        // 2. A map of old_dir -> new_dir to translate NEW file paths to OLD for snapshot creation
        let mut path_renames: HashMap<PathBuf, PathBuf> = HashMap::new();
        let mut directory_renames: Vec<(PathBuf, PathBuf)> = Vec::new();

        for edit in &plan.edits {
            if edit.edit_type == EditType::Move {
                if let Some(old_path_str) = &edit.file_path {
                    let old_path = self.to_absolute_path(Path::new(old_path_str));
                    let new_path = self.to_absolute_path(Path::new(&edit.new_text));

                    debug!(
                        old_path = %old_path.display(),
                        new_path = %new_path.display(),
                        is_directory = old_path.is_dir(),
                        "Tracked rename for snapshot lookup"
                    );

                    // Track both single file renames and directory renames
                    path_renames.insert(new_path.clone(), old_path.clone());

                    // If it's a directory, we need to be able to map paths INSIDE it
                    // Directory rename before execution, so check at OLD path
                    if old_path.is_dir() {
                        directory_renames.push((old_path, new_path));
                    }
                }
            }
        }

        // Helper closure to map NEW paths (inside renamed directories) back to OLD paths
        // This is needed because text edits reference NEW paths, but files exist at OLD paths
        let map_new_to_old = |new_path: &PathBuf| -> PathBuf {
            // Check if this NEW path is inside any renamed directory
            for (old_dir, new_dir) in &directory_renames {
                if new_path.starts_with(new_dir) {
                    // File is inside renamed directory - map it back to OLD path
                    let relative = new_path.strip_prefix(new_dir).unwrap();
                    let old_path = old_dir.join(relative);
                    debug!(
                        new_path = %new_path.display(),
                        old_path = %old_path.display(),
                        "Mapped NEW path to OLD path for snapshot creation"
                    );
                    return old_path;
                }
            }
            // Not inside a renamed directory - use path as-is
            new_path.clone()
        };

        // Step 1: Identify all files that will be affected
        let mut affected_files = std::collections::HashSet::new();

        // Main source file (may not have edits if this is a rename operation)
        // Skip empty source_file (used in multi-file workspace edits)
        if !plan.source_file.is_empty() {
            let main_file = self.to_absolute_path(Path::new(&plan.source_file));
            let snapshot_path = map_new_to_old(&main_file);
            affected_files.insert(snapshot_path);
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
                // Map NEW path to OLD path for snapshot creation
                let snapshot_path = map_new_to_old(&abs_path);
                affected_files.insert(snapshot_path);
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
            let snapshot_path = map_new_to_old(&target_file);
            affected_files.insert(snapshot_path);
        }

        // DEBUG: Log affected_files and edits_by_file before snapshot creation
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("/tmp/directory_rename_debug.log")
        {
            use std::io::Write;
            let _ = writeln!(file, "\n=== STEP 1 RESULTS ===");
            let _ = writeln!(file, "affected_files count: {}", affected_files.len());
            for path in &affected_files {
                let exists = path.exists();
                let _ = writeln!(file, "  - {} (exists={})", path.display(), exists);
            }
            let _ = writeln!(file, "edits_by_file count: {}", edits_by_file.len());
            for (path, edits) in &edits_by_file {
                let _ = writeln!(file, "  - {} ({} edits)", path, edits.len());
            }
            let _ = writeln!(file, "======================\n");
        }

        // Step 2: Create snapshots of all affected files before any modifications
        let snapshots = self.create_file_snapshots(&affected_files).await?;
        debug!(
            snapshot_count = snapshots.len(),
            files_with_edits = edits_by_file.len(),
            "Created file snapshots for atomic operation"
        );

        // DEBUG: Log snapshot contents after creation
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("/tmp/directory_rename_debug.log")
        {
            use std::io::Write;
            let _ = writeln!(file, "\n=== STEP 2 RESULTS (SNAPSHOTS) ===");
            let _ = writeln!(file, "snapshots count: {}", snapshots.len());
            for (path, content) in &snapshots {
                let _ = writeln!(file, "  - {} (content_len={})", path.display(), content.len());
            }
            let _ = writeln!(file, "===================================\n");
        }

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

                        // Write debug info to file
                        if let Ok(mut file) = std::fs::OpenOptions::new()
                            .create(true)
                            .append(true)
                            .open("/tmp/directory_rename_debug.log")
                        {
                            use std::io::Write;
                            let _ = writeln!(file, "\n=== FILE SERVICE: EXECUTING MOVE ===");
                            let _ = writeln!(file, "EditType::Move found in EditPlan:");
                            let _ = writeln!(file, "  old_path: {}", old_path_str);
                            let _ = writeln!(file, "  new_path: {}", new_path_str);
                            let _ = writeln!(file, "  description: {}", edit.description);
                        }

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
                        let rename_result = fs::rename(&abs_old_path, &abs_new_path).await;

                        // Write debug info to file
                        if let Ok(mut file) = std::fs::OpenOptions::new()
                            .create(true)
                            .append(true)
                            .open("/tmp/directory_rename_debug.log")
                        {
                            use std::io::Write;
                            match &rename_result {
                                Ok(_) => {
                                    let _ = writeln!(file, "  fs::rename succeeded!");
                                    let _ = writeln!(file, "  abs_old_path: {}", abs_old_path.display());
                                    let _ = writeln!(file, "  abs_new_path: {}", abs_new_path.display());
                                }
                                Err(e) => {
                                    let _ = writeln!(file, "  fs::rename FAILED: {}", e);
                                    let _ = writeln!(file, "  abs_old_path: {}", abs_old_path.display());
                                    let _ = writeln!(file, "  abs_new_path: {}", abs_new_path.display());
                                }
                            }
                            let _ = writeln!(file, "=====================================\n");
                        }

                        rename_result.map_err(|e| {
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

        // Step 3.5: Execute consolidation post-processing if this is a consolidation operation
        // This must run AFTER all Move operations complete but BEFORE text edits
        if let Some(ref consolidation) = plan.metadata.consolidation {
            info!(
                source_crate = %consolidation.source_crate_name,
                target_crate = %consolidation.target_crate_name,
                "Detected consolidation operation, executing post-processing"
            );

            self.execute_consolidation_post_processing(consolidation)
                .await?;
        }

        // Step 4: Apply text edits grouped by file with locking
        // Use snapshot content to avoid race conditions with file system
        // DEBUG: Log Step 4 entry
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("/tmp/directory_rename_debug.log")
        {
            use std::io::Write;
            let _ = writeln!(file, "\n=== STEP 4: APPLYING TEXT EDITS ===");
            let _ = writeln!(file, "edits_by_file count: {}", edits_by_file.len());
            let _ = writeln!(file, "path_renames count: {}", path_renames.len());
            let _ = writeln!(file, "====================================\n");
        }

        for (file_path, edits) in edits_by_file {
            // DEBUG: Log each file being processed
            if let Ok(mut file) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open("/tmp/directory_rename_debug.log")
            {
                use std::io::Write;
                let _ = writeln!(file, "Processing file in Step 4: {} ({} edits)", file_path, edits.len());
            }

            let abs_file_path = self.to_absolute_path(Path::new(&file_path));
            let file_lock = self.lock_manager.get_lock(&abs_file_path).await;
            let _guard = file_lock.write().await;

            // Convert &TextEdit to TextEdit
            let owned_edits: Vec<cb_protocol::TextEdit> =
                edits.iter().map(|e| (*e).clone()).collect();

            // Get the original content from snapshot (guarantees atomicity)
            // For renamed files, look up the snapshot using the OLD path
            let original_content = snapshots
                .get(&abs_file_path)
                .or_else(|| {
                    // If snapshot not found at new path, check if this file was renamed
                    // First check direct file renames
                    if let Some(old_path) = path_renames.get(&abs_file_path) {
                        return snapshots.get(old_path);
                    }

                    // Then check if file is inside a renamed directory
                    for (old_dir, new_dir) in &directory_renames {
                        if abs_file_path.starts_with(new_dir) {
                            let relative = abs_file_path.strip_prefix(new_dir).unwrap();
                            let old_path = old_dir.join(relative);
                            debug!(
                                abs_file_path = %abs_file_path.display(),
                                old_path = %old_path.display(),
                                "Mapped NEW path to OLD path for snapshot lookup"
                            );
                            return snapshots.get(&old_path);
                        }
                    }

                    None
                })
                .ok_or_else(|| {
                    ServerError::Internal(format!("File {} not found in snapshots", abs_file_path.display()))
                })?;

            // DEBUG: Log snapshot content length
            if let Ok(mut file) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open("/tmp/directory_rename_debug.log")
            {
                use std::io::Write;
                let _ = writeln!(file, "  Found snapshot: content_len={}", original_content.len());
            }

            if original_content.is_empty() {
                error!(
                    file_path = %file_path,
                    "BUG: Snapshot content is EMPTY for file!"
                );
            }

            // Apply edits to the snapshot content (no I/O, fully synchronous)
            match self.apply_edits_to_content(original_content, &owned_edits) {
                Ok(modified_content) => {
                    // DEBUG: Log successful edit application
                    if let Ok(mut file) = std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open("/tmp/directory_rename_debug.log")
                    {
                        use std::io::Write;
                        let _ = writeln!(file, "  Applied edits successfully, writing to: {}", abs_file_path.display());
                    }

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

                    // DEBUG: Log successful write
                    if let Ok(mut file) = std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open("/tmp/directory_rename_debug.log")
                    {
                        use std::io::Write;
                        let _ = writeln!(file, "  Write succeeded for: {}", abs_file_path.display());
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
                    // DEBUG: Log error in applying edits
                    if let Ok(mut file) = std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open("/tmp/directory_rename_debug.log")
                    {
                        use std::io::Write;
                        let _ = writeln!(file, "  ERROR applying edits: {}", e);
                    }

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

                // Force page cache invalidation on Linux systems
                // Note: posix_fadvise exists on macOS but behaves differently, so Linux-only
                #[cfg(target_os = "linux")]
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

    /// Apply text edits to file content and return the modified content (synchronous, no I/O)
    ///
    /// Delegates to cb-ast transformer for the actual text manipulation,
    /// maintaining clean separation of concerns:
    /// - FileService: Orchestrates filesystem operations
    /// - Transformer: Single source of truth for text edit logic
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

        // Create an EditPlan for the transformer
        // Note: source_file is not used by the transformer (it only needs the edits)
        let temp_plan = EditPlan {
            source_file: String::new(), // Not used by transformer
            edits: edits.to_vec(),
            dependency_updates: Vec::new(), // Not used by transformer
            validations: Vec::new(),        // Not used by transformer
            metadata: EditPlanMetadata {
                intent_name: "apply_edits".to_string(),
                intent_arguments: serde_json::json!({}),
                created_at: chrono::Utc::now(),
                complexity: 0,
                impact_areas: Vec::new(),
                consolidation: None,
            },
        };

        // Delegate to cb-ast transformer - the single source of truth for text edits
        let transform_result = transformer::apply_edit_plan(original_content, &temp_plan)
            .map_err(|e| {
                error!(
                    error = %e,
                    edits_count = edits.len(),
                    "Transformer failed to apply edits"
                );
                ServerError::Internal(format!("Failed to apply edits: {}", e))
            })?;

        // Check if any edits were skipped - this indicates an error condition
        // For atomic operations, we must fail if ANY edit cannot be applied
        if !transform_result.skipped_edits.is_empty() {
            error!(
                skipped_count = transform_result.skipped_edits.len(),
                applied_count = transform_result.applied_edits.len(),
                "Failed to apply all edits - some were skipped"
            );

            // Log details of each skipped edit
            for skipped in &transform_result.skipped_edits {
                error!(
                    reason = %skipped.reason,
                    edit_description = %skipped.edit.description,
                    "Skipped edit details"
                );
            }

            // Return error to trigger rollback for atomic guarantees
            return Err(ServerError::Internal(format!(
                "Failed to apply {} of {} edits: {}",
                transform_result.skipped_edits.len(),
                transform_result.statistics.total_edits,
                transform_result.skipped_edits
                    .iter()
                    .map(|s| s.reason.as_str())
                    .collect::<Vec<_>>()
                    .join("; ")
            )));
        }

        debug!(
            applied_count = transform_result.statistics.applied_count,
            lines_added = transform_result.statistics.lines_added,
            lines_removed = transform_result.statistics.lines_removed,
            "Applied all edits successfully via transformer"
        );

        Ok(transform_result.transformed_source)
    }

    /// Apply a dependency update (import/export change) to a file
    async fn apply_dependency_update(
        &self,
        file_path: &Path,
        update: &DependencyUpdate,
    ) -> ServerResult<bool> {
        // Delegate the dependency update to the import service, which handles AST transformations.
        self.reference_updater
            .update_import_reference(file_path, update, self.plugin_registry.all())
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
