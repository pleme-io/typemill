//! TypeScript/JavaScript refactoring operations using SWC AST
//!
//! This module provides AST-based refactoring capabilities for TypeScript/JavaScript code.

use cb_protocol::{EditPlan, EditPlanMetadata, EditLocation, EditType, TextEdit, ValidationRule, ValidationType};
use std::error::Error;

/// Code range for refactoring operations
#[derive(Debug, Clone)]
pub struct CodeRange {
    pub start_line: u32,
    pub start_col: u32,
    pub end_line: u32,
    pub end_col: u32,
}

impl From<CodeRange> for EditLocation {
    fn from(range: CodeRange) -> Self {
        EditLocation {
            start_line: range.start_line,
            start_column: range.start_col,
            end_line: range.end_line,
            end_column: range.end_col,
        }
    }
}

/// Plan inline variable refactoring for TypeScript/JavaScript
pub fn plan_inline_variable(
    source: &str,
    variable_line: u32,
    variable_col: u32,
    file_path: &str,
) -> Result<EditPlan, Box<dyn Error>> {
    // Find the variable declaration at the specified position
    let lines: Vec<&str> = source.lines().collect();

    if variable_line as usize >= lines.len() {
        return Err("Line number out of bounds".into());
    }

    let line_text = lines[variable_line as usize];

    // Simple pattern matching for variable declarations
    // Supports: const x = ..., let x = ..., var x = ...
    let var_pattern = regex::Regex::new(r"(const|let|var)\s+(\w+)\s*=\s*(.+?)(?:;|$)")?;

    if let Some(captures) = var_pattern.captures(line_text) {
        let var_name = captures.get(2).unwrap().as_str();
        let initializer = captures.get(3).unwrap().as_str().trim();

        // Find all usages of this variable in the rest of the source
        let mut edits = Vec::new();
        let var_regex = regex::Regex::new(&format!(r"\b{}\b", regex::escape(var_name)))?;

        // Replace all usages (except the declaration itself)
        for (idx, line) in lines.iter().enumerate() {
            let line_num = idx as u32;

            // Skip the declaration line
            if line_num == variable_line {
                continue;
            }

            for mat in var_regex.find_iter(line) {
                edits.push(TextEdit {
                    file_path: None,
                    edit_type: EditType::Replace,
                    location: EditLocation {
                        start_line: line_num,
                        start_column: mat.start() as u32,
                        end_line: line_num,
                        end_column: mat.end() as u32,
                    },
                    original_text: var_name.to_string(),
                    new_text: initializer.to_string(),
                    priority: 100,
                    description: format!("Inline variable '{}'", var_name),
                });
            }
        }

        // Delete the variable declaration
        edits.push(TextEdit {
            file_path: None,
            edit_type: EditType::Delete,
            location: EditLocation {
                start_line: variable_line,
                start_column: 0,
                end_line: variable_line,
                end_column: line_text.len() as u32,
            },
            original_text: line_text.to_string(),
            new_text: String::new(),
            priority: 50,
            description: format!("Remove variable declaration for '{}'", var_name),
        });

        Ok(EditPlan {
            edits,
            metadata: EditPlanMetadata {
                operation: "inline_variable".to_string(),
                description: format!("Inline variable '{}' with value '{}'", var_name, initializer),
                language: "typescript".to_string(),
                file_path: file_path.to_string(),
                affected_symbols: vec![var_name.to_string()],
                validation_rules: vec![
                    ValidationRule {
                        rule_type: ValidationType::SyntaxCheck,
                        description: "Verify syntax is valid after inlining".to_string(),
                    },
                ],
                reversible: true,
                estimated_impact: "low".to_string(),
                safety_score: 85,
                requires_user_input: false,
                intent_classification: "refactoring".to_string(),
                intent_arguments: serde_json::json!({
                    "variable_name": var_name,
                    "value": initializer
                }),
                created_at: chrono::Utc::now(),
                complexity: 3,
                impact_areas: vec!["variable_inlining".to_string()],
            },
        })
    } else {
        Err(format!(
            "Could not find variable declaration at {}:{}",
            variable_line, variable_col
        ).into())
    }
}

