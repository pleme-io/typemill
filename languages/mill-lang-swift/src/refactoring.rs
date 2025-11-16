use lazy_static::lazy_static;
use mill_foundation::protocol::{EditPlan, EditType, TextEdit};
use mill_lang_common::{
    find_literal_occurrences, is_escaped, is_valid_code_literal_location,
    refactoring::edit_plan_builder::EditPlanBuilder, ExtractConstantEditPlanBuilder, LineExtractor,
};
use mill_plugin_api::{PluginApiError, PluginResult};
use regex::Regex;

/// Extracts selected code into a new Swift function.
///
/// This refactoring operation takes a range of lines and creates a new private function
/// containing that code, replacing the original selection with a call to the new function.
///
/// # Arguments
/// * `source` - The complete Swift source code
/// * `start_line` - Zero-based starting line number of the selection
/// * `end_line` - Zero-based ending line number of the selection (inclusive)
/// * `function_name` - The name for the new function
/// * `file_path` - Path to the file being refactored
///
/// # Returns
/// * `Ok(EditPlan)` - The refactoring plan with two edits: function creation and call replacement
/// * `Err(PluginApiError)` - If the line range is invalid
///
/// # Examples
/// ```rust
/// let source = r#"func main() {
///     let x = 10
///     let y = 20
///     print(x + y)
/// }"#;
/// let plan = plan_extract_function(source, 1, 2, "calculateSum", "main.swift")?;
/// assert_eq!(plan.edits.len(), 2); // insert new function + replace with call
/// ```
pub fn plan_extract_function(
    source: &str,
    start_line: u32,
    end_line: u32,
    function_name: &str,
    file_path: &str,
) -> PluginResult<EditPlan> {
    let lines: Vec<&str> = source.lines().collect();
    if start_line > end_line || end_line as usize >= lines.len() {
        return Err(PluginApiError::invalid_input("Invalid line range"));
    }

    let selected_lines = &lines[start_line as usize..=end_line as usize];
    let selected_text = selected_lines.join("\n");

    // Find the indentation of the first line of the selection
    let indent = selected_lines[0]
        .chars()
        .take_while(|c| c.is_whitespace())
        .collect::<String>();

    let new_function_text = format!(
        "\n\n{}private func {}() {{\n{}\n{}}}\n",
        indent, function_name, selected_text, indent
    );

    // Find a place to insert the new function. For simplicity, we'll insert it after the current function.
    // This is a very rough approximation.
    let insert_line = end_line + 2;

    let insert_edit = TextEdit {
        file_path: Some(file_path.to_string()),
        edit_type: EditType::Insert,
        location: mill_foundation::protocol::EditLocation {
            start_line: insert_line,
            start_column: 0,
            end_line: insert_line,
            end_column: 0,
        },
        original_text: String::new(),
        new_text: new_function_text,
        priority: 100,
        description: format!("Create new function '{}'", function_name),
    };

    let replace_edit = TextEdit {
        file_path: Some(file_path.to_string()),
        edit_type: EditType::Replace,
        location: mill_foundation::protocol::EditLocation {
            start_line,
            start_column: 0,
            end_line: end_line + 1,
            end_column: 0,
        },
        original_text: selected_text,
        new_text: format!("{}()", function_name),
        priority: 90,
        description: format!("Replace selection with call to '{}'", function_name),
    };

    Ok(EditPlanBuilder::new(file_path, "extract_function")
        .with_edits(vec![insert_edit, replace_edit])
        .with_syntax_validation("Verify syntax after extraction")
        .with_intent_args(serde_json::json!({ "function_name": function_name }))
        .with_complexity(3)
        .with_impact_area("function_extraction")
        .build())
}

lazy_static! {
    static ref VAR_DECL_REGEX: Regex = Regex::new(r"^\s*(?:let|var)\s+([a-zA-Z0-9_]+)\s*=\s*(.*)")
        .expect("Invalid regex for Swift variable declaration");
}

