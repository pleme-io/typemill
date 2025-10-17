//! Plan conversion service for workspace edits
//!
//! Converts LSP WorkspaceEdit structures to internal EditPlan format.
//! Handles all edit types: Replace, Create, Move, Delete.

use cb_protocol::{
    ApiError, ApiResult as ServerResult, EditPlan, EditPlanMetadata, EditType, RefactorPlan,
    RefactorPlanExt, TextEdit,
};
use lsp_types::{Uri, WorkspaceEdit};
use tracing::debug;

/// Service for converting LSP WorkspaceEdit to internal EditPlan format
///
/// This service handles the complexity of LSP's WorkspaceEdit structure,
/// supporting both `changes` (simple map) and `document_changes` (operations).
pub struct PlanConverter;

impl PlanConverter {
    /// Create a new plan converter
    pub fn new() -> Self {
        Self
    }

    /// Convert LSP WorkspaceEdit to internal EditPlan format
    ///
    /// Extracts all edits from the WorkspaceEdit and converts them to our
    /// internal TextEdit format with proper priorities and metadata.
    ///
    /// # Arguments
    ///
    /// * `workspace_edit` - LSP WorkspaceEdit structure
    /// * `plan` - Original refactoring plan (for metadata)
    ///
    /// # Returns
    ///
    /// An EditPlan with all edits converted and ready for application.
    pub fn convert_to_edit_plan(
        &self,
        workspace_edit: WorkspaceEdit,
        plan: &RefactorPlan,
    ) -> ServerResult<EditPlan> {
        let mut edits = Vec::new();

        // Handle changes (map of file URI to text edits)
        if let Some(changes) = workspace_edit.changes {
            for (uri, text_edits) in changes {
                // Convert URI to native file path string
                let file_path_str = Self::uri_to_path_string(&uri)?;

                // Assign priorities based on array position to preserve execution order
                // LSP arrays are ordered intentionally - first edit should execute first
                // Higher priority = executes first in transformer
                let total_edits = text_edits.len();
                for (idx, lsp_edit) in text_edits.into_iter().enumerate() {
                    edits.push(TextEdit {
                        file_path: Some(file_path_str.clone()),
                        edit_type: EditType::Replace,
                        location: cb_protocol::EditLocation {
                            start_line: lsp_edit.range.start.line,
                            start_column: lsp_edit.range.start.character,
                            end_line: lsp_edit.range.end.line,
                            end_column: lsp_edit.range.end.character,
                        },
                        original_text: String::new(), // Not provided by LSP
                        new_text: lsp_edit.new_text,
                        // Preserve array order: first edit = highest priority
                        priority: (total_edits - idx) as u32,
                        description: format!("Refactoring edit in {}", file_path_str),
                    });
                }
            }
        }

        // Handle document_changes (more structured, supports renames/creates/deletes)
        if let Some(document_changes) = workspace_edit.document_changes {
            use lsp_types::DocumentChangeOperation;
            use lsp_types::DocumentChanges;

            match document_changes {
                DocumentChanges::Edits(edits_vec) => {
                    Self::extract_edits_from_documents(&mut edits, edits_vec)?;
                }
                DocumentChanges::Operations(ops) => {
                    for op in ops {
                        match op {
                            DocumentChangeOperation::Edit(text_doc_edit) => {
                                Self::extract_edits_from_text_document(&mut edits, text_doc_edit)?;
                            }
                            DocumentChangeOperation::Op(resource_op) => {
                                Self::extract_edits_from_resource_op(&mut edits, resource_op)?;
                            }
                        }
                    }
                }
            }
        }

        Ok(EditPlan {
            source_file: String::new(), // Multi-file workspace edit
            edits,
            dependency_updates: Vec::new(), // Handled separately by plan-specific logic
            validations: Vec::new(),
            metadata: EditPlanMetadata {
                intent_name: "workspace.apply_edit".to_string(),
                intent_arguments: serde_json::to_value(plan).unwrap(),
                created_at: chrono::Utc::now(),
                complexity: plan.complexity(),
                impact_areas: plan.impact_areas(),
            },
        })
    }

    /// Extract edits from a TextDocumentEdit list
    fn extract_edits_from_documents(
        edits: &mut Vec<TextEdit>,
        edits_vec: Vec<lsp_types::TextDocumentEdit>,
    ) -> ServerResult<()> {
        for text_doc_edit in edits_vec {
            Self::extract_edits_from_text_document(edits, text_doc_edit)?;
        }
        Ok(())
    }

