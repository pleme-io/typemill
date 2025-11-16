//! Builder for creating extract constant EditPlans with language-specific declaration formatting.
//!
//! This builder encapsulates the common pattern used across all 9 language plugins for extract constant
//! refactoring operations, eliminating ~750 lines of duplicated code.

use super::ExtractConstantAnalysis;
#[allow(unused_imports)]
use super::CodeRange; // Used in tests and doc comments
use mill_foundation::planning::{
    EditLocation, EditPlan, EditPlanMetadata, EditType, TextEdit, ValidationRule, ValidationType,
};
use std::collections::HashMap;

/// Builder for creating extract constant EditPlans with language-specific declaration formatting.
///
/// This builder encapsulates the common pattern used across all language plugins:
/// 1. Validate constant name (SCREAMING_SNAKE_CASE)
/// 2. Create constant declaration with language-specific syntax
/// 3. Generate insertion edit (priority 100)
/// 4. Generate replacement edits for all occurrences (priority 90, 89, 88...)
/// 5. Build complete EditPlan with metadata
///
/// # Example
/// ```rust,ignore
/// use mill_lang_common::ExtractConstantEditPlanBuilder;
///
/// let builder = ExtractConstantEditPlanBuilder::new(
///     analysis,
///     "TAX_RATE".to_string(),
///     "src/app.rs".to_string(),
/// );
///
/// let plan = builder.with_declaration_format(|name, value| {
///     format!("const {}: f64 = {};\n", name, value)  // Rust syntax
/// })?;
/// ```
pub struct ExtractConstantEditPlanBuilder {
    analysis: ExtractConstantAnalysis,
    name: String,
    file_path: String,
}

impl ExtractConstantEditPlanBuilder {
    /// Create a new builder instance.
    ///
    /// # Arguments
    /// * `analysis` - The extract constant analysis result
    /// * `name` - The proposed constant name (will be validated as SCREAMING_SNAKE_CASE)
    /// * `file_path` - Path to the file being refactored
    pub fn new(
        analysis: ExtractConstantAnalysis,
        name: String,
        file_path: String,
    ) -> Self {
        Self {
            analysis,
            name,
            file_path,
        }
    }

