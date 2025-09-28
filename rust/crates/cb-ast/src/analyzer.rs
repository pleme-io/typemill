//! AST analysis functionality

use crate::error::{AstError, AstResult};
use cb_core::model::IntentSpec;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Edit plan for code transformations
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EditPlan {
    /// Source file being edited
    pub source_file: String,
    /// List of individual edits to apply
    pub edits: Vec<TextEdit>,
    /// Dependencies that need to be updated
    pub dependency_updates: Vec<DependencyUpdate>,
    /// Validation rules to check after editing
    pub validations: Vec<ValidationRule>,
    /// Plan metadata
    pub metadata: EditPlanMetadata,
}

/// Individual text edit operation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TextEdit {
    /// Edit type classification
    pub edit_type: EditType,
    /// Location of the edit
    pub location: EditLocation,
    /// Original text to be replaced
    pub original_text: String,
    /// New text to insert
    pub new_text: String,
    /// Edit priority (higher numbers applied first)
    pub priority: u32,
    /// Description of what this edit does
    pub description: String,
}

/// Types of edits that can be performed
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum EditType {
    /// Rename identifier
    Rename,
    /// Add new import
    AddImport,
    /// Remove import
    RemoveImport,
    /// Update import path
    UpdateImport,
    /// Add new code
    Insert,
    /// Remove code
    Delete,
    /// Replace code
    Replace,
    /// Reformat code
    Format,
}

/// Location of an edit in the source file
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EditLocation {
    /// Start line (0-based)
    pub start_line: u32,
    /// Start column (0-based)
    pub start_column: u32,
    /// End line (0-based)
    pub end_line: u32,
    /// End column (0-based)
    pub end_column: u32,
}

/// Dependency update information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DependencyUpdate {
    /// File whose imports need updating
    pub target_file: String,
    /// Type of update needed
    pub update_type: DependencyUpdateType,
    /// Old import path/name
    pub old_reference: String,
    /// New import path/name
    pub new_reference: String,
}

/// Types of dependency updates
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum DependencyUpdateType {
    /// Update import path
    ImportPath,
    /// Update import name
    ImportName,
    /// Update export reference
    ExportReference,
}

/// Validation rule to check after editing
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ValidationRule {
    /// Rule type
    pub rule_type: ValidationType,
    /// Rule description
    pub description: String,
    /// Parameters for the validation
    pub parameters: HashMap<String, serde_json::Value>,
}

/// Types of validation that can be performed
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ValidationType {
    /// Check syntax is valid
    SyntaxCheck,
    /// Check imports resolve
    ImportResolution,
    /// Check types are correct
    TypeCheck,
    /// Check tests still pass
    TestValidation,
    /// Check formatting is correct
    FormatValidation,
}

/// Edit plan metadata
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EditPlanMetadata {
    /// Intent that generated this plan
    pub intent_name: String,
    /// Intent arguments used
    pub intent_arguments: serde_json::Value,
    /// Plan creation timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Estimated complexity (1-10)
    pub complexity: u8,
    /// Expected impact areas
    pub impact_areas: Vec<String>,
}

/// Plan a refactoring operation based on an intent
pub fn plan_refactor(intent: &IntentSpec, source: &str) -> AstResult<EditPlan> {
    match intent.name() {
        "rename_symbol" => plan_rename_symbol(intent, source),
        "add_import" => plan_add_import(intent, source),
        "remove_import" => plan_remove_import(intent, source),
        "update_import_path" => plan_update_import_path(intent, source),
        "extract_function" => plan_extract_function(intent, source),
        "inline_function" => plan_inline_function(intent, source),
        _ => Err(AstError::unsupported_syntax(format!("Intent: {}", intent.name()))),
    }
}

