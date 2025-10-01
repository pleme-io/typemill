//! Core API types and traits for the codebuddy system
//!
//! This crate provides foundational types, traits, and error handling
//! that are shared across all workspace crates. It has no dependencies
//! on other cb-* crates to prevent circular dependencies.

pub mod error;

pub use error::{ApiError, ApiResult};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Generic message type for protocol communication
/// This will be mapped to specific protocol types (MCP, LSP) in other crates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: Option<String>,
    pub method: String,
    pub params: serde_json::Value,
}

/// Import graph representation - concrete implementation from cb-ast
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ImportGraph {
    /// Source file path
    pub source_file: String,
    /// Direct imports from this file
    pub imports: Vec<ImportInfo>,
    /// Files that import this file
    pub importers: Vec<String>,
    /// Dependency graph metadata
    pub metadata: ImportGraphMetadata,
}

/// Information about a single import
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ImportInfo {
    /// The imported module path/name
    pub module_path: String,
    /// Import type (ES module, CommonJS, etc.)
    pub import_type: ImportType,
    /// Named imports
    pub named_imports: Vec<NamedImport>,
    /// Default import name (if any)
    pub default_import: Option<String>,
    /// Namespace import name (if any)
    pub namespace_import: Option<String>,
    /// Whether this is a type-only import
    pub type_only: bool,
    /// Source location in the file
    pub location: SourceLocation,
}

/// Named import information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NamedImport {
    /// Original name in the module
    pub name: String,
    /// Local alias (if renamed)
    pub alias: Option<String>,
    /// Whether this is a type-only import
    pub type_only: bool,
}

/// Import/export type classification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ImportType {
    /// ES module import (import/export)
    EsModule,
    /// CommonJS require
    CommonJs,
    /// Dynamic import()
    Dynamic,
    /// AMD require
    Amd,
    /// TypeScript import type
    TypeOnly,
    /// Python import statement
    PythonImport,
    /// Python from...import statement
    PythonFromImport,
}

/// Source location information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SourceLocation {
    /// Start line (0-based)
    pub start_line: u32,
    /// Start column (0-based)
    pub start_column: u32,
    /// End line (0-based)
    pub end_line: u32,
    /// End column (0-based)
    pub end_column: u32,
}

/// Import graph metadata
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ImportGraphMetadata {
    /// File extension/language
    pub language: String,
    /// Parsing timestamp
    pub parsed_at: chrono::DateTime<chrono::Utc>,
    /// Parser version
    pub parser_version: String,
    /// Circular dependencies detected
    pub circular_dependencies: Vec<Vec<String>>,
    /// External dependencies (not in project)
    pub external_dependencies: Vec<String>,
}

/// Edit plan for code transformations - concrete implementation from cb-ast
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
    /// Converts LSP's WorkspaceEdit format into CodeBuddy's EditPlan format.
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
                        let range = lsp_edit.get("range").ok_or_else(|| ApiError::Parse {
                            message: "LSP edit missing range".to_string(),
                        })?;

                        let start = range.get("start").ok_or_else(|| ApiError::Parse {
                            message: "LSP range missing start".to_string(),
                        })?;
                        let end = range.get("end").ok_or_else(|| ApiError::Parse {
                            message: "LSP range missing end".to_string(),
                        })?;

                        let start_line = start.get("line").and_then(|v| v.as_u64()).ok_or_else(|| ApiError::Parse {
                            message: "LSP start missing line".to_string(),
                        })? as u32;
                        let start_col = start.get("character").and_then(|v| v.as_u64()).ok_or_else(|| ApiError::Parse {
                            message: "LSP start missing character".to_string(),
                        })? as u32;
                        let end_line = end.get("line").and_then(|v| v.as_u64()).ok_or_else(|| ApiError::Parse {
                            message: "LSP end missing line".to_string(),
                        })? as u32;
                        let end_col = end.get("character").and_then(|v| v.as_u64()).ok_or_else(|| ApiError::Parse {
                            message: "LSP end missing character".to_string(),
                        })? as u32;

                        let new_text = lsp_edit.get("newText")
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
}

/// Cache statistics for monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    /// Number of cache hits
    pub hits: u64,
    /// Number of cache misses
    pub misses: u64,
    /// Number of cache invalidations
    pub invalidations: u64,
    /// Number of cache inserts
    pub inserts: u64,
    /// Current number of cached entries
    pub current_entries: usize,
}

impl CacheStats {
    /// Calculate hit ratio as a percentage
    pub fn hit_ratio(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            (self.hits as f64 / total as f64) * 100.0
        }
    }

    /// Check if cache is performing well (arbitrary threshold of 70% hit ratio)
    pub fn is_performing_well(&self) -> bool {
        self.hit_ratio() >= 70.0 && (self.hits + self.misses) >= 10
    }
}

impl std::fmt::Display for CacheStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Cache Stats: {} entries, {}/{} hits/total ({:.1}% hit ratio), {} invalidations, {} inserts",
            self.current_entries,
            self.hits,
            self.hits + self.misses,
            self.hit_ratio(),
            self.invalidations,
            self.inserts
        )
    }
}

// IntentSpec comes from cb-core::model::IntentSpec

/// AST service interface
#[async_trait]
pub trait AstService: Send + Sync {
    /// Build import graph for a file
    async fn build_import_graph(&self, file: &Path) -> ApiResult<ImportGraph>;

    /// Get cache statistics for monitoring
    async fn cache_stats(&self) -> CacheStats;
}

/// LSP service interface
#[async_trait]
pub trait LspService: Send + Sync {
    /// Send an LSP request and get response
    async fn request(&self, message: Message) -> ApiResult<Message>;

    /// Check if LSP server is available for file extension
    async fn is_available(&self, extension: &str) -> bool;

    /// Restart LSP server for given extensions
    async fn restart_servers(&self, extensions: Option<Vec<String>>) -> ApiResult<()>;

    /// Notify LSP server that a file has been opened
    async fn notify_file_opened(&self, file_path: &Path) -> ApiResult<()>;
}

/// Message dispatcher interface for transport layer
/// Note: For now using serde_json::Value for maximum flexibility,
/// this can be refined to more specific types later
#[async_trait]
pub trait MessageDispatcher: Send + Sync {
    /// Dispatch a message and return response
    async fn dispatch(&self, message: serde_json::Value) -> ApiResult<serde_json::Value>;
}
