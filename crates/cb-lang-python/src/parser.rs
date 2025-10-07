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
use cb_plugin_api::{PluginError, PluginResult, Symbol, SymbolKind};
use cb_protocol::{ImportGraph, ImportInfo, ImportType, NamedImport, SourceLocation};
use cb_lang_common::{
    SubprocessAstTool, run_ast_tool, parse_with_fallback, parse_import_alias,
    ImportGraphBuilder,
};
use regex::Regex;
use std::path::Path;
use tracing::debug;
/// List all function names in Python source code using Python's native AST parser.
/// This function spawns a Python subprocess to perform the parsing.
pub fn list_functions(source: &str) -> PluginResult<Vec<String>> {
    const AST_TOOL_PY: &str = include_str!("../resources/ast_tool.py");
    let tool = SubprocessAstTool::new("python3")
        .with_embedded_str(AST_TOOL_PY)
        .with_temp_filename("ast_tool.py")
        .with_args(vec!["{script}".to_string(), "list-functions".to_string()]);
    run_ast_tool(tool, source)
}
/// Analyze Python imports and produce an import graph.
/// Uses dual-mode parsing: Python AST parser with regex fallback.
pub fn analyze_imports(
    source: &str,
    file_path: Option<&Path>,
) -> PluginResult<ImportGraph> {
    let imports = parse_with_fallback(
        || parse_python_imports(source),
        || Ok(Vec::new()),
        "Python import parsing",
    )?;
    Ok(
        ImportGraphBuilder::new("python")
            .with_source_file(file_path)
            .with_imports(imports)
            .extract_external_dependencies(|path| !path.starts_with('.'))
            .with_parser_version("0.1.0-plugin")
            .build(),
    )
}
/// Parse Python imports using regex-based parsing
///
/// Supports both `import` and `from...import` statements with aliases.
/// This is a fast, reliable parser for common import patterns.
pub fn parse_python_imports(source: &str) -> PluginResult<Vec<ImportInfo>> {
    let mut imports = Vec::new();
    let import_re = Regex::new(r"^import\s+([\w.]+)(?:\s+as\s+(\w+))?")
        .expect("Python import regex pattern should be valid");
    let from_import_re = Regex::new(r"^from\s+([\w.]+)\s+import\s+(.+)")
        .expect("Python from-import regex pattern should be valid");
    for (line_num, line) in source.lines().enumerate() {
        let line = line.trim();
        if let Some(captures) = import_re.captures(line) {
            let module_name = captures
                .get(1)
                .expect(
                    "Python import regex should always capture module name at index 1",
                )
                .as_str();
            let alias = captures.get(2).map(|m| m.as_str().to_string());
            imports
                .push(ImportInfo {
                    module_path: module_name.to_string(),
                    import_type: ImportType::PythonImport,
                    named_imports: Vec::new(),
                    default_import: None,
                    namespace_import: alias.or_else(|| Some(module_name.to_string())),
                    type_only: false,
                    location: SourceLocation {
                        start_line: line_num as u32,
                        end_line: line_num as u32,
                        start_column: 0,
                        end_column: line.len() as u32,
                    },
                });
        } else if let Some(captures) = from_import_re.captures(line) {
            let module_name = captures
                .get(1)
                .expect(
                    "Python from-import regex should always capture module name at index 1",
                )
                .as_str();
            let imports_str = captures
                .get(2)
                .expect(
                    "Python from-import regex should always capture imports at index 2",
                )
                .as_str();
            let named_imports = parse_import_names(imports_str);
            imports
                .push(ImportInfo {
                    module_path: module_name.to_string(),
                    import_type: ImportType::PythonFromImport,
                    named_imports,
                    default_import: None,
                    namespace_import: None,
                    type_only: false,
                    location: SourceLocation {
                        start_line: line_num as u32,
                        end_line: line_num as u32,
                        start_column: 0,
                        end_column: line.len() as u32,
                    },
                });
        }
    }
    Ok(imports)
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
pub fn extract_python_functions(source: &str) -> PluginResult<Vec<PythonFunction>> {
    let mut functions = Vec::new();
    let func_re = Regex::new(r"^(\s*)(async\s+)?def\s+(\w+)\s*\(([^)]*)\)\s*:")
        .expect("Python function regex pattern should be valid");
    for (line_num, line) in source.lines().enumerate() {
        if let Some(captures) = func_re.captures(line) {
            let _indent = captures
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
            functions
                .push(PythonFunction {
                    name: name.to_string(),
                    start_line: line_num as u32,
                    end_line: line_num as u32 + 10,
                    args,
                    body_start_line: line_num as u32 + 1,
                    is_async,
                    decorators: Vec::new(),
                });
        }
    }
    Ok(functions)
}
/// Python function representation
#[derive(Debug, Clone)]
pub struct PythonFunction {
    pub name: String,
    pub start_line: u32,
    pub end_line: u32,
    pub args: Vec<String>,
    pub body_start_line: u32,
    pub is_async: bool,
    pub decorators: Vec<String>,
}
/// Extract Python variable assignments
pub fn extract_python_variables(source: &str) -> PluginResult<Vec<PythonVariable>> {
    let mut variables = Vec::new();
    let assign_re = Regex::new(r"^(\s*)(\w+)\s*=\s*(.+)")
        .expect("Python variable assignment regex pattern should be valid");
    for (line_num, line) in source.lines().enumerate() {
        if let Some(captures) = assign_re.captures(line) {
            let var_name = captures
                .get(2)
                .expect(
                    "Python assignment regex should always capture variable name at index 2",
                )
                .as_str();
            let value = captures
                .get(3)
                .expect("Python assignment regex should always capture value at index 3")
                .as_str();
            let value_type = infer_python_value_type(value);
            let is_constant = var_name.chars().all(|c| c.is_uppercase() || c == '_');
            variables
                .push(PythonVariable {
                    name: var_name.to_string(),
                    line: line_num as u32,
                    value_type,
                    is_constant,
                });
        }
    }
    Ok(variables)
}
/// Python variable representation
#[derive(Debug, Clone)]
pub struct PythonVariable {
    pub name: String,
    pub line: u32,
    pub value_type: PythonValueType,
    pub is_constant: bool,
}
#[derive(Debug, Clone, PartialEq)]
pub enum PythonValueType {
    String,
    Number,
    Boolean,
    List,
    Dict,
    Tuple,
    Set,
    None,
    Function,
    Class,
    Unknown,
}
/// Infer Python value type from source text
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
pub fn extract_symbols(source: &str) -> PluginResult<Vec<Symbol>> {
    let mut symbols = Vec::new();
    let functions = extract_python_functions(source)?;
    for func in functions {
        symbols
            .push(Symbol {
                name: func.name.clone(),
                kind: SymbolKind::Function,
                location: cb_plugin_api::SourceLocation {
                    line: func.start_line as usize,
                    column: 0,
                },
                documentation: None,
            });
    }
    let variables = extract_python_variables(source)?;
    for var in variables {
        let kind = if var.is_constant {
            SymbolKind::Constant
        } else {
            SymbolKind::Variable
        };
        symbols
            .push(Symbol {
                name: var.name.clone(),
                kind,
                location: cb_plugin_api::SourceLocation {
                    line: var.line as usize,
                    column: 0,
                },
                documentation: None,
            });
    }
    let class_re = Regex::new(r"^class\s+(\w+)")
        .expect("Python class regex pattern should be valid");
    for (line_num, line) in source.lines().enumerate() {
        if let Some(captures) = class_re.captures(line.trim()) {
            if let Some(name) = captures.get(1) {
                symbols
                    .push(Symbol {
                        name: name.as_str().to_string(),
                        kind: SymbolKind::Class,
                        location: cb_plugin_api::SourceLocation {
                            line: line_num,
                            column: 0,
                        },
                        documentation: None,
                    });
            }
        }
    }
    debug!(symbols_count = symbols.len(), "Extracted Python symbols");
    Ok(symbols)
}
/// Find the end line of a Python function
pub fn find_python_function_end(
    source: &str,
    function_start_line: u32,
) -> PluginResult<u32> {
    let lines: Vec<&str> = source.lines().collect();
    let start_line = function_start_line as usize;
    if start_line >= lines.len() {
        return Err(PluginError::parse("Invalid function start line"));
    }
    let func_line = lines[start_line];
    let func_indent = func_line.chars().take_while(|c| c.is_whitespace()).count();
    for (idx, line) in lines.iter().enumerate().skip(start_line + 1) {
        if line.trim().is_empty() {
            continue;
        }
        let line_indent = line.chars().take_while(|c| c.is_whitespace()).count();
        if line_indent <= func_indent {
            let trimmed = line.trim();
            if trimmed.starts_with("def ") || trimmed.starts_with("class ")
                || trimmed.starts_with("if __name__") || line_indent < func_indent
            {
                return Ok(idx as u32 - 1);
            }
        }
    }
    Ok(lines.len() as u32 - 1)
}
/// Get indentation level at specific line
pub fn get_python_indentation_at_line(source: &str, line: u32) -> u32 {
    let lines: Vec<&str> = source.lines().collect();
    if let Some(line_text) = lines.get(line as usize) {
        line_text.chars().take_while(|c| c.is_whitespace()).count() as u32
    } else {
        0
    }
}
/// Analyze a selected Python expression range
pub fn analyze_python_expression_range(
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
            .ok_or_else(|| PluginError::parse("Invalid line number"))?;
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
/// Find variable declaration at specific position
pub fn find_variable_at_position(
    source: &str,
    line: u32,
    col: u32,
) -> PluginResult<Option<PythonVariable>> {
    let variables = extract_python_variables(source)?;
    for var in variables {
        if var.line == line {
            let line_text = source
                .lines()
                .nth(line as usize)
                .ok_or_else(|| PluginError::parse("Invalid line number"))?;
            if let Some(var_pos) = line_text.find(&var.name) {
                let var_end = var_pos + var.name.len();
                if col >= var_pos as u32 && col <= var_end as u32 {
                    return Ok(Some(var));
                }
            }
        }
    }
    Ok(None)
}
/// Get all usages of a variable within scope
pub fn get_variable_usages_in_scope(
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
                usages
                    .push((
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
/// Find variables in scope for a given line
pub fn find_python_scope_variables(
    source: &str,
    target_line: u32,
) -> PluginResult<Vec<PythonVariable>> {
    let variables = extract_python_variables(source)?;
    let target_indent = get_python_indentation_at_line(source, target_line);
    Ok(
        variables
            .into_iter()
            .filter(|var| var.line < target_line)
            .filter(|var| {
                let var_indent = get_python_indentation_at_line(source, var.line);
                var_indent <= target_indent
            })
            .collect(),
    )
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
        assert_eq!(imports[3].named_imports[1].alias, Some("ArrayList".to_string()));
    }
    #[test]
    fn test_extract_python_functions_basic() {
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
        assert!(! functions[0].is_async);
        assert_eq!(functions[0].args.len(), 0);
        assert_eq!(functions[1].name, "async_function");
        assert!(functions[1].is_async);
        assert_eq!(functions[1].args, vec!["param1", "param2"]);
        assert_eq!(functions[2].name, "function_with_args");
        assert!(! functions[2].is_async);
        assert_eq!(functions[2].args, vec!["a", "b", "c=None"]);
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
        assert!(! variables[0].is_constant);
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
