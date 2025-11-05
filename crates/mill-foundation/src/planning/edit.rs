//! Contains all edit plan types.
#![allow(deprecated)]

use crate::protocol::ApiResult;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Edit plan for code transformations - concrete implementation from mill-ast
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EditPlan {
    /// Source file being edited
    pub source_file: String,
    /// List of individual edits to apply
    pub edits: Vec<TextEdit>,
    /// Dependencies that need to be updated
    pub dependency_updates: Vec<DependencyUpdate>,
    /// Validation rules to check after editing
    pub validations: Vec<ValidationRule>,
    /// Plan metadata
    pub metadata: EditPlanMetadata,
}

impl EditPlan {
    /// Create an EditPlan from an LSP WorkspaceEdit
    ///
    /// Converts LSP's WorkspaceEdit format into TypeMill's EditPlan format.
    /// This enables refactoring operations to use LSP server responses directly.
    ///
    /// # Arguments
    ///
    /// * `workspace_edit` - LSP WorkspaceEdit from code action or rename
    /// * `source_file` - Primary file being edited
    /// * `intent_name` - Name of the refactoring intent (e.g., "extract_function")
    ///
    /// # Returns
    ///
    /// EditPlan with converted text edits from the WorkspaceEdit
    pub fn from_lsp_workspace_edit(
        workspace_edit: &serde_json::Value,
        source_file: impl Into<String>,
        intent_name: impl Into<String>,
    ) -> ApiResult<Self> {
        let source_file = source_file.into();
        let intent_name = intent_name.into();
        let mut edits = Vec::new();

        // Extract changes from workspace edit
        if let Some(changes) = workspace_edit.get("changes").and_then(|c| c.as_object()) {
            for (uri, file_edits) in changes {
                // Convert file:// URI to path
                let file_path = uri.strip_prefix("file://").unwrap_or(uri);

                if let Some(edit_array) = file_edits.as_array() {
                    for (idx, lsp_edit) in edit_array.iter().enumerate() {
                        let range = lsp_edit.get("range").ok_or_else(|| {
                            crate::protocol::ApiError::Parse {
                                message: "LSP edit missing range".to_string(),
                            }
                        })?;

                        let start =
                            range
                                .get("start")
                                .ok_or_else(|| crate::protocol::ApiError::Parse {
                                    message: "LSP range missing start".to_string(),
                                })?;
                        let end =
                            range
                                .get("end")
                                .ok_or_else(|| crate::protocol::ApiError::Parse {
                                    message: "LSP range missing end".to_string(),
                                })?;

                        let start_line =
                            start.get("line").and_then(|v| v.as_u64()).ok_or_else(|| {
                                crate::protocol::ApiError::Parse {
                                    message: "LSP start missing line".to_string(),
                                }
                            })? as u32;
                        let start_col =
                            start
                                .get("character")
                                .and_then(|v| v.as_u64())
                                .ok_or_else(|| crate::protocol::ApiError::Parse {
                                    message: "LSP start missing character".to_string(),
                                })? as u32;
                        let end_line =
                            end.get("line").and_then(|v| v.as_u64()).ok_or_else(|| {
                                crate::protocol::ApiError::Parse {
                                    message: "LSP end missing line".to_string(),
                                }
                            })? as u32;
                        let end_col =
                            end.get("character")
                                .and_then(|v| v.as_u64())
                                .ok_or_else(|| crate::protocol::ApiError::Parse {
                                    message: "LSP end missing character".to_string(),
                                })? as u32;

                        let new_text = lsp_edit
                            .get("newText")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();

                        edits.push(TextEdit {
                            file_path: Some(file_path.to_string()),
                            edit_type: EditType::Replace,
                            location: EditLocation {
                                start_line,
                                start_column: start_col,
                                end_line,
                                end_column: end_col,
                            },
                            original_text: String::new(), // LSP doesn't provide original text
                            new_text,
                            priority: (edit_array.len() - idx) as u32, // Reverse order for priority
                            description: format!("LSP refactoring edit in {}", file_path),
                        });
                    }
                }
            }
        }

        Ok(EditPlan {
            source_file,
            edits,
            dependency_updates: Vec::new(),
            validations: vec![ValidationRule {
                rule_type: ValidationType::SyntaxCheck,
                description: "Verify syntax after LSP refactoring".to_string(),
                parameters: HashMap::new(),
            }],
            metadata: EditPlanMetadata {
                intent_name,
                intent_arguments: workspace_edit.clone(),
                created_at: chrono::Utc::now(),
                complexity: 3,
                impact_areas: vec!["refactoring".to_string()],
                consolidation: None,
            },
        })
    }
}