/// Plan a symbol rename operation
fn plan_rename_symbol(intent: &IntentSpec, source: &str) -> AstResult<EditPlan> {
    let old_name = intent.arguments().get("oldName")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AstError::analysis("Missing oldName parameter"))?;

    let new_name = intent.arguments().get("newName")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AstError::analysis("Missing newName parameter"))?;

    let source_file = intent.arguments().get("sourceFile")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    // Find all occurrences of the old name (simplified implementation)
    let mut edits = Vec::new();
    let mut priority = 100;

    for (line_num, line) in source.lines().enumerate() {
        let mut column = 0;
        while let Some(pos) = line[column..].find(old_name) {
            let actual_pos = column + pos;

            // Check if this is a word boundary (simplified check)
            let is_word_boundary = (actual_pos == 0 || !line.chars().nth(actual_pos - 1).unwrap_or(' ').is_alphanumeric())
                && (actual_pos + old_name.len() >= line.len() || !line.chars().nth(actual_pos + old_name.len()).unwrap_or(' ').is_alphanumeric());

            if is_word_boundary {
                edits.push(TextEdit {
                    edit_type: EditType::Rename,
                    location: EditLocation {
                        start_line: line_num as u32,
                        start_column: actual_pos as u32,
                        end_line: line_num as u32,
                        end_column: (actual_pos + old_name.len()) as u32,
                    },
                    original_text: old_name.to_string(),
                    new_text: new_name.to_string(),
                    priority,
                    description: format!("Rename '{}' to '{}'", old_name, new_name),
                });
                priority -= 1; // Process in order found
            }

            column = actual_pos + old_name.len();
        }
    }

    Ok(EditPlan {
        source_file: source_file.to_string(),
        edits,
        dependency_updates: Vec::new(), // Would analyze cross-file dependencies
        validations: vec![
            ValidationRule {
                rule_type: ValidationType::SyntaxCheck,
                description: "Verify syntax is still valid after rename".to_string(),
                parameters: HashMap::new(),
            },
            ValidationRule {
                rule_type: ValidationType::TypeCheck,
                description: "Verify types are still correct after rename".to_string(),
                parameters: HashMap::new(),
            },
        ],
        metadata: EditPlanMetadata {
            intent_name: intent.name().to_string(),
            intent_arguments: intent.arguments().clone(),
            created_at: chrono::Utc::now(),
            complexity: 3, // Moderate complexity
            impact_areas: vec!["identifiers".to_string(), "references".to_string()],
        },
    })
}

/// Plan an add import operation
fn plan_add_import(intent: &IntentSpec, source: &str) -> AstResult<EditPlan> {
    let module_path = intent.arguments().get("modulePath")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AstError::analysis("Missing modulePath parameter"))?;

    let import_name = intent.arguments().get("importName")
        .and_then(|v| v.as_str());

    let is_default = intent.arguments().get("isDefault")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let source_file = intent.arguments().get("sourceFile")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    // Find the best location to insert the import (simplified)
    let insert_location = find_import_insertion_point(source)?;

    let import_text = if is_default {
        format!("import {} from '{}';", import_name.unwrap_or("defaultImport"), module_path)
    } else if let Some(name) = import_name {
        format!("import {{ {} }} from '{}';", name, module_path)
    } else {
        format!("import '{}';", module_path)
    };

    let edit = TextEdit {
        edit_type: EditType::AddImport,
        location: insert_location,
        original_text: String::new(),
        new_text: format!("{}\n", import_text),
        priority: 100,
        description: format!("Add import from '{}'", module_path),
    };

    Ok(EditPlan {
        source_file: source_file.to_string(),
        edits: vec![edit],
        dependency_updates: Vec::new(),
        validations: vec![
            ValidationRule {
                rule_type: ValidationType::ImportResolution,
                description: "Verify import resolves correctly".to_string(),
                parameters: HashMap::from([
                    ("module_path".to_string(), serde_json::Value::String(module_path.to_string())),
                ]),
            },
        ],
        metadata: EditPlanMetadata {
            intent_name: intent.name().to_string(),
            intent_arguments: intent.arguments().clone(),
            created_at: chrono::Utc::now(),
            complexity: 2, // Low complexity
            impact_areas: vec!["imports".to_string()],
        },
    })
}

