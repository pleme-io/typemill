//! TypeScript/JavaScript specific refactoring logic.
use mill_foundation::protocol::{EditPlan, EditType, TextEdit};
use mill_lang_common::refactoring::edit_plan_builder::EditPlanBuilder;
use mill_lang_common::{
    find_literal_occurrences, is_escaped, is_valid_code_literal_location, CodeRange,
    ExtractConstantAnalysis, ExtractVariableAnalysis, ExtractableFunction, InlineVariableAnalysis,
    LineExtractor,
};
use mill_plugin_api::{PluginApiError, PluginResult};
use std::path::PathBuf;
use swc_common::{sync::Lrc, FileName, FilePathMapping, SourceMap};
use swc_ecma_ast::*;
use swc_ecma_parser::{lexer::Lexer, Parser, StringInput, Syntax, TsSyntax};
use swc_ecma_visit::{Visit, VisitWith};

/// Plans an extract function refactoring for TypeScript/JavaScript code.
///
/// Creates an edit plan that extracts selected lines into a new function,
/// replacing the original code with a function call.
///
/// # Arguments
/// * `source` - The TypeScript/JavaScript source code
/// * `start_line` - Zero-based starting line of the selection
/// * `end_line` - Zero-based ending line of the selection
/// * `new_function_name` - Name for the extracted function
/// * `file_path` - Path to the file being refactored
///
/// # Returns
/// Edit plan with function creation and call site replacement
pub fn plan_extract_function(
    source: &str,
    start_line: u32,
    end_line: u32,
    new_function_name: &str,
    file_path: &str,
) -> PluginResult<EditPlan> {
    let range = CodeRange {
        start_line,
        start_col: 0, // Simplified for now
        end_line,
        end_col: source.lines().nth(end_line as usize).unwrap_or("").len() as u32, // Simplified
    };
    ast_extract_function_ts_js(source, &range, new_function_name, file_path)
}

/// Plans an inline variable refactoring for TypeScript/JavaScript code.
///
/// Creates an edit plan that replaces all usages of a variable with its initializer value,
/// then removes the variable declaration.
///
/// # Arguments
/// * `source` - The TypeScript/JavaScript source code
/// * `variable_line` - Zero-based line number where the variable is declared
/// * `variable_col` - Zero-based character offset of the variable name
/// * `file_path` - Path to the file being refactored
///
/// # Returns
/// Edit plan with variable replacements and declaration removal
pub fn plan_inline_variable(
    source: &str,
    variable_line: u32,
    variable_col: u32,
    file_path: &str,
) -> PluginResult<EditPlan> {
    let analysis = analyze_inline_variable(source, variable_line, variable_col, file_path)?;
    ast_inline_variable_ts_js(source, &analysis)
}

/// Plans an extract variable refactoring for TypeScript/JavaScript code.
///
/// Creates an edit plan that extracts a selected expression into a const variable,
/// replacing the original expression with the variable name.
///
/// # Arguments
/// * `source` - The TypeScript/JavaScript source code
/// * `start_line` - Zero-based starting line of the expression
/// * `start_col` - Zero-based starting column of the expression
/// * `end_line` - Zero-based ending line of the expression
/// * `end_col` - Zero-based ending column of the expression
/// * `variable_name` - Optional name for the variable (auto-generated if None)
/// * `file_path` - Path to the file being refactored
///
/// # Returns
/// Edit plan with variable declaration and expression replacement
pub fn plan_extract_variable(
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
    ast_extract_variable_ts_js(source, &analysis, variable_name, file_path)
}

