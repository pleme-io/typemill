//! AST analysis functionality

use crate::error::{AstError, AstResult};
use cb_core::model::IntentSpec;
use cb_protocol::{
    EditLocation, EditPlan, EditPlanMetadata, EditType, TextEdit, ValidationRule, ValidationType,
};
// serde traits no longer needed here
use std::collections::HashMap;
use swc_common::{sync::Lrc, SourceMap};
use swc_ecma_codegen::{text_writer::JsWriter, Emitter};
use swc_ecma_parser::{lexer::Lexer, Parser, StringInput, Syntax, TsSyntax};

// Edit plan types now come from cb-api

/// Plan a refactoring operation based on an intent
pub fn plan_refactor(intent: &IntentSpec, source: &str) -> AstResult<EditPlan> {
    match intent.name() {
        "rename_symbol" => plan_rename_symbol(intent, source),
        "add_import" => plan_add_import(intent, source),
        "remove_import" => plan_remove_import(intent, source),
        "update_import_path" => plan_update_import_path(intent, source),
        "extract_function" => plan_extract_function(intent, source),
        "inline_function" => plan_inline_function(intent, source),
        _ => Err(AstError::unsupported_syntax(format!(
            "Intent: {}",
            intent.name()
        ))),
    }
}

/// Plan a symbol rename operation
fn plan_rename_symbol(intent: &IntentSpec, source: &str) -> AstResult<EditPlan> {
    let old_name = intent
        .arguments()
        .get("oldName")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AstError::analysis("Missing oldName parameter"))?;

    let new_name = intent
        .arguments()
        .get("newName")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AstError::analysis("Missing newName parameter"))?;

    let source_file = intent
        .arguments()
        .get("sourceFile")
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
            let is_word_boundary = (actual_pos == 0
                || !line
                    .chars()
                    .nth(actual_pos - 1)
                    .unwrap_or(' ')
                    .is_alphanumeric())
                && (actual_pos + old_name.len() >= line.len()
                    || !line
                        .chars()
                        .nth(actual_pos + old_name.len())
                        .unwrap_or(' ')
                        .is_alphanumeric());

            if is_word_boundary {
                edits.push(TextEdit {
                    file_path: None,
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
    let module_path = intent
        .arguments()
        .get("modulePath")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AstError::analysis("Missing modulePath parameter"))?;

    let import_name = intent
        .arguments()
        .get("importName")
        .and_then(|v| v.as_str());

    let is_default = intent
        .arguments()
        .get("isDefault")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let source_file = intent
        .arguments()
        .get("sourceFile")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    // Find the best location to insert the import (simplified)
    let insert_location = find_import_insertion_point(source)?;

    let import_text = if is_default {
        format!(
            "import {} from '{}';",
            import_name.unwrap_or("defaultImport"),
            module_path
        )
    } else if let Some(name) = import_name {
        format!("import {{ {} }} from '{}';", name, module_path)
    } else {
        format!("import '{}';", module_path)
    };

    let edit = TextEdit {
        file_path: None,
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
        validations: vec![ValidationRule {
            rule_type: ValidationType::ImportResolution,
            description: "Verify import resolves correctly".to_string(),
            parameters: HashMap::from([(
                "module_path".to_string(),
                serde_json::Value::String(module_path.to_string()),
            )]),
        }],
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
    let module_path = intent
        .arguments()
        .get("modulePath")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AstError::analysis("Missing modulePath parameter"))?;

    let import_name = intent
        .arguments()
        .get("importName")
        .and_then(|v| v.as_str());

    let source_file = intent
        .arguments()
        .get("sourceFile")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    let mut edits = Vec::new();

    // Find import statements to remove
    for (line_num, line) in source.lines().enumerate() {
        let line_trimmed = line.trim();

        // Check for ES module imports
        if line_trimmed.starts_with("import ")
            && line_trimmed.contains(&format!("'{}'", module_path))
        {
            if let Some(import_name) = import_name {
                // Remove specific named import
                if line_trimmed.contains(&format!("{{ {}", import_name))
                    || line_trimmed.contains(&format!("{} }}", import_name))
                    || line_trimmed.contains(&format!(" {} ", import_name))
                {
                    if line_trimmed.matches(',').count() > 0 {
                        // Multiple imports - remove just this one
                        let new_line = remove_named_import_from_line(line, import_name);
                        if new_line != line {
                            edits.push(TextEdit {
                                file_path: None,
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
                            file_path: None,
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
                    file_path: None,
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
        else if line_trimmed.contains("require(")
            && line_trimmed.contains(&format!("'{}'", module_path))
        {
            edits.push(TextEdit {
                file_path: None,
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
        validations: vec![ValidationRule {
            rule_type: ValidationType::SyntaxCheck,
            description: "Verify syntax is still valid after import removal".to_string(),
            parameters: HashMap::new(),
        }],
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
    let old_path = intent
        .arguments()
        .get("oldPath")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AstError::analysis("Missing oldPath parameter"))?;

    let new_path = intent
        .arguments()
        .get("newPath")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AstError::analysis("Missing newPath parameter"))?;

    let source_file = intent
        .arguments()
        .get("sourceFile")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    let mut edits = Vec::new();

    // Find import statements with the old path
    for (line_num, line) in source.lines().enumerate() {
        let line_trimmed = line.trim();

        if (line_trimmed.starts_with("import ") || line_trimmed.contains("require("))
            && (line_trimmed.contains(&format!("'{}'", old_path))
                || line_trimmed.contains(&format!("\"{}\"", old_path)))
        {
            let new_line = line
                .replace(&format!("'{}'", old_path), &format!("'{}'", new_path))
                .replace(&format!("\"{}\"", old_path), &format!("\"{}\"", new_path));

            if new_line != line {
                edits.push(TextEdit {
                    file_path: None,
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
                    description: format!(
                        "Update import path from '{}' to '{}'",
                        old_path, new_path
                    ),
                });
            }
        }
    }

    Ok(EditPlan {
        source_file: source_file.to_string(),
        edits,
        dependency_updates: Vec::new(),
        validations: vec![ValidationRule {
            rule_type: ValidationType::ImportResolution,
            description: "Verify new import path resolves correctly".to_string(),
            parameters: HashMap::from([(
                "new_path".to_string(),
                serde_json::Value::String(new_path.to_string()),
            )]),
        }],
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
    let function_name = intent
        .arguments()
        .get("functionName")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AstError::analysis("Missing functionName parameter"))?;

    let start_line = intent
        .arguments()
        .get("startLine")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| AstError::analysis("Missing startLine parameter"))?
        as u32;

    let end_line = intent
        .arguments()
        .get("endLine")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| AstError::analysis("Missing endLine parameter"))? as u32;

    let source_file = intent
        .arguments()
        .get("sourceFile")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    let lines: Vec<&str> = source.lines().collect();

    if start_line as usize >= lines.len()
        || end_line as usize >= lines.len()
        || start_line > end_line
    {
        return Err(AstError::analysis(
            "Invalid line range for function extraction",
        ));
    }

    // Extract the selected lines
    let extracted_lines = &lines[start_line as usize..=end_line as usize];
    let extracted_code = extracted_lines.join("\n");

    // Analyze variables used in the extracted code
    let (parameters, return_vars) =
        analyze_function_variables(&extracted_code, source, start_line, end_line)?;

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
        format!(
            "const {} = {}({});",
            return_vars[0],
            function_name,
            parameters.join(", ")
        )
    } else {
        format!(
            "const [{}] = {}({});",
            return_vars.join(", "),
            function_name,
            parameters.join(", ")
        )
    };

    edits.push(TextEdit {
        file_path: None,
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
        file_path: None,
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
    let function_name = intent
        .arguments()
        .get("functionName")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AstError::analysis("Missing functionName parameter"))?;

    let source_file = intent
        .arguments()
        .get("sourceFile")
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
    for call in function_calls.iter().rev() {
        // Process in reverse order to avoid offset issues
        let inlined_code = inline_function_call(&function_info, call)?;

        edits.push(TextEdit {
            file_path: None,
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
        file_path: None,
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
            impact_areas: vec![
                "functions".to_string(),
                "code_structure".to_string(),
                "refactoring".to_string(),
            ],
        },
    })
}

// Helper functions for refactoring operations

/// Remove a named import from an import line using AST parsing
fn remove_named_import_from_line(line: &str, import_name: &str) -> String {
    use swc_common::{FileName, FilePathMapping};
    use swc_ecma_ast::{ImportSpecifier, ModuleDecl, ModuleItem};

    // Set up SWC parser
    let cm = Lrc::new(SourceMap::new(FilePathMapping::empty()));
    let file_name = Lrc::new(FileName::Anon);
    let source_file = cm.new_source_file(file_name, line.to_string());

    // Parse the import line as TypeScript (most permissive syntax)
    let syntax = Syntax::Typescript(TsSyntax {
        tsx: true,
        decorators: true,
        ..Default::default()
    });

    let lexer = Lexer::new(
        syntax,
        Default::default(),
        StringInput::from(&*source_file),
        None,
    );
    let mut parser = Parser::new_from(lexer);

    // Try to parse as a module
    let module = match parser.parse_module() {
        Ok(module) => module,
        Err(_) => {
            // If parsing fails, return the original line unchanged
            return line.to_string();
        }
    };

    // Find the import declaration and filter out the specified import
    let mut modified = false;
    let new_items: Vec<ModuleItem> = module
        .body
        .into_iter()
        .filter_map(|item| {
            if let ModuleItem::ModuleDecl(ModuleDecl::Import(mut import_decl)) = item {
                // Filter out the import specifier matching import_name
                let original_len = import_decl.specifiers.len();
                import_decl.specifiers.retain(|spec| {
                    match spec {
                        ImportSpecifier::Named(named) => {
                            // Check both the local name and imported name
                            let local_name = named.local.sym.as_ref();
                            let imported_name =
                                named.imported.as_ref().map_or(local_name, |imp| match imp {
                                    swc_ecma_ast::ModuleExportName::Ident(ident) => {
                                        ident.sym.as_ref()
                                    }
                                    swc_ecma_ast::ModuleExportName::Str(s) => s.value.as_ref(),
                                });
                            local_name != import_name && imported_name != import_name
                        }
                        ImportSpecifier::Default(default) => {
                            default.local.sym.as_ref() != import_name
                        }
                        ImportSpecifier::Namespace(ns) => ns.local.sym.as_ref() != import_name,
                    }
                });

                // If we removed something, mark as modified
                if import_decl.specifiers.len() < original_len {
                    modified = true;
                }

                // If no specifiers left, remove the entire import
                if import_decl.specifiers.is_empty() {
                    return None;
                }

                return Some(ModuleItem::ModuleDecl(ModuleDecl::Import(import_decl)));
            }
            Some(item)
        })
        .collect();

    // If nothing was modified, return original line
    if !modified {
        return line.to_string();
    }

    // If no items left (entire import was removed), return empty string
    if new_items.is_empty() {
        return String::new();
    }

    // Emit the modified import
    let mut buf = vec![];
    {
        use swc_ecma_ast::Module;

        let new_module = Module {
            body: new_items,
            ..module
        };

        let mut emitter = Emitter {
            cfg: Default::default(),
            cm: cm.clone(),
            comments: None,
            wr: JsWriter::new(cm.clone(), "\n", &mut buf, None),
        };

        if emitter.emit_module(&new_module).is_err() {
            // If emission fails, return original line
            return line.to_string();
        }
    }

    String::from_utf8(buf)
        .unwrap_or_else(|_| line.to_string())
        .trim()
        .to_string()
}

/// Analyze variables for function extraction
fn analyze_function_variables(
    extracted_code: &str,
    _full_source: &str,
    _start_line: u32,
    _end_line: u32,
) -> AstResult<(Vec<String>, Vec<String>)> {
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
        if let Some(after_keyword) = line.strip_prefix(keyword) {
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
    // TODO: Implement proper parameter extraction from function signatures
    // Currently hardcoded to Vec::new() - needs AST parsing for full implementation
    body: String,
    location: EditLocation,
    function_text: String,
}

/// Information about a function call
#[derive(Debug, Clone)]
struct FunctionCall {
    // TODO: Implement proper argument extraction from function calls
    // Currently not populated - needs AST parsing for function call analysis
    location: EditLocation,
    call_text: String,
}

/// Find function definition in source code
fn find_function_definition(source: &str, function_name: &str) -> AstResult<FunctionInfo> {
    // Simplified function finder - in practice, you'd use proper AST parsing
    for (line_num, line) in source.lines().enumerate() {
        if line.contains(&format!("function {}", function_name))
            || line.contains(&format!("const {} =", function_name))
            || line.contains(&format!("let {} =", function_name))
        {
            return Ok(FunctionInfo {
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

    Err(AstError::analysis(format!(
        "Function '{}' not found",
        function_name
    )))
}

/// Find all calls to a function
fn find_function_calls(source: &str, function_name: &str) -> AstResult<Vec<FunctionCall>> {
    let mut calls = Vec::new();

    for (line_num, line) in source.lines().enumerate() {
        if line.contains(&format!("{}(", function_name)) {
            calls.push(FunctionCall {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remove_only_named_import() {
        let line = "import { foo } from 'module';";
        let result = remove_named_import_from_line(line, "foo");
        assert_eq!(
            result, "",
            "Should return empty string when removing the only import"
        );
    }

    #[test]
    fn test_remove_first_import_from_multiple() {
        let line = "import { foo, bar, baz } from 'module';";
        let result = remove_named_import_from_line(line, "foo");
        assert!(
            result.contains("bar") && result.contains("baz"),
            "Should keep remaining imports"
        );
        assert!(
            !result.contains("foo"),
            "Should remove the specified import"
        );
    }

    #[test]
    fn test_remove_middle_import() {
        let line = "import { foo, bar, baz } from 'module';";
        let result = remove_named_import_from_line(line, "bar");
        assert!(
            result.contains("foo") && result.contains("baz"),
            "Should keep remaining imports"
        );
        assert!(
            !result.contains("bar"),
            "Should remove the specified import"
        );
    }

    #[test]
    fn test_remove_last_import() {
        let line = "import { foo, bar, baz } from 'module';";
        let result = remove_named_import_from_line(line, "baz");
        assert!(
            result.contains("foo") && result.contains("bar"),
            "Should keep remaining imports"
        );
        assert!(
            !result.contains("baz"),
            "Should remove the specified import"
        );
    }

    #[test]
    fn test_remove_non_existent_import() {
        let line = "import { foo, bar } from 'module';";
        let result = remove_named_import_from_line(line, "nonexistent");
        assert_eq!(
            result, line,
            "Should return original line when import not found"
        );
    }

    #[test]
    fn test_remove_default_import() {
        let line = "import foo from 'module';";
        let result = remove_named_import_from_line(line, "foo");
        assert_eq!(result, "", "Should remove default import");
    }

    #[test]
    fn test_remove_aliased_import() {
        let line = "import { foo as bar } from 'module';";
        let result = remove_named_import_from_line(line, "bar");
        assert_eq!(result, "", "Should remove aliased import by local name");
    }
}