/// Find the appropriate location to insert new imports
fn find_import_insertion_point(source: &str) -> AstResult<EditLocation> {
    // Find the last import statement or the beginning of the file
    let mut last_import_line = 0;

    for (line_num, line) in source.lines().enumerate() {
        let line = line.trim();
        if line.starts_with("import ") || line.starts_with("const ") && line.contains("require(") {
            last_import_line = line_num + 1; // Insert after this line
        } else if !line.is_empty() && !line.starts_with("//") && !line.starts_with("/*") {
            // Hit non-import, non-comment code
            break;
        }
    }

    Ok(EditLocation {
        start_line: last_import_line as u32,
        start_column: 0,
        end_line: last_import_line as u32,
        end_column: 0,
    })
}

/// Plan a remove import operation
fn plan_remove_import(intent: &IntentSpec, source: &str) -> AstResult<EditPlan> {
    let module_path = intent.arguments().get("modulePath")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AstError::analysis("Missing modulePath parameter"))?;

    let import_name = intent.arguments().get("importName")
        .and_then(|v| v.as_str());

    let source_file = intent.arguments().get("sourceFile")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    let mut edits = Vec::new();

    // Find import statements to remove
    for (line_num, line) in source.lines().enumerate() {
        let line_trimmed = line.trim();

        // Check for ES module imports
        if line_trimmed.starts_with("import ") && line_trimmed.contains(&format!("'{}'", module_path)) {
            if let Some(import_name) = import_name {
                // Remove specific named import
                if line_trimmed.contains(&format!("{{ {}", import_name)) ||
                   line_trimmed.contains(&format!("{} }}", import_name)) ||
                   line_trimmed.contains(&format!(" {} ", import_name)) {

                    if line_trimmed.matches(',').count() > 0 {
                        // Multiple imports - remove just this one
                        let new_line = remove_named_import_from_line(line, import_name);
                        if new_line != line {
                            edits.push(TextEdit {
                                edit_type: EditType::UpdateImport,
                                location: EditLocation {
                                    start_line: line_num as u32,
                                    start_column: 0,
                                    end_line: line_num as u32,
                                    end_column: line.len() as u32,
                                },
                                original_text: line.to_string(),
                                new_text: new_line,
                                priority: 100,
                                description: format!("Remove '{}' from import", import_name),
                            });
                        }
                    } else {
                        // Single import - remove entire line
                        edits.push(TextEdit {
                            edit_type: EditType::RemoveImport,
                            location: EditLocation {
                                start_line: line_num as u32,
                                start_column: 0,
                                end_line: line_num as u32,
                                end_column: line.len() as u32,
                            },
                            original_text: line.to_string(),
                            new_text: String::new(),
                            priority: 100,
                            description: format!("Remove import from '{}'", module_path),
                        });
                    }
                }
            } else {
                // Remove entire import
                edits.push(TextEdit {
                    edit_type: EditType::RemoveImport,
                    location: EditLocation {
                        start_line: line_num as u32,
                        start_column: 0,
                        end_line: line_num as u32,
                        end_column: line.len() as u32,
                    },
                    original_text: line.to_string(),
                    new_text: String::new(),
                    priority: 100,
                    description: format!("Remove import from '{}'", module_path),
                });
            }
        }

        // Check for CommonJS requires
        else if line_trimmed.contains("require(") && line_trimmed.contains(&format!("'{}'", module_path)) {
            edits.push(TextEdit {
                edit_type: EditType::RemoveImport,
                location: EditLocation {
                    start_line: line_num as u32,
                    start_column: 0,
                    end_line: line_num as u32,
                    end_column: line.len() as u32,
                },
                original_text: line.to_string(),
                new_text: String::new(),
                priority: 100,
                description: format!("Remove require from '{}'", module_path),
            });
        }
    }

    Ok(EditPlan {
        source_file: source_file.to_string(),
        edits,
        dependency_updates: Vec::new(),
        validations: vec![
            ValidationRule {
                rule_type: ValidationType::SyntaxCheck,
                description: "Verify syntax is still valid after import removal".to_string(),
                parameters: HashMap::new(),
            },
        ],
        metadata: EditPlanMetadata {
            intent_name: intent.name().to_string(),
            intent_arguments: intent.arguments().clone(),
            created_at: chrono::Utc::now(),
            complexity: 2,
            impact_areas: vec!["imports".to_string()],
        },
    })
}