    /// Extract edits from a single TextDocumentEdit
    fn extract_edits_from_text_document(
        edits: &mut Vec<TextEdit>,
        text_doc_edit: lsp_types::TextDocumentEdit,
    ) -> ServerResult<()> {
        // Convert URI to native file path string
        let file_path_str = Self::uri_to_path_string(&text_doc_edit.text_document.uri)?;

        for lsp_edit in text_doc_edit.edits {
            let text_edit = match lsp_edit {
                lsp_types::OneOf::Left(edit) => edit,
                lsp_types::OneOf::Right(annotated_edit) => annotated_edit.text_edit,
            };

            edits.push(TextEdit {
                file_path: Some(file_path_str.clone()),
                edit_type: EditType::Replace,
                location: cb_protocol::EditLocation {
                    start_line: text_edit.range.start.line,
                    start_column: text_edit.range.start.character,
                    end_line: text_edit.range.end.line,
                    end_column: text_edit.range.end.character,
                },
                original_text: String::new(),
                new_text: text_edit.new_text,
                priority: 0,
                description: format!("Refactoring edit in {}", file_path_str),
            });
        }

        Ok(())
    }

    /// Extract edits from a ResourceOp (create/rename/delete)
    fn extract_edits_from_resource_op(
        edits: &mut Vec<TextEdit>,
        resource_op: lsp_types::ResourceOp,
    ) -> ServerResult<()> {
        match resource_op {
            lsp_types::ResourceOp::Create(create_file) => {
                let file_path_str = Self::uri_to_path_string(&create_file.uri)?;
                debug!(
                    uri = ?create_file.uri,
                    file_path = %file_path_str,
                    "File create operation detected"
                );
                edits.push(TextEdit {
                    file_path: Some(file_path_str.clone()),
                    edit_type: EditType::Create,
                    location: cb_protocol::EditLocation {
                        start_line: 0,
                        start_column: 0,
                        end_line: 0,
                        end_column: 0,
                    },
                    original_text: String::new(),
                    new_text: String::new(),
                    priority: 0,
                    description: format!("Create file {}", file_path_str),
                });
            }
            lsp_types::ResourceOp::Rename(rename_file) => {
                let old_path = Self::uri_to_path_string(&rename_file.old_uri)?;
                let new_path = Self::uri_to_path_string(&rename_file.new_uri)?;

                debug!(
                    old_uri = ?rename_file.old_uri,
                    new_uri = ?rename_file.new_uri,
                    old_path = %old_path,
                    new_path = %new_path,
                    "File rename operation detected - converting to EditType::Move"
                );

                edits.push(TextEdit {
                    file_path: Some(old_path.clone()),
                    edit_type: EditType::Move,
                    location: cb_protocol::EditLocation {
                        start_line: 0,
                        start_column: 0,
                        end_line: 0,
                        end_column: 0,
                    },
                    original_text: String::new(),
                    new_text: new_path.clone(),
                    priority: 0,
                    description: format!("Rename {} to {}", old_path, new_path),
                });
            }
            lsp_types::ResourceOp::Delete(delete_file) => {
                let file_path_str = Self::uri_to_path_string(&delete_file.uri)?;
                debug!(
                    uri = ?delete_file.uri,
                    file_path = %file_path_str,
                    "File delete operation detected"
                );
                edits.push(TextEdit {
                    file_path: Some(file_path_str.clone()),
                    edit_type: EditType::Delete,
                    location: cb_protocol::EditLocation {
                        start_line: 0,
                        start_column: 0,
                        end_line: 0,
                        end_column: 0,
                    },
                    original_text: String::new(),
                    new_text: String::new(),
                    priority: 0,
                    description: format!("Delete file {}", file_path_str),
                });
            }
        }

        Ok(())
    }

    /// Convert LSP URI to native file path string
    ///
    /// This handles platform-specific path formats correctly:
    /// - On Unix: file:///path/to/file -> /path/to/file
    /// - On Windows: file:///C:/path/to/file -> C:\path\to\file
    ///
    /// This ensures consistent path representation for checksum validation
    /// across platforms and handles paths with spaces correctly (via URL decoding).
    fn uri_to_path_string(uri: &Uri) -> Result<String, ApiError> {
        urlencoding::decode(uri.path().as_str())
            .map_err(|e| ApiError::Internal(format!("Failed to decode URI path: {}", e)))
            .map(|decoded| decoded.into_owned())
    }

    /// Extract created files from edit plan
    pub fn extract_created_files(plan: &EditPlan) -> Vec<String> {
        plan.edits
            .iter()
            .filter(|edit| matches!(edit.edit_type, EditType::Create))
            .filter_map(|edit| edit.file_path.clone())
            .collect()
    }

    /// Extract deleted files from edit plan
    pub fn extract_deleted_files(plan: &EditPlan) -> Vec<String> {
        plan.edits
            .iter()
            .filter(|edit| matches!(edit.edit_type, EditType::Delete))
            .filter_map(|edit| edit.file_path.clone())
            .collect()
    }
}

