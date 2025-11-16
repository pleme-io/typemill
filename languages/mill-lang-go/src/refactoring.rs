//! Go refactoring operations using tree-sitter-go AST
//!
//! This module provides AST-based refactoring capabilities for Go code.

use lazy_static::lazy_static;
use mill_foundation::protocol::{
    EditLocation, EditPlan, EditPlanMetadata, EditType, TextEdit, ValidationRule, ValidationType,
};
use mill_lang_common::{
    find_literal_occurrences, is_screaming_snake_case, CodeRange, ExtractConstantAnalysis,
    LineExtractor,
};
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
            let occurrence_ranges = find_literal_occurrences(source, &literal_value, is_valid_literal_location);
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
            let occurrence_ranges = find_literal_occurrences(source, &literal_value, is_valid_literal_location);
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
            let occurrence_ranges = find_literal_occurrences(source, &literal_value, is_valid_literal_location);
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
///
/// Handles escaped quotes properly for double-quoted strings.
/// In Go, backticks are raw strings and don't support escaping.
fn find_string_literal(line_text: &str, line: u32, character: u32) -> Option<(String, CodeRange)> {
    let col = character as usize;

    if col >= line_text.len() {
        return None;
    }

    let chars: Vec<char> = line_text.chars().collect();

    // Look for opening quote before or at cursor
    let mut open_quote_idx = None;
    let mut open_quote_char = None;

    for i in (0..=col.min(chars.len() - 1)).rev() {
        if chars[i] == '"' || chars[i] == '\'' || chars[i] == '`' {
            // Check if this quote is escaped (only relevant for double quotes)
            if chars[i] == '"' {
                let mut backslash_count = 0;
                let mut check = i;
                while check > 0 && chars[check - 1] == '\\' {
                    backslash_count += 1;
                    check -= 1;
                }
                // If even number of backslashes, this quote is not escaped
                if backslash_count % 2 == 0 {
                    open_quote_idx = Some(i);
                    open_quote_char = Some(chars[i]);
                    break;
                }
            } else {
                // Single quotes and backticks don't support escaping
                open_quote_idx = Some(i);
                open_quote_char = Some(chars[i]);
                break;
            }
        }
    }

    if let (Some(start_idx), Some(quote_char)) = (open_quote_idx, open_quote_char) {
        // Find the closing quote
        let mut i = start_idx + 1;
        while i < chars.len() {
            if chars[i] == quote_char {
                // Check if this quote is escaped (only for double quotes)
                if quote_char == '"' {
                    let mut backslash_count = 0;
                    let mut check = i;
                    while check > 0 && chars[check - 1] == '\\' {
                        backslash_count += 1;
                        check -= 1;
                    }
                    // If even number of backslashes, this quote is not escaped - it's the closing quote
                    if backslash_count % 2 == 0 {
                        let literal: String = chars[start_idx..=i].iter().collect();
                        return Some((
                            literal,
                            CodeRange {
                                start_line: line,
                                start_col: start_idx as u32,
                                end_line: line,
                                end_col: (i + 1) as u32,
                            },
                        ));
                    }
                } else {
                    // Found closing quote for single quote or backtick
                    let literal: String = chars[start_idx..=i].iter().collect();
                    return Some((
                        literal,
                        CodeRange {
                            start_line: line,
                            start_col: start_idx as u32,
                            end_line: line,
                            end_col: (i + 1) as u32,
                        },
                    ));
                }
            }
            i += 1;
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


/// Counts unescaped quotes of a specific type before a position
///
/// This function correctly handles escaped quotes (e.g., `\"`) in Go string literals.
/// In Go, backslash escapes are only valid in interpreted string literals (double quotes),
/// not in raw string literals (backticks).
fn count_unescaped_quotes(text: &str, quote_char: char) -> usize {
    let mut count = 0;
    let mut chars = text.chars().peekable();
    let mut escaped = false;

    while let Some(ch) = chars.next() {
        if escaped {
            // This character is escaped, skip it
            escaped = false;
        } else if ch == '\\' && quote_char == '"' {
            // Backslash only escapes in double-quoted strings, not backticks or single quotes
            escaped = true;
        } else if ch == quote_char {
            count += 1;
        }
    }

    count
}

/// Validates whether a position in source code is a valid location for a literal
///
/// This function checks if a position is inside a string literal or comment.
///
/// # Important limitations
/// - Multi-line raw strings (backticks) are not fully supported. This function only
///   checks the current line, so it may incorrectly identify positions inside multi-line
///   raw strings as valid. Full support would require parsing the entire file.
/// - Block comments (`/* */`) spanning multiple lines are only detected on the current line.
///
/// # Arguments
/// * `line` - The line of text to check
/// * `pos` - The position within the line to validate
/// * `_len` - The length of the literal (currently unused)
fn is_valid_literal_location(line: &str, pos: usize, _len: usize) -> bool {
    let before = &line[..pos];

    // Count unescaped quotes before the position
    let single_quotes = count_unescaped_quotes(before, '\'');
    let double_quotes = count_unescaped_quotes(before, '"');
    let backticks = count_unescaped_quotes(before, '`');

    // If an odd number of quotes appear before the position, we're inside a string
    // Note: This doesn't handle multi-line raw strings (backticks) correctly
    if single_quotes % 2 == 1 || double_quotes % 2 == 1 || backticks % 2 == 1 {
        return false;
    }

    // Check for single-line comment marker
    if let Some(comment_pos) = line.find("//") {
        // Make sure the // is not inside a string
        let before_comment = &line[..comment_pos];
        let sq = count_unescaped_quotes(before_comment, '\'');
        let dq = count_unescaped_quotes(before_comment, '"');
        let bt = count_unescaped_quotes(before_comment, '`');

        // Only treat as comment if not inside a string
        if sq % 2 == 0 && dq % 2 == 0 && bt % 2 == 0 && pos > comment_pos {
            return false;
        }
    }

    // Check for block comment markers /* */
    // Note: This only handles block comments on a single line, not multi-line block comments
    if let Some(open_pos) = line.find("/*") {
        // Make sure the /* is not inside a string
        let before_open = &line[..open_pos];
        let sq = count_unescaped_quotes(before_open, '\'');
        let dq = count_unescaped_quotes(before_open, '"');
        let bt = count_unescaped_quotes(before_open, '`');

        if sq % 2 == 0 && dq % 2 == 0 && bt % 2 == 0 {
            // We found a block comment start
            if let Some(close_pos) = line[open_pos..].find("*/") {
                let close_abs = open_pos + close_pos + 2;
                if pos >= open_pos && pos < close_abs {
                    return false;
                }
            } else {
                // Block comment starts but doesn't close on this line
                // Position is in comment if it's after the open
                if pos > open_pos {
                    return false;
                }
            }
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
        let occurrences = find_literal_occurrences(source, "42", is_valid_literal_location);
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
        let occurrences = find_literal_occurrences(source, "42", is_valid_literal_location);
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
        let occurrences = find_literal_occurrences(source, "42", is_valid_literal_location);
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

    #[test]
    fn test_count_unescaped_quotes_no_escapes() {
        assert_eq!(count_unescaped_quotes(r#"hello "world""#, '"'), 2);
        assert_eq!(count_unescaped_quotes("hello 'world'", '\''), 2);
        assert_eq!(count_unescaped_quotes("hello `world`", '`'), 2);
    }

    #[test]
    fn test_count_unescaped_quotes_with_escapes() {
        // Double quotes support escaping
        // In the string r#"hello \"world\""#:
        // The actual characters are: h e l l o   \ " w o r l d \ "
        // The backslashes escape the quotes, so there are 0 unescaped quotes
        assert_eq!(count_unescaped_quotes(r#"hello \"world\""#, '"'), 0);

        // In r#"say "hello \"world\"""#:
        // Actual chars: s a y   " h e l l o   \ " w o r l d \ " "
        // First " is unescaped, middle \" is escaped, last \" is escaped, final " is unescaped
        // So 2 unescaped quotes
        assert_eq!(count_unescaped_quotes(r#"say "hello \"world\"""#, '"'), 2);

        // In r#"\"quote\" in middle "real""#:
        // Actual chars: \ " q u o t e \ "   i n   m i d d l e   " r e a l "
        // First \" is escaped, second \" is escaped, third and fourth " are unescaped
        // So 2 unescaped quotes
        assert_eq!(count_unescaped_quotes(r#"\"quote\" in middle "real""#, '"'), 2);

        // Backticks don't support escaping (raw strings in Go)
        // The backslashes are literal characters, not escape sequences
        assert_eq!(count_unescaped_quotes(r#"hello \`world\`"#, '`'), 2);

        // Single quotes don't support escaping in Go (rune literals are different)
        assert_eq!(count_unescaped_quotes(r#"hello \'world\'"#, '\''), 2);
    }

    #[test]
    fn test_find_literal_occurrences_excludes_escaped_strings() {
        let source = r#"package main

func main() {
    x := 42
    msg := "The answer is \"42\" not 42"
}
"#;
        let occurrences = find_literal_occurrences(source, "42", is_valid_literal_location);
        // Should find the literal 42 on line 3 (x := 42)
        // Should NOT find the ones inside the string, even though one is in escaped quotes
        // Both "42" instances in the string are inside the outer quotes
        assert_eq!(occurrences.len(), 1);
        assert_eq!(occurrences[0].start_line, 3);
    }

    #[test]
    fn test_find_literal_occurrences_excludes_block_comments() {
        let source = r#"package main

func main() {
    x := 42
    /* Block comment with 42 inside */
    y := 42
}
"#;
        let occurrences = find_literal_occurrences(source, "42", is_valid_literal_location);
        // Should find 42 on lines 3 and 5, but not in the block comment on line 4
        assert_eq!(occurrences.len(), 2);
        assert_eq!(occurrences[0].start_line, 3);
        assert_eq!(occurrences[1].start_line, 5);
    }

    #[test]
    fn test_find_literal_occurrences_inline_block_comment() {
        let source = r#"package main

func main() {
    x := /* 42 */ 42
}
"#;
        let occurrences = find_literal_occurrences(source, "42", is_valid_literal_location);
        // Should find only the real 42, not the one in the inline block comment
        assert_eq!(occurrences.len(), 1);
        assert_eq!(occurrences[0].start_line, 3);
        assert_eq!(occurrences[0].start_col, 18);
    }

    #[test]
    fn test_find_literal_occurrences_raw_strings() {
        let source = r#"package main

func main() {
    x := 42
    msg := `Raw string with 42 inside`
}
"#;
        let occurrences = find_literal_occurrences(source, "42", is_valid_literal_location);
        // Should only find the first occurrence, not the one inside the raw string
        assert_eq!(occurrences.len(), 1);
        assert_eq!(occurrences[0].start_line, 3);
    }

    #[test]
    fn test_is_valid_literal_location_string_with_comment_chars() {
        // "//" inside a string should not be treated as a comment
        let line = r#"    msg := "http://example.com" and 42"#;
        assert!(is_valid_literal_location(line, 36, 2)); // 42 at end
        assert!(!is_valid_literal_location(line, 15, 2)); // Inside the string
    }

    #[test]
    fn test_is_valid_literal_location_block_comment_chars_in_string() {
        // "/*" inside a string should not be treated as a comment
        let line = r#"    msg := "/* not a comment */" and 42"#;
        assert!(is_valid_literal_location(line, 38, 2)); // 42 at end
        assert!(!is_valid_literal_location(line, 15, 2)); // Inside the string
    }

    #[test]
    fn test_negative_numbers() {
        let source = r#"package main

func main() {
    x := -42
    y := -42
}
"#;
        // Test extracting negative number - cursor on the '4' (position 10)
        // Line 3 is "    x := -42", so position 10 is the '4'
        let result = plan_extract_constant(source, 3, 10, "NEGATIVE_ANSWER", "test.go");
        assert!(result.is_ok(), "Should extract negative number literal");

        let plan = result.unwrap();
        // Should find both occurrences
        assert_eq!(plan.edits.len(), 3); // 1 declaration + 2 replacements
        assert!(plan.edits[0].new_text.contains("const NEGATIVE_ANSWER = -42"));
    }

    #[test]
    fn test_decimal_numbers() {
        let source = r#"package main

func main() {
    x := 3.14159
    y := 3.14159
}
"#;
        let result = plan_extract_constant(source, 3, 9, "PI", "test.go");
        assert!(result.is_ok(), "Should extract decimal literal");

        let plan = result.unwrap();
        assert_eq!(plan.edits.len(), 3); // 1 declaration + 2 replacements
        assert!(plan.edits[0].new_text.contains("const PI = 3.14159"));
    }

    #[test]
    fn test_extract_constant_with_escaped_quotes() {
        let source = r#"package main

func main() {
    msg1 := "Say \"hello\""
    msg2 := "Say \"hello\""
}
"#;
        // Try to extract the string literal
        let result = plan_extract_constant(source, 3, 12, "GREETING_MESSAGE", "test.go");
        assert!(result.is_ok(), "Should extract string with escaped quotes");

        let plan = result.unwrap();
        // Should find both occurrences
        assert_eq!(plan.edits.len(), 3); // 1 declaration + 2 replacements
        assert!(plan.edits[0].new_text.contains(r#"const GREETING_MESSAGE = "Say \"hello\"""#));
    }

    #[test]
    fn test_extract_constant_raw_string_backticks() {
        let source = r#"package main

func main() {
    msg1 := `Raw string literal`
    msg2 := `Raw string literal`
}
"#;
        let result = plan_extract_constant(source, 3, 12, "RAW_MESSAGE", "test.go");
        assert!(result.is_ok(), "Should extract raw string literal");

        let plan = result.unwrap();
        assert_eq!(plan.edits.len(), 3); // 1 declaration + 2 replacements
        assert!(plan.edits[0].new_text.contains("const RAW_MESSAGE = `Raw string literal`"));
    }
}