/// Inlines a Swift variable by replacing all usages with its initializer value.
///
/// This refactoring operation finds a `let` or `var` declaration, extracts its value,
/// replaces all references with that value, and removes the declaration.
///
/// # Arguments
/// * `source` - The complete Swift source code
/// * `variable_line` - Zero-based line number of the variable declaration
/// * `_variable_col` - Zero-based column (currently unused)
/// * `file_path` - Path to the file being refactored
///
/// # Returns
/// * `Ok(EditPlan)` - The refactoring plan with replacement edits for each usage plus declaration removal
/// * `Err(PluginApiError)` - If the line is not a variable declaration or line number is invalid
///
/// # Examples
/// ```rust
/// let source = r#"
/// let rate = 0.08
/// let tax = price * rate
/// let total = base * rate
/// "#;
/// let plan = plan_inline_variable(source, 1, 0, "test.swift")?;
/// assert!(plan.edits.len() >= 2); // replacements + declaration removal
/// ```
pub fn plan_inline_variable(
    source: &str,
    variable_line: u32,
    _variable_col: u32,
    file_path: &str,
) -> PluginResult<EditPlan> {
    let lines: Vec<&str> = source.lines().collect();
    if variable_line as usize >= lines.len() {
        return Err(PluginApiError::invalid_input("Invalid line number"));
    }

    let line_content = lines[variable_line as usize];
    let caps = VAR_DECL_REGEX
        .captures(line_content)
        .ok_or_else(|| PluginApiError::invalid_input("Line is not a variable declaration"))?;
    let var_name = &caps[1];
    let var_value = caps[2].trim().trim_end_matches(';').to_string();

    let mut edits = Vec::new();
    let search_pattern = format!(r"\b{}\b", var_name);
    let search_re = Regex::new(&search_pattern)
        .map_err(|e| PluginApiError::internal(format!("Invalid regex: {}", e)))?;

    for (i, line) in lines.iter().enumerate() {
        if i as u32 != variable_line {
            for mat in search_re.find_iter(line) {
                edits.push(TextEdit {
                    file_path: Some(file_path.to_string()),
                    edit_type: EditType::Replace,
                    location: mill_foundation::protocol::EditLocation {
                        start_line: i as u32,
                        start_column: mat.start() as u32,
                        end_line: i as u32,
                        end_column: mat.end() as u32,
                    },
                    original_text: var_name.to_string(),
                    new_text: var_value.clone(),
                    priority: 90,
                    description: format!("Inline variable '{}'", var_name),
                });
            }
        }
    }

    edits.push(TextEdit {
        file_path: Some(file_path.to_string()),
        edit_type: EditType::Delete,
        location: mill_foundation::protocol::EditLocation {
            start_line: variable_line,
            start_column: 0,
            end_line: variable_line + 1,
            end_column: 0,
        },
        original_text: line_content.to_string(),
        new_text: String::new(),
        priority: 100,
        description: format!("Remove declaration of '{}'", var_name),
    });

    Ok(EditPlanBuilder::new(file_path, "inline_variable")
        .with_edits(edits)
        .with_syntax_validation("Verify syntax is valid")
        .with_intent_args(serde_json::json!({ "variable_name": var_name }))
        .with_complexity(4)
        .with_impact_area("variable_inlining")
        .build())
}

/// Extracts a Swift expression into a named variable.
///
/// This refactoring operation takes an expression (single-line or multi-line) and extracts
/// it into a `let` variable declaration, replacing the original expression with the variable name.
///
/// # Arguments
/// * `source` - The complete Swift source code
/// * `start_line` - Zero-based starting line of the expression
/// * `start_col` - Zero-based starting column of the expression
/// * `end_line` - Zero-based ending line of the expression
/// * `end_col` - Zero-based ending column of the expression (exclusive)
/// * `variable_name` - Optional name for the variable (defaults to "extractedVar")
/// * `file_path` - Path to the file being refactored
///
/// # Returns
/// * `Ok(EditPlan)` - The refactoring plan with variable declaration insertion and expression replacement
/// * `Err(PluginApiError)` - If the line range is invalid
///
/// # Examples
/// ```rust
/// let source = r#"func calculate() {
///     let total = 100 * 1.08
///     return total
/// }"#;
/// let plan = plan_extract_variable(source, 1, 16, 1, 26, Some("taxRate".to_string()), "test.swift")?;
/// assert_eq!(plan.edits.len(), 2); // declaration + replacement
/// ```
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
    if start_line > end_line || end_line as usize >= lines.len() {
        return Err(PluginApiError::invalid_input("Invalid line range"));
    }

    let expression_text = if start_line == end_line {
        lines[start_line as usize][start_col as usize..end_col as usize].to_string()
    } else {
        // A rough approximation for multi-line expressions
        let mut text = String::new();
        text.push_str(&lines[start_line as usize][start_col as usize..]);
        for line in lines
            .iter()
            .take(end_line as usize)
            .skip((start_line + 1) as usize)
        {
            text.push_str(line);
        }
        text.push_str(&lines[end_line as usize][..end_col as usize]);
        text
    };

    let var_name = variable_name.unwrap_or_else(|| "extractedVar".to_string());
    let declaration_text = format!("let {} = {}", var_name, expression_text);

    // Find indentation
    let indent = LineExtractor::get_indentation_str(source, start_line);

    let insert_edit = TextEdit {
        file_path: Some(file_path.to_string()),
        edit_type: EditType::Insert,
        location: mill_foundation::protocol::EditLocation {
            start_line,
            start_column: 0,
            end_line: start_line,
            end_column: 0,
        },
        original_text: String::new(),
        new_text: format!("{}{}\n", indent, declaration_text),
        priority: 100,
        description: format!("Declare new variable '{}'", var_name),
    };

    let replace_edit = TextEdit {
        file_path: Some(file_path.to_string()),
        edit_type: EditType::Replace,
        location: mill_foundation::protocol::EditLocation {
            start_line,
            start_column: start_col,
            end_line,
            end_column: end_col,
        },
        original_text: expression_text.clone(),
        new_text: var_name.clone(),
        priority: 90,
        description: format!("Replace expression with '{}'", var_name),
    };

    Ok(EditPlanBuilder::new(file_path, "extract_variable")
        .with_edits(vec![insert_edit, replace_edit])
        .with_syntax_validation("Verify syntax after extraction")
        .with_intent_args(serde_json::json!({
            "expression": expression_text,
            "variable_name": var_name,
        }))
        .with_complexity(2)
        .with_impact_area("variable_extraction")
        .build())
}