    /// Build the EditPlan with a language-specific declaration formatter.
    ///
    /// This method performs all the necessary steps to create a complete EditPlan:
    /// 1. Validates the constant name is SCREAMING_SNAKE_CASE
    /// 2. Generates the constant declaration using the provided formatter
    /// 3. Creates an insertion edit at the specified insertion point (priority 100)
    /// 4. Creates replacement edits for each occurrence (priority 90, 89, 88, ...)
    /// 5. Builds the complete EditPlan with metadata
    ///
    /// # Arguments
    /// * `format_fn` - Function that takes (name, value) and returns the declaration string
    ///
    /// # Returns
    /// * `Ok(EditPlan)` - The complete edit plan ready for execution
    /// * `Err(String)` - If the constant name is not valid SCREAMING_SNAKE_CASE
    ///
    /// # Example Declaration Formatters
    ///
    /// ## TypeScript/JavaScript
    /// ```rust,ignore
    /// .with_declaration_format(|name, value| format!("const {} = {};\n", name, value))
    /// ```
    ///
    /// ## Python
    /// ```rust,ignore
    /// .with_declaration_format(|name, value| format!("{} = {}\n", name, value))
    /// ```
    ///
    /// ## Java
    /// ```rust,ignore
    /// .with_declaration_format(|name, value| {
    ///     let indent = get_indentation(source, insertion_point.start_line as usize);
    ///     let java_type = infer_java_type(value);
    ///     format!("{}private static final {} {} = {};\n", indent, java_type, name, value)
    /// })
    /// ```
    ///
    /// ## Rust
    /// ```rust,ignore
    /// .with_declaration_format(|name, value| {
    ///     format!("const {}: &str = {};\n", name, value)
    /// })
    /// ```
    ///
    /// ## C/C++
    /// ```rust,ignore
    /// .with_declaration_format(|name, value| {
    ///     format!("const int {} = {};\n", name, value)
    /// })
    /// ```
    pub fn with_declaration_format<F>(self, format_fn: F) -> Result<EditPlan, String>
    where
        F: FnOnce(&str, &str) -> String,
    {
        // 1. Validate name is SCREAMING_SNAKE_CASE
        use crate::validation::is_screaming_snake_case;
        if !is_screaming_snake_case(&self.name) {
            return Err(format!(
                "Constant name '{}' must be in SCREAMING_SNAKE_CASE format. Valid examples: TAX_RATE, MAX_VALUE, API_KEY, DB_TIMEOUT_MS. Requirements: only uppercase letters (A-Z), digits (0-9), and underscores; must contain at least one uppercase letter; cannot start or end with underscore.",
                self.name
            ));
        }

        // 2. Build declaration using language-specific formatter
        let declaration = format_fn(&self.name, &self.analysis.literal_value);

        let mut edits = Vec::new();

        // 3. Create insertion edit (priority 100)
        edits.push(TextEdit {
            file_path: None, // Uses source_file from EditPlan
            edit_type: EditType::Insert,
            location: EditLocation::from(self.analysis.insertion_point),
            original_text: String::new(),
            new_text: declaration.clone(),
            priority: 100,
            description: format!(
                "Extract '{}' into constant '{}'",
                self.analysis.literal_value, self.name
            ),
        });

        // 4. Create replacement edits for each occurrence
        // Priority descends: 90, 89, 88, ... to ensure deterministic ordering
        for (idx, occurrence_range) in self.analysis.occurrence_ranges.iter().enumerate() {
            let priority = 90_u32.saturating_sub(idx as u32);
            edits.push(TextEdit {
                file_path: None,
                edit_type: EditType::Replace,
                location: EditLocation::from(*occurrence_range),
                original_text: self.analysis.literal_value.clone(),
                new_text: self.name.clone(),
                priority,
                description: format!(
                    "Replace occurrence {} of literal with constant '{}'",
                    idx + 1,
                    self.name
                ),
            });
        }

        // 5. Build EditPlan with metadata
        Ok(EditPlan {
            source_file: self.file_path.clone(),
            edits,
            dependency_updates: Vec::new(),
            validations: vec![ValidationRule {
                rule_type: ValidationType::SyntaxCheck,
                description: "Verify syntax is valid after constant extraction".to_string(),
                parameters: HashMap::new(),
            }],
            metadata: EditPlanMetadata {
                intent_name: "extract_constant".to_string(),
                intent_arguments: serde_json::json!({
                    "literal": self.analysis.literal_value,
                    "constantName": self.name,
                    "occurrences": self.analysis.occurrence_ranges.len(),
                }),
                created_at: chrono::Utc::now(),
                complexity: (self.analysis.occurrence_ranges.len().min(10)) as u8,
                impact_areas: vec!["constant_extraction".to_string()],
                consolidation: None,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_basic() {
        let analysis = ExtractConstantAnalysis {
            literal_value: "42".to_string(),
            occurrence_ranges: vec![
                CodeRange::new(5, 10, 5, 12),
                CodeRange::new(7, 15, 7, 17),
            ],
            is_valid_literal: true,
            blocking_reasons: vec![],
            insertion_point: CodeRange::new(0, 0, 0, 0),
        };

        let builder = ExtractConstantEditPlanBuilder::new(
            analysis,
            "MAX_COUNT".to_string(),
            "test.ts".to_string(),
        );

        let plan = builder
            .with_declaration_format(|name, value| format!("const {} = {};\n", name, value))
            .expect("Should succeed");

        // Should have 1 insertion + 2 replacements = 3 edits
        assert_eq!(plan.edits.len(), 3);

        // Check insertion edit
        assert_eq!(plan.edits[0].priority, 100);
        assert_eq!(plan.edits[0].edit_type, EditType::Insert);
        assert_eq!(plan.edits[0].new_text, "const MAX_COUNT = 42;\n");

        // Check replacement edits
        assert_eq!(plan.edits[1].priority, 90);
        assert_eq!(plan.edits[1].edit_type, EditType::Replace);
        assert_eq!(plan.edits[1].new_text, "MAX_COUNT");

        assert_eq!(plan.edits[2].priority, 89);
        assert_eq!(plan.edits[2].edit_type, EditType::Replace);
        assert_eq!(plan.edits[2].new_text, "MAX_COUNT");

        // Check metadata
        assert_eq!(plan.metadata.intent_name, "extract_constant");
        assert_eq!(plan.metadata.complexity, 2);
        assert_eq!(plan.source_file, "test.ts");
    }

    #[test]
    fn test_builder_rejects_invalid_name() {
        let analysis = ExtractConstantAnalysis {
            literal_value: "42".to_string(),
            occurrence_ranges: vec![CodeRange::new(5, 10, 5, 12)],
            is_valid_literal: true,
            blocking_reasons: vec![],
            insertion_point: CodeRange::new(0, 0, 0, 0),
        };

        let builder = ExtractConstantEditPlanBuilder::new(
            analysis,
            "invalidName".to_string(), // Not SCREAMING_SNAKE_CASE
            "test.ts".to_string(),
        );

        let result = builder.with_declaration_format(|name, value| {
            format!("const {} = {};\n", name, value)
        });

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("must be in SCREAMING_SNAKE_CASE format"));
    }

    #[test]
    fn test_builder_priority_saturation() {
        // Test with many occurrences to verify saturating_sub behavior
        let mut occurrence_ranges = Vec::new();
        for i in 0..100 {
            occurrence_ranges.push(CodeRange::new(i, 0, i, 2));
        }

        let analysis = ExtractConstantAnalysis {
            literal_value: "x".to_string(),
            occurrence_ranges,
            is_valid_literal: true,
            blocking_reasons: vec![],
            insertion_point: CodeRange::new(0, 0, 0, 0),
        };

        let builder = ExtractConstantEditPlanBuilder::new(
            analysis,
            "X_CONST".to_string(),
            "test.txt".to_string(),
        );

        let plan = builder
            .with_declaration_format(|name, value| format!("{} = {}\n", name, value))
            .expect("Should succeed");

        // Check that priorities saturate at 0 instead of wrapping
        assert_eq!(plan.edits[1].priority, 90); // First replacement
        assert_eq!(plan.edits[91].priority, 0); // 90 - 90 = 0
        assert_eq!(plan.edits[100].priority, 0); // Still 0 (saturated)
    }

    #[test]
    fn test_builder_language_specific_formats() {
        let analysis = ExtractConstantAnalysis {
            literal_value: "\"hello\"".to_string(),
            occurrence_ranges: vec![CodeRange::new(1, 5, 1, 12)],
            is_valid_literal: true,
            blocking_reasons: vec![],
            insertion_point: CodeRange::new(0, 0, 0, 0),
        };

        // Test TypeScript format
        let builder_ts = ExtractConstantEditPlanBuilder::new(
            analysis.clone(),
            "GREETING".to_string(),
            "test.ts".to_string(),
        );
        let plan_ts = builder_ts
            .with_declaration_format(|name, value| format!("const {} = {};\n", name, value))
            .unwrap();
        assert_eq!(plan_ts.edits[0].new_text, "const GREETING = \"hello\";\n");

        // Test Python format
        let builder_py = ExtractConstantEditPlanBuilder::new(
            analysis.clone(),
            "GREETING".to_string(),
            "test.py".to_string(),
        );
        let plan_py = builder_py
            .with_declaration_format(|name, value| format!("{} = {}\n", name, value))
            .unwrap();
        assert_eq!(plan_py.edits[0].new_text, "GREETING = \"hello\"\n");

        // Test Rust format
        let builder_rs = ExtractConstantEditPlanBuilder::new(
            analysis.clone(),
            "GREETING".to_string(),
            "test.rs".to_string(),
        );
        let plan_rs = builder_rs
            .with_declaration_format(|name, value| {
                format!("const {}: &str = {};\n", name, value)
            })
            .unwrap();
        assert_eq!(
            plan_rs.edits[0].new_text,
            "const GREETING: &str = \"hello\";\n"
        );
    }
}
