use lazy_static::lazy_static;
use mill_foundation::protocol::{
    EditPlan, EditPlanMetadata, EditType, TextEdit, ValidationRule, ValidationType,
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
    let indent = lines[start_line as usize]
        .chars()
        .take_while(|c| c.is_whitespace())
        .collect::<String>();

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
    // Validate constant name follows SCREAMING_SNAKE_CASE
    if !is_screaming_snake_case(name) {
        return Err(PluginApiError::invalid_input(format!(
            "Constant name '{}' must be in SCREAMING_SNAKE_CASE format. Valid examples: TAX_RATE, MAX_VALUE, API_KEY. Requirements: only uppercase letters (A-Z), digits (0-9), and underscores; must contain at least one uppercase letter; cannot start or end with underscore.",
            name
        )));
    }

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
    let occurrence_ranges = find_swift_literal_occurrences(source, &literal_value);

    if occurrence_ranges.is_empty() {
        return Err(PluginApiError::invalid_input(
            "No occurrences of the literal found",
        ));
    }

    let mut edits = Vec::new();

    // Generate the constant declaration at the top of the file
    let declaration = format!("let {} = {}\n", name, literal_value);
    edits.push(TextEdit {
        file_path: Some(file_path.to_string()),
        edit_type: EditType::Insert,
        location: mill_foundation::protocol::EditLocation {
            start_line: 0,
            start_column: 0,
            end_line: 0,
            end_column: 0,
        },
        original_text: String::new(),
        new_text: declaration,
        priority: 100,
        description: format!("Extract '{}' into constant '{}'", literal_value, name),
    });

    // Replace all occurrences of the literal with the constant name
    for (idx, (start_line, start_col, end_col)) in occurrence_ranges.iter().enumerate() {
        let priority = 90_u32.saturating_sub(idx as u32);
        edits.push(TextEdit {
            file_path: Some(file_path.to_string()),
            edit_type: EditType::Replace,
            location: mill_foundation::protocol::EditLocation {
                start_line: *start_line,
                start_column: *start_col,
                end_line: *start_line,
                end_column: *end_col,
            },
            original_text: literal_value.clone(),
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
        dependency_updates: vec![],
        validations: vec![ValidationRule {
            rule_type: ValidationType::SyntaxCheck,
            description: "Verify syntax is valid after constant extraction".to_string(),
            parameters: Default::default(),
        }],
        metadata: EditPlanMetadata {
            intent_name: "extract_constant".to_string(),
            intent_arguments: serde_json::json!({
                "literal": literal_value,
                "constantName": name,
                "occurrences": occurrence_ranges.len(),
            }),
            created_at: chrono::Utc::now(),
            complexity: (occurrence_ranges.len().min(10)) as u8,
            impact_areas: vec!["constant_extraction".to_string()],
            consolidation: None,
        },
    })
}

/// Validates that a constant name follows the SCREAMING_SNAKE_CASE convention.
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
fn find_swift_numeric_literal(line_text: &str, col: usize) -> Option<(String, u32, u32)> {
    if col >= line_text.len() {
        return None;
    }

    // Find the start of the number (including negative sign)
    let mut start = col;
    while start > 0 {
        let ch = line_text.chars().nth(start - 1)?;
        if ch.is_ascii_digit() || ch == '.' || (start == col && ch == '-') {
            start -= 1;
        } else {
            break;
        }
    }

    // Find the end of the number
    let mut end = col;
    while end < line_text.len() {
        let ch = line_text.chars().nth(end)?;
        if ch.is_ascii_digit() || ch == '.' {
            end += 1;
        } else {
            break;
        }
    }

    if start < end && end <= line_text.len() {
        let text = &line_text[start..end];
        if text.chars().any(|c| c.is_ascii_digit()) && !text.starts_with('.') && !text.ends_with('.') {
            return Some((text.to_string(), start as u32, end as u32));
        }
    }

    None
}

/// Find string literal at cursor position
fn find_swift_string_literal(line_text: &str, col: usize) -> Option<(String, u32, u32)> {
    if col >= line_text.len() {
        return None;
    }

    // Look for opening quote before or at cursor
    for (i, ch) in line_text[..=col].char_indices().rev() {
        if ch == '"' {
            // Find closing quote after cursor
            for (j, ch2) in line_text[col..].char_indices() {
                if ch2 == '"' && col + j > i {
                    let end = col + j + 1;
                    return Some((line_text[i..end].to_string(), i as u32, end as u32));
                }
            }
            break;
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
                        || !line_text[..start]
                            .chars()
                            .last()
                            .unwrap()
                            .is_alphanumeric();
                    let after_ok = start + keyword.len() == line_text.len()
                        || !line_text[start + keyword.len()..]
                            .chars()
                            .next()
                            .unwrap()
                            .is_alphanumeric();

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

/// Finds all valid occurrences of a literal value in Swift source code.
fn find_swift_literal_occurrences(source: &str, literal_value: &str) -> Vec<(u32, u32, u32)> {
    let mut occurrences = Vec::new();
    let lines: Vec<&str> = source.lines().collect();

    for (line_idx, line_text) in lines.iter().enumerate() {
        let mut start_pos = 0;
        while let Some(pos) = line_text[start_pos..].find(literal_value) {
            let col = start_pos + pos;

            // Validate that this match is not inside a string literal or comment
            if is_valid_swift_literal_location(line_text, col, literal_value.len()) {
                occurrences.push((
                    line_idx as u32,
                    col as u32,
                    (col + literal_value.len()) as u32,
                ));
            }

            start_pos = col + 1;
        }
    }

    occurrences
}

/// Validates whether a position in source code is a valid location for a literal.
fn is_valid_swift_literal_location(line: &str, pos: usize, _len: usize) -> bool {
    // Count quotes before position to determine if we're inside a string literal
    let before = &line[..pos];
    let double_quotes = before.matches('"').count();

    // If odd number of quotes, we're inside a string
    if double_quotes % 2 == 1 {
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
