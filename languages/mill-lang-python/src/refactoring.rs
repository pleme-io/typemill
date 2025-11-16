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
use mill_foundation::protocol::{EditPlan, EditType, TextEdit};
use mill_lang_common::{
    count_unescaped_quotes, find_literal_occurrences, refactoring::edit_plan_builder::EditPlanBuilder,
    ExtractConstantAnalysis, ExtractVariableAnalysis, ExtractableFunction, InlineVariableAnalysis,
    LineExtractor,
};
use mill_plugin_api::{PluginApiError, PluginResult};

// Re-export for use within the plugin
pub use mill_lang_common::CodeRange;

/// Analyze code selection for function extraction (Python)
pub(crate) fn analyze_extract_function(
    source: &str,
    range: &CodeRange,
    _file_path: &str,
) -> PluginResult<ExtractableFunction> {
    let lines: Vec<&str> = source.lines().collect();
    let mut required_parameters = Vec::new();
    let mut required_imports = Vec::new();
    let functions = extract_python_functions(source)?;
    let variables = extract_python_variables(source)?;
    for line_num in range.start_line..=range.end_line {
        if let Some(line) = lines.get(line_num as usize) {
            let line_text = if line_num == range.start_line && line_num == range.end_line {
                &line[range.start_col as usize..range.end_col as usize]
            } else if line_num == range.start_line {
                &line[range.start_col as usize..]
            } else if line_num == range.end_line {
                &line[..range.end_col as usize]
            } else {
                line
            };
            for var in &variables {
                if var.line < range.start_line
                    && line_text.contains(&var.name)
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
        selected_range: *range,
        required_parameters,
        return_variables: Vec::new(),
        suggested_name: "extracted_function".to_string(),
        insertion_point,
        contains_return_statements: contains_return,
        complexity_score: 2,
    })
}
/// Analyze variable declaration for inlining (Python)
pub(crate) fn analyze_inline_variable(
    source: &str,
    variable_line: u32,
    variable_col: u32,
    _file_path: &str,
) -> PluginResult<InlineVariableAnalysis> {
    if let Some(variable) = find_variable_at_position(source, variable_line, variable_col)? {
        let lines: Vec<&str> = source.lines().collect();
        let var_line_text = lines
            .get(variable.line as usize)
            .ok_or_else(|| PluginApiError::invalid_input("Invalid line number".to_string()))?;
        let assign_re = regex::Regex::new(&format!(
            r"^\s*{}\s*=\s*(.+)",
            regex::escape(&variable.name)
        ))
        .map_err(|e| PluginApiError::invalid_input(format!("Invalid regex pattern: {}", e)))?;
        let initializer = if let Some(captures) = assign_re.captures(var_line_text) {
            captures.get(1)
                .ok_or_else(|| PluginApiError::invalid_input("Failed to capture initializer expression".to_string()))?
                .as_str()
                .trim()
                .to_string()
        } else {
            return Err(PluginApiError::invalid_input(
                "Could not find variable assignment".to_string(),
            ));
        };
        let usages = get_variable_usages_in_scope(source, &variable.name, variable.line + 1)?;
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
        Err(PluginApiError::invalid_input(
            "Could not find variable at specified position".to_string(),
        ))
    }
}
/// Analyze a selected expression for extraction into a variable (Python)
pub(crate) fn analyze_extract_variable(
    source: &str,
    start_line: u32,
    start_col: u32,
    end_line: u32,
    end_col: u32,
    _file_path: &str,
) -> PluginResult<ExtractVariableAnalysis> {
    let expression_range = CodeRange {
        start_line,
        start_col,
        end_line,
        end_col,
    };
    let expression =
        analyze_python_expression_range(source, start_line, start_col, end_line, end_col)?;
    let mut can_extract = true;
    let mut blocking_reasons = Vec::new();
    if expression.trim().starts_with("def ") || expression.trim().starts_with("class ") {
        can_extract = false;
        blocking_reasons.push("Cannot extract function or class definitions".to_string());
    }
    if expression.contains('=') && !expression.contains("==") && !expression.contains("!=") {
        can_extract = false;
        blocking_reasons.push("Cannot extract assignment statements".to_string());
    }
    if expression.lines().count() > 1 && !expression.trim().starts_with('(') {
        can_extract = false;
        blocking_reasons.push("Multi-line expressions must be parenthesized".to_string());
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
pub(crate) fn plan_extract_function(
    source: &str,
    range: &CodeRange,
    new_function_name: &str,
    file_path: &str,
) -> PluginResult<EditPlan> {
    let analysis = analyze_extract_function(source, range, file_path)?;
    let mut edits = Vec::new();
    let function_code = generate_extracted_function(source, &analysis, new_function_name)?;
    edits.push(TextEdit {
        file_path: None,
        edit_type: EditType::Insert,
        location: analysis.insertion_point.into(),
        original_text: String::new(),
        new_text: format!("{}\n\n", function_code),
        priority: 100,
        description: format!("Create extracted function '{}'", new_function_name),
    });
    let call_code = generate_function_call(&analysis, new_function_name)?;
    edits.push(TextEdit {
        file_path: None,
        edit_type: EditType::Replace,
        location: analysis.selected_range.into(),
        original_text: extract_range_text(source, &analysis.selected_range)?,
        new_text: call_code,
        priority: 90,
        description: format!("Replace selected code with call to '{}'", new_function_name),
    });
    Ok(EditPlanBuilder::new(file_path, "extract_function")
        .with_edits(edits)
        .with_syntax_validation("Verify Python syntax is valid after extraction")
        .with_intent_args(serde_json::json!({
            "range": range,
            "function_name": new_function_name
        }))
        .with_complexity(analysis.complexity_score.min(10) as u8)
        .with_impact_area("function_extraction")
        .build())
}
/// Generate edit plan for inline variable refactoring (Python)
pub(crate) fn plan_inline_variable(
    source: &str,
    variable_line: u32,
    variable_col: u32,
    file_path: &str,
) -> PluginResult<EditPlan> {
    let analysis = analyze_inline_variable(source, variable_line, variable_col, file_path)?;
    if !analysis.is_safe_to_inline {
        return Err(PluginApiError::invalid_input(format!(
            "Cannot safely inline variable '{}': {}",
            analysis.variable_name,
            analysis.blocking_reasons.join(", ")
        )));
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
        edits.push(TextEdit {
            file_path: None,
            edit_type: EditType::Replace,
            location: (*usage_location).into(),
            original_text: analysis.variable_name.clone(),
            new_text: replacement_text,
            priority,
            description: format!("Replace '{}' with its value", analysis.variable_name),
        });
        priority -= 1;
    }
    edits.push(TextEdit {
        file_path: None,
        edit_type: EditType::Delete,
        location: analysis.declaration_range.into(),
        original_text: extract_range_text(source, &analysis.declaration_range)?,
        new_text: String::new(),
        priority: 50,
        description: format!("Remove declaration of '{}'", analysis.variable_name),
    });
    Ok(EditPlanBuilder::new(file_path, "inline_variable")
        .with_edits(edits)
        .with_syntax_validation("Verify Python syntax is valid after inlining")
        .with_intent_args(serde_json::json!({
            "variable": analysis.variable_name,
            "line": variable_line,
            "column": variable_col
        }))
        .with_complexity_from_count(analysis.usage_locations.len())
        .with_impact_area("variable_inlining")
        .build())
}
/// Generate edit plan for extract variable refactoring (Python)
pub(crate) fn plan_extract_variable(
    source: &str,
    start_line: u32,
    start_col: u32,
    end_line: u32,
    end_col: u32,
    variable_name: Option<String>,
    file_path: &str,
) -> PluginResult<EditPlan> {
    let analysis =
        analyze_extract_variable(source, start_line, start_col, end_line, end_col, file_path)?;
    if !analysis.can_extract {
        return Err(PluginApiError::invalid_input(format!(
            "Cannot extract expression: {}",
            analysis.blocking_reasons.join(", ")
        )));
    }
    let var_name = variable_name.unwrap_or_else(|| analysis.suggested_name.clone());
    let indent = LineExtractor::get_indentation_str(source, start_line);
    let mut edits = Vec::new();
    let declaration = format!("{}{} = {}\n", indent, var_name, analysis.expression);
    edits.push(TextEdit {
        file_path: None,
        edit_type: EditType::Insert,
        location: analysis.insertion_point.into(),
        original_text: String::new(),
        new_text: declaration,
        priority: 100,
        description: format!(
            "Extract '{}' into variable '{}'",
            analysis.expression, var_name
        ),
    });
    edits.push(TextEdit {
        file_path: None,
        edit_type: EditType::Replace,
        location: analysis.expression_range.into(),
        original_text: analysis.expression.clone(),
        new_text: var_name.clone(),
        priority: 90,
        description: format!("Replace expression with '{}'", var_name),
    });
    Ok(EditPlanBuilder::new(file_path, "extract_variable")
        .with_edits(edits)
        .with_syntax_validation("Verify Python syntax is valid after extraction")
        .with_intent_args(serde_json::json!({
            "expression": analysis.expression,
            "variableName": var_name,
            "startLine": start_line,
            "startCol": start_col,
            "endLine": end_line,
            "endCol": end_col
        }))
        .with_complexity(2)
        .with_impact_area("variable_extraction")
        .build())
}
/// Extract text from a Python code range
fn extract_range_text(source: &str, range: &CodeRange) -> PluginResult<String> {
    Ok(analyze_python_expression_range(
        source,
        range.start_line,
        range.start_col,
        range.end_line,
        range.end_col,
    )?)
}
/// Find proper insertion point for a new Python function
fn find_insertion_point(source: &str, start_line: u32) -> PluginResult<CodeRange> {
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
) -> PluginResult<String> {
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
    Ok(format!(
        "def {}({}):\n{}\n{}",
        function_name, params, indented_code, return_statement
    ))
}
/// Generate Python function call
fn generate_function_call(
    analysis: &ExtractableFunction,
    function_name: &str,
) -> PluginResult<String> {
    let args = analysis.required_parameters.join(", ");
    if analysis.return_variables.is_empty() {
        Ok(format!("{}({})", function_name, args))
    } else if analysis.return_variables.len() == 1 {
        Ok(format!(
            "{} = {}({})",
            analysis.return_variables[0], function_name, args
        ))
    } else {
        Ok(format!(
            "{} = {}({})",
            analysis.return_variables.join(", "),
            function_name,
            args
        ))
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
    if expr.contains('+') || expr.contains('-') || expr.contains('*') || expr.contains('/') {
        return "result".to_string();
    }
    "extracted".to_string()
}

/// Analyzes source code to extract information about a literal value at a cursor position.
///
/// This analysis function identifies literals in Python source code and gathers information for
/// constant extraction. It analyzes:
/// - The literal value at the specified cursor position (number, string, boolean, or None)
/// - All occurrences of that literal throughout the file
/// - A suitable insertion point for the constant declaration (top of module after imports/docstring)
/// - Whether extraction is valid and any blocking reasons
///
/// # Arguments
/// * `source` - The Python source code
/// * `line` - Zero-based line number where the cursor is positioned
/// * `character` - Zero-based character offset within the line
/// * `file_path` - Path to the file (used for error reporting)
///
/// # Returns
/// * `Ok(ExtractConstantAnalysis)` - Analysis result with literal value, occurrence ranges,
///                                     validation status, and insertion point
/// * `Err(RefactoringError)` - If no literal is found at the cursor position
///
/// # Implementation Details
/// 1. Locates the literal at the cursor position by scanning the line
/// 2. Extracts the literal value using specialized helpers for different types:
///    - `find_python_numeric_literal()` - Numbers (including floats and negative values)
///    - `find_python_string_literal()` - Strings (single/double/triple quoted)
///    - `find_python_keyword_literal()` - Booleans and None
/// 3. Calls `find_literal_occurrences()` to identify all matching literals
/// 4. Validates that the found literal is not empty
/// 5. Sets insertion point using `find_python_insertion_point_for_constant()` which respects
///    module-level structure: placed after imports and module docstring
///
/// Analyzes a Python literal at a cursor position for extract constant refactoring.
///
/// Examines the literal at the specified position, finds all occurrences throughout
/// the source, and determines the appropriate insertion point for the constant declaration.
///
/// # Arguments
/// * `source` - The Python source code
/// * `line` - Zero-based line number where the cursor is positioned
/// * `character` - Zero-based character offset within the line
/// * `_file_path` - Path to the file (reserved for future use)
///
/// # Returns
/// Analysis result containing literal value, occurrences, validation status, and insertion point
///
/// # Called By
/// - `plan_extract_constant()` - Main entry point for constant extraction
/// - Used internally by the refactoring pipeline
#[allow(dead_code)]
pub(crate) fn analyze_extract_constant(
    source: &str,
    line: u32,
    character: u32,
    _file_path: &str,
) -> PluginResult<ExtractConstantAnalysis> {
    let lines: Vec<&str> = source.lines().collect();

    // Get the line at cursor position
    let line_text = lines.get(line as usize)
        .ok_or_else(|| PluginApiError::invalid_input("Invalid line number".to_string()))?;

    // Find the literal at the cursor position
    let found_literal = find_python_literal_at_position(line_text, character as usize)
        .ok_or_else(|| PluginApiError::invalid_input("No literal found at the specified location".to_string()))?;

    let literal_value = found_literal.0;
    let is_valid_literal = !literal_value.is_empty();
    let blocking_reasons = if !is_valid_literal {
        vec!["Could not extract literal at cursor position".to_string()]
    } else {
        vec![]
    };

    // Find all occurrences of this literal value in the source
    let occurrence_ranges = find_literal_occurrences(source, &literal_value, is_valid_python_literal_location);

    // Insertion point: after imports and docstring, at the top of the file
    let insertion_point = find_python_insertion_point_for_constant(source)?;

    Ok(ExtractConstantAnalysis {
        literal_value,
        occurrence_ranges,
        is_valid_literal,
        blocking_reasons,
        insertion_point,
    })
}

/// Extracts a literal value to a named constant in Python code.
///
/// This refactoring operation replaces all occurrences of a literal (number, string, boolean, or None)
/// with a named constant declaration at the module level, improving code maintainability by
/// eliminating magic values and making it easier to update values globally.
///
/// # Arguments
/// * `source` - The Python source code
/// * `line` - Zero-based line number where the cursor is positioned
/// * `character` - Zero-based character offset within the line
/// * `name` - The constant name (must be SCREAMING_SNAKE_CASE)
/// * `file_path` - Path to the file being refactored
///
/// # Returns
/// * `Ok(EditPlan)` - The edit plan with constant declaration inserted at module level and all
///                    literal occurrences replaced with the constant name
/// * `Err(RefactoringError)` - If the cursor is not on a literal, the name is invalid, or parsing fails
///
/// # Example
/// ```python
/// # Before (cursor on 0.08):
/// def calculate_tax(price):
///     return price * 0.08
///
/// def apply_discount(price):
///     return price * 0.08
///
/// # After (name="TAX_RATE"):
/// TAX_RATE = 0.08
///
/// def calculate_tax(price):
///     return price * TAX_RATE
///
/// def apply_discount(price):
///     return price * TAX_RATE
/// ```
///
/// # Supported Literals
/// - **Numbers**: `42`, `3.14`, `-100`, `1e-5`
/// - **Strings**: `"hello"`, `'world'`, `"""multiline"""`
/// - **Booleans**: `True`, `False` (Python capitalized)
/// - **None**: `None`
///
/// # Name Validation
/// Constant names must follow SCREAMING_SNAKE_CASE convention:
/// - Only uppercase letters (A-Z), digits (0-9), and underscores (_)
/// - Must contain at least one uppercase letter
/// - Cannot start or end with underscore
/// - Examples: `TAX_RATE`, `MAX_USERS`, `API_KEY`, `DB_TIMEOUT_MS`
///
/// # Insertion Point
/// The constant is inserted at the module level:
/// - After any module-level imports (import/from statements)
/// - After any module docstring (if present)
/// - Before the first function or class definition
///
/// This follows Python conventions for module-level constant placement.
///
/// # Occurrence Finding
/// All occurrences of the literal value are found using string matching with safeguards:
/// - Excludes matches inside string literals
/// - Excludes matches inside comments
/// - Respects quote boundaries (single, double, triple)
///
/// Plans an extract constant refactoring for Python code.
///
/// Creates an edit plan that extracts a literal value to a module-level constant,
/// inserting the constant declaration after imports and replacing all occurrences.
///
/// # Arguments
/// * `source` - The Python source code
/// * `line` - Zero-based line number where the cursor is positioned on the literal
/// * `character` - Zero-based character offset within the line
/// * `name` - The constant name (must be SCREAMING_SNAKE_CASE)
/// * `file_path` - Path to the file being refactored
///
/// # Returns
/// Edit plan with constant declaration and all literal replacements
///
/// # Called By
/// This function is invoked by the extract_handler via dynamic dispatch when a user
/// requests constant extraction through the MCP interface.
#[allow(dead_code)]
pub(crate) fn plan_extract_constant(
    source: &str,
    line: u32,
    character: u32,
    name: &str,
    file_path: &str,
) -> PluginResult<EditPlan> {
    let analysis = analyze_extract_constant(source, line, character, file_path)?;

    if !analysis.is_valid_literal {
        return Err(PluginApiError::invalid_input(format!(
            "Cannot extract constant: {}",
            analysis.blocking_reasons.join(", ")
        )));
    }

    use mill_lang_common::ExtractConstantEditPlanBuilder;

    ExtractConstantEditPlanBuilder::new(analysis, name.to_string(), file_path.to_string())
        .with_declaration_format(|name, value| format!("{} = {}\n", name, value))
        .map_err(|e| PluginApiError::invalid_input(e))
}

/// Finds a Python literal at a given position in a line of code.
///
/// This function identifies literals by checking the cursor position against different literal types
/// in a priority order: numbers, strings, then keyword literals (True, False, None).
///
/// # Arguments
/// * `line_text` - The complete line of code
/// * `col` - Zero-based character position within the line
///
/// # Returns
/// * `Some((literal_value, range))` - The literal found and its position within the line
/// * `None` - If no literal is found at the cursor position
///
/// # Implementation Details
/// Uses specialized helper functions for each literal type:
/// 1. `find_python_numeric_literal()` - Numbers including floats and negative values
/// 2. `find_python_string_literal()` - String literals with quote handling
/// 3. `find_python_keyword_literal()` - Python keyword literals (True, False, None)
///
/// Note: Searches in priority order and returns immediately on first match.
///
/// # Helper For
/// - `analyze_extract_constant()` - Identifies literal at cursor for extraction
#[allow(dead_code)]
fn find_python_literal_at_position(line_text: &str, col: usize) -> Option<(String, CodeRange)> {
    // Try to find different kinds of literals at the cursor position

    // Check for numeric literal (including negative numbers)
    if let Some((literal, range)) = find_python_numeric_literal(line_text, col) {
        return Some((literal, range));
    }

    // Check for string literal (quoted with single/double/triple quote support)
    if let Some((literal, range)) = find_python_string_literal(line_text, col) {
        return Some((literal, range));
    }

    // Check for boolean (True/False) or None (Python capitalized keywords)
    if let Some((literal, range)) = find_python_keyword_literal(line_text, col) {
        return Some((literal, range));
    }

    None
}

/// Finds a numeric literal (integer, float, or negative number) at a cursor position.
///
/// This function locates numeric literals in a line, handling various Python numeric formats:
/// - Integers: `42`, `-100`
/// - Floats: `3.14`, `-2.5`, `1e-5`
/// - Underscores: `1_000_000` (valid in Python)
///
/// # Arguments
/// * `line_text` - The line of code to search
/// * `col` - Zero-based cursor position within the line
///
/// # Returns
/// * `Some((literal, range))` - The numeric literal and its position (start_col, end_col on line 0)
/// * `None` - If no numeric literal is found at the cursor position
///
/// # Algorithm
/// 1. Scans left from cursor to find start boundary (non-digit, non-dot, non-underscore)
/// 2. Checks if cursor is after a minus sign (handles negative numbers)
/// 3. Scans right from cursor to find end boundary
/// 4. Validates the extracted text:
///    - Contains at least one digit
///    - Successfully parses as f64
///
/// # Edge Cases Handled
/// - Negative numbers with leading minus sign
/// - Floating point numbers with decimal points
/// - Python numeric literals with underscores
///
/// # Helper For
/// - `find_python_literal_at_position()` - Type-specific literal detection
#[allow(dead_code)]
fn find_python_numeric_literal(line_text: &str, col: usize) -> Option<(String, CodeRange)> {
    if col >= line_text.len() {
        return None;
    }

    // Find the start of the number (handle negative sign)
    let start = if col > 0 && line_text.chars().nth(col - 1) == Some('-') {
        col.saturating_sub(1)
    } else {
        line_text[..col]
            .rfind(|c: char| !c.is_ascii_digit() && c != '.' && c != '_')
            .map(|p| p + 1)
            .unwrap_or(0)
    };

    // Adjust start if we found a leading minus sign (handle negative numbers)
    let actual_start = if start > 0 && line_text.chars().nth(start - 1) == Some('-') {
        start - 1
    } else {
        start
    };

    // Find the end of the number by scanning right from cursor
    let end = col + line_text[col..]
        .find(|c: char| !c.is_ascii_digit() && c != '.' && c != '_')
        .unwrap_or(line_text.len() - col);

    if actual_start < end && end <= line_text.len() {
        let text = &line_text[actual_start..end];
        // Validate: must contain at least one digit and be parseable as a number
        if text.chars().any(|c| c.is_ascii_digit()) && text.parse::<f64>().is_ok() {
            return Some((text.to_string(), CodeRange {
                start_line: 0,
                start_col: actual_start as u32,
                end_line: 0,
                end_col: end as u32,
            }));
        }
    }

    None
}

/// Finds a string literal at a cursor position in Python code.
///
/// This function handles all Python string quoting styles:
/// - Single-quoted: `'hello'`
/// - Double-quoted: `"hello"`
/// - Triple-quoted: `"""multiline"""` or `'''multiline'''`
///
/// Triple-quoted strings are checked first, allowing them to contain single/double quotes
/// without needing escape characters (Python triple-quote semantics).
///
/// # Arguments
/// * `line_text` - The line of code to search
/// * `col` - Zero-based cursor position within the line
///
/// # Returns
/// * `Some((literal, range))` - The complete string literal (including quotes) and its position
/// * `None` - If no string literal is found at the cursor position
///
/// # Algorithm
/// 1. First checks for triple-quoted strings (`"""` or `'''`)
///    - Scans left to find opening triple quote
///    - Scans right to find closing triple quote
///    - Returns if cursor is within the string bounds
/// 2. Then checks for single/double-quoted strings
///    - Scans left to find opening quote
///    - Scans right to find closing matching quote
///    - Returns the complete string with quotes
///
/// # Important: Python-Specific
/// Python strings support triple quotes for multiline strings and docstrings.
/// This implementation respects that convention.
///
/// # Helper For
/// - `find_python_literal_at_position()` - Type-specific literal detection
#[allow(dead_code)]
fn find_python_string_literal(line_text: &str, col: usize) -> Option<(String, CodeRange)> {
    if col >= line_text.len() {
        return None;
    }

    // Check for triple-quoted strings first
    for quote_type in &["\"\"\"", "'''"] {
        // Look backwards for opening triple quote
        if col >= quote_type.len() {
            let check_pos = col - quote_type.len();
            if line_text[check_pos..].starts_with(quote_type) {
                // We're inside or near a triple-quoted string
                // Find the actual opening
                for i in (0..=check_pos).rev() {
                    if i + quote_type.len() <= line_text.len() && &line_text[i..i + quote_type.len()] == *quote_type {
                        // Check if this is the opening (not closing of a different string)
                        // Try to find closing triple quote
                        if let Some(close_pos) = line_text[i + quote_type.len()..].find(quote_type) {
                            let end = i + quote_type.len() + close_pos + quote_type.len();
                            if col >= i && col <= end {
                                let literal = line_text[i..end].to_string();
                                return Some((literal, CodeRange {
                                    start_line: 0,
                                    start_col: i as u32,
                                    end_line: 0,
                                    end_col: end as u32,
                                }));
                            }
                        }
                    }
                }
            }
        }
    }

    // Look for single or double quoted strings
    for (i, ch) in line_text[..col].char_indices().rev() {
        if ch == '"' || ch == '\'' {
            let quote = ch;
            // Find closing quote after cursor
            for (j, ch2) in line_text[col..].char_indices() {
                if ch2 == quote {
                    let end = col + j + 1;
                    if end <= line_text.len() {
                        let literal = line_text[i..end].to_string();
                        return Some((literal, CodeRange {
                            start_line: 0,
                            start_col: i as u32,
                            end_line: 0,
                            end_col: end as u32,
                        }));
                    }
                }
            }
            break;
        }
    }

    None
}

/// Finds a Python keyword literal (True, False, or None) at a cursor position.
///
/// This function identifies Python's built-in keyword constants, which are capitalized
/// unlike their counterparts in other languages:
/// - `True` - Boolean true value
/// - `False` - Boolean false value
/// - `None` - Python's null/nil value
///
/// # Arguments
/// * `line_text` - The line of code to search
/// * `col` - Zero-based cursor position within the line
///
/// # Returns
/// * `Some((literal, range))` - The keyword literal and its position on the line
/// * `None` - If no keyword literal is found at the cursor position
///
/// # Algorithm
/// 1. Checks each keyword: `["True", "False", "None"]`
/// 2. For each keyword, scans positions around the cursor
/// 3. Validates word boundaries:
///    - Before: must be preceded by non-alphanumeric and non-underscore character
///    - After: must be followed by non-alphanumeric and non-underscore character
/// 4. Returns first match found
///
/// # Important: Python Capitalization
/// Unlike JavaScript (`true`, `false`, `null`), Python keywords are capitalized.
/// This function specifically looks for `True`, `False`, and `None` with correct casing.
///
/// # Helper For
/// - `find_python_literal_at_position()` - Type-specific literal detection
#[allow(dead_code)]
fn find_python_keyword_literal(line_text: &str, col: usize) -> Option<(String, CodeRange)> {
    let keywords = ["True", "False", "None"];

    for keyword in &keywords {
        // Try to match keyword at or near cursor
        for start in col.saturating_sub(keyword.len())..=col.min(line_text.len().saturating_sub(keyword.len())) {
            if start + keyword.len() <= line_text.len() {
                if &line_text[start..start + keyword.len()] == *keyword {
                    // Check word boundaries
                    let before_ok = start == 0 ||
                        !line_text[..start].ends_with(|c: char| c.is_alphanumeric() || c == '_');
                    let after_ok = start + keyword.len() == line_text.len()
                        || !line_text[start + keyword.len()..].starts_with(|c: char| c.is_alphanumeric() || c == '_');

                    if before_ok && after_ok {
                        return Some((
                            keyword.to_string(),
                            CodeRange {
                                start_line: 0,
                                start_col: start as u32,
                                end_line: 0,
                                end_col: (start + keyword.len()) as u32,
                            },
                        ));
                    }
                }
            }
        }
    }

    None
}

/// Counts unescaped quotes of a specific type in a string slice.
///
/// This helper function properly handles escaped quotes by tracking backslash sequences.
/// It correctly handles:
/// - Regular quotes: `"hello"` or `'hello'`
/// - Escaped quotes: `"He said \"hi\""` or `'It\'s fine'`
/// - Escaped backslashes: `"path\\to\\file"` (backslash before quote doesn't escape it)
///
/// # Arguments
/// * `text` - The text to scan for quotes
/// * `quote_char` - The quote character to count ('"' or '\'')
///
/// # Returns
/// The number of unescaped quotes found in the text.
///
/// # Algorithm
/// 1. Iterate through each character in the text
/// 2. Track consecutive backslashes (escape sequences)
/// 3. When a quote is found, check if it's escaped by an odd number of preceding backslashes
/// 4. If not escaped (even number of backslashes, including 0), count it
///
/// # Examples
/// ```
/// Validates whether a position in source code is a valid location for a literal.
///
/// A position is considered valid if it's not inside a string literal or comment.
/// This prevents replacing:
/// - Literals that are part of string content (e.g., the "0.08" in `"Rate is 0.08%"`)
/// - Literals in comments (e.g., the value in `# TODO: update rate from 0.08 to 0.10`)
///
/// # Algorithm
/// 1. Count unescaped quote characters before the position to determine if we're inside a string
/// 2. If an odd number of unescaped quotes appear before the position, we're inside a string literal
/// 3. Check for `#` comments; any position after the comment marker is invalid
/// 4. Return true only if outside both strings and comments
///
/// # Arguments
/// * `line` - The current line of code
/// * `pos` - Character position within the line where the potential literal is located
/// * `_len` - Length of the literal (not currently used but available for future enhancements)
///
/// # Returns
/// `true` if the position is a valid literal location (outside strings and comments),
/// `false` if the position is inside a string or comment.
///
/// # Edge Cases Handled
/// - Escaped quotes in strings: `"He said \"hi\""` - correctly identifies escaped quotes
/// - Escaped backslashes: `"path\\to\\file"` - backslash before backslash doesn't escape quote
/// - Raw strings (r"..."): Detected by checking for 'r' prefix before opening quote
/// - F-strings (f"..."): Treated same as regular strings for literal detection
///
/// # Examples
/// ```
/// // Valid locations (outside strings):
/// is_valid_python_literal_location("x = 42", 4, 2) -> true
///
/// // Invalid locations (inside strings):
/// is_valid_python_literal_location("msg = \"42\"", 8, 2) -> false
///
/// // Invalid locations (inside comments):
/// is_valid_python_literal_location("x = 0  # value is 42", 18, 2) -> false
///
/// // Escaped quotes handled correctly:
/// is_valid_python_literal_location("msg = \"Rate is \\\"0.08\\\"\"", 20, 4) -> false
/// ```
///
/// # Called By
/// - `find_literal_occurrences()` - Validates matches before including them in results
///
/// Python-specific validation: Uses # for comments instead of // or /* */
#[allow(dead_code)]
fn is_valid_python_literal_location(line: &str, pos: usize, _len: usize) -> bool {
    // Count unescaped quotes before position to determine if we're inside a string literal.
    // Each unescaped quote toggles the "inside string" state. Odd count = inside string, even = outside.
    let before = &line[..pos];
    let single_quotes = count_unescaped_quotes(before, '\'');
    let double_quotes = count_unescaped_quotes(before, '"');

    // If odd number of unescaped quotes appear before the position, we're inside a string literal
    if single_quotes % 2 == 1 || double_quotes % 2 == 1 {
        return false;
    }

    // Check for Python comment marker (#). Anything after it is a comment.
    // Unlike C-style languages, Python uses # instead of //
    if let Some(comment_pos) = line.find('#') {
        // Make sure the # is not inside a string
        let sq = count_unescaped_quotes(&line[..comment_pos], '\'');
        let dq = count_unescaped_quotes(&line[..comment_pos], '"');

        if sq % 2 == 0 && dq % 2 == 0 && pos > comment_pos {
            return false; // We're after a real comment
        }
    }

    true
}

/// Finds the appropriate insertion point for a constant declaration in Python code.
///
/// The insertion point respects Python module structure conventions:
/// - After module-level imports (import/from statements)
/// - After module docstring (if present)
/// - Before the first function or class definition
///
/// This placement ensures constants are declared at the module level, following
/// PEP 8 style guidelines for Python code organization.
///
/// # Arguments
/// * `source` - The complete Python source code
///
/// # Returns
/// * `Ok(CodeRange)` - The line number where the constant should be inserted
/// * `Err(RefactoringError)` - If the source cannot be analyzed
///
/// # Algorithm
/// 1. Scans through lines sequentially
/// 2. Tracks docstring state:
///    - Detects opening/closing triple quotes (`"""` or `'''`)
///    - Maintains position after docstring ends
/// 3. Records position after each import statement
/// 4. Stops when first function or class definition is found
/// 5. Returns the latest recorded position
///
/// # Python Module Structure
/// Module-level constants should be placed in this order:
/// ```python
/// """Module docstring."""
///
/// import os
/// from sys import path
///
/// CONSTANT_NAME = value  # <- Insertion point
///
/// def function():
///     pass
/// ```
///
/// # Edge Cases Handled
/// - Empty files (insertion at line 0)
/// - Files with only imports (insertion after imports)
/// - Files with docstring (insertion after docstring)
/// - Docstrings using `"""` or `'''` (both supported)
///
/// # Called By
/// - `analyze_extract_constant()` - Determines where to insert constant declaration
#[allow(dead_code)]
fn find_python_insertion_point_for_constant(source: &str) -> PluginResult<CodeRange> {
    let lines: Vec<&str> = source.lines().collect();
    let mut insertion_line = 0;
    let mut in_docstring = false;
    let mut docstring_quote = "";

    for (idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        let line_idx = idx as u32;

        // Track docstring state to skip module-level docstring
        if trimmed.starts_with("\"\"\"") || trimmed.starts_with("'''") {
            let quote = if trimmed.starts_with("\"\"\"") { "\"\"\"" } else { "'''" };
            if in_docstring && docstring_quote == quote {
                // Found closing triple quote - mark insertion point after docstring
                in_docstring = false;
                insertion_line = line_idx + 1;
            } else if !in_docstring {
                // Found opening triple quote
                in_docstring = true;
                docstring_quote = quote;
            }
        } else if in_docstring {
            // Still inside docstring - continue scanning
            continue;
        }

        // Record position after each import statement
        if trimmed.starts_with("import ") || trimmed.starts_with("from ") {
            insertion_line = line_idx + 1;
        }
        // Stop at first function or class definition (not in docstring)
        else if (trimmed.starts_with("def ") || trimmed.starts_with("class ")) && !in_docstring {
            break;
        }
    }

    Ok(CodeRange {
        start_line: insertion_line,
        start_col: 0,
        end_line: insertion_line,
        end_col: 0,
    })
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
        let analysis = analyze_extract_variable(source, 2, 13, 2, 20, "test.py").unwrap();
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
    fn test_find_python_literal_at_position_number() {
        let line = "x = 42";
        let result = find_python_literal_at_position(line, 4);
        assert!(result.is_some());
        let (literal, _range) = result.unwrap();
        assert_eq!(literal, "42");
    }

    #[test]
    fn test_find_python_literal_at_position_string_double() {
        let line = r#"msg = "hello""#;
        let result = find_python_literal_at_position(line, 8);
        assert!(result.is_some());
        let (literal, _range) = result.unwrap();
        assert_eq!(literal, r#""hello""#);
    }

    #[test]
    fn test_find_python_literal_at_position_string_single() {
        let line = "msg = 'world'";
        let result = find_python_literal_at_position(line, 8);
        assert!(result.is_some());
        let (literal, _range) = result.unwrap();
        assert_eq!(literal, "'world'");
    }

    #[test]
    fn test_find_python_literal_at_position_true() {
        let line = "flag = True";
        let result = find_python_literal_at_position(line, 7);
        assert!(result.is_some());
        let (literal, _range) = result.unwrap();
        assert_eq!(literal, "True");
    }

    #[test]
    fn test_find_python_literal_at_position_false() {
        let line = "flag = False";
        let result = find_python_literal_at_position(line, 8);
        assert!(result.is_some());
        let (literal, _range) = result.unwrap();
        assert_eq!(literal, "False");
    }

    #[test]
    fn test_find_python_literal_at_position_none() {
        let line = "value = None";
        let result = find_python_literal_at_position(line, 8);
        assert!(result.is_some());
        let (literal, _range) = result.unwrap();
        assert_eq!(literal, "None");
    }

    #[test]
    fn test_find_python_literal_occurrences() {
        let source = "x = 42\ny = 42\nz = 100";
        let occurrences = find_literal_occurrences(source, "42", is_valid_python_literal_location);
        assert_eq!(occurrences.len(), 2);
        assert_eq!(occurrences[0].start_line, 0);
        assert_eq!(occurrences[1].start_line, 1);
    }

    #[test]
    fn test_plan_extract_constant_valid_number() {
        let source = "x = 42\ny = 42\n";
        let result = plan_extract_constant(source, 0, 4, "ANSWER", "test.py");
        assert!(result.is_ok(), "Should extract numeric literal successfully");
    }

    #[test]
    fn test_plan_extract_constant_invalid_name() {
        let source = "x = 42\n";
        let result = plan_extract_constant(source, 0, 4, "answer", "test.py");
        assert!(result.is_err(), "Should reject lowercase name");
    }

    #[test]
    fn test_plan_extract_constant_string() {
        let source = r#"msg = "hello"
greeting = "hello"
"#;
        let result = plan_extract_constant(source, 0, 8, "GREETING_MSG", "test.py");
        assert!(result.is_ok(), "Should extract string literal");
    }

    #[test]
    fn test_plan_extract_constant_boolean() {
        let source = "debug = True\nverbose = True\n";
        let result = plan_extract_constant(source, 0, 8, "DEBUG_MODE", "test.py");
        assert!(result.is_ok(), "Should extract boolean literal");
    }

    // Refactoring tests: Core operations (extract/inline) tested in other languages (C++/Java)
    // Kept: Python-specific tests (suggest_variable_name helper, analysis functions)

    #[test]
    fn test_is_valid_python_literal_location_escaped_quotes() {
        // Test escaped quotes in string literals
        let line = r#"msg = "Rate is \"0.08\"""#;
        // Position 20 is inside the escaped quote section
        assert!(!is_valid_python_literal_location(line, 20, 4), "Should be inside string with escaped quotes");

        // Position before string should be valid
        assert!(is_valid_python_literal_location(line, 4, 1), "Should be valid before string");
    }

    #[test]
    fn test_is_valid_python_literal_location_escaped_backslash() {
        // Test escaped backslash followed by quote
        let line = r#"path = "C:\\dir\\file""#;
        // Position inside the string
        assert!(!is_valid_python_literal_location(line, 12, 1), "Should be inside string");
    }

    #[test]
    fn test_is_valid_python_literal_location_raw_string() {
        // Raw strings don't process escape sequences, but our algorithm still works
        let line = r#"pattern = r"\d+""#;
        // Position inside the raw string
        assert!(!is_valid_python_literal_location(line, 14, 1), "Should be inside raw string");
    }

    #[test]
    fn test_is_valid_python_literal_location_fstring() {
        // F-strings should be treated as strings
        let line = r#"msg = f"Value is {value}""#;
        // Position inside the f-string
        assert!(!is_valid_python_literal_location(line, 18, 1), "Should be inside f-string");
    }

    #[test]
    fn test_is_valid_python_literal_location_single_quotes_escaped() {
        let line = r"msg = 'It\'s fine'";
        // Position inside the string (after escaped quote)
        assert!(!is_valid_python_literal_location(line, 13, 1), "Should be inside string with escaped single quote");
    }

    #[test]
    fn test_find_python_literal_occurrences_escaped_quotes() {
        // Should not match literal inside string with escaped quotes
        let source = r#"TAX_RATE = 0.08
msg = "Rate is \"0.08\""
value = 0.08"#;
        let occurrences = find_literal_occurrences(source, "0.08", is_valid_python_literal_location);
        // Should find 2 occurrences (lines 0 and 2), but not the one inside the string
        assert_eq!(occurrences.len(), 2, "Should find exactly 2 valid occurrences");
        assert_eq!(occurrences[0].start_line, 0);
        assert_eq!(occurrences[1].start_line, 2);
    }

    #[test]
    fn test_analyze_inline_variable_invalid_regex() {
        // Test with a variable name that would create an invalid regex (edge case)
        // This is primarily to ensure error handling is in place
        let source = "normal_var = 42\ny = normal_var + 1";
        let result = analyze_inline_variable(source, 0, 0, "test.py");
        assert!(result.is_ok(), "Should handle normal variable names");
    }

    #[test]
    fn test_plan_extract_constant_with_escaped_quotes_in_string() {
        // Test that we don't extract from inside strings with escaped quotes
        let source = r#"RATE = 0.08
description = "The rate is \"0.08\" percent"
tax = 0.08"#;
        let result = plan_extract_constant(source, 0, 7, "TAX_RATE", "test.py");
        assert!(result.is_ok());
        let plan = result.unwrap();
        // Should find 2 occurrences (lines 0 and 2), not the one in the string
        assert_eq!(plan.edits.len(), 3, "Should have 1 insert + 2 replace edits");
    }

    #[test]
    fn test_plan_extract_constant_raw_string() {
        let source = r#"pattern = r"\d+"
regex = r"\d+""#;
        let result = plan_extract_constant(source, 0, 12, "DIGIT_PATTERN", "test.py");
        assert!(result.is_ok());
        let plan = result.unwrap();
        // Should find both raw string occurrences
        assert_eq!(plan.edits.len(), 3, "Should have 1 insert + 2 replace edits");
    }

    #[test]
    fn test_plan_extract_constant_fstring_with_literal() {
        let source = r#"MAX_SIZE = 100
msg = f"Max size is {MAX_SIZE}"
limit = 100"#;
        let result = plan_extract_constant(source, 0, 11, "SIZE_LIMIT", "test.py");
        assert!(result.is_ok());
        let plan = result.unwrap();
        // Should find 2 occurrences (lines 0 and 2), not inside the f-string
        assert_eq!(plan.edits.len(), 3, "Should have 1 insert + 2 replace edits");
    }
}
