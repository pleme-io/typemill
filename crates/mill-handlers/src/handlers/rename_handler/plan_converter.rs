//! Shared converter for EditPlan to LSP WorkspaceEdit
//!
//! This module provides a single source of truth for converting EditPlan
//! (internal planning format) to WorkspaceEdit (LSP protocol format) for rename operations.

use codebuddy_foundation::protocol::{
    ApiError as ServerError, ApiResult as ServerResult, EditPlan,
};
use lsp_types::{
    DocumentChangeOperation, DocumentChanges, OptionalVersionedTextDocumentIdentifier, RenameFile,
    ResourceOp, TextDocumentEdit, TextEdit, Uri, WorkspaceEdit,
};
use std::collections::HashMap;
use std::path::Path;
use tracing::debug;

/// Convert EditPlan to WorkspaceEdit for rename operations
///
/// This function creates a WorkspaceEdit that contains:
/// 1. A RenameFile operation (old_path → new_path)
/// 2. TextEdit operations for all affected files (import updates, etc.)
///
/// # Arguments
///
/// * `edit_plan` - Internal edit plan from MoveService
/// * `old_abs` - Absolute path to source file/directory
/// * `new_abs` - Absolute path to destination file/directory
///
/// # Returns
///
/// LSP WorkspaceEdit ready for execution
pub fn editplan_to_workspace_edit(
    edit_plan: &EditPlan,
    old_abs: &Path,
    new_abs: &Path,
) -> ServerResult<WorkspaceEdit> {
    debug!(
        old_path = %old_abs.display(),
        new_path = %new_abs.display(),
        edits_count = edit_plan.edits.len(),
        "Converting EditPlan to WorkspaceEdit"
    );

    // Convert paths to URIs
    let old_url = url::Url::from_file_path(old_abs)
        .map_err(|_| ServerError::Internal(format!("Invalid old path: {}", old_abs.display())))?;

    let old_uri: Uri = old_url
        .as_str()
        .parse()
        .map_err(|e| ServerError::Internal(format!("Failed to parse old URI: {}", e)))?;

    let new_url = url::Url::from_file_path(new_abs)
        .map_err(|_| ServerError::Internal(format!("Invalid new path: {}", new_abs.display())))?;

    let new_uri: Uri = new_url
        .as_str()
        .parse()
        .map_err(|e| ServerError::Internal(format!("Failed to parse new URI: {}", e)))?;

    // Create document changes list starting with rename operation
    let mut document_changes = vec![DocumentChangeOperation::Op(ResourceOp::Rename(
        RenameFile {
            old_uri,
            new_uri,
            options: None,
            annotation_id: None,
        },
    ))];

    // Group text edits by file
    #[allow(clippy::mutable_key_type)]
    let mut files_with_edits: HashMap<Uri, Vec<TextEdit>> = HashMap::new();

    for edit in &edit_plan.edits {
        if let Some(ref file_path) = edit.file_path {
            let path = Path::new(file_path);

            debug!(
                file_path = %file_path,
                edit_type = ?edit.edit_type,
                description = %edit.description,
                "Processing edit for WorkspaceEdit conversion"
            );

            // Convert file path to URI
            let file_url = url::Url::from_file_path(path).map_err(|_| {
                ServerError::Internal(format!("Invalid file path for edit: {}", file_path))
            })?;

            let file_uri: Uri = file_url
                .as_str()
                .parse()
                .map_err(|e| ServerError::Internal(format!("Failed to parse URI: {}", e)))?;

            // Convert to LSP TextEdit
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
                .or_default()
                .push(lsp_edit);
        } else {
            debug!(
                edit_type = ?edit.edit_type,
                description = %edit.description,
                "Skipping edit with no file_path"
            );
        }
    }

    debug!(
        unique_files_with_edits = files_with_edits.len(),
        "Grouped edits by file"
    );

    // Add text document edits
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

    Ok(WorkspaceEdit {
        changes: None,
        document_changes: Some(DocumentChanges::Operations(document_changes)),
        change_annotations: None,
    })
}
