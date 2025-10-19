#![allow(dead_code, unused_variables, clippy::mutable_key_type, clippy::needless_range_loop, clippy::ptr_arg, clippy::manual_clamp)]

use crate::handlers::tools::ToolHandlerContext;
use super::{RenamePlanParams, RenameHandler};
use cb_protocol::{
    refactor_plan::{PlanMetadata, PlanSummary, PlanWarning, RenamePlan},
    ApiError as ServerError, ApiResult as ServerResult,
};
use lsp_types::{
    DocumentChangeOperation, DocumentChanges, OptionalVersionedTextDocumentIdentifier, RenameFile,
    ResourceOp, TextDocumentEdit, TextEdit, Uri, WorkspaceEdit,
};
use std::collections::HashMap;
use std::path::Path;
use tracing::{debug, info};

impl RenameHandler {
    /// Auto-detect if this is a consolidation move
    ///
    /// Detects when moving a Rust crate into another crate's src/ directory.
    /// Pattern: crates/source-crate → crates/target-crate/src/module
    fn is_consolidation_move(old_path: &Path, new_path: &Path) -> bool {
        // Check if source is a Cargo package
        let has_source_cargo = old_path.join("Cargo.toml").exists();

        // Check if target path is inside another crate's src/ directory
        let mut target_in_src = false;
        let mut parent_has_cargo = false;

        for ancestor in new_path.ancestors() {
            if ancestor.file_name().and_then(|n| n.to_str()) == Some("src") {
                target_in_src = true;
                // Check if this src's parent has Cargo.toml
                if let Some(crate_root) = ancestor.parent() {
                    if crate_root.join("Cargo.toml").exists() {
                        parent_has_cargo = true;
                        break;
                    }
                }
            }
        }

        has_source_cargo && target_in_src && parent_has_cargo
    }

