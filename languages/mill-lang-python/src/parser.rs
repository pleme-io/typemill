//! Python AST parsing functionality
//!
//! This module provides Python source code parsing, including:
//! - Import statement extraction (import, from...import)
//! - Function and method extraction
//! - Variable and constant extraction
//! - Symbol identification for code intelligence
//!
//! Implements dual-mode parsing:
//! 1. Python native AST via subprocess (high accuracy, requires python3)
//! 2. Regex-based fallback parsing (always available, good for common cases)
use crate::constants::{
    CLASS_DEF_PATTERN, FROM_IMPORT_PATTERN, FUNCTION_DEF_PATTERN, IMPORT_PATTERN, PARSER_VERSION,
    VARIABLE_ASSIGN_PATTERN,
};
use mill_foundation::protocol::{ImportGraph, ImportInfo, ImportType, NamedImport, SourceLocation};
use mill_lang_common::{
    parse_import_alias, parse_with_fallback, run_ast_tool_async, ImportGraphBuilder,
    SubprocessAstTool,
};
use mill_plugin_api::{PluginApiError, PluginResult, Symbol, SymbolKind};
use std::path::Path;

/// List all function names in Python source code using Python's native AST parser.
/// This function spawns a Python subprocess to perform the parsing.
pub(crate) async fn list_functions(source: &str) -> PluginResult<Vec<String>> {
    const AST_TOOL_PY: &str = include_str!("../resources/ast_tool.py");
    let tool = SubprocessAstTool::new("python3")
        .with_embedded_str(AST_TOOL_PY)
        .with_temp_filename("ast_tool.py")
        .with_args(vec!["{script}".to_string(), "list-functions".to_string()]);
    run_ast_tool_async(tool, source).await.map_err(Into::into)
}

/// Analyze Python imports and produce an import graph.
/// Uses dual-mode parsing: Python AST parser with regex fallback.
pub(crate) fn analyze_imports(source: &str, file_path: Option<&Path>) -> PluginResult<ImportGraph> {
    let imports = parse_with_fallback(
        || parse_python_imports(source),
        || Ok(Vec::new()),
        "Python import parsing",
    )?;
    Ok(ImportGraphBuilder::new("python")
        .with_source_file(file_path)
        .with_imports(imports)
        .extract_external_dependencies(|path| !path.starts_with('.'))
        .with_parser_version(PARSER_VERSION)
        .build())
}

/// Structure to hold results of parsing Python source code
#[derive(Debug, Clone)]
pub(crate) struct PythonParseResult {
    pub symbols: Vec<Symbol>,
    pub imports: Vec<ImportInfo>,
    pub functions: Vec<PythonFunction>,
    pub variables: Vec<PythonVariable>,
}

