use crate::handlers::common::calculate_checksums_for_directory_rename;
use crate::handlers::tools::ToolHandlerContext;
use super::{RenamePlanParams, RenameHandler};
use codebuddy_foundation::protocol::{
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

        // Resolve paths against workspace root, not CWD
        let workspace_root = &context.app_state.project_root;
        let old_path = if Path::new(&params.target.path).is_absolute() {
            Path::new(&params.target.path).to_path_buf()
        } else {
            workspace_root.join(&params.target.path)
        };
        let new_path = if Path::new(&params.new_name).is_absolute() {
            Path::new(&params.new_name).to_path_buf()
        } else {
            workspace_root.join(&params.new_name)
        };

        // Determine if this is a consolidation (explicit flag or auto-detect)
        let is_consolidation = params.options.consolidate
            .unwrap_or_else(|| Self::is_consolidation_move(&old_path, &new_path));

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
            .plan_rename_directory_with_imports(&old_path, &new_path, rename_scope.as_ref())
            .await?;

        debug!(
            edits_count = edit_plan.edits.len(),
            "Got EditPlan with text edits for import updates"
        );

        // Also get basic metadata from the old dry-run method
        let dry_run_result = context
            .app_state
            .file_service
            .rename_directory_with_imports(&old_path, &new_path, true, is_consolidation, None, false)
            .await?;

        // Extract metadata from dry-run result
        // Note: dry_run_result is DryRunnable<Value>
        let files_to_move = dry_run_result
            .result
            .get("files_to_move")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize;

        // For directory rename, we need to calculate checksums for all files being moved
        // Paths are already resolved against workspace root, so canonicalize directly
        let abs_old = std::fs::canonicalize(&old_path).unwrap_or_else(|_| old_path.clone());

        // Calculate abs_new early so we can use it for checksum fallback logic
        // new_path is already resolved against workspace root or is absolute
        let abs_new = if new_path.exists() {
            std::fs::canonicalize(&new_path).unwrap_or_else(|_| new_path.clone())
        } else {
            // For non-existent paths, canonicalize parent and join filename
            let parent = new_path.parent().unwrap_or(workspace_root);
            let parent_abs = std::fs::canonicalize(parent)
                .unwrap_or_else(|_| parent.to_path_buf());
            parent_abs.join(new_path.file_name().unwrap_or(new_path.as_os_str()))
        };

        // Calculate checksums for all affected files using shared utility
        // IMPORTANT: Checksums are stored with paths at the OLD/CURRENT location.
        // Validation happens BEFORE the rename, so files exist at their old location.
        let file_checksums =
            calculate_checksums_for_directory_rename(&abs_old, &edit_plan.edits, context).await?;

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

        debug!(
            document_changes_count = document_changes.len(),
            "Created WorkspaceEdit with document changes"
        );

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

        debug!(
            checksum_count = file_checksums.len(),
            "Generated file checksums for rename plan"
        );

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
