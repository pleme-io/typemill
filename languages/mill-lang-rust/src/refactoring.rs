//! Rust refactoring operations using syn AST
//!
//! This module provides AST-based refactoring capabilities for Rust code.

use crate::constants;
use mill_foundation::protocol::{EditLocation, EditPlan, EditType, TextEdit};
use mill_lang_common::{
    find_literal_occurrences, is_valid_code_literal_location,
    refactoring::edit_plan_builder::EditPlanBuilder, CodeRange, ExtractConstantAnalysis,
    ExtractConstantEditPlanBuilder, LineExtractor,
};
use mill_plugin_api::{PluginApiError, PluginResult};

/// Plan extract function refactoring for Rust
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
        "\n{}fn {}() {{\n{}\n{}}}\n",
        indent, function_name, selected_code, indent
    );

    // Generate function call
    let function_call = format!("{}{}();", indent, function_name);

    let mut edits = Vec::new();

    // Replace selected code with function call FIRST (priority 100)
    // This must be applied before the insertion to avoid line offset issues
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
        priority: 100,
        description: format!("Replace code with call to '{}'", function_name),
    });

    // Insert new function above the selected code SECOND (priority 90)
    // After the replacement, this inserts at the now-vacant location
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
        priority: 90,
        description: format!("Create extracted function '{}'", function_name),
    });

    Ok(EditPlanBuilder::new(file_path, "extract_function")
        .with_edits(edits)
        .with_syntax_validation("Verify Rust syntax is valid after extraction")
        .with_intent_args(serde_json::json!({
            "function_name": function_name,
            "line_count": end_line - start_line + 1
        }))
        .with_complexity(5)
        .with_impact_area("function_extraction")
        .build())
}

/// Plan extract variable refactoring for Rust
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

    let var_name = variable_name.unwrap_or_else(|| "extracted".to_string());

    // Get indentation
    let indent = LineExtractor::get_indentation_str(source, start_line);

    // Generate variable declaration
    let declaration = format!("{}let {} = {};\n", indent, var_name, expression.trim());

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

    Ok(EditPlanBuilder::new(file_path, "extract_variable")
        .with_edits(edits)
        .with_syntax_validation("Verify Rust syntax is valid after extraction")
        .with_intent_args(serde_json::json!({
            "variable_name": var_name,
            "expression": expression
        }))
        .with_complexity(3)
        .with_impact_area("variable_extraction")
        .build())
}

/// Infer the explicit type from a literal value
fn infer_literal_type(literal: &str) -> &'static str {
    // Check for boolean
    if literal == "true" || literal == "false" {
        return "bool";
    }

    // Check for string literals (regular or raw)
    if literal.starts_with('"') || literal.starts_with("r\"") || literal.starts_with("r#") {
        return "&str";
    }

    // Check for numeric literals with type suffixes
    // Extract suffix by finding where digits end
    let trimmed = literal.trim_start_matches('-'); // Handle negative numbers
    let mut digit_end = 0;

    for (i, ch) in trimmed.chars().enumerate() {
        if ch.is_ascii_digit() || ch == '.' || ch == '_' {
            digit_end = i + 1;
        } else {
            break;
        }
    }

    // Check if there's a suffix after the digits
    if digit_end < trimmed.len() {
        let suffix = &trimmed[digit_end..];
        match suffix {
            "i8" => return "i8",
            "i16" => return "i16",
            "i32" => return "i32",
            "i64" => return "i64",
            "i128" => return "i128",
            "isize" => return "isize",
            "u8" => return "u8",
            "u16" => return "u16",
            "u32" => return "u32",
            "u64" => return "u64",
            "u128" => return "u128",
            "usize" => return "usize",
            "f32" => return "f32",
            "f64" => return "f64",
            _ => {}
        }
    }

    // Check for float (contains '.')
    if literal.contains('.') {
        return "f64";
    }

    // Default to i32 for integers
    "i32"
}