/// Parse all Python source code elements in a single pass.
///
/// This function iterates over the source lines once and extracts:
/// - Symbols (Functions, Classes, Variables, Constants)
/// - Imports
/// - Function metadata
/// - Variable metadata
pub(crate) fn parse_source_code(source: &str) -> PluginResult<PythonParseResult> {
    let mut symbols = Vec::new();
    let mut imports = Vec::new();
    let mut functions = Vec::new();
    let mut variables = Vec::new();

    enum ScopeData {
        Function(PythonFunction),
        Class,
    }

    // Stack: (start_line, indentation, ScopeData, pending_symbol)
    let mut active_scopes: Vec<(u32, usize, ScopeData, Symbol)> = Vec::new();
    let mut last_line_idx = 0;

    for (line_num, line) in source.lines().enumerate() {
        let line_num = line_num as u32;
        last_line_idx = line_num;
        let trimmed = line.trim();

        // Handle imports
        if let Some(captures) = IMPORT_PATTERN.captures(trimmed) {
            let module_name = captures
                .get(1)
                .expect("Python import regex should always capture module name at index 1")
                .as_str();
            let alias = captures.get(2).map(|m| m.as_str().to_string());
            imports.push(ImportInfo {
                module_path: module_name.to_string(),
                import_type: ImportType::PythonImport,
                named_imports: Vec::new(),
                default_import: None,
                namespace_import: alias.or_else(|| Some(module_name.to_string())),
                type_only: false,
                location: SourceLocation {
                    start_line: line_num,
                    end_line: line_num,
                    start_column: 0,
                    end_column: trimmed.len() as u32,
                },
            });
        } else if let Some(captures) = FROM_IMPORT_PATTERN.captures(trimmed) {
            let module_name = captures
                .get(1)
                .expect("Python from-import regex should always capture module name at index 1")
                .as_str();
            let imports_str = captures
                .get(2)
                .expect("Python from-import regex should always capture imports at index 2")
                .as_str();
            let named_imports = parse_import_names(imports_str);
            imports.push(ImportInfo {
                module_path: module_name.to_string(),
                import_type: ImportType::PythonFromImport,
                named_imports,
                default_import: None,
                namespace_import: None,
                type_only: false,
                location: SourceLocation {
                    start_line: line_num,
                    end_line: line_num,
                    start_column: 0,
                    end_column: trimmed.len() as u32,
                },
            });
        }

        // Skip comments and empty lines for indentation tracking
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // Calculate indentation
        let indent = line.chars().take_while(|c| c.is_whitespace()).count();

        // Check if any active scopes ended
        while let Some((_, func_indent, _, _)) = active_scopes.last() {
            if indent <= *func_indent {
                let (_, _, scope_data, mut symbol) = active_scopes.pop().unwrap();
                let end_line = line_num.saturating_sub(1);
                symbol.end_location = Some(mill_plugin_api::SourceLocation {
                    line: end_line as usize,
                    column: 0,
                });
                symbols.push(symbol);

                if let ScopeData::Function(mut func) = scope_data {
                    func.end_line = end_line;
                    functions.push(func);
                }
            } else {
                break;
            }
        }

        // Function definitions
        if let Some(captures) = FUNCTION_DEF_PATTERN.captures(line) {
            let _indent_str = captures
                .get(1)
                .expect("Python function regex should always capture indent at index 1")
                .as_str();
            let is_async = captures.get(2).is_some();
            let name = captures
                .get(3)
                .expect("Python function regex should always capture name at index 3")
                .as_str();
            let args_str = captures
                .get(4)
                .expect("Python function regex should always capture args at index 4")
                .as_str();
            let args = if args_str.trim().is_empty() {
                Vec::new()
            } else {
                args_str
                    .split(',')
                    .map(|arg| arg.split_whitespace().next().unwrap_or("").to_string())
                    .filter(|arg| !arg.is_empty())
                    .collect()
            };

            let func = PythonFunction {
                name: name.to_string(),
                start_line: line_num,
                end_line: line_num, // Will update when scope closes
                args,
                body_start_line: line_num + 1,
                is_async,
                decorators: Vec::new(),
            };

            let symbol = Symbol {
                name: name.to_string(),
                kind: SymbolKind::Function,
                location: mill_plugin_api::SourceLocation {
                    line: line_num as usize,
                    column: 0,
                },
                end_location: None, // Will update when scope closes
                documentation: None,
            };

            let func_indent = _indent_str.len();
            active_scopes.push((line_num, func_indent, ScopeData::Function(func), symbol));
            continue;
        }

        // Class definitions
        // CLASS_DEF_PATTERN is `^class\s+(\w+)`. It expects start of string (no indent).
        // But classes can be indented.
        // We use trimmed line for regex, but we must use line indentation for scope.
        if let Some(captures) = CLASS_DEF_PATTERN.captures(trimmed) {
            if let Some(name) = captures.get(1) {
                let symbol = Symbol {
                    name: name.as_str().to_string(),
                    kind: SymbolKind::Class,
                    location: mill_plugin_api::SourceLocation {
                        line: line_num as usize,
                        column: 0,
                    },
                    end_location: None, // Will update when scope closes
                    documentation: None,
                };

                // Indent is already calculated
                active_scopes.push((line_num, indent, ScopeData::Class, symbol));
                continue;
            }
        }

        // Variable assignments
        if let Some(captures) = VARIABLE_ASSIGN_PATTERN.captures(line) {
            let var_name = captures
                .get(2)
                .expect("Python assignment regex should always capture variable name at index 2")
                .as_str();
            let value = captures
                .get(3)
                .expect("Python assignment regex should always capture value at index 3")
                .as_str();

            let is_constant = var_name.chars().all(|c| c.is_uppercase() || c == '_');

            let value_type = infer_python_value_type(value);
            variables.push(PythonVariable {
                name: var_name.to_string(),
                line: line_num,
                value_type,
                is_constant,
            });

            let kind = if is_constant {
                SymbolKind::Constant
            } else {
                SymbolKind::Variable
            };

            symbols.push(Symbol {
                name: var_name.to_string(),
                kind,
                location: mill_plugin_api::SourceLocation {
                    line: line_num as usize,
                    column: 0,
                },
                end_location: Some(mill_plugin_api::SourceLocation {
                    line: line_num as usize,
                    column: 0,
                }),
                documentation: None,
            });
        }
    }

    // Close remaining scopes
    for (_, _, scope_data, mut symbol) in active_scopes.into_iter().rev() {
        symbol.end_location = Some(mill_plugin_api::SourceLocation {
            line: last_line_idx as usize,
            column: 0,
        });
        symbols.push(symbol);

        if let ScopeData::Function(mut func) = scope_data {
            func.end_line = last_line_idx;
            functions.push(func);
        }
    }

    // Sort functions by start_line to maintain expected order
    functions.sort_by_key(|f| f.start_line);
    // Sort symbols by start line (and column? usually 0)
    symbols.sort_by_key(|s| s.location.line);

    Ok(PythonParseResult {
        symbols,
        imports,
        functions,
        variables,
    })
}

