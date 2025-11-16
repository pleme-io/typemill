//! Builder for creating EditPlan instances with consistent structure and validation.
//!
//! This builder eliminates duplication of EditPlan construction across language plugins
//! by providing a fluent API for building plans with sensible defaults.
//!
//! # Design Rationale
//!
//! All 9 language plugins have nearly identical EditPlan construction code in their
//! refactoring functions (`plan_extract_variable`, `plan_inline_variable`, `plan_extract_function`).
//! This builder encapsulates the common pattern and eliminates ~450 LOC of duplication.
//!
//! # Example
//! ```rust
//! use mill_lang_common::refactoring::edit_plan_builder::EditPlanBuilder;
//! use mill_foundation::planning::TextEdit;
//!
//! let plan = EditPlanBuilder::new("src/app.rs", "inline_variable")
//!     .with_edits(vec![/* edits */])
//!     .with_syntax_validation("Verify syntax after inlining")
//!     .with_intent_args(serde_json::json!({ "variable": "foo" }))
//!     .with_complexity(3)
//!     .with_impact_area("variable_inlining")
//!     .build();
//! ```

use mill_foundation::planning::{
    EditPlan, EditPlanMetadata, TextEdit, ValidationRule, ValidationType,
};
use std::collections::HashMap;

/// Builder for creating EditPlan instances.
///
/// Provides a fluent interface for constructing EditPlan objects with consistent
/// defaults and validation. Eliminates ~450 LOC of duplicated code across language plugins.
///
/// # Common Usage Patterns
///
/// ## Inline Variable
/// ```rust,ignore
/// EditPlanBuilder::new(file_path, "inline_variable")
///     .with_edits(edits)
///     .with_syntax_validation("Verify syntax is valid after inlining")
///     .with_intent_args(json!({ "variable": var_name }))
///     .with_complexity_from_count(usage_count)
///     .with_impact_area("variable_inlining")
///     .build()
/// ```
///
/// ## Extract Function
/// ```rust,ignore
/// EditPlanBuilder::new(file_path, "extract_function")
///     .with_edits(edits)
///     .with_syntax_validation("Verify syntax is valid after extraction")
///     .with_intent_args(json!({
///         "function_name": name,
///         "line_count": line_count
///     }))
///     .with_complexity(5)
///     .with_impact_area("function_extraction")
///     .build()
/// ```
///
/// ## Extract Variable with Type Checking
/// ```rust,ignore
/// EditPlanBuilder::new(file_path, "extract_variable")
///     .with_edits(edits)
///     .with_syntax_validation("Verify syntax is valid")
///     .with_type_check_validation()  // Additional validation
///     .with_intent_args(json!({ "variable_name": name }))
///     .with_complexity(3)
///     .with_impact_areas(vec!["variable_extraction", "readability"])
///     .build()
/// ```
pub struct EditPlanBuilder {
    source_file: String,
    intent_name: String,
    edits: Vec<TextEdit>,
    validations: Vec<ValidationRule>,
    intent_arguments: serde_json::Value,
    complexity: u8,
    impact_areas: Vec<String>,
}

impl EditPlanBuilder {
    /// Create a new EditPlanBuilder.
    ///
    /// # Arguments
    /// * `source_file` - Path to the file being refactored
    /// * `intent_name` - Name of the refactoring intent (e.g., "inline_variable", "extract_function")
    ///
    /// # Example
    /// ```rust
    /// use mill_lang_common::refactoring::edit_plan_builder::EditPlanBuilder;
    ///
    /// let builder = EditPlanBuilder::new("src/app.rs", "inline_variable");
    /// ```
    pub fn new(source_file: impl Into<String>, intent_name: impl Into<String>) -> Self {
        Self {
            source_file: source_file.into(),
            intent_name: intent_name.into(),
            edits: Vec::new(),
            validations: Vec::new(),
            intent_arguments: serde_json::json!({}),
            complexity: 1,
            impact_areas: Vec::new(),
        }
    }

    /// Set the edits for this plan.
    ///
    /// # Example
    /// ```rust,ignore
    /// builder.with_edits(vec![
    ///     TextEdit { /* ... */ },
    ///     TextEdit { /* ... */ },
    /// ])
    /// ```
    pub fn with_edits(mut self, edits: Vec<TextEdit>) -> Self {
        self.edits = edits;
        self
    }

