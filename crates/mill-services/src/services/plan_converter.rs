//! Plan conversion service for workspace edits
//!
//! Converts LSP WorkspaceEdit structures to internal EditPlan format.
//! Handles all edit types: Replace, Create, Move, Delete.

use mill_foundation::protocol::{ ApiError , ApiResult as ServerResult , ConsolidationMetadata , EditPlan , EditPlanMetadata , EditType , RefactorPlan , RefactorPlanExt , TextEdit , };
use lsp_types::{Uri, WorkspaceEdit};
use std::path::Path;
use tracing::{debug, info};

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
        if let Some(ref changes) = workspace_edit.changes {
            for (uri, text_edits) in changes {
                // Convert URI to native file path string
                let file_path_str = Self::uri_to_path_string(uri)?;

                // Assign priorities based on array position to preserve execution order
                // LSP arrays are ordered intentionally - first edit should execute first
                // Higher priority = executes first in transformer
                let total_edits = text_edits.len();
                for (idx, lsp_edit) in text_edits.iter().enumerate() {
                    edits.push(TextEdit {
                        file_path: Some(file_path_str.clone()),
                        edit_type: EditType::Replace,
                        location: mill_foundation::protocol::EditLocation {
                            start_line: lsp_edit.range.start.line,
                            start_column: lsp_edit.range.start.character,
                            end_line: lsp_edit.range.end.line,
                            end_column: lsp_edit.range.end.character,
                        },
                        original_text: String::new(), // Not provided by LSP
                        new_text: lsp_edit.new_text.clone(),
                        // Preserve array order: first edit = highest priority
                        priority: (total_edits - idx) as u32,
                        description: format!("Refactoring edit in {}", file_path_str),
                    });
                }
            }
        }

        // Handle document_changes (more structured, supports renames/creates/deletes)
        if let Some(ref document_changes) = workspace_edit.document_changes {
            use lsp_types::DocumentChangeOperation;
            use lsp_types::DocumentChanges;

            match document_changes {
                DocumentChanges::Edits(edits_vec) => {
                    Self::extract_edits_from_documents(&mut edits, edits_vec.clone())?;
                }
                DocumentChanges::Operations(ops) => {
                    for op in ops {
                        match op {
                            DocumentChangeOperation::Edit(text_doc_edit) => {
                                Self::extract_edits_from_text_document(
                                    &mut edits,
                                    text_doc_edit.clone(),
                                )?;
                            }
                            DocumentChangeOperation::Op(resource_op) => {
                                Self::extract_edits_from_resource_op(
                                    &mut edits,
                                    resource_op.clone(),
                                )?;
                            }
                        }
                    }
                }
            }
        }

        // Extract consolidation metadata if this is a consolidation operation
        let consolidation = self.extract_consolidation_metadata(plan, &workspace_edit)?;

        // Extract refactoring kind from plan for intent name
        let intent_name = match plan {
            RefactorPlan::RenamePlan(p) => &p.metadata.kind,
            RefactorPlan::ExtractPlan(p) => &p.metadata.kind,
            RefactorPlan::InlinePlan(p) => &p.metadata.kind,
            RefactorPlan::MovePlan(p) => &p.metadata.kind,
            RefactorPlan::ReorderPlan(p) => &p.metadata.kind,
            RefactorPlan::TransformPlan(p) => &p.metadata.kind,
            RefactorPlan::DeletePlan(p) => &p.metadata.kind,
        };

        Ok(EditPlan {
            source_file: String::new(), // Multi-file workspace edit
            edits,
            dependency_updates: Vec::new(), // Handled separately by plan-specific logic
            validations: Vec::new(),
            metadata: EditPlanMetadata {
                intent_name: intent_name.clone(),
                intent_arguments: serde_json::to_value(plan).unwrap(),
                created_at: chrono::Utc::now(),
                complexity: plan.complexity(),
                impact_areas: plan.impact_areas(),
                consolidation,
            },
        })
    }

    /// Extract consolidation metadata from a RenamePlan
    ///
    /// Parses the WorkspaceEdit to extract source/target paths and determine
    /// crate names, module names, and crate roots for consolidation operations.
    fn extract_consolidation_metadata(
        &self,
        plan: &RefactorPlan,
        workspace_edit: &WorkspaceEdit,
    ) -> ServerResult<Option<ConsolidationMetadata>> {
        // Only process RenamePlan with is_consolidation flag
        let _rename_plan = match plan {
            RefactorPlan::RenamePlan(rp) if rp.is_consolidation => rp,
            _ => return Ok(None),
        };

        info!("Extracting consolidation metadata from RenamePlan");

        // Find the RenameFile operation in document_changes
        let rename_op = workspace_edit
            .document_changes
            .as_ref()
            .and_then(|dc| match dc {
                lsp_types::DocumentChanges::Operations(ops) => ops.iter().find_map(|op| match op {
                    lsp_types::DocumentChangeOperation::Op(lsp_types::ResourceOp::Rename(r)) => {
                        Some(r)
                    }
                    _ => None,
                }),
                _ => None,
            })
            .ok_or_else(|| {
                ApiError::Internal("Consolidation plan missing RenameFile operation".to_string())
            })?;

        // Convert URIs to paths
        let old_path = Self::uri_to_path_string(&rename_op.old_uri)?;
        let new_path = Self::uri_to_path_string(&rename_op.new_uri)?;

        let old_path_buf = Path::new(&old_path).to_path_buf();
        let new_path_buf = Path::new(&new_path).to_path_buf();

        // Extract source crate name and path
        let source_crate_name = old_path_buf
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| {
                ApiError::Internal(format!("Cannot extract crate name from: {}", old_path))
            })?
            .to_string();

        let source_crate_path = old_path.clone();

        // Extract target crate root and module name
        // Pattern: /path/to/crates/target-crate/src/module
        let target_module_name = new_path_buf
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| {
                ApiError::Internal(format!("Cannot extract module name from: {}", new_path))
            })?
            .to_string();

        let target_module_path = new_path.clone();

        // Find the src/ ancestor and its parent (the target crate root)
        let target_crate_path = new_path_buf
            .ancestors()
            .find(|p| p.file_name().and_then(|n| n.to_str()) == Some("src"))
            .and_then(|src_dir| src_dir.parent())
            .ok_or_else(|| {
                ApiError::Internal(format!(
                    "Cannot find target crate root (src/ parent) for: {}",
                    new_path
                ))
            })?
            .to_string_lossy()
            .to_string();

        let target_crate_name = Path::new(&target_crate_path)
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| {
                ApiError::Internal(format!(
                    "Cannot extract target crate name from: {}",
                    target_crate_path
                ))
            })?
            .to_string();

        info!(
            source_crate = %source_crate_name,
            target_crate = %target_crate_name,
            target_module = %target_module_name,
            "Extracted consolidation metadata"
        );

        Ok(Some(ConsolidationMetadata {
            is_consolidation: true,
            source_crate_name,
            target_crate_name,
            target_module_name,
            source_crate_path,
            target_crate_path,
            target_module_path,
        }))
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
                location: mill_foundation::protocol::EditLocation {
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
                    location: mill_foundation::protocol::EditLocation {
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
                    location: mill_foundation::protocol::EditLocation {
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
                    location: mill_foundation::protocol::EditLocation {
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
                    location: mill_foundation::protocol::EditLocation {
                        start_line: 0,
                        start_column: 0,
                        end_line: 0,
                        end_column: 0,
                    },
                    original_text: String::new(),
                    new_text: String::new(),
                    priority: 0,
                    description: String::new(),
                },
                TextEdit {
                    file_path: Some("existing.rs".to_string()),
                    edit_type: EditType::Replace,
                    location: mill_foundation::protocol::EditLocation {
                        start_line: 0,
                        start_column: 0,
                        end_line: 0,
                        end_column: 0,
                    },
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
                complexity: 1, // Low complexity (1-10 scale)
                impact_areas: vec![],
                consolidation: None,
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
                    location: mill_foundation::protocol::EditLocation {
                        start_line: 0,
                        start_column: 0,
                        end_line: 0,
                        end_column: 0,
                    },
                    original_text: String::new(),
                    new_text: String::new(),
                    priority: 0,
                    description: String::new(),
                },
                TextEdit {
                    file_path: Some("existing.rs".to_string()),
                    edit_type: EditType::Replace,
                    location: mill_foundation::protocol::EditLocation {
                        start_line: 0,
                        start_column: 0,
                        end_line: 0,
                        end_column: 0,
                    },
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
                complexity: 1, // Low complexity (1-10 scale)
                impact_areas: vec![],
                consolidation: None,
            },
        };

        let deleted = PlanConverter::extract_deleted_files(&plan);
        assert_eq!(deleted.len(), 1);
        assert_eq!(deleted[0], "old_file.rs");
    }
}