/// Extracts a literal value to a named constant across the entire file.
///
/// This refactoring operation replaces all occurrences of a literal (number, string, boolean, or null)
/// with a named constant declaration at the top of the file, improving code maintainability by
/// eliminating magic values and making it easier to update values globally.
///
/// # Arguments
/// * `source` - The TypeScript/JavaScript source code
/// * `line` - Zero-based line number where the cursor is positioned
/// * `character` - Zero-based character offset within the line
/// * `name` - The constant name (must be SCREAMING_SNAKE_CASE)
/// * `file_path` - Path to the file being refactored
///
/// # Returns
/// * `Ok(EditPlan)` - The edit plan with constant declaration inserted at the top and all
///   literal occurrences replaced with the constant name
/// * `Err(PluginError)` - If the cursor is not on a literal, the name is invalid, or parsing fails
///
/// # Example
/// ```typescript
/// // Before (cursor on 0.08):
/// function calculateTax(price: number): number {
///   return price * 0.08;
/// }
///
/// function applyDiscount(price: number): number {
///   return price * 0.08;
/// }
///
/// // After (name="TAX_RATE"):
/// const TAX_RATE = 0.08;
///
/// function calculateTax(price: number): number {
///   return price * TAX_RATE;
/// }
///
/// function applyDiscount(price: number): number {
///   return price * TAX_RATE;
/// }
/// ```
///
/// # Supported Literals
/// - **Numbers**: `42`, `3.14`, `-100`, `1e-5`
/// - **Strings**: `"hello"`, `'world'`, `` `template` ``
/// - **Booleans**: `true`, `false`
/// - **Null**: `null`
///
/// # Name Validation
/// Constant names must follow SCREAMING_SNAKE_CASE convention:
/// - Only uppercase letters (A-Z), digits (0-9), and underscores (_)
/// - Must contain at least one uppercase letter
/// - Cannot start or end with underscore
/// - Examples: `TAX_RATE`, `MAX_USERS`, `API_KEY`, `DB_TIMEOUT_MS`
///
/// # Literal Detection Strategy
/// The function uses the SWC parser to identify the AST position, then scans the source code
/// at the cursor position to find the literal's exact boundaries. This hybrid approach ensures
/// accuracy across different literal types (numeric, string, boolean, null).
///
/// # Occurrence Finding
/// All occurrences of the literal value are found using string matching with safeguards:
/// - Excludes matches inside string literals and comments
/// - Respects quote boundaries (single, double, backtick)
/// - Avoids replacing literals inside `//` comments
///
/// # Called By
/// This function is invoked by the extract_handler via dynamic dispatch when a user
/// requests constant extraction through the MCP interface.
#[allow(dead_code)]
pub fn plan_extract_constant(
    source: &str,
    line: u32,
    character: u32,
    name: &str,
    file_path: &str,
) -> PluginResult<EditPlan> {
    let analysis = analyze_extract_constant(source, line, character, file_path)?;
    ast_extract_constant_ts_js(source, &analysis, name, file_path)
}

fn ast_extract_function_ts_js(
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
        new_text: format!("\n{}\n", function_code),
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
        .with_syntax_validation("Verify syntax is valid after extraction")
        .with_type_check_validation()
        .with_intent_args(serde_json::json!({
            "range": range,
            "function_name": new_function_name
        }))
        .with_complexity(analysis.complexity_score.min(10) as u8)
        .with_impact_area("function_extraction")
        .build())
}

fn ast_inline_variable_ts_js(
    source: &str,
    analysis: &InlineVariableAnalysis,
) -> PluginResult<EditPlan> {
    if !analysis.is_safe_to_inline {
        return Err(PluginApiError::internal(format!(
            "Cannot safely inline variable '{}': {}",
            analysis.variable_name,
            analysis.blocking_reasons.join(", ")
        )));
    }

    let mut edits = Vec::new();
    let mut priority = 100;

    for usage_location in &analysis.usage_locations {
        let replacement_text = if analysis
            .initializer_expression
            .contains(|c: char| c.is_whitespace() || "+-*/%".contains(c))
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

    Ok(EditPlanBuilder::new("inline_variable", "inline_variable")
        .with_edits(edits)
        .with_syntax_validation("Verify syntax is valid after inlining")
        .with_intent_args(serde_json::json!({
            "variable": analysis.variable_name,
        }))
        .with_complexity_from_count(analysis.usage_locations.len())
        .with_impact_area("variable_inlining")
        .build())
}

fn ast_extract_variable_ts_js(
    source: &str,
    analysis: &ExtractVariableAnalysis,
    variable_name: Option<String>,
    file_path: &str,
) -> PluginResult<EditPlan> {
    if !analysis.can_extract {
        return Err(PluginApiError::internal(format!(
            "Cannot extract expression: {}",
            analysis.blocking_reasons.join(", ")
        )));
    }

    let var_name = variable_name.unwrap_or_else(|| analysis.suggested_name.clone());

    let indent = LineExtractor::get_indentation_str(source, analysis.insertion_point.start_line);

    let mut edits = Vec::new();

    let declaration = format!("const {} = {};\n{}", var_name, analysis.expression, indent);
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
        .with_syntax_validation("Verify syntax is valid after extraction")
        .with_intent_args(serde_json::json!({
            "expression": analysis.expression,
            "variableName": var_name,
        }))
        .with_complexity(2)
        .with_impact_area("variable_extraction")
        .build())
}

// --- Analysis Functions (moved from mill-ast) ---