/// Plan extract function refactoring for TypeScript/JavaScript
pub fn plan_extract_function(
    source: &str,
    start_line: u32,
    end_line: u32,
    function_name: &str,
    file_path: &str,
) -> Result<EditPlan, Box<dyn Error>> {
    let lines: Vec<&str> = source.lines().collect();

    if start_line as usize >= lines.len() || end_line as usize >= lines.len() {
        return Err("Line range out of bounds".into());
    }

    // Extract the selected lines
    let selected_lines: Vec<&str> = lines[start_line as usize..=end_line as usize].to_vec();
    let selected_code = selected_lines.join("\n");

    // Get indentation of first line
    let first_line = lines[start_line as usize];
    let indent_count = first_line.len() - first_line.trim_start().len();
    let indent = " ".repeat(indent_count);

    // Generate new function
    let new_function = format!(
        "\n{}function {}() {{\n{}\n{}}}\n",
        indent,
        function_name,
        selected_code,
        indent
    );

    // Generate function call
    let function_call = format!("{}{}();", indent, function_name);

    let mut edits = Vec::new();

    // Insert new function above the selected code
    edits.push(TextEdit {
        file_path: None,
        edit_type: EditType::Insert,
        location: EditLocation {
            start_line: start_line,
            start_column: 0,
            end_line: start_line,
            end_column: 0,
        },
        original_text: String::new(),
        new_text: new_function,
        priority: 100,
        description: format!("Create extracted function '{}'", function_name),
    });

    // Replace selected code with function call
    edits.push(TextEdit {
        file_path: None,
        edit_type: EditType::Replace,
        location: EditLocation {
            start_line,
            start_column: 0,
            end_line,
            end_column: lines[end_line as usize].len() as u32,
        },
        original_text: selected_code.clone(),
        new_text: function_call,
        priority: 90,
        description: format!("Replace code with call to '{}'", function_name),
    });

    Ok(EditPlan {
        edits,
        metadata: EditPlanMetadata {
            operation: "extract_function".to_string(),
            description: format!("Extract {} lines into function '{}'", end_line - start_line + 1, function_name),
            language: "typescript".to_string(),
            file_path: file_path.to_string(),
            affected_symbols: vec![function_name.to_string()],
            validation_rules: vec![
                ValidationRule {
                    rule_type: ValidationType::SyntaxCheck,
                    description: "Verify syntax is valid after extraction".to_string(),
                },
            ],
            reversible: true,
            estimated_impact: "medium".to_string(),
            safety_score: 75,
            requires_user_input: false,
            intent_classification: "refactoring".to_string(),
            intent_arguments: serde_json::json!({
                "function_name": function_name,
                "line_count": end_line - start_line + 1
            }),
            created_at: chrono::Utc::now(),
            complexity: 5,
            impact_areas: vec!["function_extraction".to_string()],
        },
    })
}

/// Plan extract variable refactoring for TypeScript/JavaScript
pub fn plan_extract_variable(
    source: &str,
    start_line: u32,
    start_col: u32,
    end_line: u32,
    end_col: u32,
    variable_name: Option<String>,
    file_path: &str,
) -> Result<EditPlan, Box<dyn Error>> {
    let lines: Vec<&str> = source.lines().collect();

    if start_line as usize >= lines.len() || end_line as usize >= lines.len() {
        return Err("Line range out of bounds".into());
    }

    // Extract the expression
    let expression = if start_line == end_line {
        let line = lines[start_line as usize];
        line[start_col as usize..end_col as usize].to_string()
    } else {
        // Multi-line expression
        let mut expr_lines = Vec::new();
        for (idx, line) in lines.iter().enumerate() {
            let line_num = idx as u32;
            if line_num == start_line {
                expr_lines.push(&line[start_col as usize..]);
            } else if line_num == end_line {
                expr_lines.push(&line[..end_col as usize]);
            } else if line_num > start_line && line_num < end_line {
                expr_lines.push(*line);
            }
        }
        expr_lines.join("\n")
    };

    let var_name = variable_name.unwrap_or_else(|| "extracted".to_string());

    // Get indentation
    let line = lines[start_line as usize];
    let indent_count = line.len() - line.trim_start().len();
    let indent = " ".repeat(indent_count);

    // Generate variable declaration
    let declaration = format!("{}const {} = {};\n", indent, var_name, expression.trim());

    let mut edits = Vec::new();

    // Insert variable declaration above
    edits.push(TextEdit {
        file_path: None,
        edit_type: EditType::Insert,
        location: EditLocation {
            start_line,
            start_column: 0,
            end_line: start_line,
            end_column: 0,
        },
        original_text: String::new(),
        new_text: declaration,
        priority: 100,
        description: format!("Declare variable '{}'", var_name),
    });

    // Replace expression with variable name
    edits.push(TextEdit {
        file_path: None,
        edit_type: EditType::Replace,
        location: EditLocation {
            start_line,
            start_column: start_col,
            end_line,
            end_column: end_col,
        },
        original_text: expression.clone(),
        new_text: var_name.clone(),
        priority: 90,
        description: format!("Replace expression with '{}'", var_name),
    });

    Ok(EditPlan {
        edits,
        metadata: EditPlanMetadata {
            operation: "extract_variable".to_string(),
            description: format!("Extract expression into variable '{}'", var_name),
            language: "typescript".to_string(),
            file_path: file_path.to_string(),
            affected_symbols: vec![var_name.clone()],
            validation_rules: vec![
                ValidationRule {
                    rule_type: ValidationType::SyntaxCheck,
                    description: "Verify syntax is valid after extraction".to_string(),
                },
            ],
            reversible: true,
            estimated_impact: "low".to_string(),
            safety_score: 85,
            requires_user_input: false,
            intent_classification: "refactoring".to_string(),
            intent_arguments: serde_json::json!({
                "variable_name": var_name,
                "expression": expression
            }),
            created_at: chrono::Utc::now(),
            complexity: 3,
            impact_areas: vec!["variable_extraction".to_string()],
        },
    })
}