/// Find a Rust literal at a given position in a line of code
fn find_rust_literal_at_position(line_text: &str, col: usize) -> Option<(String, CodeRange)> {
    // Try to find different kinds of literals at the cursor position

    // Check for string literal (including raw strings)
    if let Some((literal, range)) = find_rust_string_literal(line_text, col) {
        return Some((literal, range));
    }

    // Check for numeric literal (including negative numbers)
    if let Some((literal, range)) = find_rust_numeric_literal(line_text, col) {
        return Some((literal, range));
    }

    // Check for boolean (true/false)
    if let Some((literal, range)) = find_rust_keyword_literal(line_text, col) {
        return Some((literal, range));
    }

    None
}

/// Find a string literal (regular or raw) at a cursor position
/// Supports:
/// - Regular strings: "hello"
/// - Raw strings: r"hello", r#"hello"#, r##"hello"##
/// - Escaped quotes: "He said \"hi\""
fn find_rust_string_literal(line_text: &str, col: usize) -> Option<(String, CodeRange)> {
    if col >= line_text.len() {
        return None;
    }

    let bytes = line_text.as_bytes();

    // Try to detect raw string first (r"..." or r#"..."#)
    // Scan backwards from cursor+1 to include current position
    let mut pos = col + 1;
    while pos > 0 {
        pos -= 1;

        // Check if this position starts with 'r' followed by optional hashes and a quote
        if bytes[pos] == b'r' {
            let mut hash_count = 0;
            let mut check_pos = pos + 1;

            // Count hashes after 'r'
            while check_pos < bytes.len() && bytes[check_pos] == b'#' {
                hash_count += 1;
                check_pos += 1;
            }

            // Check if there's an opening quote after the hashes
            if check_pos < bytes.len() && bytes[check_pos] == b'"' {
                let quote_pos = check_pos;

                // Build closing delimiter
                let mut closing = String::from("\"");
                for _ in 0..hash_count {
                    closing.push('#');
                }

                // Find the closing delimiter
                if let Some(end_offset) = line_text[quote_pos + 1..].find(&closing) {
                    let end = quote_pos + 1 + end_offset + closing.len();

                    // Check if cursor is within this raw string
                    if col >= pos && col < end {
                        return Some((
                            line_text[pos..end].to_string(),
                            CodeRange {
                                start_line: 0,
                                start_col: pos as u32,
                                end_line: 0,
                                end_col: end as u32,
                            },
                        ));
                    }
                }

                // Even if not found or cursor not in range, we found an 'r' prefix
                // so don't continue looking for other raw strings
                break;
            }
        }
    }

    // Try regular string literal
    // Scan backwards to find opening quote (including current position)
    let mut pos = col + 1; // Start one position ahead so we check col itself
    loop {
        if pos == 0 {
            break;
        }
        pos -= 1;

        if bytes[pos] == b'"' {
            // Check if it's escaped by counting backslashes before it
            let mut backslash_count = 0;
            let mut check_pos = pos;
            while check_pos > 0 && bytes[check_pos - 1] == b'\\' {
                backslash_count += 1;
                check_pos -= 1;
            }

            // If even number of backslashes (or zero), this quote is not escaped
            if backslash_count % 2 == 0 {
                // Found the opening quote, now find the closing quote
                let mut end = pos + 1;
                while end < bytes.len() {
                    if bytes[end] == b'"' {
                        // Check if this closing quote is escaped
                        let mut bs_count = 0;
                        let mut check = end;
                        while check > 0 && bytes[check - 1] == b'\\' {
                            bs_count += 1;
                            check -= 1;
                        }

                        // If even number of backslashes, this is the closing quote
                        if bs_count % 2 == 0 {
                            // Verify cursor is within this string
                            if col >= pos && col <= end {
                                return Some((
                                    line_text[pos..=end].to_string(),
                                    CodeRange {
                                        start_line: 0,
                                        start_col: pos as u32,
                                        end_line: 0,
                                        end_col: (end + 1) as u32,
                                    },
                                ));
                            }
                            break;
                        }
                    }
                    end += 1;
                }
                break;
            }
        }
    }

    None
}