// ============================================================================
// Extract Constant Support
// ============================================================================

/// Analyzes source code to extract information about a literal value at a cursor position.
///
/// # Arguments
/// * `source` - The Swift source code
/// * `line` - Zero-based line number where the cursor is positioned
/// * `character` - Zero-based character offset within the line
/// * `_file_path` - Path to the file (for future use)
///
/// # Returns
/// * `Ok(ExtractConstantAnalysis)` - Analysis result with literal value, occurrence ranges,
///                                     validation status, and insertion point
/// * `Err(PluginApiError)` - If no literal is found at the cursor position
pub(crate) fn analyze_extract_constant(
    source: &str,
    line: u32,
    character: u32,
    _file_path: &str,
) -> PluginResult<mill_lang_common::ExtractConstantAnalysis> {
    let lines: Vec<&str> = source.lines().collect();

    // Get the line at cursor position
    let line_text = lines
        .get(line as usize)
        .ok_or_else(|| PluginApiError::invalid_input("Invalid line number"))?;

    // Find the literal at the cursor position
    let (literal_value, _literal_range) = find_swift_literal_at_position(line_text, character as usize)
        .ok_or_else(|| {
            PluginApiError::invalid_input(
                "Cursor is not positioned on a literal value. Extract constant only works on numbers, strings, and booleans.",
            )
        })?;

    // Find all occurrences of this literal value in the source
    let occurrence_ranges =
        find_literal_occurrences(source, &literal_value, is_valid_swift_literal_location);

    // For Swift, constants are typically declared at the top of the file or class
    let insertion_point = mill_lang_common::CodeRange::new(0, 0, 0, 0);

    Ok(mill_lang_common::ExtractConstantAnalysis {
        literal_value,
        occurrence_ranges,
        is_valid_literal: true,
        blocking_reasons: vec![],
        insertion_point,
    })
}

/// Extracts a literal value to a named constant in Swift code.
///
/// This refactoring operation replaces all occurrences of a literal (number, string, or boolean)
/// with a named constant declaration at the file level, improving code maintainability by
/// eliminating magic values.
///
/// # Arguments
/// * `source` - The Swift source code
/// * `line` - Zero-based line number where the cursor is positioned
/// * `character` - Zero-based character offset within the line
/// * `name` - The constant name (must be SCREAMING_SNAKE_CASE)
/// * `file_path` - Path to the file being refactored
///
/// # Returns
/// * `Ok(EditPlan)` - The edit plan with constant declaration and replacements
/// * `Err(PluginApiError)` - If the cursor is not on a literal or the name is invalid
pub fn plan_extract_constant(
    source: &str,
    line: u32,
    character: u32,
    name: &str,
    file_path: &str,
) -> PluginResult<EditPlan> {
    let analysis = analyze_extract_constant(source, line, character, file_path)?;

    ExtractConstantEditPlanBuilder::new(analysis, name.to_string(), file_path.to_string())
        .with_declaration_format(|name, value| format!("let {} = {}\n", name, value))
        .map_err(|e| PluginApiError::invalid_input(e))
}

/// Finds a Swift literal at a given position in a line of code.
///
/// This function attempts to identify any Swift literal (numeric, string, or boolean)
/// at the cursor position by trying each literal type in sequence.
///
/// # Arguments
/// * `line_text` - The complete line of code
/// * `col` - Zero-based character position within the line
///
/// # Returns
/// * `Some((literal, (start, end)))` - The literal string and its start/end column positions
/// * `None` - If no literal is found at the cursor position
///
/// # Supported Literals
/// Tries in order: numeric → string → boolean
fn find_swift_literal_at_position(line_text: &str, col: usize) -> Option<(String, (u32, u32))> {
    // Check for numeric literal
    if let Some((literal, start, end)) = find_swift_numeric_literal(line_text, col) {
        return Some((literal, (start, end)));
    }

    // Check for string literal
    if let Some((literal, start, end)) = find_swift_string_literal(line_text, col) {
        return Some((literal, (start, end)));
    }

    // Check for boolean literal
    if let Some((literal, start, end)) = find_swift_boolean_literal(line_text, col) {
        return Some((literal, (start, end)));
    }

    None
}

