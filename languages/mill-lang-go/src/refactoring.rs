//! Go refactoring operations using tree-sitter-go AST
//!
//! This module provides AST-based refactoring capabilities for Go code.

use lazy_static::lazy_static;
use mill_foundation::protocol::{
    EditLocation, EditPlan, EditPlanMetadata, EditType, TextEdit, ValidationRule, ValidationType,
};
use mill_lang_common::{CodeRange, LineExtractor};
use mill_plugin_api::{PluginApiError, PluginResult};
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
        return Err(PluginApiError::invalid_input("Line range out of bounds"));
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
        return Err(PluginApiError::invalid_input("Line range out of bounds"));
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

/// Analysis result for extract constant refactoring
#[derive(Debug, Clone)]
pub struct ExtractConstantAnalysis {
    /// The literal value to extract
    pub literal_value: String,
    /// All locations where this same literal value appears
    pub occurrence_ranges: Vec<CodeRange>,
    /// Whether this is a valid literal to extract
    pub is_valid_literal: bool,
    /// Blocking reasons if extraction is not valid
    pub blocking_reasons: Vec<String>,
    /// Where to insert the constant declaration
    pub insertion_point: CodeRange,
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
        return Err(PluginApiError::invalid_input("Line number out of bounds"));
    }

    let line_text = lines[variable_line as usize];

    if let Some(captures) = VAR_PATTERN.captures(line_text) {
        let var_name = captures.get(1).map_or("", |m| m.as_str());
        let initializer = captures.get(2).map_or("", |m| m.as_str()).trim();

        if var_name.is_empty() {
            return Err(PluginApiError::internal(
                "Could not extract variable name".to_string(),
            ));
        }

        // Find all usages of this variable in the rest of the source
        let mut edits = Vec::new();
        let var_regex = regex::Regex::new(&format!(r"\b{}\b", regex::escape(var_name)))
            .map_err(|e| PluginApiError::internal(e.to_string()))?;

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
        Err(PluginApiError::internal(format!(
            "Could not find variable declaration at line {}",
            variable_line
        )))
    }
}

/// Plan extract constant refactoring for Go
///
/// Extracts a literal value (number, string, boolean) to a named constant at the package level.
/// Follows Go naming conventions: SCREAMING_SNAKE_CASE or PascalCase.
///
/// # Arguments
/// * `source` - The Go source code
/// * `line` - Zero-based line number where the cursor is positioned
/// * `character` - Zero-based character offset within the line
/// * `name` - The constant name (SCREAMING_SNAKE_CASE or PascalCase)
/// * `file_path` - Path to the file being refactored
///
/// # Returns
/// * `Ok(EditPlan)` - The edit plan with constant declaration and replacements
/// * `Err(PluginError)` - If the cursor is not on a literal or the name is invalid
pub fn plan_extract_constant(
    source: &str,
    line: u32,
    character: u32,
    name: &str,
    file_path: &str,
) -> PluginResult<EditPlan> {
    let analysis = analyze_extract_constant(source, line, character)?;

    if !analysis.is_valid_literal {
        return Err(PluginApiError::internal(format!(
            "Cannot extract constant: {}",
            analysis.blocking_reasons.join(", ")
        )));
    }

    // Validate that the name is in SCREAMING_SNAKE_CASE or PascalCase format
    if !is_valid_go_constant_name(name) {
        return Err(PluginApiError::invalid_input(format!(
            "Constant name '{}' must be in SCREAMING_SNAKE_CASE or PascalCase format. Valid examples: TAX_RATE, MaxValue, API_KEY, DbTimeoutMs. Requirements: SCREAMING_SNAKE_CASE (only uppercase letters, digits, underscores; must contain at least one uppercase letter; cannot start or end with underscore) OR PascalCase (starts with uppercase letter, camelCase thereafter).",
            name
        )));
    }

    let mut edits = Vec::new();

    // Generate the constant declaration and insert it at the top of the file
    let declaration = format!("const {} = {}\n", name, analysis.literal_value);
    edits.push(TextEdit {
        file_path: None,
        edit_type: EditType::Insert,
        location: analysis.insertion_point.into(),
        original_text: String::new(),
        new_text: declaration,
        priority: 100,
        description: format!(
            "Extract '{}' into constant '{}'",
            analysis.literal_value, name
        ),
    });

    // Replace all occurrences of the literal with the constant name
    for (idx, occurrence_range) in analysis.occurrence_ranges.iter().enumerate() {
        let priority = 90_u32.saturating_sub(idx as u32);
        edits.push(TextEdit {
            file_path: None,
            edit_type: EditType::Replace,
            location: (*occurrence_range).into(),
            original_text: analysis.literal_value.clone(),
            new_text: name.to_string(),
            priority,
            description: format!(
                "Replace occurrence {} of literal with constant '{}'",
                idx + 1,
                name
            ),
        });
    }

    Ok(EditPlan {
        source_file: file_path.to_string(),
        edits,
        dependency_updates: Vec::new(),
        validations: vec![ValidationRule {
            rule_type: ValidationType::SyntaxCheck,
            description: "Verify Go syntax is valid after constant extraction".to_string(),
            parameters: HashMap::new(),
        }],
        metadata: EditPlanMetadata {
            intent_name: "extract_constant".to_string(),
            intent_arguments: serde_json::json!({
                "literal": analysis.literal_value,
                "constantName": name,
                "occurrences": analysis.occurrence_ranges.len(),
            }),
            created_at: chrono::Utc::now(),
            complexity: (analysis.occurrence_ranges.len().min(10)) as u8,
            impact_areas: vec!["constant_extraction".to_string()],
            consolidation: None,
        },
    })
}