/// Plan an update import path operation
fn plan_update_import_path(intent: &IntentSpec, source: &str) -> AstResult<EditPlan> {
    let old_path = intent.arguments().get("oldPath")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AstError::analysis("Missing oldPath parameter"))?;

    let new_path = intent.arguments().get("newPath")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AstError::analysis("Missing newPath parameter"))?;

    let source_file = intent.arguments().get("sourceFile")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    let mut edits = Vec::new();

    // Find import statements with the old path
    for (line_num, line) in source.lines().enumerate() {
        let line_trimmed = line.trim();

        if (line_trimmed.starts_with("import ") || line_trimmed.contains("require(")) &&
           (line_trimmed.contains(&format!("'{}'", old_path)) || line_trimmed.contains(&format!("\"{}\"", old_path))) {

            let new_line = line.replace(&format!("'{}'", old_path), &format!("'{}'", new_path))
                               .replace(&format!("\"{}\"", old_path), &format!("\"{}\"", new_path));

            if new_line != line {
                edits.push(TextEdit {
                    edit_type: EditType::UpdateImport,
                    location: EditLocation {
                        start_line: line_num as u32,
                        start_column: 0,
                        end_line: line_num as u32,
                        end_column: line.len() as u32,
                    },
                    original_text: line.to_string(),
                    new_text: new_line,
                    priority: 100,
                    description: format!("Update import path from '{}' to '{}'", old_path, new_path),
                });
            }
        }
    }

    Ok(EditPlan {
        source_file: source_file.to_string(),
        edits,
        dependency_updates: Vec::new(),
        validations: vec![
            ValidationRule {
                rule_type: ValidationType::ImportResolution,
                description: "Verify new import path resolves correctly".to_string(),
                parameters: HashMap::from([
                    ("new_path".to_string(), serde_json::Value::String(new_path.to_string())),
                ]),
            },
        ],
        metadata: EditPlanMetadata {
            intent_name: intent.name().to_string(),
            intent_arguments: intent.arguments().clone(),
            created_at: chrono::Utc::now(),
            complexity: 3,
            impact_areas: vec!["imports".to_string(), "dependencies".to_string()],
        },
    })
}

/// Plan an extract function operation
fn plan_extract_function(intent: &IntentSpec, source: &str) -> AstResult<EditPlan> {
    let function_name = intent.arguments().get("functionName")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AstError::analysis("Missing functionName parameter"))?;

    let start_line = intent.arguments().get("startLine")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| AstError::analysis("Missing startLine parameter"))? as u32;

    let end_line = intent.arguments().get("endLine")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| AstError::analysis("Missing endLine parameter"))? as u32;

    let source_file = intent.arguments().get("sourceFile")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    let lines: Vec<&str> = source.lines().collect();

    if start_line as usize >= lines.len() || end_line as usize >= lines.len() || start_line > end_line {
        return Err(AstError::analysis("Invalid line range for function extraction"));
    }

    // Extract the selected lines
    let extracted_lines = &lines[start_line as usize..=end_line as usize];
    let extracted_code = extracted_lines.join("\n");

    // Analyze variables used in the extracted code
    let (parameters, return_vars) = analyze_function_variables(&extracted_code, source, start_line, end_line)?;

    // Create the new function
    let mut function_def = format!("function {}(", function_name);
    if !parameters.is_empty() {
        function_def.push_str(&parameters.join(", "));
    }
    function_def.push_str(") {\n");

    // Indent extracted code
    for line in extracted_lines {
        function_def.push_str(&format!("  {}\n", line));
    }

    // Add return statement if needed
    if !return_vars.is_empty() {
        function_def.push_str(&format!("  return {};\n", return_vars.join(", ")));
    }

    function_def.push_str("}\n\n");

    let mut edits = Vec::new();

    // 1. Replace extracted code with function call
    let function_call = if return_vars.is_empty() {
        format!("{}({});", function_name, parameters.join(", "))
    } else if return_vars.len() == 1 {
        format!("const {} = {}({});", return_vars[0], function_name, parameters.join(", "))
    } else {
        format!("const [{}] = {}({});", return_vars.join(", "), function_name, parameters.join(", "))
    };

    edits.push(TextEdit {
        edit_type: EditType::Replace,
        location: EditLocation {
            start_line,
            start_column: 0,
            end_line,
            end_column: lines[end_line as usize].len() as u32,
        },
        original_text: extracted_code,
        new_text: function_call,
        priority: 90,
        description: format!("Replace extracted code with call to {}", function_name),
    });

    // 2. Insert new function definition
    let insertion_point = find_function_insertion_point(source, start_line)?;
    edits.push(TextEdit {
        edit_type: EditType::Insert,
        location: insertion_point,
        original_text: String::new(),
        new_text: function_def,
        priority: 100,
        description: format!("Insert new function {}", function_name),
    });

    Ok(EditPlan {
        source_file: source_file.to_string(),
        edits,
        dependency_updates: Vec::new(),
        validations: vec![
            ValidationRule {
                rule_type: ValidationType::SyntaxCheck,
                description: "Verify syntax is still valid after function extraction".to_string(),
                parameters: HashMap::new(),
            },
            ValidationRule {
                rule_type: ValidationType::TestValidation,
                description: "Verify functionality is preserved after extraction".to_string(),
                parameters: HashMap::new(),
            },
        ],
        metadata: EditPlanMetadata {
            intent_name: intent.name().to_string(),
            intent_arguments: intent.arguments().clone(),
            created_at: chrono::Utc::now(),
            complexity: 8, // High complexity
            impact_areas: vec!["functions".to_string(), "code_structure".to_string()],
        },
    })
}