/// Analyzes a code range for extract function refactoring feasibility.
///
/// Examines the selected code to determine parameters, return values, and complexity.
///
/// # Arguments
/// * `source` - The TypeScript/JavaScript source code
/// * `range` - The code range to analyze for extraction
/// * `file_path` - Path to the file being analyzed
///
/// # Returns
/// Analysis result containing extractable function metadata
pub fn analyze_extract_function(
    source: &str,
    range: &CodeRange,
    file_path: &str,
) -> PluginResult<ExtractableFunction> {
    let _cm = create_source_map(source, file_path)?;
    let _module = parse_module(source, file_path)?;
    let analyzer = ExtractFunctionAnalyzer::new(source, *range);
    analyzer.finalize()
}

/// Analyzes a variable declaration for inline refactoring feasibility.
///
/// Examines the variable to find its initializer value and all usage locations.
///
/// # Arguments
/// * `source` - The TypeScript/JavaScript source code
/// * `variable_line` - Zero-based line number of the variable declaration
/// * `variable_col` - Zero-based column offset of the variable name
/// * `file_path` - Path to the file being analyzed
///
/// # Returns
/// Analysis result containing variable name, initializer, and usage locations
pub fn analyze_inline_variable(
    source: &str,
    variable_line: u32,
    variable_col: u32,
    file_path: &str,
) -> PluginResult<InlineVariableAnalysis> {
    let cm = create_source_map(source, file_path)?;
    let module = parse_module(source, file_path)?;
    let mut analyzer = InlineVariableAnalyzer::new(source, variable_line, variable_col, cm);
    module.visit_with(&mut analyzer);
    analyzer.finalize()
}

