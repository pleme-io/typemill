use lazy_static::lazy_static;
use mill_foundation::protocol::{
    EditPlan, EditPlanMetadata, EditType, TextEdit, ValidationRule, ValidationType,
};
use mill_lang_common::{
    find_literal_occurrences, is_escaped, is_valid_code_literal_location,
    ExtractConstantEditPlanBuilder, LineExtractor,
};
use mill_plugin_api::{PluginApiError, PluginResult};
use regex::Regex;

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

    Ok(EditPlan {
        source_file: file_path.to_string(),
        edits: vec![insert_edit, replace_edit],
        dependency_updates: vec![],
        validations: vec![ValidationRule {
            rule_type: ValidationType::SyntaxCheck,
            description: "Verify syntax after extraction".to_string(),
            parameters: Default::default(),
        }],
        metadata: EditPlanMetadata {
            intent_name: "extract_function".to_string(),
            intent_arguments: serde_json::json!({ "function_name": function_name }),
            created_at: chrono::Utc::now(),
            complexity: 3,
            impact_areas: vec!["function_extraction".to_string()],
            consolidation: None,
        },
    })
}

lazy_static! {
    static ref VAR_DECL_REGEX: Regex = Regex::new(r"^\s*(?:let|var)\s+([a-zA-Z0-9_]+)\s*=\s*(.*)")
        .expect("Invalid regex for Swift variable declaration");
}

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

    Ok(EditPlan {
        source_file: file_path.to_string(),
        edits,
        dependency_updates: vec![],
        validations: vec![ValidationRule {
            rule_type: ValidationType::SyntaxCheck,
            description: "Verify syntax is valid".to_string(),
            parameters: Default::default(),
        }],
        metadata: EditPlanMetadata {
            intent_name: "inline_variable".to_string(),
            intent_arguments: serde_json::json!({ "variable_name": var_name }),
            created_at: chrono::Utc::now(),
            complexity: 4,
            impact_areas: vec!["variable_inlining".to_string()],
            consolidation: None,
        },
    })
}

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

    Ok(EditPlan {
        source_file: file_path.to_string(),
        edits: vec![insert_edit, replace_edit],
        dependency_updates: vec![],
        validations: vec![ValidationRule {
            rule_type: ValidationType::SyntaxCheck,
            description: "Verify syntax after extraction".to_string(),
            parameters: Default::default(),
        }],
        metadata: EditPlanMetadata {
            intent_name: "extract_variable".to_string(),
            intent_arguments: serde_json::json!({
                "expression": expression_text,
                "variable_name": var_name,
            }),
            created_at: chrono::Utc::now(),
            complexity: 2,
            impact_areas: vec!["variable_extraction".to_string()],
            consolidation: None,
        },
    })
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
        .with_declaration_format(|name, value| {
            format!("let {} = {}\n", name, value)
        })
        .map_err(|e| PluginApiError::invalid_input(e))
}

/// Finds a Swift literal at a given position in a line of code.
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
                    if !before_sign.is_alphanumeric() && before_sign != '_' && before_sign != ')' && before_sign != ']' {
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
            while end < chars.len() && (chars[end] == '0' || chars[end] == '1' || chars[end] == '_') {
                end += 1;
            }
        } else if next == 'o' {
            // Octal: 0o77, 0o755
            end += 2;
            while end < chars.len() && ((chars[end] >= '0' && chars[end] <= '7') || chars[end] == '_') {
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

/// Helper to check if a character is part of a numeric literal
fn is_numeric_char(ch: Option<char>) -> bool {
    match ch {
        Some(c) => c.is_ascii_digit() || c == '.' || c == '_',
        None => false,
    }
}

/// Scans forward from a position to find the end of a regular number (not hex/binary/octal)
/// Handles: integers, floats, scientific notation (e.g., 1.5e-10, 2E+5)
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

/// Validates that a string represents a valid Swift number
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

    for (i, ch) in line_text[..=col.min(line_text.len().saturating_sub(1))].char_indices().rev() {
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
                return Some((line_text[start_pos..pos + 1].to_string(), start_pos as u32, (pos + 1) as u32));
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