    /// Generate plan for directory rename using FileService
    pub(crate) async fn plan_directory_rename(
        &self,
        params: &RenamePlanParams,
        context: &ToolHandlerContext,
    ) -> ServerResult<RenamePlan> {
        debug!(
            old_path = %params.target.path,
            new_path = %params.new_name,
            "Planning directory rename"
        );

        let old_path = Path::new(&params.target.path);
        let new_path = Path::new(&params.new_name);

        // Determine if this is a consolidation (explicit flag or auto-detect)
        let is_consolidation = params.options.consolidate
            .unwrap_or_else(|| Self::is_consolidation_move(old_path, new_path));

        if is_consolidation {
            info!(
                old_path = %old_path.display(),
                new_path = %new_path.display(),
                "Detected consolidation move - will merge Cargo.toml and update imports"
            );
        }

        // Get scope configuration from options
        let rename_scope = params.options.to_rename_scope();

        // Get the EditPlan with import updates
        let edit_plan = context
            .app_state
            .file_service
            .plan_rename_directory_with_imports(old_path, new_path, rename_scope.as_ref())
            .await?;

        debug!(
            edits_count = edit_plan.edits.len(),
            "Got EditPlan with text edits for import updates"
        );

        // Also get basic metadata from the old dry-run method
        let dry_run_result = context
            .app_state
            .file_service
            .rename_directory_with_imports(old_path, new_path, true, is_consolidation, None, false)
            .await?;

        // Extract metadata from dry-run result
        // Note: dry_run_result is DryRunnable<Value>
        let files_to_move = dry_run_result
            .result
            .get("files_to_move")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize;

        // For directory rename, we need to calculate checksums for all files being moved
        let abs_old = std::fs::canonicalize(old_path).unwrap_or_else(|_| old_path.to_path_buf());

        // Calculate abs_new early so we can use it for checksum fallback logic
        let abs_new = if new_path.is_absolute() {
            std::fs::canonicalize(new_path.parent().unwrap_or(Path::new(".")))
                .unwrap_or_else(|_| new_path.parent().unwrap_or(Path::new(".")).to_path_buf())
                .join(new_path.file_name().unwrap_or(new_path.as_os_str()))
        } else {
            // For relative paths, resolve against current working directory
            let cwd = std::env::current_dir().unwrap_or_else(|_| Path::new(".").to_path_buf());
            let parent = new_path.parent().unwrap_or(Path::new("."));
            let parent_abs = if parent == Path::new(".") {
                cwd.clone()
            } else {
                cwd.join(parent)
            };
            parent_abs.join(new_path.file_name().unwrap_or(new_path.as_os_str()))
        };

        let mut file_checksums = HashMap::new();

        // Walk directory to collect files and calculate checksums
        // IMPORTANT: Store checksums with paths at the OLD/CURRENT location.
        // Validation happens BEFORE the rename, so files exist at their old location.
        let walker = ignore::WalkBuilder::new(&abs_old).hidden(false).build();
        for entry in walker.flatten() {
            if entry.path().is_file() {
                if let Ok(content) = context.app_state.file_service.read_file(entry.path()).await {
                    // Store checksum with current (old) path where file exists now
                    file_checksums.insert(
                        entry.path().to_string_lossy().to_string(),
                        super::utils::calculate_checksum(&content),
                    );
                }
            }
        }

        // Add checksums for files being updated (import updates outside the moved directory)
        for edit in &edit_plan.edits {
            if let Some(ref file_path) = edit.file_path {
                let path = Path::new(file_path);

                // Skip files inside the directory being moved (they're covered by directory walk above)
                // Only checksum files OUTSIDE the moved directory that are being edited
                if path.exists() && !path.starts_with(&abs_old) {
                    if let Ok(content) = context.app_state.file_service.read_file(path).await {
                        // Store checksum with current path where file exists
                        file_checksums.insert(
                            file_path.clone(),
                            super::utils::calculate_checksum(&content),
                        );
                    }
                }
            }
        }

        // Create WorkspaceEdit with both rename operation AND import updates
        let old_url = url::Url::from_file_path(&abs_old)
            .map_err(|_| ServerError::Internal(format!("Invalid old path: {}", abs_old.display())))?;

        let old_uri: Uri = old_url
            .as_str()
            .parse()
            .map_err(|e| ServerError::Internal(format!("Failed to parse URI: {}", e)))?;

        // abs_new was calculated earlier for checksum fallback logic

        let new_url = url::Url::from_file_path(&abs_new)
            .map_err(|_| ServerError::Internal(format!("Invalid new path: {}", abs_new.display())))?;

        let new_uri: Uri = new_url
            .as_str()
            .parse()
            .map_err(|e| ServerError::Internal(format!("Failed to parse URI: {}", e)))?;

        // Create document changes list with both rename operation AND text edits
        let mut document_changes = vec![
            // First, the rename operation
            DocumentChangeOperation::Op(ResourceOp::Rename(RenameFile {
                old_uri,
                new_uri,
                options: None,
                annotation_id: None,
            })),
        ];

        // Then, add text edits for updating imports in external files
        let mut files_with_edits = HashMap::new();

        // DEBUG: Log all edits from EditPlan before conversion
        let all_edit_paths: std::collections::HashSet<_> = edit_plan.edits
            .iter()
            .filter_map(|e| e.file_path.as_deref())
            .collect();

        // Write debug info to file since MCP server logs aren't captured by tests
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("/tmp/directory_rename_debug.log")
        {
            use std::io::Write;
            let _ = writeln!(file, "\n===== DIRECTORY RENAME DEBUG =====");
            let _ = writeln!(file, "Total edits from EditPlan: {}", edit_plan.edits.len());
            let _ = writeln!(file, "abs_old: {}", abs_old.display());
            let _ = writeln!(file, "abs_new: {}", abs_new.display());
            let _ = writeln!(file, "Edit file paths: {:?}", all_edit_paths);
            let _ = writeln!(file, "==================================\n");
        }

        debug!(
            total_edits = edit_plan.edits.len(),
            edit_paths = ?all_edit_paths,
            abs_old = %abs_old.display(),
            abs_new = %abs_new.display(),
            "Preparing to convert EditPlan to WorkspaceEdit"
        );

        let mut edits_added_count = 0;
        for edit in &edit_plan.edits {
            if let Some(ref file_path) = edit.file_path {
                let path = Path::new(file_path);

                // DEBUG: Log each edit to file
                if let Ok(mut file) = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open("/tmp/directory_rename_debug.log")
                {
                    use std::io::Write;
                    let _ = writeln!(file, "Processing edit: file_path={}, edit_type={:?}, description={}, exists={}",
                        file_path, edit.edit_type, edit.description, path.exists());
                }

                // DEBUG: Log each edit being processed
                debug!(
                    file_path = %file_path,
                    edit_type = ?edit.edit_type,
                    description = %edit.description,
                    "Processing edit for WorkspaceEdit conversion"
                );

                let file_url = url::Url::from_file_path(path).map_err(|_| {
                    ServerError::Internal(format!("Invalid file path for edit: {}", file_path))
                })?;
                let file_uri: Uri = file_url
                    .as_str()
                    .parse()
                    .map_err(|e| ServerError::Internal(format!("Failed to parse URI: {}", e)))?;

                let lsp_edit = TextEdit {
                    range: lsp_types::Range {
                        start: lsp_types::Position {
                            line: edit.location.start_line,
                            character: edit.location.start_column,
                        },
                        end: lsp_types::Position {
                            line: edit.location.end_line,
                            character: edit.location.end_column,
                        },
                    },
                    new_text: edit.new_text.clone(),
                };

                files_with_edits
                    .entry(file_uri)
                    .or_insert_with(Vec::new)
                    .push(lsp_edit);

                edits_added_count += 1;
                debug!(
                    file_path = %file_path,
                    "Successfully added edit to WorkspaceEdit"
                );
            } else {
                debug!(
                    edit_type = ?edit.edit_type,
                    description = %edit.description,
                    "Skipping edit with no file_path"
                );
            }
        }

        // DEBUG: Log summary to file
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("/tmp/directory_rename_debug.log")
        {
            use std::io::Write;
            let _ = writeln!(file, "\nSummary:");
            let _ = writeln!(file, "  Edits added to WorkspaceEdit: {}", edits_added_count);
            let _ = writeln!(file, "  Unique files with edits: {}", files_with_edits.len());
            let _ = writeln!(file, "===================================\n");
        }

        debug!(
            edits_added_to_workspace_edit = edits_added_count,
            unique_files_with_edits = files_with_edits.len(),
            "Finished converting EditPlan to WorkspaceEdit"
        );

        // Add all text document edits
        for (uri, edits) in files_with_edits {
            document_changes.push(DocumentChangeOperation::Edit(TextDocumentEdit {
                text_document: OptionalVersionedTextDocumentIdentifier {
                    uri,
                    version: Some(0),
                },
                edits: edits.into_iter().map(lsp_types::OneOf::Left).collect(),
            }));
        }

        // DEBUG: Log document_changes count
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("/tmp/directory_rename_debug.log")
        {
            use std::io::Write;
            let _ = writeln!(file, "\n=== WORKSPACE EDIT ===");
            let _ = writeln!(file, "Total document_changes operations: {}", document_changes.len());
            for (i, op) in document_changes.iter().enumerate() {
                match op {
                    DocumentChangeOperation::Op(ResourceOp::Rename(r)) => {
                        let _ = writeln!(file, "  [{}] RenameFile: {:?} -> {:?}", i, r.old_uri, r.new_uri);
                    }
                    DocumentChangeOperation::Edit(e) => {
                        let _ = writeln!(file, "  [{}] TextEdit: {:?} ({} edits)", i, e.text_document.uri, e.edits.len());
                    }
                    _ => {
                        let _ = writeln!(file, "  [{}] Other operation", i);
                    }
                }
            }
            let _ = writeln!(file, "======================\n");
        }

        let workspace_edit = WorkspaceEdit {
            changes: None,
            document_changes: Some(DocumentChanges::Operations(document_changes)),
            change_annotations: None,
        };

        // Build summary
        let summary = PlanSummary {
            affected_files: files_to_move,
            created_files: files_to_move,
            deleted_files: files_to_move,
        };

        // Add warning if this is a Cargo package
        let mut warnings = Vec::new();
        if dry_run_result
            .result
            .get("is_cargo_package")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            warnings.push(PlanWarning {
                code: "CARGO_PACKAGE_RENAME".to_string(),
                message: "Renaming a Cargo package will update workspace members and dependencies"
                    .to_string(),
                candidates: None,
            });
        }

