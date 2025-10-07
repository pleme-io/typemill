//! Python-specific refactoring operations
//!
//! This module provides AST-based refactoring capabilities for Python code including:
//! - Extract function: Extract selected code into a new function
//! - Inline variable: Replace variable usages with their initializer
//! - Extract variable: Extract an expression into a named variable
//!
//! These refactoring operations analyze Python code structure and generate edit plans
//! that can be applied to transform the code while preserving semantics.
use crate::parser::{
    analyze_python_expression_range, extract_python_functions, extract_python_variables,
    find_variable_at_position, get_variable_usages_in_scope,
};
use cb_lang_common::LineExtractor;
use cb_protocol::{
    EditLocation, EditPlan, EditPlanMetadata, EditType, TextEdit, ValidationRule,
    ValidationType,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
/// Range of code for refactoring operations
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CodeRange {
    pub start_line: u32,
    pub start_col: u32,
    pub end_line: u32,
    pub end_col: u32,
}
/// Information about a function that can be extracted
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExtractableFunction {
    pub selected_range: CodeRange,
    pub required_parameters: Vec<String>,
    pub return_variables: Vec<String>,
    pub suggested_name: String,
    pub insertion_point: CodeRange,
    pub contains_return_statements: bool,
    pub complexity_score: u32,
}
/// Analysis result for inline variable refactoring
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InlineVariableAnalysis {
    pub variable_name: String,
    pub declaration_range: CodeRange,
    pub initializer_expression: String,
    pub usage_locations: Vec<CodeRange>,
    pub is_safe_to_inline: bool,
    pub blocking_reasons: Vec<String>,
}
/// Analysis result for extract variable refactoring
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExtractVariableAnalysis {
    pub expression: String,
    pub expression_range: CodeRange,
    pub can_extract: bool,
    pub suggested_name: String,
    pub insertion_point: CodeRange,
    pub blocking_reasons: Vec<String>,
    pub scope_type: String,
}
/// Convert CodeRange to EditLocation
impl From<CodeRange> for EditLocation {
    fn from(range: CodeRange) -> Self {
        EditLocation {
            start_line: range.start_line,
            start_column: range.start_col,
            end_line: range.end_line,
            end_column: range.end_col,
        }
    }
}
/// Error type for refactoring operations
#[derive(Debug, thiserror::Error)]
pub enum RefactoringError {
    #[error("Analysis error: {0}")]
    Analysis(String),
    #[error("Parse error: {0}")]
    Parse(String),
}
pub type RefactoringResult<T> = Result<T, RefactoringError>;
impl From<cb_plugin_api::PluginError> for RefactoringError {
    fn from(err: cb_plugin_api::PluginError) -> Self {
        RefactoringError::Parse(err.to_string())
    }
}
/// Analyze code selection for function extraction (Python)
pub fn analyze_extract_function(
    source: &str,
    range: &CodeRange,
    _file_path: &str,
) -> RefactoringResult<ExtractableFunction> {
    let lines: Vec<&str> = source.lines().collect();
    let mut required_parameters = Vec::new();
    let mut required_imports = Vec::new();
    let functions = extract_python_functions(source)?;
    let variables = extract_python_variables(source)?;
    for line_num in range.start_line..=range.end_line {
        if let Some(line) = lines.get(line_num as usize) {
            let line_text = if line_num == range.start_line && line_num == range.end_line
            {
                &line[range.start_col as usize..range.end_col as usize]
            } else if line_num == range.start_line {
                &line[range.start_col as usize..]
            } else if line_num == range.end_line {
                &line[..range.end_col as usize]
            } else {
                line
            };
            for var in &variables {
                if var.line < range.start_line && line_text.contains(&var.name)
                    && !required_parameters.contains(&var.name)
                {
                    required_parameters.push(var.name.clone());
                }
            }
            for func in &functions {
                if func.start_line < range.start_line
                    && line_text.contains(&format!("{}(", func.name))
                    && !required_imports.contains(&func.name)
                {
                    required_imports.push(func.name.clone());
                }
            }
        }
    }
    let selected_text = extract_range_text(source, range)?;
    let contains_return = selected_text.contains("return ");
    let insertion_point = find_insertion_point(source, range.start_line)?;
    Ok(ExtractableFunction {
        selected_range: range.clone(),
        required_parameters,
        return_variables: Vec::new(),
        suggested_name: "extracted_function".to_string(),
        insertion_point,
        contains_return_statements: contains_return,
        complexity_score: 2,
    })
}
/// Analyze variable declaration for inlining (Python)
pub fn analyze_inline_variable(
    source: &str,
    variable_line: u32,
    variable_col: u32,
    _file_path: &str,
) -> RefactoringResult<InlineVariableAnalysis> {
    if let Some(variable) = find_variable_at_position(
        source,
        variable_line,
        variable_col,
    )? {
        let lines: Vec<&str> = source.lines().collect();
        let var_line_text = lines
            .get(variable.line as usize)
            .ok_or_else(|| RefactoringError::Analysis(
                "Invalid line number".to_string(),
            ))?;
        let assign_re = regex::Regex::new(
                &format!(r"^\s*{}\s*=\s*(.+)", regex::escape(& variable.name)),
            )
            .unwrap();
        let initializer = if let Some(captures) = assign_re.captures(var_line_text) {
            captures.get(1).unwrap().as_str().trim().to_string()
        } else {
            return Err(
                RefactoringError::Analysis(
                    "Could not find variable assignment".to_string(),
                ),
            );
        };
        let usages = get_variable_usages_in_scope(
            source,
            &variable.name,
            variable.line + 1,
        )?;
        let usage_locations: Vec<CodeRange> = usages
            .into_iter()
            .map(|(line, start_col, end_col)| CodeRange {
                start_line: line,
                start_col,
                end_line: line,
                end_col,
            })
            .collect();
        Ok(InlineVariableAnalysis {
            variable_name: variable.name,
            declaration_range: CodeRange {
                start_line: variable.line,
                start_col: 0,
                end_line: variable.line,
                end_col: var_line_text.len() as u32,
            },
            initializer_expression: initializer,
            usage_locations,
            is_safe_to_inline: true,
            blocking_reasons: Vec::new(),
        })
    } else {
        Err(
            RefactoringError::Analysis(
                "Could not find variable at specified position".to_string(),
            ),
        )
    }
}
/// Analyze a selected expression for extraction into a variable (Python)
pub fn analyze_extract_variable(
    source: &str,
    start_line: u32,
    start_col: u32,
    end_line: u32,
    end_col: u32,
    _file_path: &str,
) -> RefactoringResult<ExtractVariableAnalysis> {
    let expression_range = CodeRange {
        start_line,
        start_col,
        end_line,
        end_col,
    };
    let expression = analyze_python_expression_range(
        source,
        start_line,
        start_col,
        end_line,
        end_col,
    )?;
    let mut can_extract = true;
    let mut blocking_reasons = Vec::new();
    if expression.trim().starts_with("def ") || expression.trim().starts_with("class ") {
        can_extract = false;
        blocking_reasons
            .push("Cannot extract function or class definitions".to_string());
    }
    if expression.contains('=') && !expression.contains("==")
        && !expression.contains("!=")
    {
        can_extract = false;
        blocking_reasons.push("Cannot extract assignment statements".to_string());
    }
    if expression.lines().count() > 1 && !expression.trim().starts_with('(') {
        can_extract = false;
        blocking_reasons
            .push("Multi-line expressions must be parenthesized".to_string());
    }
    let suggested_name = suggest_variable_name(&expression);
    let insertion_point = CodeRange {
        start_line,
        start_col: 0,
        end_line: start_line,
        end_col: 0,
    };
    Ok(ExtractVariableAnalysis {
        expression,
        expression_range,
        can_extract,
        suggested_name,
        insertion_point,
        blocking_reasons,
        scope_type: "function".to_string(),
    })
}
/// Generate edit plan for extract function refactoring (Python)
pub fn plan_extract_function(
    source: &str,
    range: &CodeRange,
    new_function_name: &str,
    file_path: &str,
) -> RefactoringResult<EditPlan> {
    let analysis = analyze_extract_function(source, range, file_path)?;
    let mut edits = Vec::new();
    let function_code = generate_extracted_function(
        source,
        &analysis,
        new_function_name,
    )?;
    edits
        .push(TextEdit {
            file_path: None,
            edit_type: EditType::Insert,
            location: analysis.insertion_point.clone().into(),
            original_text: String::new(),
            new_text: format!("{}\n\n", function_code),
            priority: 100,
            description: format!("Create extracted function '{}'", new_function_name),
        });
    let call_code = generate_function_call(&analysis, new_function_name)?;
    edits
        .push(TextEdit {
            file_path: None,
            edit_type: EditType::Replace,
            location: analysis.selected_range.clone().into(),
            original_text: extract_range_text(source, &analysis.selected_range)?,
            new_text: call_code,
            priority: 90,
            description: format!(
                "Replace selected code with call to '{}'", new_function_name
            ),
        });
    Ok(EditPlan {
        source_file: file_path.to_string(),
        edits,
        dependency_updates: Vec::new(),
        validations: vec![
            ValidationRule { rule_type : ValidationType::SyntaxCheck, description :
            "Verify Python syntax is valid after extraction".to_string(), parameters :
            HashMap::new(), }
        ],
        metadata: EditPlanMetadata {
            intent_name: "extract_function".to_string(),
            intent_arguments: serde_json::json!(
                { "range" : range, "function_name" : new_function_name }
            ),
            created_at: chrono::Utc::now(),
            complexity: analysis.complexity_score.min(10) as u8,
            impact_areas: vec!["function_extraction".to_string()],
        },
    })
}
/// Generate edit plan for inline variable refactoring (Python)
pub fn plan_inline_variable(
    source: &str,
    variable_line: u32,
    variable_col: u32,
    file_path: &str,
) -> RefactoringResult<EditPlan> {
    let analysis = analyze_inline_variable(
        source,
        variable_line,
        variable_col,
        file_path,
    )?;
    if !analysis.is_safe_to_inline {
        return Err(
            RefactoringError::Analysis(
                format!(
                    "Cannot safely inline variable '{}': {}", analysis.variable_name,
                    analysis.blocking_reasons.join(", ")
                ),
            ),
        );
    }
    let mut edits = Vec::new();
    let mut priority = 100;
    for usage_location in &analysis.usage_locations {
        let replacement_text = if analysis.initializer_expression.contains(' ')
            && (analysis.initializer_expression.contains('+')
                || analysis.initializer_expression.contains('-')
                || analysis.initializer_expression.contains('*')
                || analysis.initializer_expression.contains('/')
                || analysis.initializer_expression.contains('%'))
        {
            format!("({})", analysis.initializer_expression)
        } else {
            analysis.initializer_expression.clone()
        };
        edits
            .push(TextEdit {
                file_path: None,
                edit_type: EditType::Replace,
                location: usage_location.clone().into(),
                original_text: analysis.variable_name.clone(),
                new_text: replacement_text,
                priority,
                description: format!(
                    "Replace '{}' with its value", analysis.variable_name
                ),
            });
        priority -= 1;
    }
    edits
        .push(TextEdit {
            file_path: None,
            edit_type: EditType::Delete,
            location: analysis.declaration_range.clone().into(),
            original_text: extract_range_text(source, &analysis.declaration_range)?,
            new_text: String::new(),
            priority: 50,
            description: format!("Remove declaration of '{}'", analysis.variable_name),
        });
    Ok(EditPlan {
        source_file: file_path.to_string(),
        edits,
        dependency_updates: Vec::new(),
        validations: vec![
            ValidationRule { rule_type : ValidationType::SyntaxCheck, description :
            "Verify Python syntax is valid after inlining".to_string(), parameters :
            HashMap::new(), }
        ],
        metadata: EditPlanMetadata {
            intent_name: "inline_variable".to_string(),
            intent_arguments: serde_json::json!(
                { "variable" : analysis.variable_name, "line" : variable_line, "column" :
                variable_col }
            ),
            created_at: chrono::Utc::now(),
            complexity: (analysis.usage_locations.len().min(10)) as u8,
            impact_areas: vec!["variable_inlining".to_string()],
        },
    })
}
/// Generate edit plan for extract variable refactoring (Python)
pub fn plan_extract_variable(
    source: &str,
    start_line: u32,
    start_col: u32,
    end_line: u32,
    end_col: u32,
    variable_name: Option<String>,
    file_path: &str,
) -> RefactoringResult<EditPlan> {
    let analysis = analyze_extract_variable(
        source,
        start_line,
        start_col,
        end_line,
        end_col,
        file_path,
    )?;
    if !analysis.can_extract {
        return Err(
            RefactoringError::Analysis(
                format!(
                    "Cannot extract expression: {}", analysis.blocking_reasons.join(", ")
                ),
            ),
        );
    }
    let var_name = variable_name.unwrap_or_else(|| analysis.suggested_name.clone());
    let indent = LineExtractor::get_indentation_str(source, start_line);
    let mut edits = Vec::new();
    let declaration = format!("{}{} = {}\n", indent, var_name, analysis.expression);
    edits
        .push(TextEdit {
            file_path: None,
            edit_type: EditType::Insert,
            location: analysis.insertion_point.clone().into(),
            original_text: String::new(),
            new_text: declaration,
            priority: 100,
            description: format!(
                "Extract '{}' into variable '{}'", analysis.expression, var_name
            ),
        });
    edits
        .push(TextEdit {
            file_path: None,
            edit_type: EditType::Replace,
            location: analysis.expression_range.clone().into(),
            original_text: analysis.expression.clone(),
            new_text: var_name.clone(),
            priority: 90,
            description: format!("Replace expression with '{}'", var_name),
        });
    Ok(EditPlan {
        source_file: file_path.to_string(),
        edits,
        dependency_updates: Vec::new(),
        validations: vec![
            ValidationRule { rule_type : ValidationType::SyntaxCheck, description :
            "Verify Python syntax is valid after extraction".to_string(), parameters :
            HashMap::new(), }
        ],
        metadata: EditPlanMetadata {
            intent_name: "extract_variable".to_string(),
            intent_arguments: serde_json::json!(
                { "expression" : analysis.expression, "variableName" : var_name,
                "startLine" : start_line, "startCol" : start_col, "endLine" : end_line,
                "endCol" : end_col }
            ),
            created_at: chrono::Utc::now(),
            complexity: 2,
            impact_areas: vec!["variable_extraction".to_string()],
        },
    })
}
/// Extract text from a Python code range
fn extract_range_text(source: &str, range: &CodeRange) -> RefactoringResult<String> {
    Ok(
        analyze_python_expression_range(
            source,
            range.start_line,
            range.start_col,
            range.end_line,
            range.end_col,
        )?,
    )
}
/// Find proper insertion point for a new Python function
fn find_insertion_point(source: &str, start_line: u32) -> RefactoringResult<CodeRange> {
    let lines: Vec<&str> = source.lines().collect();
    let mut insertion_line = 0;
    for (idx, line) in lines.iter().enumerate() {
        let line_idx = idx as u32;
        if line_idx >= start_line {
            break;
        }
        let trimmed = line.trim();
        if trimmed.starts_with("def ") || trimmed.starts_with("class ") {
            insertion_line = line_idx;
        }
    }
    Ok(CodeRange {
        start_line: insertion_line,
        start_col: 0,
        end_line: insertion_line,
        end_col: 0,
    })
}
/// Generate Python function code for extraction
fn generate_extracted_function(
    source: &str,
    analysis: &ExtractableFunction,
    function_name: &str,
) -> RefactoringResult<String> {
    let params = analysis.required_parameters.join(", ");
    let extracted_code = extract_range_text(source, &analysis.selected_range)?;
    let indented_code = extracted_code
        .lines()
        .map(|line| {
            if line.trim().is_empty() {
                line.to_string()
            } else {
                format!("    {}", line)
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    let return_statement = if analysis.return_variables.is_empty() {
        String::new()
    } else if analysis.return_variables.len() == 1 {
        format!("    return {}", analysis.return_variables[0])
    } else {
        format!("    return {}", analysis.return_variables.join(", "))
    };
    Ok(
        format!(
            "def {}({}):\n{}\n{}", function_name, params, indented_code, return_statement
        ),
    )
}
/// Generate Python function call
fn generate_function_call(
    analysis: &ExtractableFunction,
    function_name: &str,
) -> RefactoringResult<String> {
    let args = analysis.required_parameters.join(", ");
    if analysis.return_variables.is_empty() {
        Ok(format!("{}({})", function_name, args))
    } else if analysis.return_variables.len() == 1 {
        Ok(format!("{} = {}({})", analysis.return_variables[0], function_name, args))
    } else {
        Ok(
            format!(
                "{} = {}({})", analysis.return_variables.join(", "), function_name, args
            ),
        )
    }
}
/// Suggest a Python variable name based on the expression
fn suggest_variable_name(expression: &str) -> String {
    let expr = expression.trim();
    if expr.contains("len(") {
        return "length".to_string();
    }
    if expr.contains(".split(") {
        return "parts".to_string();
    }
    if expr.contains(".join(") {
        return "joined".to_string();
    }
    if expr.starts_with('"') || expr.starts_with('\'') {
        return "text".to_string();
    }
    if expr.parse::<f64>().is_ok() {
        return "value".to_string();
    }
    if expr == "True" || expr == "False" {
        return "flag".to_string();
    }
    if expr.starts_with('[') {
        return "items".to_string();
    }
    if expr.starts_with('{') {
        return "data".to_string();
    }
    if expr.contains('+') || expr.contains('-') || expr.contains('*')
        || expr.contains('/')
    {
        return "result".to_string();
    }
    "extracted".to_string()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_suggest_variable_name_len() {
        assert_eq!(suggest_variable_name("len(items)"), "length");
    }
    #[test]
    fn test_suggest_variable_name_split() {
        assert_eq!(suggest_variable_name("text.split(',')"), "parts");
    }
    #[test]
    fn test_suggest_variable_name_string() {
        assert_eq!(suggest_variable_name("\"hello\""), "text");
    }
    #[test]
    fn test_suggest_variable_name_number() {
        assert_eq!(suggest_variable_name("42"), "value");
    }
    #[test]
    fn test_suggest_variable_name_list() {
        assert_eq!(suggest_variable_name("[1, 2, 3]"), "items");
    }
    #[test]
    fn test_suggest_variable_name_arithmetic() {
        assert_eq!(suggest_variable_name("a + b"), "result");
    }
    #[test]
    fn test_suggest_variable_name_default() {
        assert_eq!(suggest_variable_name("some_function()"), "extracted");
    }
    #[test]
    fn test_extract_variable_analysis_simple() {
        let source = r#"
def calculate():
    result = 10 + 20
    return result
"#;
        let analysis = analyze_extract_variable(source, 2, 13, 2, 20, "test.py")
            .unwrap();
        assert!(analysis.can_extract);
        assert_eq!(analysis.expression.trim(), "10 + 20");
        assert_eq!(analysis.suggested_name, "result");
    }
    #[test]
    fn test_inline_variable_analysis() {
        let source = r#"x = 42
y = x + 1
z = x * 2"#;
        let analysis = analyze_inline_variable(source, 0, 0, "test.py").unwrap();
        assert_eq!(analysis.variable_name, "x");
        assert_eq!(analysis.initializer_expression, "42");
        assert_eq!(analysis.usage_locations.len(), 2);
        assert!(analysis.is_safe_to_inline);
    }
    #[test]
    fn test_plan_extract_variable() {
        let source = r#"
def test():
    x = len([1, 2, 3])
    return x
"#;
        let plan = plan_extract_variable(
                source,
                2,
                8,
                2,
                22,
                Some("count".to_string()),
                "test.py",
            )
            .unwrap();
        assert_eq!(plan.edits.len(), 2);
        assert_eq!(plan.metadata.intent_name, "extract_variable");
    }
    #[test]
    fn test_plan_inline_variable() {
        let source = r#"x = 10
y = x + 5"#;
        let plan = plan_inline_variable(source, 0, 0, "test.py").unwrap();
        assert_eq!(plan.edits.len(), 2);
        assert_eq!(plan.metadata.intent_name, "inline_variable");
    }
    #[test]
    fn test_plan_extract_function_simple() {
        let source = r#"
def main():
    x = 1
    y = 2
    result = x + y
    return result
"#;
        let range = CodeRange {
            start_line: 3,
            start_col: 4,
            end_line: 4,
            end_col: 18,
        };
        let plan = plan_extract_function(source, &range, "calculate_sum", "test.py")
            .unwrap();
        assert_eq!(plan.edits.len(), 2);
        assert_eq!(plan.metadata.intent_name, "extract_function");
    }
}