/// Parse Python imports using regex-based parsing
pub(crate) fn parse_python_imports(source: &str) -> PluginResult<Vec<ImportInfo>> {
    Ok(parse_source_code(source)?.imports)
}

/// Parse import names from "from ... import ..." statements
fn parse_import_names(imports_str: &str) -> Vec<NamedImport> {
    if imports_str.trim() == "*" {
        return Vec::new();
    }
    imports_str
        .split(',')
        .map(|part| {
            let (name, alias) = parse_import_alias(part.trim());
            NamedImport {
                name,
                alias,
                type_only: false,
            }
        })
        .collect()
}

/// Extract Python function definitions with metadata
pub(crate) fn extract_python_functions(source: &str) -> PluginResult<Vec<PythonFunction>> {
    Ok(parse_source_code(source)?.functions)
}

/// Python function representation
#[derive(Debug, Clone)]
pub(crate) struct PythonFunction {
    pub name: String,
    pub start_line: u32,
    #[allow(dead_code)] // Future enhancement: Function body analysis
    pub end_line: u32,
    #[allow(dead_code)] // Future enhancement: Parameter analysis
    pub args: Vec<String>,
    #[allow(dead_code)] // Future enhancement: Scope analysis
    pub body_start_line: u32,
    #[allow(dead_code)] // Future enhancement: Async function analysis
    pub is_async: bool,
    #[allow(dead_code)] // Future enhancement: Decorator analysis
    pub decorators: Vec<String>,
}

/// Extract Python variable assignments
#[allow(dead_code)]
pub(crate) fn extract_python_variables(source: &str) -> PluginResult<Vec<PythonVariable>> {
    Ok(parse_source_code(source)?.variables)
}

/// Python variable representation
#[derive(Debug, Clone)]
pub(crate) struct PythonVariable {
    pub name: String,
    pub line: u32,
    #[allow(dead_code)] // Future enhancement: Type-based refactoring
    pub value_type: PythonValueType,
    #[allow(dead_code)]
    pub is_constant: bool,
}
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum PythonValueType {
    String,
    Number,
    Boolean,
    List,
    Dict,
    Tuple,
    #[allow(dead_code)] // Future enhancement: Set literal detection
    Set,
    None,
    Function,
    Class,
    Unknown,
}