/// Find a numeric literal (integer, float, or negative number) at a cursor position
fn find_rust_numeric_literal(line_text: &str, col: usize) -> Option<(String, CodeRange)> {
    if col >= line_text.len() {
        return None;
    }

    let bytes = line_text.as_bytes();

    // Determine where to start scanning for the number
    // If cursor is on a minus sign, start there
    let scan_start =
        if (bytes[col] == b'-' && col + 1 < bytes.len() && bytes[col + 1].is_ascii_digit())
            || bytes[col].is_ascii_digit()
            || bytes[col] == b'.'
        {
            col
        } else {
            // Cursor not on a number
            return None;
        };

    // Find the start of the number by scanning backwards
    let mut start = scan_start;
    while start > 0 {
        let prev = bytes[start - 1];
        if prev.is_ascii_digit() || prev == b'.' || prev == b'_' {
            start -= 1;
        } else if prev == b'-' && start == scan_start {
            // Include leading minus for negative numbers
            start -= 1;
            break;
        } else {
            break;
        }
    }

    // Find the end of the number by scanning forwards
    let mut end = scan_start + 1;
    while end < bytes.len() {
        let ch = bytes[end];
        if ch.is_ascii_digit() || ch == b'.' || ch == b'_' {
            end += 1;
        } else {
            break;
        }
    }

    // Check for type suffix (i32, u64, f32, etc.)
    if end < bytes.len() && bytes[end].is_ascii_alphabetic() {
        let suffix_start = end;
        while end < bytes.len() && bytes[end].is_ascii_alphanumeric() {
            end += 1;
        }

        // Validate it's a known numeric type suffix
        let suffix = &line_text[suffix_start..end];
        let valid_suffixes = [
            "i8", "i16", "i32", "i64", "i128", "isize", "u8", "u16", "u32", "u64", "u128", "usize",
            "f32", "f64",
        ];

        if !valid_suffixes.contains(&suffix) {
            // Not a valid suffix, backtrack
            end = suffix_start;
        }
    }

    if start < end && end <= line_text.len() {
        let text = &line_text[start..end];
        // Validate: must contain at least one digit
        // For validation, strip the type suffix if present
        let num_part = text.trim_start_matches('-');
        let has_digit = num_part.chars().any(|c| c.is_ascii_digit());

        if has_digit {
            return Some((
                text.to_string(),
                CodeRange {
                    start_line: 0,
                    start_col: start as u32,
                    end_line: 0,
                    end_col: end as u32,
                },
            ));
        }
    }

    None
}

/// Find a Rust keyword literal (true or false) at a cursor position
fn find_rust_keyword_literal(line_text: &str, col: usize) -> Option<(String, CodeRange)> {
    let keywords = ["true", "false"];

    for keyword in &keywords {
        // Try to match keyword at or near cursor
        for start in col.saturating_sub(keyword.len())
            ..=col.min(line_text.len().saturating_sub(keyword.len()))
        {
            if start + keyword.len() <= line_text.len()
                && &line_text[start..start + keyword.len()] == *keyword
            {
                // Check word boundaries
                let before_ok = start == 0
                    || !line_text[..start].ends_with(|c: char| c.is_alphanumeric() || c == '_');
                let after_ok = start + keyword.len() == line_text.len()
                    || !line_text[start + keyword.len()..]
                        .starts_with(|c: char| c.is_alphanumeric() || c == '_');

                if before_ok && after_ok {
                    return Some((
                        keyword.to_string(),
                        CodeRange {
                            start_line: 0,
                            start_col: start as u32,
                            end_line: 0,
                            end_col: (start + keyword.len()) as u32,
                        },
                    ));
                }
            }
        }
    }

    None
}

// is_valid_rust_literal_location is now provided by mill_lang_common::is_valid_code_literal_location
fn is_valid_rust_literal_location(line: &str, pos: usize, len: usize) -> bool {
    is_valid_code_literal_location(line, pos, len)
}

