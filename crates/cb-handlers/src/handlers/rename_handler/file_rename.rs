#![allow(dead_code, unused_variables, clippy::mutable_key_type, clippy::needless_range_loop, clippy::ptr_arg, clippy::manual_clamp)]

use crate::handlers::tools::ToolHandlerContext;
use super::{RenamePlanParams, RenameHandler};
use codebuddy_foundation::protocol::{
    refactor_plan::{PlanMetadata, PlanSummary, RenamePlan},
    ApiError as ServerError, ApiResult as ServerResult,
};
use lsp_types::{
    DocumentChangeOperation, DocumentChanges, OptionalVersionedTextDocumentIdentifier, RenameFile,
    ResourceOp, TextDocumentEdit, TextEdit, Uri, WorkspaceEdit,
};
use std::collections::HashMap;
use std::path::Path;
use tracing::debug;

impl RenameHandler {
    /// Generate plan for file rename using FileService
    pub(crate) async fn plan_file_rename(
        &self,
        params: &RenamePlanParams,
        context: &ToolHandlerContext,
    ) -> ServerResult<RenamePlan> {
        debug!(
            old_path = %params.target.path,
            new_path = %params.new_name,
            "Planning file rename"
        );

        let old_path = Path::new(&params.target.path);
        let new_path = Path::new(&params.new_name);

        // Get scope configuration from options
        let rename_scope = params.options.to_rename_scope();

        // Call the new FileService method to get the EditPlan
        let edit_plan = context
            .app_state
            .file_service
            .plan_rename_file_with_imports(old_path, new_path, rename_scope.as_ref())
            .await?;

        let abs_old = std::fs::canonicalize(old_path).unwrap_or_else(|_| old_path.to_path_buf());
        let abs_new = std::fs::canonicalize(new_path.parent().unwrap_or(Path::new(".")))
            .unwrap_or_else(|_| new_path.parent().unwrap_or(Path::new(".")).to_path_buf())
            .join(new_path.file_name().unwrap_or(new_path.as_os_str()));

        debug!(
            edits_count = edit_plan.edits.len(),
            dependency_updates_count = edit_plan.dependency_updates.len(),
            "Got EditPlan with text edits for reference updates"
        );

        // DEBUG: Log detailed edit information for same-crate moves
        if !edit_plan.edits.is_empty() {
            tracing::info!(
                edits_count = edit_plan.edits.len(),
                first_edit_file = ?edit_plan.edits.first().and_then(|e| e.file_path.as_ref()),
                first_edit_type = ?edit_plan.edits.first().map(|e| &e.edit_type),
                "plan_file_rename: Received edits from FileService"
            );
        } else {
            tracing::warn!(
                old_path = %old_path.display(),
                new_path = %new_path.display(),
                "plan_file_rename: No edits received from FileService for file rename!"
            );
        }

        // Read file content for checksum
        let content = context
            .app_state
            .file_service
            .read_file(&abs_old)
            .await
            .map_err(|e| {
                ServerError::Internal(format!("Failed to read file for checksum: {}", e))
            })?;

        // Calculate checksums for all affected files
        let mut file_checksums = HashMap::new();
        file_checksums.insert(
            abs_old.to_string_lossy().to_string(),
            super::utils::calculate_checksum(&content),
        );

        // Add checksums for files being updated
        for edit in &edit_plan.edits {
            if let Some(ref file_path) = edit.file_path {
                let path = Path::new(file_path);
                if path.exists() && path != abs_old.as_path() {
                    if let Ok(content) = context.app_state.file_service.read_file(path).await {
                        file_checksums.insert(
                            path.to_string_lossy().to_string(),
                            super::utils::calculate_checksum(&content),
                        );
                    }
                }
            }
        }

        let old_url = url::Url::from_file_path(&abs_old)
            .map_err(|_| ServerError::Internal(format!("Invalid old path: {}", abs_old.display())))?;

        let old_uri: Uri = old_url
            .as_str()
            .parse()
            .map_err(|e| ServerError::Internal(format!("Failed to parse URI: {}", e)))?;

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

        // Then, add text edits for updating references in other files
        let mut files_with_edits = HashMap::new();
        for edit in &edit_plan.edits {
            if let Some(ref file_path) = edit.file_path {
                let path = Path::new(file_path);
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
            }
        }

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

        let workspace_edit = WorkspaceEdit {
            changes: None,
            document_changes: Some(DocumentChanges::Operations(document_changes.clone())),
            change_annotations: None,
        };

        // DEBUG: Log the final WorkspaceEdit structure
        tracing::info!(
            document_changes_count = document_changes.len(),
            has_text_edits = document_changes.iter().any(|dc| matches!(dc, DocumentChangeOperation::Edit(_))),
            "plan_file_rename: Built WorkspaceEdit with document changes"
        );

        // Build summary from actual edit plan
        let affected_files = 1 + file_checksums.len().saturating_sub(1); // Target file + files being updated

        let summary = PlanSummary {
            affected_files,
            created_files: 1,
            deleted_files: 1,
        };

        // No warnings for simple file rename
        let warnings = Vec::new();

        // Determine language from extension
        let extension = old_path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("unknown");
        let language = super::utils::extension_to_language(extension);

        // Build metadata
        let metadata = PlanMetadata {
            plan_version: "1.0".to_string(),
            kind: "rename".to_string(),
            language,
            estimated_impact: super::utils::estimate_impact(affected_files),
            created_at: chrono::Utc::now().to_rfc3339(),
        };

        Ok(RenamePlan {
            edits: workspace_edit,
            summary,
            warnings,
            metadata,
            file_checksums,
            is_consolidation: false, // File renames are never consolidations
        })
    }
}
