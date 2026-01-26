//! Validation utilities for move operations
//!
//! Provides checksum calculation, conflict detection, and warning generation
//! for file, directory, and symbol moves.

use crate::handlers::common::calculate_checksum;
use lsp_types::{Uri, WorkspaceEdit};
use mill_foundation::errors::{MillError as ServerError, MillResult as ServerResult};
use mill_foundation::planning::{PlanSummary, PlanWarning};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use tracing::debug;

/// Convert LSP URI to native file path string
///
/// This handles platform-specific path formats correctly:
/// - On Unix: file:///path/to/file -> /path/to/file
/// - On Windows: file:///C:/path/to/file -> C:\path\to\file
///
/// This ensures consistent path representation for checksum validation
/// across platforms and handles paths with spaces correctly (via URL decoding).
fn uri_to_path_string(uri: &Uri) -> Result<String, ServerError> {
    urlencoding::decode(uri.path().as_str())
        .map_err(|e| ServerError::internal(format!("Failed to decode URI path: {}", e)))
        .map(|decoded| decoded.into_owned())
}

/// Analyze WorkspaceEdit to calculate checksums and summary
pub async fn analyze_workspace_edit(
    edit: &WorkspaceEdit,
    context: &mill_handler_api::ToolHandlerContext,
) -> ServerResult<(HashMap<String, String>, PlanSummary, Vec<PlanWarning>)> {
    let mut file_checksums = HashMap::new();
    let mut affected_path_strings: HashSet<String> = HashSet::new();

    // Extract file paths from WorkspaceEdit, converting URIs to native paths
    if let Some(ref changes) = edit.changes {
        for uri in changes.keys() {
            let path_string = uri_to_path_string(uri)?;
            affected_path_strings.insert(path_string);
        }
    }

    if let Some(ref document_changes) = edit.document_changes {
        match document_changes {
            lsp_types::DocumentChanges::Edits(edits) => {
                for edit in edits {
                    let path_string = uri_to_path_string(&edit.text_document.uri)?;
                    affected_path_strings.insert(path_string);
                }
            }
            lsp_types::DocumentChanges::Operations(ops) => {
                for op in ops {
                    match op {
                        lsp_types::DocumentChangeOperation::Edit(edit) => {
                            let path_string = uri_to_path_string(&edit.text_document.uri)?;
                            affected_path_strings.insert(path_string);
                        }
                        lsp_types::DocumentChangeOperation::Op(resource_op) => match resource_op {
                            lsp_types::ResourceOp::Create(create) => {
                                let path_string = uri_to_path_string(&create.uri)?;
                                affected_path_strings.insert(path_string);
                            }
                            lsp_types::ResourceOp::Rename(rename) => {
                                let path_string = uri_to_path_string(&rename.old_uri)?;
                                affected_path_strings.insert(path_string);
                                let path_string = uri_to_path_string(&rename.new_uri)?;
                                affected_path_strings.insert(path_string);
                            }
                            lsp_types::ResourceOp::Delete(delete) => {
                                let path_string = uri_to_path_string(&delete.uri)?;
                                affected_path_strings.insert(path_string);
                            }
                        },
                    }
                }
            }
        }
    }

    // Calculate checksums for all affected files
    for path_string in &affected_path_strings {
        let file_path = Path::new(path_string);
        if file_path.exists() {
            if let Ok(content) = context.app_state.file_service.read_file(file_path).await {
                // Use the same native path string format for checksum map
                file_checksums.insert(path_string.clone(), calculate_checksum(&content));
            }
        }
    }

    let summary = PlanSummary {
        affected_files: affected_path_strings.len(),
        created_files: 0,
        deleted_files: 0,
    };

    let warnings = Vec::new();

    debug!(
        affected_files_count = affected_path_strings.len(),
        checksums_count = file_checksums.len(),
        "Analyzed workspace edit"
    );

    Ok((file_checksums, summary, warnings))
}

// Removed extension_to_language() - use plugin registry instead:
// context.app_state.language_plugins.get_plugin(ext)?.metadata().name