/// Individual text edit operation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TextEdit {
    /// File path for this edit (relative to project root)
    /// If None, uses the source_file from EditPlan
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_path: Option<String>,
    /// Edit type classification
    pub edit_type: EditType,
    /// Location of the edit
    pub location: EditLocation,
    /// Original text to be replaced
    pub original_text: String,
    /// New text to insert
    pub new_text: String,
    /// Edit priority (higher numbers applied first)
    pub priority: u32,
    /// Description of what this edit does
    pub description: String,
}

/// Types of edits that can be performed
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum EditType {
    /// Rename identifier
    Rename,
    /// Add new import
    AddImport,
    /// Remove import
    RemoveImport,
    /// Update import path
    UpdateImport,
    /// Add new code
    Insert,
    /// Remove code
    Delete,
    /// Replace code
    Replace,
    /// Reformat code
    Format,
    /// Create a new file
    Create,
    /// Move/rename a file
    Move,
}

/// Location of an edit in the source file
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EditLocation {
    /// Start line (0-based)
    pub start_line: u32,
    /// Start column (0-based)
    pub start_column: u32,
    /// End line (0-based)
    pub end_line: u32,
    /// End column (0-based)
    pub end_column: u32,
}

/// Dependency update information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DependencyUpdate {
    /// File whose imports need updating
    pub target_file: String,
    /// Type of update needed
    pub update_type: DependencyUpdateType,
    /// Old import path/name
    pub old_reference: String,
    /// New import path/name
    pub new_reference: String,
}

/// Types of dependency updates
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum DependencyUpdateType {
    /// Update import path
    ImportPath,
    /// Update import name
    ImportName,
    /// Update export reference
    ExportReference,
}

/// Validation rule to check after editing
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ValidationRule {
    /// Rule type
    pub rule_type: ValidationType,
    /// Rule description
    pub description: String,
    /// Parameters for the validation
    pub parameters: HashMap<String, serde_json::Value>,
}

/// Types of validation that can be performed
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ValidationType {
    /// Check syntax is valid
    SyntaxCheck,
    /// Check imports resolve
    ImportResolution,
    /// Check types are correct
    TypeCheck,
    /// Check tests still pass
    TestValidation,
    /// Check formatting is correct
    FormatValidation,
}

/// Edit plan metadata
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EditPlanMetadata {
    /// Intent that generated this plan
    pub intent_name: String,
    /// Intent arguments used
    pub intent_arguments: serde_json::Value,
    /// Plan creation timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Estimated complexity (1-10)
    pub complexity: u8,
    /// Expected impact areas
    pub impact_areas: Vec<String>,
    /// Consolidation metadata (for Rust crate consolidation operations)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub consolidation: Option<ConsolidationMetadata>,
}

/// Metadata for Rust crate consolidation operations
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ConsolidationMetadata {
    /// Whether this is a consolidation operation
    pub is_consolidation: bool,
    /// The crate being consolidated (source)
    pub source_crate_name: String,
    /// The target crate receiving the consolidated code
    pub target_crate_name: String,
    /// The module name in the target crate
    pub target_module_name: String,
    /// Absolute path to source crate root
    pub source_crate_path: String,
    /// Absolute path to target crate root
    pub target_crate_path: String,
    /// Absolute path to target module directory
    pub target_module_path: String,
}
