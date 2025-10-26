//! Dry-run preview generator for refactoring plans
//!
//! Generates human-readable previews of what would happen if a plan were applied,
//! without actually modifying any files.

use mill_foundation::protocol::{EditPlan, EditType};
use serde::Serialize;
use std::collections::HashSet;

/// Service for generating dry-run previews of refactoring plans
///
/// This service creates preview structures showing what files would be
/// modified, created, or deleted if a plan were applied.
pub struct DryRunGenerator;

impl DryRunGenerator {
    /// Create a new dry-run generator
    pub fn new() -> Self {
        Self
    }

    /// Generate a dry-run preview for an edit plan
    ///
    /// Returns a structure showing all files that would be affected,
    /// categorized by operation type (modified, created, deleted).
    ///
    /// # Arguments
    ///
    /// * `plan` - The edit plan to preview
    /// * `warnings` - Optional warnings to include in the preview
    ///
    /// # Returns
    ///
    /// A DryRunResult suitable for serialization and display
    pub fn create_dry_run_result(&self, plan: &EditPlan, warnings: Vec<String>) -> DryRunResult {
        // Extract unique modified files (exclude Create/Delete operations)
        let modified_files: Vec<String> = plan
            .edits
            .iter()
            .filter(|edit| matches!(edit.edit_type, EditType::Replace | EditType::Move))
            .filter_map(|edit| edit.file_path.clone())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();

        let created_files = Self::extract_created_files(plan);
        let deleted_files = Self::extract_deleted_files(plan);

        DryRunResult {
            success: true,
            applied_files: modified_files,
            created_files,
            deleted_files,
            warnings,
            validation: None,
            rollback_available: false, // Dry run doesn't apply changes
        }
    }

    /// Extract files that would be created from an edit plan
    pub fn extract_created_files(plan: &EditPlan) -> Vec<String> {
        plan.edits
            .iter()
            .filter(|edit| matches!(edit.edit_type, EditType::Create))
            .filter_map(|edit| edit.file_path.clone())
            .collect()
    }

    /// Extract files that would be deleted from an edit plan
    pub fn extract_deleted_files(plan: &EditPlan) -> Vec<String> {
        plan.edits
            .iter()
            .filter(|edit| matches!(edit.edit_type, EditType::Delete))
            .filter_map(|edit| edit.file_path.clone())
            .collect()
    }
}

impl Default for DryRunGenerator {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of a dry-run preview operation
#[derive(Debug, Serialize)]
pub struct DryRunResult {
    /// Whether the preview generation succeeded
    pub success: bool,
    /// Files that would be modified
    pub applied_files: Vec<String>,
    /// Files that would be created
    pub created_files: Vec<String>,
    /// Files that would be deleted
    pub deleted_files: Vec<String>,
    /// Warnings about the operation
    pub warnings: Vec<String>,
    /// Validation result (always None for dry runs)
    pub validation: Option<ValidationResult>,
    /// Whether rollback would be available (always false for dry runs)
    pub rollback_available: bool,
}

/// Placeholder for validation result (not used in dry runs)
#[derive(Debug, Serialize)]
pub struct ValidationResult {
    pub passed: bool,
    pub command: String,
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use mill_foundation::protocol::{EditLocation, EditPlanMetadata, TextEdit};

    fn create_test_plan(edits: Vec<TextEdit>) -> EditPlan {
        EditPlan {
            source_file: String::new(),
            edits,
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
        }
    }

    fn default_location() -> EditLocation {
        EditLocation {
            start_line: 0,
            start_column: 0,
            end_line: 0,
            end_column: 0,
        }
    }