    /// Add a syntax validation rule.
    ///
    /// This is the most common validation used across all refactoring operations.
    ///
    /// # Example
    /// ```rust
    /// use mill_lang_common::refactoring::edit_plan_builder::EditPlanBuilder;
    ///
    /// let builder = EditPlanBuilder::new("src/app.rs", "inline_variable")
    ///     .with_syntax_validation("Verify syntax is valid after inlining");
    /// ```
    pub fn with_syntax_validation(mut self, description: impl Into<String>) -> Self {
        self.validations.push(ValidationRule {
            rule_type: ValidationType::SyntaxCheck,
            description: description.into(),
            parameters: HashMap::new(),
        });
        self
    }

    /// Add a type check validation rule.
    ///
    /// Used for typed languages (TypeScript, Rust, Java, etc.) to ensure type consistency.
    ///
    /// # Example
    /// ```rust
    /// use mill_lang_common::refactoring::edit_plan_builder::EditPlanBuilder;
    ///
    /// let builder = EditPlanBuilder::new("src/app.ts", "extract_variable")
    ///     .with_syntax_validation("Verify syntax is valid")
    ///     .with_type_check_validation();
    /// ```
    pub fn with_type_check_validation(mut self) -> Self {
        self.validations.push(ValidationRule {
            rule_type: ValidationType::TypeCheck,
            description: "Verify types are valid after refactoring".to_string(),
            parameters: HashMap::new(),
        });
        self
    }

    /// Add a custom validation rule.
    ///
    /// Use this for validation types not covered by the convenience methods.
    ///
    /// # Example
    /// ```rust,ignore
    /// builder.with_validation(ValidationRule {
    ///     rule_type: ValidationType::TestValidation,
    ///     description: "Run tests to verify behavior".to_string(),
    ///     parameters: HashMap::new(),
    /// })
    /// ```
    pub fn with_validation(mut self, validation: ValidationRule) -> Self {
        self.validations.push(validation);
        self
    }

    /// Set the intent arguments (JSON value).
    ///
    /// Intent arguments capture the parameters used for the refactoring operation,
    /// useful for debugging and audit trails.
    ///
    /// # Example
    /// ```rust
    /// use mill_lang_common::refactoring::edit_plan_builder::EditPlanBuilder;
    ///
    /// let builder = EditPlanBuilder::new("src/app.rs", "inline_variable")
    ///     .with_intent_args(serde_json::json!({
    ///         "variable": "count",
    ///         "line": 42,
    ///         "column": 10
    ///     }));
    /// ```
    pub fn with_intent_args(mut self, args: serde_json::Value) -> Self {
        self.intent_arguments = args;
        self
    }

    /// Set the complexity score (1-10).
    ///
    /// Complexity helps prioritize refactoring operations and estimate impact.
    /// The value is automatically clamped to the range 1-10.
    ///
    /// # Example
    /// ```rust
    /// use mill_lang_common::refactoring::edit_plan_builder::EditPlanBuilder;
    ///
    /// let builder = EditPlanBuilder::new("src/app.rs", "extract_function")
    ///     .with_complexity(7);
    /// ```
    pub fn with_complexity(mut self, complexity: u8) -> Self {
        self.complexity = complexity.max(1).min(10);
        self
    }

    /// Calculate complexity from a count (clamped to 10).
    ///
    /// Common pattern: complexity = min(item_count, 10)
    ///
    /// This is useful for refactoring operations where complexity correlates
    /// with the number of occurrences or edits.
    ///
    /// # Example
    /// ```rust
    /// use mill_lang_common::refactoring::edit_plan_builder::EditPlanBuilder;
    ///
    /// // Inline variable with 15 usages -> complexity = 10
    /// let builder = EditPlanBuilder::new("src/app.rs", "inline_variable")
    ///     .with_complexity_from_count(15);
    /// ```
    pub fn with_complexity_from_count(mut self, count: usize) -> Self {
        self.complexity = (count.min(10)) as u8;
        self.complexity = self.complexity.max(1); // Ensure at least 1
        self
    }

    /// Add a single impact area.
    ///
    /// Impact areas are tags that categorize the effect of the refactoring.
    ///
    /// # Example
    /// ```rust
    /// use mill_lang_common::refactoring::edit_plan_builder::EditPlanBuilder;
    ///
    /// let builder = EditPlanBuilder::new("src/app.rs", "inline_variable")
    ///     .with_impact_area("variable_inlining");
    /// ```
    pub fn with_impact_area(mut self, area: impl Into<String>) -> Self {
        self.impact_areas.push(area.into());
        self
    }