/// Plan an inline function operation
fn plan_inline_function(intent: &IntentSpec, source: &str) -> AstResult<EditPlan> {
    let function_name = intent.arguments().get("functionName")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AstError::analysis("Missing functionName parameter"))?;

    let source_file = intent.arguments().get("sourceFile")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    // Find the function definition
    let function_info = find_function_definition(source, function_name)?;

    // Find all function calls
    let function_calls = find_function_calls(source, function_name)?;

    if function_calls.is_empty() {
        return Err(AstError::analysis("No function calls found to inline"));
    }

    let mut edits = Vec::new();

    // Replace each function call with the function body
    for call in function_calls.iter().rev() { // Process in reverse order to avoid offset issues
        let inlined_code = inline_function_call(&function_info, call)?;

        edits.push(TextEdit {
            edit_type: EditType::Replace,
            location: call.location.clone(),
            original_text: call.call_text.clone(),
            new_text: inlined_code,
            priority: 90,
            description: format!("Inline call to {}", function_name),
        });
    }

    // Remove the original function definition
    edits.push(TextEdit {
        edit_type: EditType::Delete,
        location: function_info.location.clone(),
        original_text: function_info.function_text.clone(),
        new_text: String::new(),
        priority: 100,
        description: format!("Remove function definition for {}", function_name),
    });

    Ok(EditPlan {
        source_file: source_file.to_string(),
        edits,
        dependency_updates: Vec::new(),
        validations: vec![
            ValidationRule {
                rule_type: ValidationType::SyntaxCheck,
                description: "Verify syntax is still valid after function inlining".to_string(),
                parameters: HashMap::new(),
            },
            ValidationRule {
                rule_type: ValidationType::TestValidation,
                description: "Verify functionality is preserved after inlining".to_string(),
                parameters: HashMap::new(),
            },
        ],
        metadata: EditPlanMetadata {
            intent_name: intent.name().to_string(),
            intent_arguments: intent.arguments().clone(),
            created_at: chrono::Utc::now(),
            complexity: 9, // Very high complexity
            impact_areas: vec!["functions".to_string(), "code_structure".to_string(), "refactoring".to_string()],
        },
    })
}

// Helper functions for refactoring operations

/// Remove a named import from an import line
fn remove_named_import_from_line(line: &str, import_name: &str) -> String {
    // This is a simplified implementation
    line.replace(&format!("{}, ", import_name), "")
        .replace(&format!(", {}", import_name), "")
        .replace(&format!("{{ {} }}", import_name), "")
        .replace(import_name, "")
}