    #[test]
    fn test_dry_run_with_modified_files() {
        let generator = DryRunGenerator::new();

        let plan = create_test_plan(vec![TextEdit {
            file_path: Some("src/main.rs".to_string()),
            edit_type: EditType::Replace,
            location: default_location(),
            original_text: String::new(),
            new_text: String::new(),
            priority: 0,
            description: String::new(),
        }]);

        let result = generator.create_dry_run_result(&plan, vec![]);

        assert!(result.success);
        assert_eq!(result.applied_files.len(), 1);
        assert!(result.applied_files.contains(&"src/main.rs".to_string()));
        assert!(result.created_files.is_empty());
        assert!(result.deleted_files.is_empty());
        assert!(!result.rollback_available);
    }

    #[test]
    fn test_dry_run_with_created_files() {
        let generator = DryRunGenerator::new();

        let plan = create_test_plan(vec![TextEdit {
            file_path: Some("src/new.rs".to_string()),
            edit_type: EditType::Create,
            location: default_location(),
            original_text: String::new(),
            new_text: String::new(),
            priority: 0,
            description: String::new(),
        }]);

        let result = generator.create_dry_run_result(&plan, vec![]);

        assert!(result.success);
        assert_eq!(result.created_files.len(), 1);
        assert!(result.created_files.contains(&"src/new.rs".to_string()));
        assert!(result.applied_files.is_empty());
        assert!(result.deleted_files.is_empty());
    }

    #[test]
    fn test_dry_run_with_deleted_files() {
        let generator = DryRunGenerator::new();

        let plan = create_test_plan(vec![TextEdit {
            file_path: Some("src/old.rs".to_string()),
            edit_type: EditType::Delete,
            location: default_location(),
            original_text: String::new(),
            new_text: String::new(),
            priority: 0,
            description: String::new(),
        }]);

        let result = generator.create_dry_run_result(&plan, vec![]);

        assert!(result.success);
        assert_eq!(result.deleted_files.len(), 1);
        assert!(result.deleted_files.contains(&"src/old.rs".to_string()));
        assert!(result.applied_files.is_empty());
        assert!(result.created_files.is_empty());
    }

    #[test]
    fn test_dry_run_with_warnings() {
        let generator = DryRunGenerator::new();

        let plan = create_test_plan(vec![]);
        let warnings = vec!["Warning 1".to_string(), "Warning 2".to_string()];

        let result = generator.create_dry_run_result(&plan, warnings);

        assert!(result.success);
        assert_eq!(result.warnings.len(), 2);
        assert!(result.warnings.contains(&"Warning 1".to_string()));
        assert!(result.warnings.contains(&"Warning 2".to_string()));
    }

    #[test]
    fn test_dry_run_deduplicates_modified_files() {
        let generator = DryRunGenerator::new();

        // Multiple edits to the same file
        let plan = create_test_plan(vec![
            TextEdit {
                file_path: Some("src/main.rs".to_string()),
                edit_type: EditType::Replace,
                location: default_location(),
                original_text: String::new(),
                new_text: String::new(),
                priority: 0,
                description: String::new(),
            },
            TextEdit {
                file_path: Some("src/main.rs".to_string()),
                edit_type: EditType::Replace,
                location: default_location(),
                original_text: String::new(),
                new_text: String::new(),
                priority: 0,
                description: String::new(),
            },
        ]);

        let result = generator.create_dry_run_result(&plan, vec![]);

        assert_eq!(result.applied_files.len(), 1);
        assert!(result.applied_files.contains(&"src/main.rs".to_string()));
    }

    #[test]
    fn test_dry_run_with_move_operations() {
        let generator = DryRunGenerator::new();

        let plan = create_test_plan(vec![TextEdit {
            file_path: Some("src/old.rs".to_string()),
            edit_type: EditType::Move,
            location: default_location(),
            original_text: String::new(),
            new_text: "src/new.rs".to_string(),
            priority: 0,
            description: String::new(),
        }]);

        let result = generator.create_dry_run_result(&plan, vec![]);

        assert!(result.success);
        // Move operations count as modifications
        assert_eq!(result.applied_files.len(), 1);
        assert!(result.applied_files.contains(&"src/old.rs".to_string()));
    }
}
