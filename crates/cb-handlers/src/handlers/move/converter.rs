#![allow(dead_code, unused_variables, clippy::mutable_key_type, clippy::needless_range_loop, clippy::ptr_arg, clippy::manual_clamp)]

//! Convert EditPlan (internal format) to MovePlan (MCP protocol format)
//!
//! This module bridges the gap between:
//! - EditPlan: Internal planning format used by MoveService
//! - MovePlan: LSP-based format expected by MCP protocol

use codebuddy_foundation::protocol::{
    refactor_plan::{MovePlan, PlanMetadata, PlanSummary},
    ApiError as ServerError, ApiResult as ServerResult, EditPlan, TextEdit as ProtocolTextEdit,
};
use lsp_types::{
    DocumentChangeOperation, DocumentChanges, OptionalVersionedTextDocumentIdentifier, RenameFile,
    ResourceOp, TextDocumentEdit, TextEdit as LspTextEdit,
};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use tracing::{debug, info};

use super::validation::{calculate_checksum, estimate_impact, extension_to_language};
use crate::handlers::tools::ToolHandlerContext;

/// Convert EditPlan to MovePlan for MCP protocol
///
/// This function:
/// 1. Reads all affected files and calculates checksums
/// 2. Builds LSP WorkspaceEdit from EditPlan
/// 3. Creates summary with affected file counts
/// 4. Generates metadata with language and impact estimates
pub async fn editplan_to_moveplan(
    edit_plan: EditPlan,
    old_path: &Path,
    new_path: &Path,
    context: &ToolHandlerContext,
    operation_id: &str,
) -> ServerResult<MovePlan> {
    info!(
        operation_id = %operation_id,
        old_path = %old_path.display(),
        new_path = %new_path.display(),
        edits_count = edit_plan.edits.len(),
        "Converting EditPlan to MovePlan"
    );

    // Resolve absolute paths
    let abs_old = std::fs::canonicalize(old_path).unwrap_or_else(|_| old_path.to_path_buf());
    let abs_new = std::fs::canonicalize(new_path.parent().unwrap_or(Path::new(".")))
        .unwrap_or_else(|_| new_path.parent().unwrap_or(Path::new(".")).to_path_buf())
        .join(new_path.file_name().unwrap_or(new_path.as_os_str()));

    // Calculate checksums for all affected files
    let file_checksums =
        calculate_file_checksums(&edit_plan.edits, &abs_old, context).await?;

    // Build LSP WorkspaceEdit
    let workspace_edit = build_workspace_edit(&abs_old, &abs_new, &edit_plan.edits)?;

    // Build summary
    let summary = PlanSummary {
        affected_files: 1 + file_checksums.len().saturating_sub(1),
        created_files: 1,
        deleted_files: 1,
    };

    // Build metadata
    let extension = old_path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("unknown");
    let language = extension_to_language(extension);
    let impact = estimate_impact(summary.affected_files);

    let metadata = PlanMetadata {
        plan_version: "1.0".to_string(),
        kind: "move".to_string(),
        language,
        estimated_impact: impact,
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    let warnings = Vec::new();

    debug!(
        affected_files = summary.affected_files,
        checksums_count = file_checksums.len(),
        "MovePlan generated"
    );

    Ok(MovePlan {
        edits: workspace_edit,
        summary,
        warnings,
        metadata,
        file_checksums,
    })
}

/// Calculate checksums for all affected files
async fn calculate_file_checksums(
    edits: &[ProtocolTextEdit],
    source_path: &Path,
    context: &ToolHandlerContext,
) -> ServerResult<HashMap<String, String>> {
    let mut file_checksums = HashMap::new();
    let mut affected_files = HashSet::new();

    // Add source file
    affected_files.insert(source_path.to_path_buf());

    // Add all files mentioned in edits
    for edit in edits {
        if let Some(ref file_path) = edit.file_path {
            affected_files.insert(Path::new(file_path).to_path_buf());
        }
    }

    // Read and checksum each file
    for file_path in affected_files {
        if file_path.exists() {
            if let Ok(content) = context.app_state.file_service.read_file(&file_path).await {
                file_checksums.insert(
                    file_path.to_string_lossy().to_string(),
                    calculate_checksum(&content),
                );
            }
        }
    }

    debug!(
        files_count = file_checksums.len(),
        "Calculated checksums for affected files"
    );

    Ok(file_checksums)
}

/// Build LSP WorkspaceEdit from EditPlan edits
fn build_workspace_edit(
    old_path: &Path,
    new_path: &Path,
    edits: &[ProtocolTextEdit],
) -> ServerResult<lsp_types::WorkspaceEdit> {
    // Start with the rename operation
    let old_uri = url::Url::from_file_path(old_path)
        .map_err(|_| ServerError::InvalidRequest("Invalid source file path".into()))?;
    let new_uri = url::Url::from_file_path(new_path)
        .map_err(|_| ServerError::InvalidRequest("Invalid destination file path".into()))?;

    let mut document_changes = vec![DocumentChangeOperation::Op(ResourceOp::Rename(
        RenameFile {
            old_uri: old_uri.as_str().parse().map_err(|e| {
                ServerError::Internal(format!("Failed to parse old URI: {}", e))
            })?,
            new_uri: new_uri.as_str().parse().map_err(|e| {
                ServerError::Internal(format!("Failed to parse new URI: {}", e))
            })?,
            options: None,
            annotation_id: None,
        },
    ))];

    // Group text edits by file
    let mut files_with_edits = HashMap::new();
    for edit in edits {
        if let Some(ref file_path) = edit.file_path {
            let path = Path::new(file_path);
            let file_uri = url::Url::from_file_path(path).map_err(|_| {
                ServerError::Internal(format!("Invalid file path for edit: {}", file_path))
            })?;

            let lsp_edit = LspTextEdit {
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
                .entry(file_uri.as_str().parse().map_err(|e| {
                    ServerError::Internal(format!("Failed to parse URI: {}", e))
                })?)
                .or_insert_with(Vec::new)
                .push(lsp_edit);
        }
    }

    // Add text document edits
    for (uri, text_edits) in files_with_edits {
        document_changes.push(DocumentChangeOperation::Edit(TextDocumentEdit {
            text_document: OptionalVersionedTextDocumentIdentifier {
                uri,
                version: Some(0),
            },
            edits: text_edits
                .into_iter()
                .map(lsp_types::OneOf::Left)
                .collect(),
        }));
    }

    Ok(lsp_types::WorkspaceEdit {
        changes: None,
        document_changes: Some(DocumentChanges::Operations(document_changes)),
        change_annotations: None,
    })
}