/// Analyzes source code to extract information about a literal value at a cursor position
fn analyze_extract_constant(
    source: &str,
    line: u32,
    character: u32,
) -> PluginResult<ExtractConstantAnalysis> {
    let lines: Vec<&str> = source.lines().collect();

    if let Some(line_text) = lines.get(line as usize) {
        // Try to find different kinds of literals at the cursor position

        // Check for numeric literal
        if let Some((literal_value, _range)) = find_numeric_literal(line_text, line, character) {
            let occurrence_ranges = find_literal_occurrences(source, &literal_value);
            return Ok(ExtractConstantAnalysis {
                literal_value,
                occurrence_ranges,
                is_valid_literal: true,
                blocking_reasons: vec![],
                insertion_point: find_insertion_point(source),
            });
        }

        // Check for string literal (quoted)
        if let Some((literal_value, _range)) = find_string_literal(line_text, line, character) {
            let occurrence_ranges = find_literal_occurrences(source, &literal_value);
            return Ok(ExtractConstantAnalysis {
                literal_value,
                occurrence_ranges,
                is_valid_literal: true,
                blocking_reasons: vec![],
                insertion_point: find_insertion_point(source),
            });
        }

        // Check for boolean literal
        if let Some((literal_value, _range)) = find_keyword_literal(line_text, line, character) {
            let occurrence_ranges = find_literal_occurrences(source, &literal_value);
            return Ok(ExtractConstantAnalysis {
                literal_value,
                occurrence_ranges,
                is_valid_literal: true,
                blocking_reasons: vec![],
                insertion_point: find_insertion_point(source),
            });
        }
    }

    Err(PluginApiError::internal(
        "Cursor is not positioned on a literal value. Extract constant only works on numbers, strings, and booleans.".to_string(),
    ))
}

/// Find numeric literal at cursor position
fn find_numeric_literal(line_text: &str, line: u32, character: u32) -> Option<(String, CodeRange)> {
    let col = character as usize;

    if col >= line_text.len() {
        return None;
    }

    // Find the start of the number
    let start = line_text[..col]
        .rfind(|c: char| !c.is_ascii_digit() && c != '.' && c != '-')
        .map(|p| p + 1)
        .unwrap_or(0);

    // Find the end of the number
    let end = col + line_text[col..]
        .find(|c: char| !c.is_ascii_digit() && c != '.')
        .unwrap_or(line_text.len() - col);

    if start < end && end <= line_text.len() {
        let text = &line_text[start..end];
        if text.chars().any(|c| c.is_ascii_digit()) {
            return Some((
                text.to_string(),
                CodeRange {
                    start_line: line,
                    start_col: start as u32,
                    end_line: line,
                    end_col: end as u32,
                },
            ));
        }
    }
    None
}