/// Analyzes an expression range for extract variable refactoring feasibility.
///
/// Examines the selected expression to validate extractability and suggest a variable name.
///
/// # Arguments
/// * `source` - The TypeScript/JavaScript source code
/// * `start_line` - Zero-based starting line of the expression
/// * `start_col` - Zero-based starting column of the expression
/// * `end_line` - Zero-based ending line of the expression
/// * `end_col` - Zero-based ending column of the expression
/// * `file_path` - Path to the file being analyzed
///
/// # Returns
/// Analysis result containing expression text, suggested name, and validation status
pub fn analyze_extract_variable(
    source: &str,
    start_line: u32,
    start_col: u32,
    end_line: u32,
    end_col: u32,
    file_path: &str,
) -> PluginResult<ExtractVariableAnalysis> {
    let cm: Lrc<SourceMap> = Default::default();
    let fm = cm.new_source_file(
        FileName::Real(PathBuf::from(file_path)).into(),
        source.to_string(),
    );
    let lexer = Lexer::new(
        Syntax::Typescript(TsSyntax {
            tsx: file_path.ends_with(".tsx"),
            decorators: true,
            ..Default::default()
        }),
        Default::default(),
        StringInput::from(&*fm),
        None,
    );
    let mut parser = Parser::new_from(lexer);
    match parser.parse_module() {
        Ok(_module) => {
            let expression_range = CodeRange {
                start_line,
                start_col,
                end_line,
                end_col,
            };
            let expression = extract_range_text(source, &expression_range)?;
            let (can_extract, blocking_reasons) = check_extractability(&expression);
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
        Err(e) => Err(PluginApiError::parse(format!(
            "Failed to parse file: {:?}",
            e
        ))),
    }
}

/// Analyzes source code to extract information about a literal value at a cursor position.
///
/// This analysis function parses the TypeScript/JavaScript source code using SWC and identifies:
/// - The literal value at the specified cursor position (number, string, boolean, or null)
/// - All occurrences of that literal throughout the file
/// - A suitable insertion point for the constant declaration (top of file)
/// - Whether extraction is valid and any blocking reasons
///
/// # Arguments
/// * `source` - The TypeScript/JavaScript source code
/// * `line` - Zero-based line number where the cursor is positioned
/// * `character` - Zero-based character offset within the line
/// * `file_path` - Path to the file (used to detect .tsx files)
///
/// # Returns
/// * `Ok(ExtractConstantAnalysis)` - Analysis result with literal value, occurrence ranges,
///   validation status, and insertion point
/// * `Err(PluginError)` - If parsing fails or no literal is found at the cursor position
///
/// # Implementation Details
/// 1. Parses the source code using SWC with TypeScript syntax support
/// 2. Uses `LiteralFinder` visitor to locate the literal at the cursor position
/// 3. Calls `find_literal_occurrences()` to identify all matching literals
/// 4. Validates that the found literal is not empty
/// 5. Sets insertion point to line 0 (top of file) for constant declarations
///
/// Analyzes a literal at a cursor position for extract constant refactoring.
///
/// Examines the literal at the specified position, finds all occurrences,
/// and determines the insertion point for the constant declaration.
///
/// # Arguments
/// * `source` - The TypeScript/JavaScript source code
/// * `line` - Zero-based line number where the cursor is positioned
/// * `character` - Zero-based character offset within the line
/// * `file_path` - Path to the file being analyzed
///
/// # Returns
/// Analysis result containing literal value, occurrences, and insertion point
///
/// # Called By
/// - `plan_extract_constant()` - Main entry point for constant extraction
/// - Used internally by the refactoring pipeline
#[allow(dead_code)]
pub(crate) fn analyze_extract_constant(
    source: &str,
    line: u32,
    character: u32,
    file_path: &str,
) -> PluginResult<ExtractConstantAnalysis> {
    let cm = create_source_map(source, file_path)?;
    let file_name = Lrc::new(FileName::Real(std::path::PathBuf::from(file_path)));
    let source_file = cm.new_source_file(file_name, source.to_string());
    let lexer = Lexer::new(
        Syntax::Typescript(TsSyntax {
            tsx: file_path.ends_with(".tsx"),
            decorators: false,
            dts: false,
            no_early_errors: true,
            disallow_ambiguous_jsx_like: true,
        }),
        Default::default(),
        StringInput::from(&*source_file),
        None,
    );
    let mut parser = Parser::new_from(lexer);
    match parser.parse_module() {
        Ok(module) => {
            // Find the literal node at the specified location
            let mut finder = LiteralFinder::new(line, character, source);
            finder.visit_module(&module);

            match finder.found_literal {
                Some((literal_value, _literal_range)) => {
                    // Find all occurrences of this literal value
                    let occurrence_ranges =
                        find_literal_occurrences(source, &literal_value, is_valid_literal_location);
                    let is_valid_literal = !literal_value.is_empty();
                    let blocking_reasons = if !is_valid_literal {
                        vec!["Could not extract literal at cursor position".to_string()]
                    } else {
                        vec![]
                    };

                    // Insertion point: top of file (line 0, column 0)
                    let insertion_point = CodeRange {
                        start_line: 0,
                        start_col: 0,
                        end_line: 0,
                        end_col: 0,
                    };

                    Ok(ExtractConstantAnalysis {
                        literal_value,
                        occurrence_ranges,
                        is_valid_literal,
                        blocking_reasons,
                        insertion_point,
                    })
                }
                None => Err(PluginApiError::internal(
                    "Cursor is not positioned on a literal value. Extract constant only works on numbers, strings, booleans, and null.".to_string(),
                )),
            }
        }
        Err(e) => Err(PluginApiError::parse(format!(
            "Failed to parse file: {:?}",
            e
        ))),
    }
}

/// Generates the EditPlan for constant extraction.
///
/// This internal function constructs the actual edits needed to perform the refactoring:
/// 1. Creates the constant declaration to be inserted at the top of the file
/// 2. Creates replacement edits for all occurrences of the literal value
/// 3. Assembles the complete EditPlan with proper priorities and metadata
///
/// # Arguments
/// * `_source` - The source code (not currently used but available for future enhancements)
/// * `analysis` - The analysis result from `analyze_extract_constant()`
/// * `name` - The constant name to use (must be SCREAMING_SNAKE_CASE)
/// * `file_path` - Path to the file being refactored
///
/// # Returns
/// * `Ok(EditPlan)` - The complete edit plan ready for application
/// * `Err(PluginError)` - If the literal is invalid or the name doesn't match SCREAMING_SNAKE_CASE
///
/// # Called By
/// - `plan_extract_constant()` - Main entry point for constant extraction
#[allow(dead_code)]
fn ast_extract_constant_ts_js(
    _source: &str,
    analysis: &ExtractConstantAnalysis,
    name: &str,
    file_path: &str,
) -> PluginResult<EditPlan> {
    if !analysis.is_valid_literal {
        return Err(PluginApiError::internal(format!(
            "Cannot extract constant: {}",
            analysis.blocking_reasons.join(", ")
        )));
    }

    use mill_lang_common::ExtractConstantEditPlanBuilder;

    ExtractConstantEditPlanBuilder::new(analysis.clone(), name.to_string(), file_path.to_string())
        .with_declaration_format(|name, value| format!("const {} = {};\n", name, value))
        .map_err(PluginApiError::invalid_input)
}

// --- Visitors (moved from mill-ast) ---

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

/// Validates that a string represents a valid TypeScript/JavaScript number
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
        return cleaned.len() > 2 && cleaned[2..].chars().all(|c| ('0'..='7').contains(&c));
    }

    // For regular numbers, try parsing as f64
    // This handles integers, floats, scientific notation, and negative numbers
    cleaned.parse::<f64>().is_ok()
}

/// Visitor to find a literal at a specific line and character position
struct LiteralFinder {
    target_line: u32,
    target_character: u32,
    source: String,
    found_literal: Option<(String, CodeRange)>,
}

