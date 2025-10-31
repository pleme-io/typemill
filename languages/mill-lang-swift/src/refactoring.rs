use mill_foundation::protocol::{
    EditPlan, EditPlanMetadata, EditType, TextEdit, ValidationRule, ValidationType,
};
use lazy_static::lazy_static;
use mill_plugin_api::{PluginError, PluginResult};
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
        return Err(PluginError::invalid_input("Invalid line range"));
    }

    let selected_lines = &lines[start_line as usize..=end_line as usize];
    let selected_text = selected_lines.join("\n");

    // Find the indentation of the first line of the selection
    let indent = selected_lines[0].chars().take_while(|c| c.is_whitespace()).collect::<String>();

    let new_function_text = format!(
        "\n\n{}private func {}() {{\n{}\n{}}}\n",
        indent,
        function_name,
        selected_text,
        indent
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
    static ref VAR_DECL_REGEX: Regex =
        Regex::new(r"^\s*(?:let|var)\s+([a-zA-Z0-9_]+)\s*=\s*(.*)")
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
        return Err(PluginError::invalid_input("Invalid line number"));
    }

    let line_content = lines[variable_line as usize];
    let caps = VAR_DECL_REGEX
        .captures(line_content)
        .ok_or_else(|| PluginError::invalid_input("Line is not a variable declaration"))?;
    let var_name = &caps[1];
    let var_value = caps[2].trim().trim_end_matches(';').to_string();

    let mut edits = Vec::new();
    let search_pattern = format!(r"\b{}\b", var_name);
    let search_re = Regex::new(&search_pattern)
        .map_err(|e| PluginError::internal(format!("Invalid regex: {}", e)))?;

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
        return Err(PluginError::invalid_input("Invalid line range"));
    }

    let expression_text = if start_line == end_line {
        lines[start_line as usize][start_col as usize..end_col as usize].to_string()
    } else {
        // A rough approximation for multi-line expressions
        let mut text = String::new();
        text.push_str(&lines[start_line as usize][start_col as usize..]);
        for line in lines.iter().take(end_line as usize).skip((start_line + 1) as usize) {
            text.push_str(line);
        }
        text.push_str(&lines[end_line as usize][..end_col as usize]);
        text
    };

    let var_name = variable_name.unwrap_or_else(|| "extractedVar".to_string());
    let declaration_text = format!("let {} = {}", var_name, expression_text);

    // Find indentation
    let indent = lines[start_line as usize].chars().take_while(|c| c.is_whitespace()).collect::<String>();

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