/// Analyze variables for function extraction
fn analyze_function_variables(extracted_code: &str, _full_source: &str, _start_line: u32, _end_line: u32) -> AstResult<(Vec<String>, Vec<String>)> {
    // Simplified analysis - in practice, you'd use a proper AST
    let parameters = Vec::new();
    let mut return_vars = Vec::new();

    // Very basic analysis looking for variable usage patterns
    for line in extracted_code.lines() {
        if line.contains("let ") || line.contains("const ") || line.contains("var ") {
            // Extract variable declarations that might need to be returned
            if let Some(var_name) = extract_declared_variable(line) {
                return_vars.push(var_name);
            }
        }
    }

    // This would need much more sophisticated analysis in practice
    Ok((parameters, return_vars))
}

/// Extract declared variable name from a line
fn extract_declared_variable(line: &str) -> Option<String> {
    let line = line.trim();
    for keyword in &["let ", "const ", "var "] {
        if line.starts_with(keyword) {
            let after_keyword = &line[keyword.len()..];
            if let Some(eq_pos) = after_keyword.find('=') {
                let var_name = after_keyword[..eq_pos].trim();
                return Some(var_name.to_string());
            }
        }
    }
    None
}

/// Find insertion point for a new function
fn find_function_insertion_point(_source: &str, _near_line: u32) -> AstResult<EditLocation> {
    // Find a good place to insert the function - typically before the function that contains the extracted code
    // For now, just insert at the beginning of the file
    Ok(EditLocation {
        start_line: 0,
        start_column: 0,
        end_line: 0,
        end_column: 0,
    })
}

/// Information about a function definition
#[derive(Debug, Clone)]
struct FunctionInfo {
    name: String,
    // TODO: Implement proper parameter extraction from function signatures
    // Currently hardcoded to Vec::new() - needs AST parsing for full implementation
    parameters: Vec<String>,
    body: String,
    location: EditLocation,
    function_text: String,
}

/// Information about a function call
#[derive(Debug, Clone)]
struct FunctionCall {
    name: String,
    // TODO: Implement proper argument extraction from function calls
    // Currently not populated - needs AST parsing for function call analysis
    arguments: Vec<String>,
    location: EditLocation,
    call_text: String,
}

/// Find function definition in source code
fn find_function_definition(source: &str, function_name: &str) -> AstResult<FunctionInfo> {
    // Simplified function finder - in practice, you'd use proper AST parsing
    for (line_num, line) in source.lines().enumerate() {
        if line.contains(&format!("function {}", function_name)) ||
           line.contains(&format!("const {} =", function_name)) ||
           line.contains(&format!("let {} =", function_name)) {

            return Ok(FunctionInfo {
                name: function_name.to_string(),
                parameters: Vec::new(), // Would extract from actual function signature
                body: "// Function body would be extracted here".to_string(),
                location: EditLocation {
                    start_line: line_num as u32,
                    start_column: 0,
                    end_line: line_num as u32,
                    end_column: line.len() as u32,
                },
                function_text: line.to_string(),
            });
        }
    }

    Err(AstError::analysis(format!("Function '{}' not found", function_name)))
}

/// Find all calls to a function
fn find_function_calls(source: &str, function_name: &str) -> AstResult<Vec<FunctionCall>> {
    let mut calls = Vec::new();

    for (line_num, line) in source.lines().enumerate() {
        if line.contains(&format!("{}(", function_name)) {
            calls.push(FunctionCall {
                name: function_name.to_string(),
                arguments: Vec::new(), // Would extract actual arguments
                location: EditLocation {
                    start_line: line_num as u32,
                    start_column: 0,
                    end_line: line_num as u32,
                    end_column: line.len() as u32,
                },
                call_text: line.to_string(),
            });
        }
    }

    Ok(calls)
}

/// Inline a function call with the function body
fn inline_function_call(function_info: &FunctionInfo, _call: &FunctionCall) -> AstResult<String> {
    // Simplified inlining - replace the call with the function body
    // In practice, you'd need to handle parameter substitution, variable scoping, etc.
    Ok(format!("{{ {} }}", function_info.body))
}