/// Find numeric literal at cursor position
/// Handles: integers, floats, negative numbers, hex (0xFF), binary (0b101), octal (0o77)
fn find_swift_numeric_literal(line_text: &str, col: usize) -> Option<(String, u32, u32)> {
    if col >= line_text.len() {
        return None;
    }

    let chars: Vec<char> = line_text.chars().collect();

    // Handle the case where cursor is right after a number or on a hex digit
    let mut start = col;
    if col > 0 {
        if let Some(&ch) = chars.get(col) {
            // If not on a numeric or hex char, try the previous position
            if !is_numeric_char(Some(ch)) && !ch.is_ascii_hexdigit() {
                start = col.saturating_sub(1);
            }
        }
    }

    // Scan backwards to find the start of the number
    while start > 0 {
        let prev_char = chars.get(start.saturating_sub(1));
        if let Some(&ch) = prev_char {
            if is_numeric_char(Some(ch)) || ch.is_ascii_hexdigit() {
                start -= 1;
            } else if ch == 'x' || ch == 'X' || ch == 'b' || ch == 'B' || ch == 'o' || ch == 'O' {
                // Might be part of 0x, 0b, 0o prefix - keep going
                start -= 1;
            } else if ch == '-' || ch == '+' {
                // Check if this is a sign (not an operator)
                if start == 1 {
                    start -= 1;
                    break;
                } else if let Some(&before_sign) = chars.get(start.saturating_sub(2)) {
                    if !before_sign.is_alphanumeric()
                        && before_sign != '_'
                        && before_sign != ')'
                        && before_sign != ']'
                    {
                        start -= 1;
                        break;
                    }
                }
                break;
            } else {
                break;
            }
        } else {
            break;
        }
    }

    // Scan forward to find the end
    let mut end = start;

    // Check for hex (0x), binary (0b), or octal (0o) prefix
    if end < chars.len() && chars[end] == '0' && end + 1 < chars.len() {
        let next = chars[end + 1].to_ascii_lowercase();
        if next == 'x' {
            // Hexadecimal: 0xFF, 0x1A2B
            end += 2;
            while end < chars.len() && (chars[end].is_ascii_hexdigit() || chars[end] == '_') {
                end += 1;
            }
        } else if next == 'b' {
            // Binary: 0b1010, 0b1111_0000
            end += 2;
            while end < chars.len() && (chars[end] == '0' || chars[end] == '1' || chars[end] == '_')
            {
                end += 1;
            }
        } else if next == 'o' {
            // Octal: 0o77, 0o755
            end += 2;
            while end < chars.len()
                && ((chars[end] >= '0' && chars[end] <= '7') || chars[end] == '_')
            {
                end += 1;
            }
        } else {
            // Regular number
            end = scan_regular_number(line_text, start)?;
        }
    } else {
        // Regular number (including negative, floats, scientific notation)
        end = scan_regular_number(line_text, start)?;
    }

    if start < end && end <= line_text.len() {
        let text = &line_text[start..end];
        // Validate that this is actually a valid number
        if is_valid_number(text) {
            return Some((text.to_string(), start as u32, end as u32));
        }
    }

    None
}

/// Checks if a character is part of a numeric literal in Swift.
///
/// # Arguments
/// * `ch` - Optional character to check
///
/// # Returns
/// * `true` - If the character is a digit, decimal point, or underscore
/// * `false` - Otherwise or if None
fn is_numeric_char(ch: Option<char>) -> bool {
    match ch {
        Some(c) => c.is_ascii_digit() || c == '.' || c == '_',
        None => false,
    }
}

/// Scans forward from a position to find the end of a regular number (not hex/binary/octal).
///
/// This function identifies the boundaries of standard numeric literals including
/// integers, floating-point numbers, and scientific notation.
///
/// # Arguments
/// * `line_text` - The complete line of code
/// * `start` - Starting position to scan from
///
/// # Returns
/// * `Some(end_position)` - The position after the last character of the number
/// * `None` - If no valid number is found at the start position
///
/// # Supported Formats
/// - Integers: `42`, `-100`
/// - Floats: `3.14`, `-2.5`
/// - Scientific notation: `1.5e-10`, `2E+5`
/// - With underscores: `1_000_000`
fn scan_regular_number(line_text: &str, start: usize) -> Option<usize> {
    let chars: Vec<char> = line_text.chars().collect();
    let mut pos = start;

    // Skip optional sign
    if pos < chars.len() && (chars[pos] == '-' || chars[pos] == '+') {
        pos += 1;
    }

    // Scan digits before decimal point
    let digit_start = pos;
    while pos < chars.len() && (chars[pos].is_ascii_digit() || chars[pos] == '_') {
        pos += 1;
    }

    // Handle decimal point
    if pos < chars.len() && chars[pos] == '.' {
        pos += 1;
        // Scan digits after decimal point
        while pos < chars.len() && (chars[pos].is_ascii_digit() || chars[pos] == '_') {
            pos += 1;
        }
    }

    // Must have at least one digit
    if pos == digit_start || (pos == digit_start + 1 && chars.get(digit_start) == Some(&'.')) {
        return None;
    }

    // Handle scientific notation (e or E)
    if pos < chars.len() {
        let ch = chars[pos].to_ascii_lowercase();
        if ch == 'e' {
            pos += 1;
            // Optional sign after 'e'
            if pos < chars.len() && (chars[pos] == '+' || chars[pos] == '-') {
                pos += 1;
            }
            // Must have digits in exponent
            let exp_start = pos;
            while pos < chars.len() && (chars[pos].is_ascii_digit() || chars[pos] == '_') {
                pos += 1;
            }
            if pos == exp_start {
                // Invalid: 'e' without exponent
                return None;
            }
        }
    }

    Some(pos)
}

