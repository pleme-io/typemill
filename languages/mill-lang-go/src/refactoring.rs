//! Go refactoring operations using tree-sitter-go AST
//!
//! This module provides AST-based refactoring capabilities for Go code.

use mill_lang_common::LineExtractor;
use mill_foundation::protocol::{
    EditLocation, EditPlan, EditPlanMetadata, EditType, TextEdit, ValidationRule, ValidationType,
};
use lazy_static::lazy_static;
use mill_plugin_api::{PluginError, PluginResult};
use std::collections::HashMap;

/// Plan extract function refactoring for Go
pub fn plan_extract_function(
    source: &str,
    start_line: u32,
    end_line: u32,
    function_name: &str,
    file_path: &str,
) -> PluginResult<EditPlan> {
    let lines: Vec<&str> = source.lines().collect();

    if start_line as usize >= lines.len() || end_line as usize >= lines.len() {
        return Err(PluginError::invalid_input("Line range out of bounds"));
    }

    // Extract the selected lines
    let selected_lines: Vec<&str> = lines[start_line as usize..=end_line as usize].to_vec();
    let selected_code = selected_lines.join("\n");

    // Get indentation of first line
    let indent = LineExtractor::get_indentation_str(source, start_line);

    // Generate new function
    let new_function = format!(
        "\n{}func {}() {{\n{}\n{}}}\n",
        indent, function_name, selected_code, indent
    );

    // Generate function call
    let function_call = format!("{}{}()", indent, function_name);

    let mut edits = Vec::new();

    // Insert new function above the selected code
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
        source_file: file_path.to_string(),
        edits,
        dependency_updates: Vec::new(),
        validations: vec![ValidationRule {
            rule_type: ValidationType::SyntaxCheck,
            description: "Verify Go syntax is valid after extraction".to_string(),
            parameters: HashMap::new(),
        }],
        metadata: EditPlanMetadata {
            intent_name: "extract_function".to_string(),
            intent_arguments: serde_json::json!({
                "function_name": function_name,
                "line_count": end_line - start_line + 1
            }),
            created_at: chrono::Utc::now(),
            complexity: 5,
            impact_areas: vec!["function_extraction".to_string()],
            consolidation: None,
        },
    })
}

/// Plan extract variable refactoring for Go
pub fn plan_extract_variable(
    source: &str,
    start_line: u32,
    start_col: u32,
    end_line: u32,
    end_col: u32,
    variable_name: Option<String>,
    file_path: &str,
) -> PluginResult<EditPlan> {
    let lines: Vec<&str> = source.lines().collect();

    if start_line as usize >= lines.len() || end_line as usize >= lines.len() {
        return Err(PluginError::invalid_input("Line range out of bounds"));
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

    let var_name = variable_name.unwrap_or_default();

    // Get indentation
    let indent = LineExtractor::get_indentation_str(source, start_line);

    // Generate variable declaration (Go short declaration)
    let declaration = format!("{}{} := {};\n", indent, var_name, expression.trim());

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
        source_file: file_path.to_string(),
        edits,
        dependency_updates: Vec::new(),
        validations: vec![ValidationRule {
            rule_type: ValidationType::SyntaxCheck,
            description: "Verify Go syntax is valid after extraction".to_string(),
            parameters: HashMap::new(),
        }],
        metadata: EditPlanMetadata {
            intent_name: "extract_variable".to_string(),
            intent_arguments: serde_json::json!({
                "variable_name": var_name,
                "expression": expression
            }),
            created_at: chrono::Utc::now(),
            complexity: 3,
            impact_areas: vec!["variable_extraction".to_string()],
            consolidation: None,
        },
    })
}

lazy_static! {
    static ref VAR_PATTERN: regex::Regex =
        regex::Regex::new(r"(?:var\s+)?(\w+)\s*:?=\s*(.+?)(?:$)")
            .expect("Invalid regex for Go variable parsing");
}

/// Plan inline variable refactoring for Go
pub fn plan_inline_variable(
    source: &str,
    variable_line: u32,
    _variable_col: u32,
    file_path: &str,
) -> PluginResult<EditPlan> {
    let lines: Vec<&str> = source.lines().collect();

    if variable_line as usize >= lines.len() {
        return Err(PluginError::invalid_input("Line number out of bounds"));
    }

    let line_text = lines[variable_line as usize];

    if let Some(captures) = VAR_PATTERN.captures(line_text) {
        let var_name = captures.get(1).map_or("", |m| m.as_str());
        let initializer = captures.get(2).map_or("", |m| m.as_str()).trim();

        if var_name.is_empty() {
            return Err(PluginError::internal(
                "Could not extract variable name".to_string(),
            ));
        }

        // Find all usages of this variable in the rest of the source
        let mut edits = Vec::new();
        let var_regex = regex::Regex::new(&format!(r"\b{}\b", regex::escape(var_name)))
            .map_err(|e| PluginError::internal(e.to_string()))?;

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
            source_file: file_path.to_string(),
            edits,
            dependency_updates: Vec::new(),
            validations: vec![ValidationRule {
                rule_type: ValidationType::SyntaxCheck,
                description: "Verify Go syntax is valid after inlining".to_string(),
                parameters: HashMap::new(),
            }],
            metadata: EditPlanMetadata {
                intent_name: "inline_variable".to_string(),
                intent_arguments: serde_json::json!({
                    "variable_name": var_name,
                    "value": initializer
                }),
                created_at: chrono::Utc::now(),
                complexity: 3,
                impact_areas: vec!["variable_inlining".to_string()],
                consolidation: None,
            },
        })
    } else {
        Err(PluginError::internal(format!(
            "Could not find variable declaration at line {}",
            variable_line
        )))
    }
}