impl LiteralFinder {
    fn new(line: u32, character: u32, source: &str) -> Self {
        Self {
            target_line: line,
            target_character: character,
            source: source.to_string(),
            found_literal: None,
        }
    }

    fn visit_module(&mut self, _module: &Module) {
        // Find literals by scanning source text at the target position
        self.find_literal_at_position();
    }

    fn find_literal_at_position(&mut self) {
        let lines: Vec<&str> = self.source.lines().collect();

        if let Some(line_text) = lines.get(self.target_line as usize) {
            // Try to find different kinds of literals at the cursor position

            // Check for numeric literal
            if let Some(range) = self.find_numeric_literal(line_text) {
                self.found_literal = Some((
                    line_text[range.start_col as usize..range.end_col as usize].to_string(),
                    range,
                ));
                return;
            }

            // Check for string literal (quoted)
            if let Some(range) = self.find_string_literal(line_text) {
                self.found_literal = Some((
                    line_text[range.start_col as usize..range.end_col as usize].to_string(),
                    range,
                ));
                return;
            }

            // Check for boolean or null
            if let Some((literal_value, range)) = self.find_keyword_literal(line_text) {
                self.found_literal = Some((literal_value, range));
            }
        }
    }

    fn find_numeric_literal(&self, line_text: &str) -> Option<CodeRange> {
        let col = self.target_character as usize;
        if col >= line_text.len() {
            return None;
        }

        // Try to find the start of a numeric literal
        // TypeScript supports: integers, floats, negative numbers, scientific notation, hex, binary, octal

        // Scan backwards to find potential start of number
        let mut start = col;

        // Handle the case where cursor is right after a number
        if col > 0 && !is_numeric_char(line_text.chars().nth(col)) {
            start = col.saturating_sub(1);
        }

        // Scan backwards to find the actual start
        while start > 0 {
            let prev_char = line_text.chars().nth(start.saturating_sub(1));
            if let Some(ch) = prev_char {
                if is_numeric_char(Some(ch)) {
                    start -= 1;
                } else if ch == '-' || ch == '+' {
                    // Check if this is a sign (not an operator)
                    // It's a sign if preceded by non-identifier character or at start
                    if start == 1 {
                        start -= 1;
                        break;
                    } else if let Some(before_sign) = line_text.chars().nth(start.saturating_sub(2))
                    {
                        if !before_sign.is_alphanumeric()
                            && before_sign != '_'
                            && before_sign != ')'
                            && before_sign != ']'
                        {
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
        let chars: Vec<char> = line_text.chars().collect();

        // Check for hex (0x), binary (0b), or octal (0o) prefix
        if end < chars.len() && chars.get(end) == Some(&'0') && end + 1 < chars.len() {
            let next = chars[end + 1].to_ascii_lowercase();
            if next == 'x' {
                // Hexadecimal
                end += 2;
                while end < chars.len() && chars[end].is_ascii_hexdigit() {
                    end += 1;
                }
            } else if next == 'b' {
                // Binary
                end += 2;
                while end < chars.len() && (chars[end] == '0' || chars[end] == '1') {
                    end += 1;
                }
            } else if next == 'o' {
                // Octal
                end += 2;
                while end < chars.len() && chars[end] >= '0' && chars[end] <= '7' {
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
                return Some(CodeRange {
                    start_line: self.target_line,
                    start_col: start as u32,
                    end_line: self.target_line,
                    end_col: end as u32,
                });
            }
        }

        None
    }

    fn find_string_literal(&self, line_text: &str) -> Option<CodeRange> {
        let col = self.target_character as usize;
        if col > line_text.len() {
            return None;
        }

        // Look for opening quote before cursor
        // We need to find an unescaped quote
        let mut opening_quote: Option<(char, usize)> = None;

        for (i, ch) in line_text[..=col.min(line_text.len().saturating_sub(1))]
            .char_indices()
            .rev()
        {
            if (ch == '"' || ch == '\'' || ch == '`') && !is_escaped(line_text, i) {
                opening_quote = Some((ch, i));
                break;
            }
        }

        if let Some((quote_char, start_pos)) = opening_quote {
            // Find the matching closing quote after cursor, skipping escaped quotes
            let mut pos = col;
            let chars: Vec<char> = line_text.chars().collect();

            while pos < chars.len() {
                if chars[pos] == quote_char && !is_escaped(line_text, pos) {
                    // Found unescaped closing quote
                    return Some(CodeRange {
                        start_line: self.target_line,
                        start_col: start_pos as u32,
                        end_line: self.target_line,
                        end_col: (pos + 1) as u32,
                    });
                }
                pos += 1;
            }
        }

        None
    }

    fn find_keyword_literal(&self, line_text: &str) -> Option<(String, CodeRange)> {
        let col = self.target_character as usize;
        let keywords = ["true", "false", "null"];

        for keyword in &keywords {
            // Try to match keyword at or near cursor
            for start in col.saturating_sub(keyword.len())..=col {
                if start + keyword.len() <= line_text.len()
                    && &line_text[start..start + keyword.len()] == *keyword
                {
                    // Check word boundaries
                    let before_ok =
                        start == 0 || !line_text[..start].ends_with(|c: char| c.is_alphanumeric());
                    let after_ok = start + keyword.len() == line_text.len()
                        || !line_text[start + keyword.len()..]
                            .starts_with(|c: char| c.is_alphanumeric());

                    if before_ok && after_ok {
                        return Some((
                            keyword.to_string(),
                            CodeRange {
                                start_line: self.target_line,
                                start_col: start as u32,
                                end_line: self.target_line,
                                end_col: (start + keyword.len()) as u32,
                            },
                        ));
                    }
                }
            }
        }
        None
    }
}

struct ExtractFunctionAnalyzer {
    selection_range: CodeRange,
    contains_return: bool,
    complexity_score: u32,
}

impl ExtractFunctionAnalyzer {
    fn new(_source: &str, range: CodeRange) -> Self {
        Self {
            selection_range: range,
            contains_return: false,
            complexity_score: 1,
        }
    }
    fn finalize(self) -> PluginResult<ExtractableFunction> {
        let range_copy = self.selection_range;
        Ok(ExtractableFunction {
            selected_range: range_copy,
            required_parameters: Vec::new(),
            return_variables: Vec::new(),
            suggested_name: "extracted_function".to_string(),
            insertion_point: CodeRange {
                start_line: self.selection_range.start_line.saturating_sub(1),
                start_col: 0,
                end_line: self.selection_range.start_line.saturating_sub(1),
                end_col: 0,
            },
            contains_return_statements: self.contains_return,
            complexity_score: self.complexity_score,
        })
    }
}

struct InlineVariableAnalyzer {
    #[allow(dead_code)]
    target_line: u32,
    variable_info: Option<InlineVariableAnalysis>,
}

impl InlineVariableAnalyzer {
    fn new(_source: &str, line: u32, _col: u32, _source_map: Lrc<SourceMap>) -> Self {
        Self {
            target_line: line,
            variable_info: None,
        }
    }
    fn finalize(self) -> PluginResult<InlineVariableAnalysis> {
        self.variable_info.ok_or_else(|| {
            PluginApiError::internal("Could not find variable declaration at specified location")
        })
    }
}

impl Visit for InlineVariableAnalyzer {
    // Simplified visit implementation
}

// --- Helper Functions (moved from mill-ast) ---

fn check_extractability(expression: &str) -> (bool, Vec<String>) {
    let mut can_extract = true;
    let mut blocking_reasons = Vec::new();
    if expression.starts_with("function ") || expression.starts_with("class ") {
        can_extract = false;
        blocking_reasons.push("Cannot extract function or class declarations".to_string());
    }
    if expression.starts_with("const ")
        || expression.starts_with("let ")
        || expression.starts_with("var ")
    {
        can_extract = false;
        blocking_reasons.push("Cannot extract variable declarations".to_string());
    }
    if expression.contains(';') && !expression.starts_with('(') {
        can_extract = false;
        blocking_reasons.push("Selection contains multiple statements".to_string());
    }
    (can_extract, blocking_reasons)
}

fn create_source_map(source: &str, file_path: &str) -> PluginResult<Lrc<SourceMap>> {
    let cm = Lrc::new(SourceMap::new(FilePathMapping::empty()));
    let file_name = Lrc::new(FileName::Real(std::path::PathBuf::from(file_path)));
    cm.new_source_file(file_name, source.to_string());
    Ok(cm)
}

fn parse_module(source: &str, file_path: &str) -> PluginResult<Module> {
    let cm = create_source_map(source, file_path)?;
    let file_name = Lrc::new(FileName::Real(std::path::PathBuf::from(file_path)));
    let source_file = cm.new_source_file(file_name, source.to_string());
    let lexer = Lexer::new(
        Syntax::Typescript(TsSyntax {
            tsx: file_path.ends_with(".tsx"),
            decorators: false,
            dts: false,
            no_early_errors: true,
            disallow_ambiguous_jsx_like: true,
        }),
        Default::default(),
        StringInput::from(&*source_file),
        None,
    );
    let mut parser = Parser::new_from(lexer);
    parser
        .parse_module()
        .map_err(|e| PluginApiError::parse(format!("Failed to parse module: {:?}", e)))
}

fn extract_range_text(source: &str, range: &CodeRange) -> PluginResult<String> {
    let lines: Vec<&str> = source.lines().collect();
    if range.start_line == range.end_line {
        let line = lines
            .get(range.start_line as usize)
            .ok_or_else(|| PluginApiError::internal("Invalid line number"))?;
        Ok(line[range.start_col as usize..range.end_col as usize].to_string())
    } else {
        let mut result = String::new();
        if let Some(first_line) = lines.get(range.start_line as usize) {
            result.push_str(&first_line[range.start_col as usize..]);
            result.push('\n');
        }
        for line_idx in (range.start_line + 1)..range.end_line {
            if let Some(line) = lines.get(line_idx as usize) {
                result.push_str(line);
                result.push('\n');
            }
        }
        if let Some(last_line) = lines.get(range.end_line as usize) {
            result.push_str(&last_line[..range.end_col as usize]);
        }
        Ok(result)
    }
}

fn generate_extracted_function(
    source: &str,
    analysis: &ExtractableFunction,
    function_name: &str,
) -> PluginResult<String> {
    let params = analysis.required_parameters.join(", ");
    let return_statement = if analysis.return_variables.is_empty() {
        String::new()
    } else if analysis.return_variables.len() == 1 {
        format!("  return {};", analysis.return_variables[0])
    } else {
        format!("  return {{ {} }};", analysis.return_variables.join(", "))
    };
    let extracted_code = extract_range_text(source, &analysis.selected_range)?;
    Ok(format!(
        "function {}({}) {{\n  {}\n{}\n}}",
        function_name, params, extracted_code, return_statement
    ))
}

fn generate_function_call(
    analysis: &ExtractableFunction,
    function_name: &str,
) -> PluginResult<String> {
    let args = analysis.required_parameters.join(", ");
    if analysis.return_variables.is_empty() {
        Ok(format!("{}({});", function_name, args))
    } else if analysis.return_variables.len() == 1 {
        Ok(format!(
            "const {} = {}({});",
            analysis.return_variables[0], function_name, args
        ))
    } else {
        Ok(format!(
            "const {{ {} }} = {}({});",
            analysis.return_variables.join(", "),
            function_name,
            args
        ))
    }
}

fn suggest_variable_name(expression: &str) -> String {
    let expr = expression.trim();
    if expr.contains("getElementById") {
        return "element".to_string();
    }
    if expr.contains(".length") {
        return "length".to_string();
    }
    if expr.starts_with('"') || expr.starts_with('\'') || expr.starts_with('`') {
        return "text".to_string();
    }
    if expr.parse::<f64>().is_ok() {
        return "value".to_string();
    }
    if expr == "true" || expr == "false" {
        return "flag".to_string();
    }
    if expr.contains('+') || expr.contains('-') || expr.contains('*') || expr.contains('/') {
        return "result".to_string();
    }
    if expr.starts_with('[') {
        return "items".to_string();
    }
    if expr.starts_with('{') {
        return "obj".to_string();
    }
    "extracted".to_string()
}

// is_valid_literal_location is now provided by mill_lang_common::is_valid_code_literal_location
// We keep the same name via a simple alias to minimize code changes
#[allow(dead_code)]
fn is_valid_literal_location(line: &str, pos: usize, len: usize) -> bool {
    is_valid_code_literal_location(line, pos, len)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_literal_occurrences() {
        let source = "const x = 42;\nconst y = 42;\nconst z = 100;";
        let occurrences = find_literal_occurrences(source, "42", is_valid_literal_location);
        assert_eq!(occurrences.len(), 2);
        assert_eq!(occurrences[0].start_line, 0);
        assert_eq!(occurrences[1].start_line, 1);
    }

    #[test]
    fn test_plan_extract_constant_valid() {
        let source = "const x = 42;\nconst y = 42;\n";
        let result = plan_extract_constant(source, 0, 10, "ANSWER", "test.ts");
        assert!(
            result.is_ok(),
            "Should extract numeric literal successfully"
        );
    }

    #[test]
    fn test_plan_extract_constant_invalid_name() {
        let source = "const x = 42;\n";
        let result = plan_extract_constant(source, 0, 10, "answer", "test.ts");
        assert!(result.is_err(), "Should reject lowercase name");
    }

    // Edge case tests for numeric literals

    #[test]
    fn test_extract_constant_negative_number() {
        let source = "const x = -42;\n";
        let result = plan_extract_constant(source, 0, 11, "NEGATIVE_VALUE", "test.ts");
        assert!(
            result.is_ok(),
            "Should extract negative number: {:?}",
            result
        );
    }

    #[test]
    fn test_extract_constant_scientific_notation_lowercase() {
        let source = "const x = 1e-5;\n";
        let result = plan_extract_constant(source, 0, 11, "SMALL_VALUE", "test.ts");
        assert!(
            result.is_ok(),
            "Should extract scientific notation (lowercase e): {:?}",
            result
        );
    }

    #[test]
    fn test_extract_constant_scientific_notation_uppercase() {
        let source = "const x = 2.5E10;\n";
        let result = plan_extract_constant(source, 0, 12, "BIG_VALUE", "test.ts");
        assert!(
            result.is_ok(),
            "Should extract scientific notation (uppercase E): {:?}",
            result
        );
    }

    #[test]
    fn test_extract_constant_hexadecimal() {
        let source = "const x = 0xFF;\n";
        let result = plan_extract_constant(source, 0, 11, "HEX_VALUE", "test.ts");
        assert!(result.is_ok(), "Should extract hexadecimal: {:?}", result);
    }

    #[test]
    fn test_extract_constant_binary() {
        let source = "const x = 0b1010;\n";
        let result = plan_extract_constant(source, 0, 12, "BINARY_VALUE", "test.ts");
        assert!(result.is_ok(), "Should extract binary: {:?}", result);
    }

    #[test]
    fn test_extract_constant_octal() {
        let source = "const x = 0o777;\n";
        let result = plan_extract_constant(source, 0, 12, "OCTAL_VALUE", "test.ts");
        assert!(result.is_ok(), "Should extract octal: {:?}", result);
    }

    // Edge case tests for string literals with escaped quotes

    #[test]
    fn test_extract_constant_string_with_escaped_quotes() {
        let source = format!("{}\n", r#"const msg = "He said \"hello\"";"#);
        let result = plan_extract_constant(&source, 0, 15, "GREETING", "test.ts");
        assert!(
            result.is_ok(),
            "Should extract string with escaped quotes: {:?}",
            result
        );
    }

    #[test]
    fn test_is_valid_literal_location_escaped_quotes() {
        let line = r#"const s = "He said \"42\"";"#;
        // Position 20 is inside the escaped quote area
        assert!(
            !is_valid_literal_location(line, 20, 2),
            "Should detect inside string with escaped quotes"
        );
    }

    #[test]
    fn test_is_valid_literal_location_outside_string() {
        let line = "const x = 42;";
        assert!(
            is_valid_literal_location(line, 10, 2),
            "Should allow literal outside string"
        );
    }

    #[test]
    fn test_is_valid_literal_location_inside_comment() {
        let line = "const x = 0; // rate is 42";
        assert!(
            !is_valid_literal_location(line, 24, 2),
            "Should reject literal inside comment"
        );
    }

    #[test]
    fn test_find_literal_occurrences_skips_strings() {
        let source = r#"const x = 42; const s = "42";"#;
        let occurrences = find_literal_occurrences(source, "42", is_valid_literal_location);
        // Should only find the numeric 42, not the string "42"
        assert_eq!(
            occurrences.len(),
            1,
            "Should find only numeric literal, not string"
        );
        assert_eq!(
            occurrences[0].start_col, 10,
            "Should find numeric 42 at correct position"
        );
    }

    #[test]
    fn test_find_literal_occurrences_skips_comments() {
        let source = "const x = 42; // 42 in comment";
        let occurrences = find_literal_occurrences(source, "42", is_valid_literal_location);
        // Should only find the first 42, not the one in the comment
        assert_eq!(occurrences.len(), 1, "Should skip literal in comment");
    }

    #[test]
    fn test_is_valid_number_helper() {
        // Valid numbers
        assert!(is_valid_number("42"), "Should validate integer");
        assert!(is_valid_number("-42"), "Should validate negative integer");
        assert!(is_valid_number("3.14"), "Should validate float");
        assert!(
            is_valid_number("1e-5"),
            "Should validate scientific notation"
        );
        assert!(
            is_valid_number("2.5E10"),
            "Should validate scientific notation with uppercase E"
        );
        assert!(is_valid_number("0xFF"), "Should validate hexadecimal");
        assert!(is_valid_number("0b1010"), "Should validate binary");
        assert!(is_valid_number("0o777"), "Should validate octal");

        // Invalid numbers
        assert!(!is_valid_number(""), "Should reject empty string");
        assert!(!is_valid_number("abc"), "Should reject non-numeric string");
        assert!(!is_valid_number("0x"), "Should reject incomplete hex");
        assert!(!is_valid_number("0b"), "Should reject incomplete binary");
    }
}