/// Validates if a string represents a valid Swift number.
///
/// This function performs comprehensive validation of Swift numeric formats including
/// special prefixes (0x, 0b, 0o) and scientific notation.
///
/// # Arguments
/// * `text` - The string to validate as a number
///
/// # Returns
/// * `true` - If the text is a valid Swift numeric literal
/// * `false` - If the text is not a valid number format
///
/// # Supported Formats
/// - Hexadecimal: `0xFF`, `0x1A2B`
/// - Binary: `0b1010`, `0b1111_0000`
/// - Octal: `0o77`, `0o755`
/// - Decimal: `42`, `-100`, `123_456`
/// - Floating-point: `3.14`, `-2.5`, `1.5e-10`
fn is_valid_number(text: &str) -> bool {
    if text.is_empty() {
        return false;
    }

    // Remove underscores (numeric separators)
    let cleaned = text.replace('_', "");

    // Check for hex, binary, octal
    if cleaned.starts_with("0x") || cleaned.starts_with("0X") {
        return cleaned.len() > 2 && cleaned[2..].chars().all(|c| c.is_ascii_hexdigit());
    }
    if cleaned.starts_with("0b") || cleaned.starts_with("0B") {
        return cleaned.len() > 2 && cleaned[2..].chars().all(|c| c == '0' || c == '1');
    }
    if cleaned.starts_with("0o") || cleaned.starts_with("0O") {
        return cleaned.len() > 2 && cleaned[2..].chars().all(|c| c >= '0' && c <= '7');
    }

    // For regular numbers, try parsing as f64
    // This handles integers, floats, scientific notation, and negative numbers
    cleaned.parse::<f64>().is_ok()
}

/// Find string literal at cursor position
/// Properly handles escaped quotes (e.g., "He said \"hi\"")
fn find_swift_string_literal(line_text: &str, col: usize) -> Option<(String, u32, u32)> {
    if col > line_text.len() {
        return None;
    }

    // Look for opening quote before cursor - must be unescaped
    let mut opening_quote_pos: Option<usize> = None;

    for (i, ch) in line_text[..=col.min(line_text.len().saturating_sub(1))]
        .char_indices()
        .rev()
    {
        if ch == '"' && !is_escaped(line_text, i) {
            opening_quote_pos = Some(i);
            break;
        }
    }

    if let Some(start_pos) = opening_quote_pos {
        // Find the matching closing quote after cursor, skipping escaped quotes
        let mut pos = col;
        let chars: Vec<char> = line_text.chars().collect();

        while pos < chars.len() {
            if chars[pos] == '"' && !is_escaped(line_text, pos) {
                // Found unescaped closing quote
                return Some((
                    line_text[start_pos..pos + 1].to_string(),
                    start_pos as u32,
                    (pos + 1) as u32,
                ));
            }
            pos += 1;
        }
    }

    None
}

/// Find boolean literal at cursor position
fn find_swift_boolean_literal(line_text: &str, col: usize) -> Option<(String, u32, u32)> {
    let keywords = ["true", "false"];

    for keyword in &keywords {
        // Try to match keyword at or near cursor
        for start in col.saturating_sub(keyword.len())..=col {
            if start + keyword.len() <= line_text.len() {
                if &line_text[start..start + keyword.len()] == *keyword {
                    // Check word boundaries
                    let before_ok = start == 0
                        || line_text[..start]
                            .chars()
                            .last()
                            .map_or(false, |c| !c.is_alphanumeric());
                    let after_ok = start + keyword.len() == line_text.len()
                        || line_text[start + keyword.len()..]
                            .chars()
                            .next()
                            .map_or(false, |c| !c.is_alphanumeric());

                    if before_ok && after_ok {
                        return Some((
                            keyword.to_string(),
                            start as u32,
                            (start + keyword.len()) as u32,
                        ));
                    }
                }
            }
        }
    }

    None
}