        // Add consolidation-specific warning
        if is_consolidation {
            let target_crate_root = new_path
                .ancestors()
                .find(|p| {
                    p.file_name()
                        .and_then(|n| n.to_str())
                        .map(|n| n == "src")
                        .unwrap_or(false)
                        && p.parent()
                            .map(|parent| parent.join("Cargo.toml").exists())
                            .unwrap_or(false)
                })
                .and_then(|src_dir| src_dir.parent())
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "target crate".to_string());

            let module_name = new_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("module");

            warnings.push(PlanWarning {
                code: "CONSOLIDATION_MANUAL_STEP".to_string(),
                message: format!(
                    "After consolidation, manually add 'pub mod {};' to {}/src/lib.rs to expose the consolidated code",
                    module_name, target_crate_root
                ),
                candidates: None,
            });
        }

        // Build metadata
        let metadata = PlanMetadata {
            plan_version: "1.0".to_string(),
            kind: "rename".to_string(),
            language: "rust".to_string(), // Assume Rust for directory renames with Cargo
            estimated_impact: super::utils::estimate_impact(files_to_move),
            created_at: chrono::Utc::now().to_rfc3339(),
        };

        // DEBUG: Log checksums to file
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("/tmp/directory_rename_debug.log")
        {
            use std::io::Write;
            let _ = writeln!(file, "\n=== CHECKSUMS IN RENAMEPLAN ===");
            for (path, checksum) in &file_checksums {
                let _ = writeln!(file, "  {}: {}", path, checksum);
            }
            let _ = writeln!(file, "================================\n");
        }

        Ok(RenamePlan {
            edits: workspace_edit,
            summary,
            warnings,
            metadata,
            file_checksums,
            is_consolidation,
        })
    }
}