/// Find the appropriate insertion point for a constant declaration in Rust code
fn find_rust_insertion_point_for_constant(source: &str) -> PluginResult<CodeRange> {
    let lines: Vec<&str> = source.lines().collect();
    let mut insertion_line = 0;

    for (idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        let line_idx = idx as u32;

        // Record position after each use statement
        if trimmed.starts_with("use ") {
            insertion_line = line_idx + 1;
        }
        // Stop at first function, struct, impl, trait, const, or static definition
        else if trimmed.starts_with("fn ")
            || trimmed.starts_with("pub fn ")
            || trimmed.starts_with("struct ")
            || trimmed.starts_with("pub struct ")
            || trimmed.starts_with("impl ")
            || trimmed.starts_with("trait ")
            || trimmed.starts_with("pub trait ")
            || trimmed.starts_with("const ")
            || trimmed.starts_with("pub const ")
            || trimmed.starts_with("static ")
            || trimmed.starts_with("pub static ")
        {
            break;
        }
    }

    Ok(CodeRange {
        start_line: insertion_line,
        start_col: 0,
        end_line: insertion_line,
        end_col: 0,
    })
}

/// Analyze source code to extract information about a literal value at a cursor position
pub(crate) fn analyze_extract_constant(
    source: &str,
    line: u32,
    character: u32,
    _file_path: &str,
) -> PluginResult<ExtractConstantAnalysis> {
    let lines: Vec<&str> = source.lines().collect();

    // Get the line at cursor position
    let line_text = lines
        .get(line as usize)
        .ok_or_else(|| PluginApiError::invalid_input("Invalid line number"))?;

    // Find the literal at the cursor position
    let found_literal =
        find_rust_literal_at_position(line_text, character as usize).ok_or_else(|| {
            PluginApiError::invalid_input("No literal found at the specified location")
        })?;

    let literal_value = found_literal.0;
    let is_valid_literal = !literal_value.is_empty();
    let blocking_reasons = if !is_valid_literal {
        vec!["Could not extract literal at cursor position".to_string()]
    } else {
        vec![]
    };

    // Find all occurrences of this literal value in the source
    let occurrence_ranges =
        find_literal_occurrences(source, &literal_value, is_valid_rust_literal_location);

    // Insertion point: after use statements, at the top of the file
    let insertion_point = find_rust_insertion_point_for_constant(source)?;

    Ok(ExtractConstantAnalysis {
        literal_value,
        occurrence_ranges,
        is_valid_literal,
        blocking_reasons,
        insertion_point,
    })
}

/// Plan extract constant refactoring for Rust
pub fn plan_extract_constant(
    source: &str,
    line: u32,
    character: u32,
    name: &str,
    file_path: &str,
) -> PluginResult<EditPlan> {
    let analysis = analyze_extract_constant(source, line, character, file_path)?;

    // Rust needs explicit type annotation
    let rust_type = infer_literal_type(&analysis.literal_value);

    ExtractConstantEditPlanBuilder::new(analysis, name.to_string(), file_path.to_string())
        .with_declaration_format(|name, value| {
            format!("const {}: {} = {};\n", name, rust_type, value)
        })
        .map_err(PluginApiError::invalid_input)
}