/// Infers the Python value type from source text.
fn infer_python_value_type(value: &str) -> PythonValueType {
    let value = value.trim();
    if value.starts_with('"') || value.starts_with('\'') {
        PythonValueType::String
    } else if value.starts_with('[') && value.ends_with(']') {
        PythonValueType::List
    } else if value.starts_with('{') && value.ends_with('}') {
        PythonValueType::Dict
    } else if value.starts_with('(') && value.ends_with(')') {
        PythonValueType::Tuple
    } else if value == "True" || value == "False" {
        PythonValueType::Boolean
    } else if value == "None" {
        PythonValueType::None
    } else if value.chars().all(|c| c.is_ascii_digit() || c == '.') {
        PythonValueType::Number
    } else if value.starts_with("def ") || value.starts_with("lambda ") {
        PythonValueType::Function
    } else if value.starts_with("class ") {
        PythonValueType::Class
    } else {
        PythonValueType::Unknown
    }
}

/// Extract symbols from Python source code for code intelligence
#[allow(dead_code)]
pub(crate) fn extract_symbols(source: &str) -> PluginResult<Vec<Symbol>> {
    Ok(parse_source_code(source)?.symbols)
}

/// Finds the end line of a Python function by analyzing indentation levels.
/// DEPRECATED: Use parse_source_code instead which handles scope ending in a single pass.
#[allow(dead_code)]
pub(crate) fn find_python_function_end(
    lines: &[&str],
    function_start_line: u32,
) -> PluginResult<u32> {
    let start_line = function_start_line as usize;
    if start_line >= lines.len() {
        return Err(PluginApiError::parse("Invalid function start line"));
    }
    let func_line = lines[start_line];
    let func_indent = func_line.chars().take_while(|c| c.is_whitespace()).count();
    for (idx, line) in lines.iter().enumerate().skip(start_line + 1) {
        if line.trim().is_empty() {
            continue;
        }
        let line_indent = line.chars().take_while(|c| c.is_whitespace()).count();
        if line_indent <= func_indent {
            return Ok(idx as u32 - 1);
        }
    }
    Ok(lines.len() as u32 - 1)
}

/// Gets the indentation level (number of leading whitespace characters) at a specific line.
#[allow(dead_code)] // Future enhancement: Indentation-aware refactoring
pub(crate) fn get_python_indentation_at_line(source: &str, line: u32) -> u32 {
    let lines: Vec<&str> = source.lines().collect();
    if let Some(line_text) = lines.get(line as usize) {
        line_text.chars().take_while(|c| c.is_whitespace()).count() as u32
    } else {
        0
    }
}

/// Analyzes and extracts a Python expression from a selected range.
pub(crate) fn analyze_python_expression_range(
    source: &str,
    start_line: u32,
    start_col: u32,
    end_line: u32,
    end_col: u32,
) -> PluginResult<String> {
    let lines: Vec<&str> = source.lines().collect();
    if start_line == end_line {
        let line = lines
            .get(start_line as usize)
            .ok_or_else(|| PluginApiError::parse("Invalid line number"))?;
        Ok(line[start_col as usize..end_col as usize].to_string())
    } else {
        let mut result = String::new();
        if let Some(first_line) = lines.get(start_line as usize) {
            result.push_str(&first_line[start_col as usize..]);
            result.push('\n');
        }
        for line_idx in (start_line + 1)..end_line {
            if let Some(line) = lines.get(line_idx as usize) {
                result.push_str(line);
                result.push('\n');
            }
        }
        if let Some(last_line) = lines.get(end_line as usize) {
            result.push_str(&last_line[..end_col as usize]);
        }
        Ok(result)
    }
}