    /// Set multiple impact areas.
    ///
    /// # Example
    /// ```rust
    /// use mill_lang_common::refactoring::edit_plan_builder::EditPlanBuilder;
    ///
    /// let builder = EditPlanBuilder::new("src/app.rs", "extract_variable")
    ///     .with_impact_areas(vec![
    ///         "variable_extraction".to_string(),
    ///         "readability".to_string()
    ///     ]);
    /// ```
    pub fn with_impact_areas(mut self, areas: Vec<String>) -> Self {
        self.impact_areas = areas;
        self
    }

    /// Build the EditPlan.
    ///
    /// # Example
    /// ```rust
    /// use mill_lang_common::refactoring::edit_plan_builder::EditPlanBuilder;
    ///
    /// let plan = EditPlanBuilder::new("src/app.rs", "inline_variable")
    ///     .with_complexity(3)
    ///     .build();
    /// ```
    pub fn build(self) -> EditPlan {
        EditPlan {
            source_file: self.source_file,
            edits: self.edits,
            dependency_updates: Vec::new(),
            validations: self.validations,
            metadata: EditPlanMetadata {
                intent_name: self.intent_name,
                intent_arguments: self.intent_arguments,
                created_at: chrono::Utc::now(),
                complexity: self.complexity,
                impact_areas: self.impact_areas,
                consolidation: None,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mill_foundation::planning::{EditLocation, EditType};

    #[test]
    fn test_basic_builder() {
        let plan = EditPlanBuilder::new("test.rs", "test_refactoring")
            .with_complexity(5)
            .build();

        assert_eq!(plan.source_file, "test.rs");
        assert_eq!(plan.metadata.intent_name, "test_refactoring");
        assert_eq!(plan.metadata.complexity, 5);
        assert_eq!(plan.edits.len(), 0);
        assert_eq!(plan.validations.len(), 0);
    }

    #[test]
    fn test_with_syntax_validation() {
        let plan = EditPlanBuilder::new("test.rs", "inline_variable")
            .with_syntax_validation("Check syntax")
            .build();

        assert_eq!(plan.validations.len(), 1);
        assert_eq!(plan.validations[0].rule_type, ValidationType::SyntaxCheck);
        assert_eq!(plan.validations[0].description, "Check syntax");
    }

    #[test]
    fn test_with_type_check_validation() {
        let plan = EditPlanBuilder::new("test.ts", "extract_variable")
            .with_type_check_validation()
            .build();

        assert_eq!(plan.validations.len(), 1);
        assert_eq!(plan.validations[0].rule_type, ValidationType::TypeCheck);
        assert_eq!(
            plan.validations[0].description,
            "Verify types are valid after refactoring"
        );
    }

    #[test]
    fn test_multiple_validations() {
        let plan = EditPlanBuilder::new("test.ts", "extract_function")
            .with_syntax_validation("Verify syntax is valid after extraction")
            .with_type_check_validation()
            .build();

        assert_eq!(plan.validations.len(), 2);
        assert_eq!(plan.validations[0].rule_type, ValidationType::SyntaxCheck);
        assert_eq!(plan.validations[1].rule_type, ValidationType::TypeCheck);
    }

    #[test]
    fn test_complexity_from_count() {
        let plan = EditPlanBuilder::new("test.rs", "inline")
            .with_complexity_from_count(15) // Should clamp to 10
            .build();

        assert_eq!(plan.metadata.complexity, 10);
    }

    #[test]
    fn test_complexity_from_zero_count() {
        let plan = EditPlanBuilder::new("test.rs", "inline")
            .with_complexity_from_count(0) // Should default to 1
            .build();

        assert_eq!(plan.metadata.complexity, 1);
    }

    #[test]
    fn test_complexity_clamping() {
        // Test upper bound
        let plan_high = EditPlanBuilder::new("test.rs", "refactor")
            .with_complexity(15) // Should clamp to 10
            .build();
        assert_eq!(plan_high.metadata.complexity, 10);

        // Test lower bound
        let plan_low = EditPlanBuilder::new("test.rs", "refactor")
            .with_complexity(0) // Should clamp to 1
            .build();
        assert_eq!(plan_low.metadata.complexity, 1);

        // Test normal range
        let plan_normal = EditPlanBuilder::new("test.rs", "refactor")
            .with_complexity(5)
            .build();
        assert_eq!(plan_normal.metadata.complexity, 5);
    }

    #[test]
    fn test_single_impact_area() {
        let plan = EditPlanBuilder::new("test.rs", "extract_function")
            .with_impact_area("readability")
            .build();

        assert_eq!(plan.metadata.impact_areas.len(), 1);
        assert_eq!(plan.metadata.impact_areas[0], "readability");
    }

    #[test]
    fn test_multiple_impact_areas() {
        let plan = EditPlanBuilder::new("test.rs", "extract_function")
            .with_impact_area("readability")
            .with_impact_area("maintainability")
            .build();

        assert_eq!(plan.metadata.impact_areas.len(), 2);
        assert_eq!(plan.metadata.impact_areas[0], "readability");
        assert_eq!(plan.metadata.impact_areas[1], "maintainability");
    }

    #[test]
    fn test_with_impact_areas() {
        let plan = EditPlanBuilder::new("test.rs", "extract_function")
            .with_impact_areas(vec![
                "readability".to_string(),
                "maintainability".to_string(),
            ])
            .build();

        assert_eq!(plan.metadata.impact_areas.len(), 2);
    }

    #[test]
    fn test_intent_arguments() {
        let plan = EditPlanBuilder::new("test.rs", "extract")
            .with_intent_args(serde_json::json!({
                "function_name": "test_func",
                "line_count": 10
            }))
            .build();

        assert!(plan.metadata.intent_arguments.get("function_name").is_some());
        assert_eq!(
            plan.metadata.intent_arguments.get("function_name").unwrap(),
            "test_func"
        );
    }

    #[test]
    fn test_with_edits() {
        let edits = vec![
            TextEdit {
                file_path: None,
                edit_type: EditType::Replace,
                location: EditLocation {
                    start_line: 0,
                    start_column: 0,
                    end_line: 0,
                    end_column: 5,
                },
                original_text: "hello".to_string(),
                new_text: "world".to_string(),
                priority: 100,
                description: "Test edit".to_string(),
            },
            TextEdit {
                file_path: None,
                edit_type: EditType::Insert,
                location: EditLocation {
                    start_line: 1,
                    start_column: 0,
                    end_line: 1,
                    end_column: 0,
                },
                original_text: String::new(),
                new_text: "new line".to_string(),
                priority: 90,
                description: "Insert new line".to_string(),
            },
        ];

        let plan = EditPlanBuilder::new("test.rs", "refactor")
            .with_edits(edits.clone())
            .build();

        assert_eq!(plan.edits.len(), 2);
        assert_eq!(plan.edits[0].new_text, "world");
        assert_eq!(plan.edits[1].new_text, "new line");
    }

    #[test]
    fn test_complete_inline_variable_example() {
        // Simulate real-world inline variable usage
        let plan = EditPlanBuilder::new("src/app.rs", "inline_variable")
            .with_edits(vec![])
            .with_syntax_validation("Verify syntax is valid after inlining")
            .with_intent_args(serde_json::json!({
                "variable": "count",
                "line": 42,
                "column": 10
            }))
            .with_complexity_from_count(7) // 7 usages
            .with_impact_area("variable_inlining")
            .build();

        assert_eq!(plan.source_file, "src/app.rs");
        assert_eq!(plan.metadata.intent_name, "inline_variable");
        assert_eq!(plan.metadata.complexity, 7);
        assert_eq!(plan.validations.len(), 1);
        assert_eq!(plan.metadata.impact_areas, vec!["variable_inlining"]);
    }

    #[test]
    fn test_complete_extract_function_example() {
        // Simulate real-world extract function usage
        let plan = EditPlanBuilder::new("src/service.ts", "extract_function")
            .with_edits(vec![])
            .with_syntax_validation("Verify syntax is valid after extraction")
            .with_type_check_validation()
            .with_intent_args(serde_json::json!({
                "function_name": "processData",
                "line_count": 15
            }))
            .with_complexity(6)
            .with_impact_area("function_extraction")
            .build();

        assert_eq!(plan.source_file, "src/service.ts");
        assert_eq!(plan.metadata.intent_name, "extract_function");
        assert_eq!(plan.metadata.complexity, 6);
        assert_eq!(plan.validations.len(), 2); // Syntax + Type check
        assert_eq!(plan.metadata.impact_areas, vec!["function_extraction"]);
    }

    #[test]
    fn test_default_values() {
        let plan = EditPlanBuilder::new("test.rs", "refactor").build();

        // Check defaults
        assert_eq!(plan.metadata.complexity, 1);
        assert!(plan.metadata.impact_areas.is_empty());
        assert!(plan.edits.is_empty());
        assert!(plan.validations.is_empty());
        assert!(plan.dependency_updates.is_empty());
        assert_eq!(plan.metadata.intent_arguments, serde_json::json!({}));
        assert!(plan.metadata.consolidation.is_none());
    }
}