/// Find string literal at cursor position
fn find_string_literal(line_text: &str, line: u32, character: u32) -> Option<(String, CodeRange)> {
    let col = character as usize;

    if col >= line_text.len() {
        return None;
    }

    // Look for opening quote before cursor
    for (i, ch) in line_text[..=col].char_indices().rev() {
        if ch == '"' || ch == '\'' || ch == '`' {
            // Find closing quote after cursor
            let quote = ch;
            for (j, ch2) in line_text[col..].char_indices() {
                if ch2 == quote && col + j > i {
                    let literal = &line_text[i..=col + j];
                    return Some((
                        literal.to_string(),
                        CodeRange {
                            start_line: line,
                            start_col: i as u32,
                            end_line: line,
                            end_col: (col + j + 1) as u32,
                        },
                    ));
                }
            }
            break;
        }
    }
    None
}

/// Find keyword literal (true, false) at cursor position
fn find_keyword_literal(line_text: &str, line: u32, character: u32) -> Option<(String, CodeRange)> {
    let col = character as usize;
    let keywords = ["true", "false"];

    for keyword in &keywords {
        // Try to match keyword at or near cursor
        for start in col.saturating_sub(keyword.len())..=col.min(line_text.len().saturating_sub(1)) {
            if start + keyword.len() <= line_text.len() {
                if &line_text[start..start + keyword.len()] == *keyword {
                    // Check word boundaries
                    let before_ok = start == 0 || !line_text[..start].ends_with(|c: char| c.is_alphanumeric());
                    let after_ok = start + keyword.len() == line_text.len()
                        || !line_text[start + keyword.len()..].starts_with(|c: char| c.is_alphanumeric());

                    if before_ok && after_ok {
                        return Some((
                            keyword.to_string(),
                            CodeRange {
                                start_line: line,
                                start_col: start as u32,
                                end_line: line,
                                end_col: (start + keyword.len()) as u32,
                            },
                        ));
                    }
                }
            }
        }
    }
    None
}

/// Find insertion point for constant declaration (after package declaration)
fn find_insertion_point(source: &str) -> CodeRange {
    let lines: Vec<&str> = source.lines().collect();

    // Find the first non-package, non-import, non-comment line
    for (idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if !trimmed.starts_with("package ")
            && !trimmed.starts_with("import ")
            && !trimmed.starts_with("//")
            && !trimmed.starts_with("/*")
            && !trimmed.is_empty()
        {
            // Insert before this line
            return CodeRange {
                start_line: idx as u32,
                start_col: 0,
                end_line: idx as u32,
                end_col: 0,
            };
        }
    }

    // If no suitable location found, insert at the beginning
    CodeRange {
        start_line: 0,
        start_col: 0,
        end_line: 0,
        end_col: 0,
    }
}

/// Find all occurrences of a literal value in source code
fn find_literal_occurrences(source: &str, literal_value: &str) -> Vec<CodeRange> {
    let mut occurrences = Vec::new();
    let lines: Vec<&str> = source.lines().collect();

    for (line_idx, line_text) in lines.iter().enumerate() {
        let mut start_pos = 0;
        while let Some(pos) = line_text[start_pos..].find(literal_value) {
            let col = start_pos + pos;

            // Validate that this match is not inside a string literal or comment
            if is_valid_literal_location(line_text, col, literal_value.len()) {
                occurrences.push(CodeRange {
                    start_line: line_idx as u32,
                    start_col: col as u32,
                    end_line: line_idx as u32,
                    end_col: (col + literal_value.len()) as u32,
                });
            }

            start_pos = col + 1;
        }
    }

    occurrences
}

/// Validates whether a position in source code is a valid location for a literal
fn is_valid_literal_location(line: &str, pos: usize, _len: usize) -> bool {
    let before = &line[..pos];
    let single_quotes = before.matches('\'').count();
    let double_quotes = before.matches('"').count();
    let backticks = before.matches('`').count();

    // If an odd number of quotes appear before the position, we're inside a string
    if single_quotes % 2 == 1 || double_quotes % 2 == 1 || backticks % 2 == 1 {
        return false;
    }

    // Check for single-line comment marker
    if let Some(comment_pos) = line.find("//") {
        if pos > comment_pos {
            return false;
        }
    }

    true
}