/// Finds a Python variable declaration at a specific cursor position.
pub(crate) fn find_variable_at_position(
    source: &str,
    line: u32,
    col: u32,
) -> PluginResult<Option<PythonVariable>> {
    let line_text = source
        .lines()
        .nth(line as usize)
        .ok_or_else(|| PluginApiError::parse("Invalid line number"))?;

    if let Some(captures) = VARIABLE_ASSIGN_PATTERN.captures(line_text) {
        let var_match = captures
            .get(2)
            .expect("Python assignment regex should always capture variable name at index 2");
        let var_name = var_match.as_str();
        let var_start = var_match.start();
        let var_end = var_match.end();

        if col >= var_start as u32 && col <= var_end as u32 {
            let value = captures
                .get(3)
                .expect("Python assignment regex should always capture value at index 3")
                .as_str();
            let value_type = infer_python_value_type(value);
            let is_constant = var_name.chars().all(|c| c.is_uppercase() || c == '_');

            return Ok(Some(PythonVariable {
                name: var_name.to_string(),
                line,
                value_type,
                is_constant,
            }));
        }
    }
    Ok(None)
}

/// Get all usages of a variable within scope
pub(crate) fn get_variable_usages_in_scope(
    source: &str,
    variable_name: &str,
    from_line: u32,
) -> PluginResult<Vec<(u32, u32, u32)>> {
    let mut usages = Vec::new();
    let lines: Vec<&str> = source.lines().collect();
    for (line_idx, line_text) in lines.iter().enumerate() {
        let line_idx = line_idx as u32;
        if line_idx < from_line {
            continue;
        }
        let mut start = 0;
        while let Some(pos) = line_text[start..].find(variable_name) {
            let actual_pos = start + pos;
            let is_word_boundary = (actual_pos == 0
                || !line_text
                    .chars()
                    .nth(actual_pos - 1)
                    .unwrap_or(' ')
                    .is_alphanumeric())
                && (actual_pos + variable_name.len() >= line_text.len()
                    || !line_text
                        .chars()
                        .nth(actual_pos + variable_name.len())
                        .unwrap_or(' ')
                        .is_alphanumeric());
            if is_word_boundary {
                usages.push((
                    line_idx,
                    actual_pos as u32,
                    (actual_pos + variable_name.len()) as u32,
                ));
            }
            start = actual_pos + 1;
        }
    }
    Ok(usages)
}