/// Validates whether a position in source code is a valid location for a literal.
/// A position is considered valid if it's not inside a string literal or comment.
///
// is_valid_swift_literal_location is now provided by mill_lang_common::is_valid_code_literal_location
fn is_valid_swift_literal_location(line: &str, pos: usize, len: usize) -> bool {
    is_valid_code_literal_location(line, pos, len)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // Extract Function Tests
    // ========================================================================

    #[test]
    fn test_extract_function_valid_single_line() {
        let source = r#"
func calculateTotal() {
    let price = 100
    let tax = price * 0.08
    return price + tax
}
"#;
        let result = plan_extract_function(source, 3, 3, "calculateTax", "test.swift");
        assert!(result.is_ok(), "Should extract single line successfully");
        let plan = result.unwrap();
        assert_eq!(plan.edits.len(), 2, "Should have insert and replace edits");
        assert!(plan.edits[0].new_text.contains("calculateTax"));
        assert!(plan.edits[0].new_text.contains("private func"));
    }

    #[test]
    fn test_extract_function_valid_multiline() {
        let source = r#"
func processData() {
    let input = getData()
    let normalized = normalize(input)
    let validated = validate(normalized)
    return validated
}
"#;
        let result = plan_extract_function(source, 2, 3, "processInput", "test.swift");
        assert!(result.is_ok(), "Should extract multiple lines");
        let plan = result.unwrap();
        assert_eq!(plan.edits.len(), 2);
        assert!(plan.edits[0].new_text.contains("processInput()"));
    }

    #[test]
    fn test_extract_function_invalid_range_start_after_end() {
        let source = "func test() { let x = 1 }";
        let result = plan_extract_function(source, 5, 2, "extracted", "test.swift");
        assert!(result.is_err(), "Should reject invalid range (start > end)");
    }

    #[test]
    fn test_extract_function_invalid_range_out_of_bounds() {
        let source = "func test() { let x = 1 }";
        let result = plan_extract_function(source, 0, 100, "extracted", "test.swift");
        assert!(result.is_err(), "Should reject out of bounds range");
    }

    #[test]
    fn test_extract_function_preserves_indentation() {
        let source = r#"
class Calculator {
    func compute() {
        let a = 10
        let b = 20
        return a + b
    }
}
"#;
        let result = plan_extract_function(source, 3, 4, "sum", "test.swift");
        assert!(result.is_ok());
        let plan = result.unwrap();
        // Check that indentation is preserved
        let insert_edit = &plan.edits[0];
        assert!(insert_edit.new_text.contains("    private func"), "Should preserve class indentation");
    }

    #[test]
    fn test_extract_function_edit_plan_structure() {
        let source = r#"
func example() {
    let x = 42
    print(x)
}
"#;
        let result = plan_extract_function(source, 2, 2, "initializeX", "test.swift");
        assert!(result.is_ok());
        let plan = result.unwrap();

        // Verify EditPlan structure
        assert_eq!(plan.metadata.intent_name, "extract_function");
        assert!(plan.edits.len() == 2);

        // First edit should be Insert (new function)
        assert_eq!(plan.edits[0].edit_type, EditType::Insert);
        assert_eq!(plan.edits[0].priority, 100);

        // Second edit should be Replace (call site)
        assert_eq!(plan.edits[1].edit_type, EditType::Replace);
        assert_eq!(plan.edits[1].priority, 90);
    }

    // ========================================================================
    // Extract Variable Tests
    // ========================================================================

    #[test]
    fn test_extract_variable_valid_simple_expression() {
        let source = r#"func calculate() {
    let total = 100 * 1.08
    return total
}"#;
        let result = plan_extract_variable(source, 1, 16, 1, 26, Some("taxRate".to_string()), "test.swift");
        assert!(result.is_ok(), "Should extract simple expression");
        let plan = result.unwrap();
        assert_eq!(plan.edits.len(), 2);
        assert!(plan.edits[0].new_text.contains("let taxRate = 100 * 1.08"));
        assert_eq!(plan.edits[1].new_text, "taxRate");
    }

    #[test]
    fn test_extract_variable_with_default_name() {
        let source = r#"
func test() {
    print(5 + 10)
}
"#;
        let result = plan_extract_variable(source, 2, 10, 2, 16, None, "test.swift");
        assert!(result.is_ok());
        let plan = result.unwrap();
        assert!(plan.edits[0].new_text.contains("let extractedVar"));
    }

    #[test]
    fn test_extract_variable_multiline_expression() {
        let source = r#"
func compute() {
    let result = calculateValue(
        param1: 10,
        param2: 20
    )
}
"#;
        let result = plan_extract_variable(source, 2, 17, 4, 5, Some("value".to_string()), "test.swift");
        assert!(result.is_ok(), "Should handle multiline expressions");
    }

    #[test]
    fn test_extract_variable_invalid_range() {
        let source = "let x = 5";
        // Out of bounds end column
        let result = plan_extract_variable(source, 0, 8, 0, 9, None, "test.swift");
        assert!(result.is_ok(), "Should handle valid range");

        // Test truly invalid range (end_line > lines.len())
        let result2 = plan_extract_variable(source, 0, 8, 100, 5, None, "test.swift");
        assert!(result2.is_err(), "Should reject out of bounds line");
    }

    #[test]
    fn test_extract_variable_empty_range() {
        let source = "let x = 42";
        let result = plan_extract_variable(source, 0, 5, 0, 5, None, "test.swift");
        assert!(result.is_ok(), "Should handle empty range");
        let plan = result.unwrap();
        assert_eq!(plan.edits[0].new_text.trim(), "let extractedVar =");
    }

    #[test]
    fn test_extract_variable_with_indentation() {
        let source = r#"
class Test {
    func method() {
        let x = 10 + 20
    }
}
"#;
        let result = plan_extract_variable(source, 3, 16, 3, 23, Some("sum".to_string()), "test.swift");
        assert!(result.is_ok());
        let plan = result.unwrap();
        // Should preserve indentation
        assert!(plan.edits[0].new_text.starts_with("        let sum"));
    }

    // ========================================================================
    // Inline Variable Tests
    // ========================================================================

    #[test]
    fn test_inline_variable_valid_let_declaration() {
        let source = r#"
let rate = 0.08
let tax = price * rate
let total = base * rate
"#;
        let result = plan_inline_variable(source, 1, 0, "test.swift");
        assert!(result.is_ok(), "Should inline variable successfully");
        let plan = result.unwrap();

        // Should have 2 replacements (for rate usages) + 1 delete (declaration)
        assert!(plan.edits.len() >= 2, "Should have at least 2 edits");

        // Check that we're replacing with the value
        let replace_edits: Vec<_> = plan.edits.iter()
            .filter(|e| e.edit_type == EditType::Replace)
            .collect();
        assert!(replace_edits.iter().any(|e| e.new_text.contains("0.08")));
    }

    #[test]
    fn test_inline_variable_var_declaration() {
        let source = r#"
var count = 5
let doubled = count * 2
"#;
        let result = plan_inline_variable(source, 1, 0, "test.swift");
        assert!(result.is_ok(), "Should handle var declarations");
    }

    #[test]
    fn test_inline_variable_single_usage() {
        let source = r#"
let temp = 42
print(temp)
"#;
        let result = plan_inline_variable(source, 1, 0, "test.swift");
        assert!(result.is_ok());
        let plan = result.unwrap();
        // Should have 1 replace + 1 delete
        assert!(plan.edits.len() >= 2);
    }

    #[test]
    fn test_inline_variable_no_usages() {
        let source = r#"
let unused = 100
let x = 50
"#;
        let result = plan_inline_variable(source, 1, 0, "test.swift");
        assert!(result.is_ok(), "Should handle unused variables");
        let plan = result.unwrap();
        // Should at least delete the declaration
        let delete_edits: Vec<_> = plan.edits.iter()
            .filter(|e| e.edit_type == EditType::Delete)
            .collect();
        assert_eq!(delete_edits.len(), 1);
    }

    #[test]
    fn test_inline_variable_invalid_line_number() {
        let source = "let x = 5";
        let result = plan_inline_variable(source, 100, 0, "test.swift");
        assert!(result.is_err(), "Should reject invalid line number");
    }

    #[test]
    fn test_inline_variable_not_a_declaration() {
        let source = r#"
func test() {
    print("hello")
}
"#;
        let result = plan_inline_variable(source, 2, 0, "test.swift");
        assert!(result.is_err(), "Should reject non-variable line");
    }

    // ========================================================================
    // Extract Constant Tests
    // ========================================================================

    #[test]
    fn test_extract_constant_numeric_literal() {
        let source = r#"
let price = 100
let tax = price * 0.08
let total = price + (price * 0.08)
"#;
        let result = plan_extract_constant(source, 2, 18, "TAX_RATE", "test.swift");
        assert!(result.is_ok(), "Should extract numeric constant");
        let plan = result.unwrap();

        // Should have 1 insert + 2 replacements (for two 0.08 occurrences)
        assert_eq!(plan.edits.len(), 3);
        assert!(plan.edits[0].new_text.contains("let TAX_RATE = 0.08"));
    }

    #[test]
    fn test_extract_constant_string_literal() {
        let source = r#"
let greeting = "Hello"
let message = "Hello"
"#;
        let result = plan_extract_constant(source, 1, 16, "GREETING", "test.swift");
        assert!(result.is_ok(), "Should extract string constant");
        let plan = result.unwrap();
        assert!(plan.edits[0].new_text.contains("let GREETING = \"Hello\""));
    }

    #[test]
    fn test_extract_constant_boolean_true() {
        let source = r#"
let debug = true
let verbose = true
"#;
        let result = plan_extract_constant(source, 1, 12, "DEBUG_MODE", "test.swift");
        assert!(result.is_ok(), "Should extract boolean constant");
        let plan = result.unwrap();
        assert!(plan.edits[0].new_text.contains("let DEBUG_MODE = true"));
    }

    #[test]
    fn test_extract_constant_boolean_false() {
        let source = r#"
let enabled = false
if enabled == false {
    print("disabled")
}
"#;
        let result = plan_extract_constant(source, 1, 14, "DISABLED", "test.swift");
        assert!(result.is_ok(), "Should extract false constant");
    }

    #[test]
    fn test_extract_constant_negative_number() {
        let source = r#"let offset = -10
let adjustment = -10"#;
        // Cursor on the digit part of the negative number (position 14 is on '1')
        let result = plan_extract_constant(source, 0, 14, "OFFSET_VALUE", "test.swift");
        assert!(result.is_ok(), "Should handle negative numbers: {:?}", result.err());

        let plan = result.unwrap();
        // Should find both -10 occurrences
        assert_eq!(plan.edits.len(), 3, "Should have 1 insert + 2 replace edits");
    }

    #[test]
    fn test_extract_constant_float_number() {
        let source = r#"
let pi = 3.14159
let circumference = diameter * 3.14159
"#;
        let result = plan_extract_constant(source, 1, 9, "PI", "test.swift");
        assert!(result.is_ok(), "Should handle float literals");
    }

    #[test]
    fn test_extract_constant_hex_literal() {
        let source = r#"
let color = 0xFF5733
let accent = 0xFF5733
"#;
        let result = plan_extract_constant(source, 1, 12, "BRAND_COLOR", "test.swift");
        assert!(result.is_ok(), "Should handle hex literals");
    }

    #[test]
    fn test_extract_constant_binary_literal() {
        let source = r#"
let mask = 0b1111
let flags = 0b1111
"#;
        let result = plan_extract_constant(source, 1, 11, "FULL_MASK", "test.swift");
        assert!(result.is_ok(), "Should handle binary literals");
    }

    #[test]
    fn test_extract_constant_octal_literal() {
        let source = r#"
let permissions = 0o755
let mode = 0o755
"#;
        let result = plan_extract_constant(source, 1, 18, "DEFAULT_PERMISSIONS", "test.swift");
        assert!(result.is_ok(), "Should handle octal literals");
    }

    #[test]
    fn test_extract_constant_scientific_notation() {
        let source = r#"
let small = 1.5e-10
let tiny = 1.5e-10
"#;
        let result = plan_extract_constant(source, 1, 12, "EPSILON", "test.swift");
        assert!(result.is_ok(), "Should handle scientific notation");
    }

    #[test]
    fn test_extract_constant_invalid_name_lowercase() {
        let source = "let x = 42";
        let result = plan_extract_constant(source, 0, 8, "badname", "test.swift");
        assert!(result.is_err(), "Should reject lowercase constant name");
    }

    #[test]
    fn test_extract_constant_invalid_name_mixed_case() {
        let source = "let x = 42";
        let result = plan_extract_constant(source, 0, 8, "BadName", "test.swift");
        assert!(result.is_err(), "Should reject mixed case constant name");
    }

    #[test]
    fn test_extract_constant_cursor_not_on_literal() {
        let source = "let myVariable = 42";
        let result = plan_extract_constant(source, 0, 4, "CONSTANT", "test.swift");
        assert!(result.is_err(), "Should fail when cursor not on literal");
    }

    #[test]
    fn test_extract_constant_skip_string_content() {
        let source = r#"
let rate = 0.08
let msg = "Rate is 0.08"
let tax = 0.08
"#;
        let result = plan_extract_constant(source, 1, 11, "TAX_RATE", "test.swift");
        assert!(result.is_ok());
        let plan = result.unwrap();
        // Should find 2 occurrences (lines 1 and 3), not the one inside the string
        assert_eq!(plan.edits.len(), 3, "Should have 1 insert + 2 replace edits (not the string content)");
    }

    #[test]
    fn test_extract_constant_skip_comment_content() {
        let source = r#"
let value = 42
// The answer is 42
let answer = 42
"#;
        let result = plan_extract_constant(source, 1, 12, "ANSWER", "test.swift");
        assert!(result.is_ok());
        let plan = result.unwrap();
        // Should find 2 occurrences (lines 1 and 3), not the one in comment
        assert_eq!(plan.edits.len(), 3, "Should have 1 insert + 2 replace edits (not the comment)");
    }

    #[test]
    fn test_extract_constant_escaped_quotes_in_string() {
        let source = r#"
let msg = "He said \"hello\""
let greeting = "hello"
"#;
        let result = plan_extract_constant(source, 2, 16, "GREETING_TEXT", "test.swift");
        assert!(result.is_ok(), "Should handle escaped quotes in strings");
    }

    // ========================================================================
    // Helper Function Tests
    // ========================================================================

    #[test]
    fn test_find_swift_numeric_literal_integer() {
        let line = "let x = 42";
        let result = find_swift_numeric_literal(line, 8);
        assert!(result.is_some());
        let (literal, start, end) = result.unwrap();
        assert_eq!(literal, "42");
        assert_eq!(start, 8);
        assert_eq!(end, 10);
    }

    #[test]
    fn test_find_swift_numeric_literal_negative() {
        let line = "let x = -100";
        let result = find_swift_numeric_literal(line, 9);
        assert!(result.is_some());
        let (literal, _start, _end) = result.unwrap();
        assert_eq!(literal, "-100");
    }

    #[test]
    fn test_find_swift_numeric_literal_float() {
        let line = "let pi = 3.14159";
        let result = find_swift_numeric_literal(line, 10);
        assert!(result.is_some());
        let (literal, _start, _end) = result.unwrap();
        assert_eq!(literal, "3.14159");
    }

    #[test]
    fn test_find_swift_string_literal_double_quotes() {
        let line = r#"let msg = "hello""#;
        let result = find_swift_string_literal(line, 11);
        assert!(result.is_some());
        let (literal, _start, _end) = result.unwrap();
        assert_eq!(literal, r#""hello""#);
    }

    #[test]
    fn test_find_swift_boolean_literal_true() {
        let line = "let flag = true";
        let result = find_swift_boolean_literal(line, 11);
        assert!(result.is_some());
        let (literal, _start, _end) = result.unwrap();
        assert_eq!(literal, "true");
    }

    #[test]
    fn test_find_swift_boolean_literal_false() {
        let line = "let disabled = false";
        let result = find_swift_boolean_literal(line, 15);
        assert!(result.is_some());
        let (literal, _start, _end) = result.unwrap();
        assert_eq!(literal, "false");
    }

    #[test]
    fn test_is_valid_number() {
        assert!(is_valid_number("42"));
        assert!(is_valid_number("-42"));
        assert!(is_valid_number("3.14"));
        assert!(is_valid_number("1e-5"));
        assert!(is_valid_number("2.5E10"));
        assert!(is_valid_number("0xFF"));
        assert!(is_valid_number("0b1010"));
        assert!(is_valid_number("0o777"));
        assert!(!is_valid_number(""));
        assert!(!is_valid_number("abc"));
        assert!(!is_valid_number("0x"));
    }
}