/// Validates that a constant name follows Go naming conventions
///
/// Go constants can be either:
/// 1. SCREAMING_SNAKE_CASE: TAX_RATE, MAX_VALUE, API_KEY
/// 2. PascalCase: TaxRate, MaxValue, ApiKey
fn is_valid_go_constant_name(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }

    // Check for SCREAMING_SNAKE_CASE
    if is_screaming_snake_case(name) {
        return true;
    }

    // Check for PascalCase
    if is_pascal_case(name) {
        return true;
    }

    false
}

/// Check if name is in SCREAMING_SNAKE_CASE
fn is_screaming_snake_case(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }

    // Must not start or end with underscore
    if name.starts_with('_') || name.ends_with('_') {
        return false;
    }

    // Check each character - only uppercase, digits, and underscores allowed
    for ch in name.chars() {
        match ch {
            'A'..='Z' | '0'..='9' | '_' => continue,
            _ => return false,
        }
    }

    // Must have at least one uppercase letter
    name.chars().any(|c| c.is_ascii_uppercase())
}

/// Check if name is in PascalCase
fn is_pascal_case(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }

    let mut chars = name.chars();

    // Must start with uppercase letter
    if let Some(first) = chars.next() {
        if !first.is_ascii_uppercase() {
            return false;
        }
    } else {
        return false;
    }

    // Rest can be alphanumeric
    for ch in chars {
        if !ch.is_alphanumeric() {
            return false;
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plan_extract_constant_valid_number() {
        let source = r#"package main

func main() {
    x := 42
    y := 42
}
"#;
        let result = plan_extract_constant(source, 3, 9, "ANSWER", "test.go");
        assert!(result.is_ok(), "Should extract numeric literal successfully");

        let plan = result.unwrap();
        assert_eq!(plan.edits.len(), 3); // 1 declaration + 2 replacements

        // Check declaration
        assert_eq!(plan.edits[0].edit_type, EditType::Insert);
        assert!(plan.edits[0].new_text.contains("const ANSWER = 42"));

        // Check replacements
        assert_eq!(plan.edits[1].edit_type, EditType::Replace);
        assert_eq!(plan.edits[1].new_text, "ANSWER");
        assert_eq!(plan.edits[2].edit_type, EditType::Replace);
        assert_eq!(plan.edits[2].new_text, "ANSWER");
    }

    #[test]
    fn test_plan_extract_constant_string() {
        let source = r#"package main

func main() {
    msg := "hello"
    fmt.Println("hello")
}
"#;
        let result = plan_extract_constant(source, 3, 11, "GREETING", "test.go");
        assert!(result.is_ok(), "Should extract string literal successfully");

        let plan = result.unwrap();
        assert_eq!(plan.edits.len(), 3); // 1 declaration + 2 replacements

        // Check declaration
        assert!(plan.edits[0].new_text.contains("const GREETING = \"hello\""));
    }

    #[test]
    fn test_plan_extract_constant_boolean() {
        let source = r#"package main

func main() {
    enabled := true
    if true {
        fmt.Println("on")
    }
}
"#;
        let result = plan_extract_constant(source, 3, 15, "ENABLED", "test.go");
        assert!(result.is_ok(), "Should extract boolean literal successfully");

        let plan = result.unwrap();
        assert_eq!(plan.edits.len(), 3); // 1 declaration + 2 replacements

        // Check declaration
        assert!(plan.edits[0].new_text.contains("const ENABLED = true"));
    }

    #[test]
    fn test_plan_extract_constant_invalid_name() {
        let source = r#"package main

func main() {
    x := 42
}
"#;
        // Test lowercase name (invalid)
        let result = plan_extract_constant(source, 3, 9, "answer", "test.go");
        assert!(result.is_err(), "Should reject lowercase name");

        // Test name starting with underscore (invalid)
        let result = plan_extract_constant(source, 3, 9, "_ANSWER", "test.go");
        assert!(result.is_err(), "Should reject name starting with underscore");

        // Test name ending with underscore (invalid)
        let result = plan_extract_constant(source, 3, 9, "ANSWER_", "test.go");
        assert!(result.is_err(), "Should reject name ending with underscore");
    }

    #[test]
    fn test_plan_extract_constant_pascal_case() {
        let source = r#"package main

func main() {
    x := 3.14
}
"#;
        // PascalCase should be valid for Go constants
        let result = plan_extract_constant(source, 3, 9, "PiValue", "test.go");
        assert!(result.is_ok(), "Should accept PascalCase constant name");

        let plan = result.unwrap();
        assert!(plan.edits[0].new_text.contains("const PiValue = 3.14"));
    }

    #[test]
    fn test_plan_extract_constant_not_on_literal() {
        let source = r#"package main

func main() {
    x := 42
}
"#;
        // Cursor not on a literal (on 'x' instead)
        let result = plan_extract_constant(source, 3, 4, "ANSWER", "test.go");
        assert!(result.is_err(), "Should error when cursor is not on a literal");
    }

    #[test]
    fn test_is_screaming_snake_case() {
        assert!(is_screaming_snake_case("TAX_RATE"));
        assert!(is_screaming_snake_case("MAX_VALUE"));
        assert!(is_screaming_snake_case("A"));
        assert!(is_screaming_snake_case("PI"));
        assert!(is_screaming_snake_case("API_KEY_V2"));

        assert!(!is_screaming_snake_case(""));
        assert!(!is_screaming_snake_case("_TAX_RATE")); // starts with underscore
        assert!(!is_screaming_snake_case("TAX_RATE_")); // ends with underscore
        assert!(!is_screaming_snake_case("tax_rate")); // lowercase
        assert!(!is_screaming_snake_case("TaxRate")); // PascalCase
        assert!(!is_screaming_snake_case("tax-rate")); // kebab-case
    }

    #[test]
    fn test_is_pascal_case() {
        assert!(is_pascal_case("TaxRate"));
        assert!(is_pascal_case("MaxValue"));
        assert!(is_pascal_case("A"));
        assert!(is_pascal_case("ApiKeyV2"));

        assert!(!is_pascal_case(""));
        assert!(!is_pascal_case("taxRate")); // camelCase
        assert!(!is_pascal_case("TAX_RATE")); // SCREAMING_SNAKE_CASE
        assert!(!is_pascal_case("tax_rate")); // snake_case
        assert!(!is_pascal_case("Tax-Rate")); // has hyphen
    }

    #[test]
    fn test_find_literal_occurrences() {
        let source = r#"package main

func main() {
    x := 42
    y := 42
    z := 100
}
"#;
        let occurrences = find_literal_occurrences(source, "42");
        assert_eq!(occurrences.len(), 2);
        assert_eq!(occurrences[0].start_line, 3);
        assert_eq!(occurrences[1].start_line, 4);
    }

    #[test]
    fn test_find_literal_occurrences_excludes_strings() {
        let source = r#"package main

func main() {
    x := 42
    msg := "The answer is 42"
}
"#;
        let occurrences = find_literal_occurrences(source, "42");
        // Should only find the first occurrence, not the one inside the string
        assert_eq!(occurrences.len(), 1);
        assert_eq!(occurrences[0].start_line, 3);
    }

    #[test]
    fn test_find_literal_occurrences_excludes_comments() {
        let source = r#"package main

func main() {
    x := 42
    // The answer is 42
}
"#;
        let occurrences = find_literal_occurrences(source, "42");
        // Should only find the first occurrence, not the one in the comment
        assert_eq!(occurrences.len(), 1);
        assert_eq!(occurrences[0].start_line, 3);
    }

    #[test]
    fn test_find_insertion_point_after_package() {
        let source = r#"package main

import "fmt"

func main() {
}
"#;
        let insertion = find_insertion_point(source);
        // Should insert after import, before func
        assert_eq!(insertion.start_line, 4);
    }

    #[test]
    fn test_find_insertion_point_no_imports() {
        let source = r#"package main

func main() {
}
"#;
        let insertion = find_insertion_point(source);
        // Should insert after package, before func
        assert_eq!(insertion.start_line, 2);
    }
}