/// Finds all variables that are in scope at a given line based on Python indentation rules.
#[allow(dead_code)] // Future enhancement: Scope-aware variable analysis
pub(crate) fn find_python_scope_variables(
    source: &str,
    target_line: u32,
) -> PluginResult<Vec<PythonVariable>> {
    let variables = extract_python_variables(source)?;
    let target_indent = get_python_indentation_at_line(source, target_line);
    Ok(variables
        .into_iter()
        .filter(|var| var.line < target_line)
        .filter(|var| {
            let var_indent = get_python_indentation_at_line(source, var.line);
            var_indent <= target_indent
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_parse_python_imports_basic() {
        let source = r#"
import os
import sys as system
from pathlib import Path
from typing import Dict, List as ArrayList
"#;
        let imports = parse_python_imports(source).unwrap();
        assert_eq!(imports.len(), 4);
        assert_eq!(imports[0].module_path, "os");
        assert_eq!(imports[0].namespace_import, Some("os".to_string()));
        assert_eq!(imports[0].import_type, ImportType::PythonImport);
        assert_eq!(imports[1].module_path, "sys");
        assert_eq!(imports[1].namespace_import, Some("system".to_string()));
        assert_eq!(imports[2].module_path, "pathlib");
        assert_eq!(imports[2].import_type, ImportType::PythonFromImport);
        assert_eq!(imports[2].named_imports[0].name, "Path");
        assert_eq!(imports[3].module_path, "typing");
        assert_eq!(imports[3].named_imports.len(), 2);
        assert_eq!(
            imports[3].named_imports[1].alias,
            Some("ArrayList".to_string())
        );
    }
    #[tokio::test]
    async fn test_extract_python_functions_basic() {
        let source = r#"
def simple_function():
    pass

async def async_function(param1, param2):
    return param1 + param2

def function_with_args(a, b, c=None):
    return a + b
"#;
        let functions = extract_python_functions(source).unwrap();
        assert_eq!(functions.len(), 3);
        assert_eq!(functions[0].name, "simple_function");
        assert!(!functions[0].is_async);
        assert_eq!(functions[0].args.len(), 0);
        assert_eq!(functions[1].name, "async_function");
        assert!(functions[1].is_async);
        assert_eq!(functions[1].args, vec!["param1", "param2"]);
        assert_eq!(functions[2].name, "function_with_args");
        assert!(!functions[2].is_async);
        assert_eq!(functions[2].args, vec!["a", "b", "c=None"]);
    }

    #[test]
    fn test_extract_python_functions_with_comments() {
        let source = r#"
def func_with_comments():
    # This is a comment at indentation 4
    x = 1
# This is a comment at indentation 0
    y = 2
    return x + y

def another_func():
    pass
"#;
        let functions = extract_python_functions(source).unwrap();
        assert_eq!(functions.len(), 2);
        assert_eq!(functions[0].name, "func_with_comments");
        // func_with_comments ends when another_func starts (line 8) or empty line 7?
        // In the new parser:
        // Line 1: def (indent 0). Stack [func:0].
        // Line 2: # (skip)
        // Line 3: x=1 (indent 4). 4 > 0.
        // Line 4: # (skip)
        // Line 5: y=2 (indent 4). 4 > 0.
        // Line 6: return (indent 4). 4 > 0.
        // Line 7: empty (skip)
        // Line 8: def (indent 0). 0 <= 0. Pop func.
        // Pop happens at Line 8.
        // end_line = 8 - 1 = 7.
        assert_eq!(functions[0].end_line, 7);
        assert_eq!(functions[1].name, "another_func");
    }
    #[test]
    fn test_extract_python_variables_basic() {
        let source = r#"
name = "John"
age = 30
is_active = True
items = [1, 2, 3]
config = {"key": "value"}
CONSTANT_VALUE = "constant"
"#;
        let variables = extract_python_variables(source).unwrap();
        assert_eq!(variables.len(), 6);
        assert_eq!(variables[0].name, "name");
        assert_eq!(variables[0].value_type, PythonValueType::String);
        assert!(!variables[0].is_constant);
        assert_eq!(variables[1].name, "age");
        assert_eq!(variables[1].value_type, PythonValueType::Number);
        assert_eq!(variables[2].name, "is_active");
        assert_eq!(variables[2].value_type, PythonValueType::Boolean);
        assert_eq!(variables[3].name, "items");
        assert_eq!(variables[3].value_type, PythonValueType::List);
        assert_eq!(variables[4].name, "config");
        assert_eq!(variables[4].value_type, PythonValueType::Dict);
        assert_eq!(variables[5].name, "CONSTANT_VALUE");
        assert!(variables[5].is_constant);
    }
    #[test]
    fn test_parse_import_names() {
        let imports = parse_import_names("Dict, List as ArrayList, Set");
        assert_eq!(imports.len(), 3);
        assert_eq!(imports[0].name, "Dict");
        assert_eq!(imports[0].alias, None);
        assert_eq!(imports[1].name, "List");
        assert_eq!(imports[1].alias, Some("ArrayList".to_string()));
        assert_eq!(imports[2].name, "Set");
        assert_eq!(imports[2].alias, None);
    }
    #[test]
    fn test_extract_symbols() {
        let source = r#"
CONSTANT = 42

def my_function():
    pass

class MyClass:
    pass

variable = "value"
"#;
        let symbols = extract_symbols(source).unwrap();
        assert!(symbols.len() >= 4);
        let has_constant = symbols
            .iter()
            .any(|s| s.name == "CONSTANT" && s.kind == SymbolKind::Constant);
        let has_function = symbols
            .iter()
            .any(|s| s.name == "my_function" && s.kind == SymbolKind::Function);
        let has_class = symbols
            .iter()
            .any(|s| s.name == "MyClass" && s.kind == SymbolKind::Class);
        let has_variable = symbols
            .iter()
            .any(|s| s.name == "variable" && s.kind == SymbolKind::Variable);
        assert!(has_constant, "Should extract constant");
        assert!(has_function, "Should extract function");
        assert!(has_class, "Should extract class");
        assert!(has_variable, "Should extract variable");
    }
}