/// Plan inline variable refactoring for Rust
pub fn plan_inline_variable(
    source: &str,
    variable_line: u32,
    variable_col: u32,
    file_path: &str,
) -> PluginResult<EditPlan> {
    let lines: Vec<&str> = source.lines().collect();

    if variable_line as usize >= lines.len() {
        return Err(PluginApiError::invalid_input("Line number out of bounds"));
    }

    let line_text = lines[variable_line as usize];

    // Guard clause: This function handles `let` bindings and `const` declarations
    // Prevents catastrophic backtracking on fn declarations
    let trimmed = line_text.trim();
    if !trimmed.starts_with("let ") && !trimmed.starts_with("const ") {
        return Err(PluginApiError::invalid_input(format!(
            "Not a `let` binding or `const` declaration at line {}. Only variables and constants can be inlined with this function.",
            variable_line + 1
        )));
    }

    // Pattern matching for variable declarations and constants
    // Supports: let x = ..., let mut x = ..., const X: Type = ...
    let var_pattern = constants::variable_decl_pattern();

    if let Some(captures) = var_pattern.captures(line_text) {
        let var_name = captures
            .get(1)
            .ok_or_else(|| {
                PluginApiError::internal("Regex missing capture group 1 for variable name")
            })?
            .as_str();
        let initializer = captures
            .get(2)
            .ok_or_else(|| {
                PluginApiError::internal("Regex missing capture group 2 for initializer")
            })?
            .as_str()
            .trim();

        // Find all usages of this variable in the rest of the source
        let mut edits = Vec::new();
        let var_regex = constants::word_boundary_pattern(var_name).map_err(|e| {
            PluginApiError::internal(format!("Failed to create regex pattern: {}", e))
        })?;

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

        Ok(EditPlanBuilder::new(file_path, "inline_variable")
            .with_edits(edits)
            .with_syntax_validation("Verify Rust syntax is valid after inlining")
            .with_intent_args(serde_json::json!({
                "variable_name": var_name,
                "value": initializer
            }))
            .with_complexity(3)
            .with_impact_area("variable_inlining")
            .build())
    } else {
        Err(PluginApiError::invalid_input(format!(
            "Could not find variable declaration at {}:{}",
            variable_line, variable_col
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_rust_literal_at_position_number() {
        let line = "let x = 42;";
        let result = find_rust_literal_at_position(line, 8);
        assert!(result.is_some());
        let (literal, _range) = result.unwrap();
        assert_eq!(literal, "42");
    }

    #[test]
    fn test_find_rust_literal_at_position_true() {
        let line = "let flag = true;";
        let result = find_rust_literal_at_position(line, 11);
        assert!(result.is_some());
        let (literal, _range) = result.unwrap();
        assert_eq!(literal, "true");
    }

    #[test]
    fn test_find_rust_literal_at_position_false() {
        let line = "let flag = false;";
        let result = find_rust_literal_at_position(line, 12);
        assert!(result.is_some());
        let (literal, _range) = result.unwrap();
        assert_eq!(literal, "false");
    }

    #[test]
    fn test_find_rust_literal_occurrences() {
        let source = "let x = 42;\nlet y = 42;\nlet z = 100;";
        let occurrences = find_literal_occurrences(source, "42", is_valid_rust_literal_location);
        assert_eq!(occurrences.len(), 2);
        assert_eq!(occurrences[0].start_line, 0);
        assert_eq!(occurrences[1].start_line, 1);
    }

    #[test]
    fn test_plan_extract_constant_valid_number() {
        let source = "let x = 42;\nlet y = 42;\n";
        let result = plan_extract_constant(source, 0, 8, "ANSWER", "test.rs");
        assert!(
            result.is_ok(),
            "Should extract numeric literal successfully"
        );

        let plan = result.unwrap();
        assert_eq!(plan.edits.len(), 3); // 1 declaration + 2 replacements

        // Check that the declaration is first (priority 100)
        assert_eq!(plan.edits[0].priority, 100);
        assert!(plan.edits[0].new_text.contains("const ANSWER"));
        assert!(plan.edits[0].new_text.contains("42"));
    }

    #[test]
    fn test_plan_extract_constant_invalid_name() {
        let source = "let x = 42;\n";
        let result = plan_extract_constant(source, 0, 8, "answer", "test.rs");
        assert!(result.is_err(), "Should reject lowercase name");
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("SCREAMING_SNAKE_CASE"));
    }

    #[test]
    fn test_plan_extract_constant_boolean() {
        let source = "let debug = true;\nlet verbose = true;\n";
        let result = plan_extract_constant(source, 0, 12, "DEBUG_MODE", "test.rs");
        assert!(result.is_ok(), "Should extract boolean literal");

        let plan = result.unwrap();
        assert_eq!(plan.edits.len(), 3); // 1 declaration + 2 replacements

        // Check declaration
        assert!(plan.edits[0].new_text.contains("const DEBUG_MODE"));
        assert!(plan.edits[0].new_text.contains("true"));
    }

    #[test]
    fn test_plan_extract_constant_no_literal_at_position() {
        let source = "let x = 42;\n";
        // Position 0 is not on a literal
        let result = plan_extract_constant(source, 0, 0, "ANSWER", "test.rs");
        assert!(result.is_err(), "Should fail when cursor not on literal");
        assert!(result.unwrap_err().to_string().contains("No literal found"));
    }

    #[test]
    fn test_find_rust_insertion_point_after_uses() {
        let source = r#"use std::collections::HashMap;
use std::io;

fn main() {
    println!("Hello");
}
"#;
        let result = find_rust_insertion_point_for_constant(source);
        assert!(result.is_ok());
        let point = result.unwrap();
        // Should insert after line 1 (second use statement), which is line 2 (0-indexed)
        assert_eq!(point.start_line, 2);
    }

    #[test]
    fn test_find_rust_insertion_point_no_uses() {
        let source = r#"fn main() {
    println!("Hello");
}
"#;
        let result = find_rust_insertion_point_for_constant(source);
        assert!(result.is_ok());
        let point = result.unwrap();
        // Should insert at the top (line 0)
        assert_eq!(point.start_line, 0);
    }

    #[test]
    fn test_analyze_extract_constant() {
        let source = "let x = 42;\nlet y = 42;\nlet z = 100;\n";
        let result = analyze_extract_constant(source, 0, 8, "test.rs");
        assert!(result.is_ok());

        let analysis = result.unwrap();
        assert_eq!(analysis.literal_value, "42");
        assert_eq!(analysis.occurrence_ranges.len(), 2);
        assert!(analysis.is_valid_literal);
        assert!(analysis.blocking_reasons.is_empty());
    }

    #[test]
    fn test_is_valid_literal_location_inside_string() {
        let line = r#"let msg = "The answer is 42";"#;
        // Position 21 is the '4' inside the string
        assert!(!is_valid_rust_literal_location(line, 21, 2));
    }

    #[test]
    fn test_is_valid_literal_location_inside_comment() {
        let line = "let x = 10; // TODO: change to 42";
        // Position 31 is the '4' inside the comment
        assert!(!is_valid_rust_literal_location(line, 31, 2));
    }

    #[test]
    fn test_is_valid_literal_location_valid() {
        let line = "let x = 42;";
        // Position 8 is the '4' in the actual literal
        assert!(is_valid_rust_literal_location(line, 8, 2));
    }

    // New comprehensive tests for string literal support

    #[test]
    fn test_find_rust_string_literal_regular() {
        let line = r#"let msg = "hello";"#;
        let result = find_rust_literal_at_position(line, 10);
        assert!(result.is_some());
        let (literal, range) = result.unwrap();
        assert_eq!(literal, r#""hello""#);
        assert_eq!(range.start_col, 10);
        assert_eq!(range.end_col, 17);
    }

    #[test]
    fn test_find_rust_string_literal_with_escaped_quotes() {
        let line = r#"let msg = "He said \"hi\"";"#;
        let result = find_rust_literal_at_position(line, 10);
        assert!(result.is_some());
        let (literal, _range) = result.unwrap();
        assert_eq!(literal, r#""He said \"hi\"""#);
    }

    #[test]
    fn test_find_rust_string_literal_raw() {
        let line = r#"let path = r"C:\Users\file";"#;
        let result = find_rust_literal_at_position(line, 11);
        assert!(result.is_some());
        let (literal, range) = result.unwrap();
        assert_eq!(literal, r#"r"C:\Users\file""#);
        assert_eq!(range.start_col, 11);
    }

    #[test]
    fn test_find_rust_string_literal_raw_with_hashes() {
        let line = r##"let text = r#"raw "string" with quotes"#;"##;
        let result = find_rust_literal_at_position(line, 11);
        assert!(result.is_some());
        let (literal, _range) = result.unwrap();
        assert_eq!(literal, r##"r#"raw "string" with quotes"#"##);
    }

    #[test]
    fn test_find_rust_string_literal_raw_with_multiple_hashes() {
        let line = r###"let text = r##"raw "string" with "quotes"##;"###;
        let result = find_rust_literal_at_position(line, 11);
        assert!(result.is_some());
        let (literal, _range) = result.unwrap();
        assert_eq!(literal, r###"r##"raw "string" with "quotes"##"###);
    }

    #[test]
    fn test_plan_extract_constant_string_literal() {
        let source = r#"let api = "https://api.example.com";
let backup = "https://api.example.com";
"#;
        let result = plan_extract_constant(source, 0, 10, "API_URL", "test.rs");
        assert!(result.is_ok(), "Should extract string literal successfully");

        let plan = result.unwrap();
        assert_eq!(plan.edits.len(), 3); // 1 declaration + 2 replacements

        // Check that the declaration has the correct type
        assert!(plan.edits[0].new_text.contains("const API_URL: &str"));
        assert!(plan.edits[0]
            .new_text
            .contains(r#""https://api.example.com""#));
    }

    #[test]
    fn test_infer_literal_type_bool() {
        assert_eq!(infer_literal_type("true"), "bool");
        assert_eq!(infer_literal_type("false"), "bool");
    }

    #[test]
    fn test_infer_literal_type_integers() {
        assert_eq!(infer_literal_type("42"), "i32");
        assert_eq!(infer_literal_type("100"), "i32");
        assert_eq!(infer_literal_type("42u64"), "u64");
        assert_eq!(infer_literal_type("100i64"), "i64");
        assert_eq!(infer_literal_type("255u8"), "u8");
    }

    #[test]
    fn test_infer_literal_type_floats() {
        assert_eq!(infer_literal_type("3.14"), "f64");
        assert_eq!(infer_literal_type("2.5"), "f64");
        assert_eq!(infer_literal_type("1.0f32"), "f32");
    }

    #[test]
    fn test_infer_literal_type_strings() {
        assert_eq!(infer_literal_type(r#""hello""#), "&str");
        assert_eq!(infer_literal_type(r#"r"raw""#), "&str");
        assert_eq!(infer_literal_type(r##"r#"raw"#"##), "&str");
    }

    #[test]
    fn test_is_valid_literal_location_with_escaped_quotes() {
        let line = r#"let s = "escaped \"quote\" here"; let x = 42;"#;
        // Position 44 is the '4' in the literal 42 (outside the string)
        assert!(is_valid_rust_literal_location(line, 44, 2));
        // Position 15 is inside the string
        assert!(!is_valid_rust_literal_location(line, 15, 1));
    }

    #[test]
    fn test_is_valid_literal_location_comment_after_string() {
        let line = r#"let s = "text"; // comment with "quotes""#;
        // Position 38 is inside the comment
        assert!(!is_valid_rust_literal_location(line, 38, 1));
    }

    #[test]
    fn test_find_rust_numeric_literal_negative() {
        let line = "let x = -42;";
        let result = find_rust_literal_at_position(line, 8);
        assert!(result.is_some());
        let (literal, _range) = result.unwrap();
        assert_eq!(literal, "-42");
    }

    #[test]
    fn test_find_rust_numeric_literal_float() {
        let line = "let pi = 3.14159;";
        let result = find_rust_literal_at_position(line, 9);
        assert!(result.is_some());
        let (literal, _range) = result.unwrap();
        assert_eq!(literal, "3.14159");
    }

    #[test]
    fn test_plan_extract_constant_with_type_suffix() {
        let source = "let timeout = 5000u64;\nlet delay = 5000u64;\n";
        let result = plan_extract_constant(source, 0, 14, "TIMEOUT_MS", "test.rs");
        assert!(result.is_ok());

        let plan = result.unwrap();
        // Check that the inferred type matches the suffix
        assert!(plan.edits[0].new_text.contains("const TIMEOUT_MS: u64"));
    }
}
