//! Python AST parsing functionality using RustPython parser
//!
//! Note: Currently using basic regex-based parsing due to RustPython parser API changes.
//! Full AST parsing will be re-implemented with the stable API.

use crate::error::{AstError, AstResult};
use crate::parser::{ImportInfo, ImportType, NamedImport, SourceLocation};
use regex::Regex;

/// Parse Python imports using regex-based parsing (temporary implementation)
pub fn parse_python_imports_ast(source: &str) -> AstResult<Vec<ImportInfo>> {
    let mut imports = Vec::new();

    // Regex patterns for Python imports
    let import_re = Regex::new(r"^import\s+([\w.]+)(?:\s+as\s+(\w+))?").unwrap();
    let from_import_re = Regex::new(r"^from\s+([\w.]+)\s+import\s+(.+)").unwrap();

    for (line_num, line) in source.lines().enumerate() {
        let line = line.trim();

        if let Some(captures) = import_re.captures(line) {
            let module_name = captures.get(1).unwrap().as_str();
            let alias = captures.get(2).map(|m| m.as_str().to_string());

            imports.push(ImportInfo {
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
            let module_name = captures.get(1).unwrap().as_str();
            let imports_str = captures.get(2).unwrap().as_str();

            let named_imports = parse_import_names(imports_str);

            imports.push(ImportInfo {
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
    let mut named_imports = Vec::new();

    // Handle "import *" case
    if imports_str.trim() == "*" {
        return named_imports; // Empty for wildcard imports
    }

    // Split by comma and parse each import
    for import_part in imports_str.split(',') {
        let import_part = import_part.trim();

        if let Some((name, alias)) = import_part.split_once(" as ") {
            named_imports.push(NamedImport {
                name: name.trim().to_string(),
                alias: Some(alias.trim().to_string()),
                type_only: false,
            });
        } else {
            named_imports.push(NamedImport {
                name: import_part.to_string(),
                alias: None,
                type_only: false,
            });
        }
    }

    named_imports
}

/// Extract Python function definitions (simplified regex-based implementation)
pub fn extract_python_functions(source: &str) -> AstResult<Vec<PythonFunction>> {
    let mut functions = Vec::new();

    // Regex for function definitions
    let func_re = Regex::new(r"^(\s*)(async\s+)?def\s+(\w+)\s*\(([^)]*)\)\s*:").unwrap();

    for (line_num, line) in source.lines().enumerate() {
        if let Some(captures) = func_re.captures(line) {
            let _indent = captures.get(1).unwrap().as_str();
            let is_async = captures.get(2).is_some();
            let name = captures.get(3).unwrap().as_str();
            let args_str = captures.get(4).unwrap().as_str();

            let args = if args_str.trim().is_empty() {
                Vec::new()
            } else {
                args_str
                    .split(',')
                    .map(|arg| {
                        arg.split_whitespace()
                            .next()
                            .unwrap_or("")
                            .to_string()
                    })
                    .filter(|arg| !arg.is_empty())
                    .collect()
            };

            functions.push(PythonFunction {
                name: name.to_string(),
                start_line: line_num as u32,
                end_line: line_num as u32 + 10, // Rough estimate
                args,
                body_start_line: line_num as u32 + 1,
                is_async,
                decorators: Vec::new(), // Would need more complex parsing
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

/// Extract Python variable assignments (simplified regex-based implementation)
pub fn extract_python_variables(source: &str) -> AstResult<Vec<PythonVariable>> {
    let mut variables = Vec::new();

    // Regex for variable assignments
    let assign_re = Regex::new(r"^(\s*)(\w+)\s*=\s*(.+)").unwrap();

    for (line_num, line) in source.lines().enumerate() {
        if let Some(captures) = assign_re.captures(line) {
            let var_name = captures.get(2).unwrap().as_str();
            let value = captures.get(3).unwrap().as_str();

            let value_type = infer_python_value_type_simple(value);
            let is_constant = var_name.chars().all(|c| c.is_uppercase() || c == '_');

            variables.push(PythonVariable {
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

#[derive(Debug, Clone)]
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

/// Simple value type inference from source text
fn infer_python_value_type_simple(value: &str) -> PythonValueType {
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

/// Find variables in scope for a given line
pub fn find_python_scope_variables(
    source: &str,
    target_line: u32,
) -> AstResult<Vec<PythonVariable>> {
    let variables = extract_python_variables(source)?;

    // For Python, variables are in scope if they're declared before the target line
    // and at the same or lower indentation level
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

/// Analyze a selected Python expression range
pub fn analyze_python_expression_range(
    source: &str,
    start_line: u32,
    start_col: u32,
    end_line: u32,
    end_col: u32,
) -> AstResult<String> {
    let lines: Vec<&str> = source.lines().collect();

    if start_line == end_line {
        // Single line
        let line = lines
            .get(start_line as usize)
            .ok_or_else(|| AstError::analysis("Invalid line number"))?;
        Ok(line[start_col as usize..end_col as usize].to_string())
    } else {
        // Multi-line
        let mut result = String::new();

        // First line
        if let Some(first_line) = lines.get(start_line as usize) {
            result.push_str(&first_line[start_col as usize..]);
            result.push('\n');
        }

        // Middle lines
        for line_idx in (start_line + 1)..end_line {
            if let Some(line) = lines.get(line_idx as usize) {
                result.push_str(line);
                result.push('\n');
            }
        }

        // Last line
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
) -> AstResult<Option<PythonVariable>> {
    let variables = extract_python_variables(source)?;

    for var in variables {
        if var.line == line {
            let line_text = source
                .lines()
                .nth(line as usize)
                .ok_or_else(|| AstError::analysis("Invalid line number"))?;

            // Check if the column position is within the variable name
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
) -> AstResult<Vec<(u32, u32, u32)>> {
    let mut usages = Vec::new();
    let lines: Vec<&str> = source.lines().collect();

    for (line_idx, line_text) in lines.iter().enumerate() {
        let line_idx = line_idx as u32;

        // Skip lines before the variable declaration
        if line_idx < from_line {
            continue;
        }

        // Find all occurrences of the variable name in this line
        let mut start = 0;
        while let Some(pos) = line_text[start..].find(variable_name) {
            let actual_pos = start + pos;

            // Check if this is a whole word (not part of another identifier)
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

/// Find the end line of a Python function
pub fn find_python_function_end(source: &str, function_start_line: u32) -> AstResult<u32> {
    let lines: Vec<&str> = source.lines().collect();
    let start_line = function_start_line as usize;

    if start_line >= lines.len() {
        return Err(AstError::analysis("Invalid function start line"));
    }

    // Get the indentation of the function definition
    let func_line = lines[start_line];
    let func_indent = func_line.chars().take_while(|c| c.is_whitespace()).count();

    // Find the next line with same or less indentation, or a new function/class
    for (idx, line) in lines.iter().enumerate().skip(start_line + 1) {
        if line.trim().is_empty() {
            continue; // Skip empty lines
        }

        let line_indent = line.chars().take_while(|c| c.is_whitespace()).count();

        // If we find a line with same or less indentation that's not part of the function body
        if line_indent <= func_indent {
            // Check if it's a new function, class, or other top-level construct
            let trimmed = line.trim();
            if trimmed.starts_with("def ")
                || trimmed.starts_with("class ")
                || trimmed.starts_with("if __name__")
                || line_indent < func_indent
            {
                return Ok(idx as u32 - 1);
            }
        }
    }

    // If we reach the end of the file, the function ends at the last line
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

        let imports = parse_python_imports_ast(source).unwrap();
        assert_eq!(imports.len(), 4);

        // Test simple import
        assert_eq!(imports[0].module_path, "os");
        assert_eq!(imports[0].namespace_import, Some("os".to_string()));
        assert_eq!(imports[0].import_type, ImportType::PythonImport);

        // Test import with alias
        assert_eq!(imports[1].module_path, "sys");
        assert_eq!(imports[1].namespace_import, Some("system".to_string()));

        // Test from import
        assert_eq!(imports[2].module_path, "pathlib");
        assert_eq!(imports[2].import_type, ImportType::PythonFromImport);
        assert_eq!(imports[2].named_imports[0].name, "Path");

        // Test from import with aliases
        assert_eq!(imports[3].module_path, "typing");
        assert_eq!(imports[3].named_imports.len(), 2);
        assert_eq!(
            imports[3].named_imports[1].alias,
            Some("ArrayList".to_string())
        );
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

        // Test simple function
        assert_eq!(functions[0].name, "simple_function");
        assert!(!functions[0].is_async);
        assert_eq!(functions[0].args.len(), 0);

        // Test async function
        assert_eq!(functions[1].name, "async_function");
        assert!(functions[1].is_async);
        assert_eq!(functions[1].args, vec!["param1", "param2"]);

        // Test function with args
        assert_eq!(functions[2].name, "function_with_args");
        assert!(!functions[2].is_async);
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

        // Test string variable
        assert_eq!(variables[0].name, "name");
        assert!(matches!(variables[0].value_type, PythonValueType::String));
        assert!(!variables[0].is_constant);

        // Test number variable
        assert_eq!(variables[1].name, "age");
        assert!(matches!(variables[1].value_type, PythonValueType::Number));

        // Test boolean variable
        assert_eq!(variables[2].name, "is_active");
        assert!(matches!(variables[2].value_type, PythonValueType::Boolean));

        // Test list variable
        assert_eq!(variables[3].name, "items");
        assert!(matches!(variables[3].value_type, PythonValueType::List));

        // Test dict variable
        assert_eq!(variables[4].name, "config");
        assert!(matches!(variables[4].value_type, PythonValueType::Dict));

        // Test constant
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
    fn test_value_type_inference() {
        assert!(matches!(
            infer_python_value_type_simple("\"hello\""),
            PythonValueType::String
        ));
        assert!(matches!(
            infer_python_value_type_simple("42"),
            PythonValueType::Number
        ));
        assert!(matches!(
            infer_python_value_type_simple("True"),
            PythonValueType::Boolean
        ));
        assert!(matches!(
            infer_python_value_type_simple("[1, 2, 3]"),
            PythonValueType::List
        ));
        assert!(matches!(
            infer_python_value_type_simple("{\"a\": 1}"),
            PythonValueType::Dict
        ));
        assert!(matches!(
            infer_python_value_type_simple("None"),
            PythonValueType::None
        ));
    }
}