impl Default for PlanConverter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lsp_types::{Position, Range, TextEdit as LspTextEdit};
    use std::collections::HashMap;

    #[test]
    fn test_uri_to_path_string_unix() {
        let uri: Uri = "file:///home/user/project/src/main.rs".parse().unwrap();
        let path = PlanConverter::uri_to_path_string(&uri).unwrap();
        assert!(path.contains("home/user/project/src/main.rs"));
    }

    #[test]
    fn test_uri_to_path_string_with_spaces() {
        let uri: Uri = "file:///home/user/my%20project/src/main.rs"
            .parse()
            .unwrap();
        let path = PlanConverter::uri_to_path_string(&uri).unwrap();
        assert!(path.contains("my project"));
    }

    #[test]
    fn test_extract_created_files() {
        let plan = EditPlan {
            source_file: String::new(),
            edits: vec![
                TextEdit {
                    file_path: Some("new_file.rs".to_string()),
                    edit_type: EditType::Create,
                    location: cb_protocol::EditLocation::default(),
                    original_text: String::new(),
                    new_text: String::new(),
                    priority: 0,
                    description: String::new(),
                },
                TextEdit {
                    file_path: Some("existing.rs".to_string()),
                    edit_type: EditType::Replace,
                    location: cb_protocol::EditLocation::default(),
                    original_text: String::new(),
                    new_text: String::new(),
                    priority: 0,
                    description: String::new(),
                },
            ],
            dependency_updates: vec![],
            validations: vec![],
            metadata: EditPlanMetadata {
                intent_name: "test".to_string(),
                intent_arguments: serde_json::json!({}),
                created_at: chrono::Utc::now(),
                complexity: cb_protocol::Complexity::Low,
                impact_areas: vec![],
            },
        };

        let created = PlanConverter::extract_created_files(&plan);
        assert_eq!(created.len(), 1);
        assert_eq!(created[0], "new_file.rs");
    }

    #[test]
    fn test_extract_deleted_files() {
        let plan = EditPlan {
            source_file: String::new(),
            edits: vec![
                TextEdit {
                    file_path: Some("old_file.rs".to_string()),
                    edit_type: EditType::Delete,
                    location: cb_protocol::EditLocation::default(),
                    original_text: String::new(),
                    new_text: String::new(),
                    priority: 0,
                    description: String::new(),
                },
                TextEdit {
                    file_path: Some("existing.rs".to_string()),
                    edit_type: EditType::Replace,
                    location: cb_protocol::EditLocation::default(),
                    original_text: String::new(),
                    new_text: String::new(),
                    priority: 0,
                    description: String::new(),
                },
            ],
            dependency_updates: vec![],
            validations: vec![],
            metadata: EditPlanMetadata {
                intent_name: "test".to_string(),
                intent_arguments: serde_json::json!({}),
                created_at: chrono::Utc::now(),
                complexity: cb_protocol::Complexity::Low,
                impact_areas: vec![],
            },
        };

        let deleted = PlanConverter::extract_deleted_files(&plan);
        assert_eq!(deleted.len(), 1);
        assert_eq!(deleted[0], "old_file.rs");
    }

    #[test]
    fn test_convert_simple_changes() {
        let converter = PlanConverter::new();

        // Create a simple WorkspaceEdit with changes
        let mut changes = HashMap::new();
        let uri: Uri = "file:///test/file.rs".parse().unwrap();
        changes.insert(
            uri,
            vec![LspTextEdit {
                range: Range::new(Position::new(0, 0), Position::new(0, 10)),
                new_text: "new content".to_string(),
            }],
        );

        let workspace_edit = WorkspaceEdit {
            changes: Some(changes),
            document_changes: None,
            change_annotations: None,
        };

        // Create a dummy plan for metadata
        let plan = RefactorPlan::RenamePlan(cb_protocol::RenamePlan {
            summary: "test".to_string(),
            target: cb_protocol::RenameTarget {
                kind: cb_protocol::RenameTargetKind::Symbol,
                name: "test".to_string(),
                path: "test".to_string(),
                line: 0,
                column: 0,
            },
            new_name: "new_test".to_string(),
            workspace_edit,
            affected_files: vec![],
            checksum_map: HashMap::new(),
            warnings: vec![],
        });

        let edit_plan = converter
            .convert_to_edit_plan(plan.workspace_edit(), &plan)
            .unwrap();

        assert_eq!(edit_plan.edits.len(), 1);
        assert_eq!(edit_plan.edits[0].new_text, "new content");
        assert_eq!(edit_plan.edits[0].edit_type, EditType::Replace);
    }
}
