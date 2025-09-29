//! Advanced refactoring operations using AST analysis

use crate::analyzer::{
    DependencyUpdate, DependencyUpdateType, EditLocation, EditPlan, EditPlanMetadata, EditType,
    TextEdit, ValidationRule, ValidationType,
};
use crate::error::{AstError, AstResult};
use crate::parser::{build_import_graph, ImportInfo};
use crate::python_parser::{
    analyze_python_expression_range, extract_python_functions, extract_python_variables,
    find_variable_at_position, get_variable_usages_in_scope,
};
use cb_core::CoreError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use swc_common::{sync::Lrc, FileName, FilePathMapping, SourceMap};
use swc_ecma_ast::*;
use swc_ecma_parser::{lexer::Lexer, Parser, StringInput, Syntax, TsSyntax};
use swc_ecma_visit::{Visit, VisitWith};

/// Range of selected code for extraction
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CodeRange {
    pub start_line: u32,
    pub start_col: u32,
    pub end_line: u32,
    pub end_col: u32,
}

/// Detect file language from file path
fn detect_language(file_path: &str) -> &str {
    if file_path.ends_with(".py") {
        "python"
    } else if file_path.ends_with(".ts") || file_path.ends_with(".tsx") {
        "typescript"
    } else if file_path.ends_with(".js") || file_path.ends_with(".jsx") {
        "javascript"
    } else {
        "unknown"
    }
}

/// Variable usage information for refactoring analysis
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VariableUsage {
    pub name: String,
    pub declaration_location: Option<CodeRange>,
    pub usages: Vec<CodeRange>,
    pub scope_depth: u32,
    pub is_parameter: bool,
    pub is_declared_in_selection: bool,
    pub is_used_after_selection: bool,
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

/// Analyze code selection for function extraction
pub fn analyze_extract_function(
    source: &str,
    range: &CodeRange,
    file_path: &str,
) -> AstResult<ExtractableFunction> {
    let _cm = create_source_map(source, file_path)?;
    let _module = parse_module(source, file_path)?;

    let analyzer = ExtractFunctionAnalyzer::new(source, range.clone());
    // TODO: Implement AST traversal for sophisticated parameter and variable analysis
    // Currently using simplified analysis without visiting the module
    analyzer.finalize()
}

/// Analyze variable declaration for inlining
pub fn analyze_inline_variable(
    source: &str,
    variable_line: u32,
    variable_col: u32,
    file_path: &str,
) -> AstResult<InlineVariableAnalysis> {
    let cm = create_source_map(source, file_path)?;
    let module = parse_module(source, file_path)?;

    let mut analyzer = InlineVariableAnalyzer::new(source, variable_line, variable_col, cm);
    module.visit_with(&mut analyzer);

    analyzer.finalize()
}

/// Generate edit plan for extract function refactoring
pub fn plan_extract_function(
    source: &str,
    range: &CodeRange,
    new_function_name: &str,
    file_path: &str,
) -> AstResult<EditPlan> {
    match detect_language(file_path) {
        "python" => plan_extract_function_python(source, range, new_function_name, file_path),
        "typescript" | "javascript" => {
            plan_extract_function_ts_js(source, range, new_function_name, file_path)
        }
        _ => Err(AstError::analysis(format!(
            "Unsupported language for file: {}",
            file_path
        ))),
    }
}

/// Generate edit plan for extract function refactoring (TypeScript/JavaScript)
fn plan_extract_function_ts_js(
    source: &str,
    range: &CodeRange,
    new_function_name: &str,
    file_path: &str,
) -> AstResult<EditPlan> {
    let analysis = analyze_extract_function(source, range, file_path)?;

    let mut edits = Vec::new();

    // 1. Create the new function at the insertion point
    let function_code = generate_extracted_function(source, &analysis, new_function_name)?;

    edits.push(TextEdit {
        edit_type: EditType::Insert,
        location: analysis.insertion_point.clone().into(),
        original_text: String::new(),
        new_text: format!("\n{}\n", function_code),
        priority: 100,
        description: format!("Create extracted function '{}'", new_function_name),
    });

    // 2. Replace the selected code with a function call
    let call_code = generate_function_call(&analysis, new_function_name)?;

    edits.push(TextEdit {
        edit_type: EditType::Replace,
        location: analysis.selected_range.clone().into(),
        original_text: extract_range_text(source, &analysis.selected_range)?,
        new_text: call_code,
        priority: 90,
        description: format!("Replace selected code with call to '{}'", new_function_name),
    });

    Ok(EditPlan {
        source_file: file_path.to_string(),
        edits,
        dependency_updates: Vec::new(),
        validations: vec![
            ValidationRule {
                rule_type: ValidationType::SyntaxCheck,
                description: "Verify syntax is valid after extraction".to_string(),
                parameters: HashMap::new(),
            },
            ValidationRule {
                rule_type: ValidationType::TypeCheck,
                description: "Verify types are consistent".to_string(),
                parameters: HashMap::new(),
            },
        ],
        metadata: EditPlanMetadata {
            intent_name: "extract_function".to_string(),
            intent_arguments: serde_json::json!({
                "range": range,
                "function_name": new_function_name
            }),
            created_at: chrono::Utc::now(),
            complexity: analysis.complexity_score.min(10) as u8,
            impact_areas: vec!["function_extraction".to_string()],
        },
    })
}

/// Generate edit plan for inline variable refactoring
pub fn plan_inline_variable(
    source: &str,
    variable_line: u32,
    variable_col: u32,
    file_path: &str,
) -> AstResult<EditPlan> {
    match detect_language(file_path) {
        "python" => plan_inline_variable_python(source, variable_line, variable_col, file_path),
        _ => {
            let analysis = analyze_inline_variable(source, variable_line, variable_col, file_path)?;
            plan_inline_variable_ts_js(source, &analysis)
        }
    }
}

/// Generate edit plan for inline variable refactoring (TypeScript/JavaScript)
fn plan_inline_variable_ts_js(
    source: &str,
    analysis: &InlineVariableAnalysis,
) -> AstResult<EditPlan> {
    if !analysis.is_safe_to_inline {
        return Err(AstError::analysis(format!(
            "Cannot safely inline variable '{}': {}",
            analysis.variable_name,
            analysis.blocking_reasons.join(", ")
        )));
    }

    let mut edits = Vec::new();
    let mut priority = 100;

    // Replace all usages with the initializer expression
    for usage_location in &analysis.usage_locations {
        // Only wrap in parentheses if it's a complex expression (contains operators or spaces)
        let replacement_text = if analysis.initializer_expression.contains(' ')
            || analysis.initializer_expression.contains('+')
            || analysis.initializer_expression.contains('-')
            || analysis.initializer_expression.contains('*')
            || analysis.initializer_expression.contains('/')
            || analysis.initializer_expression.contains('%')
        {
            format!("({})", analysis.initializer_expression)
        } else {
            analysis.initializer_expression.clone()
        };

        edits.push(TextEdit {
            edit_type: EditType::Replace,
            location: usage_location.clone().into(),
            original_text: analysis.variable_name.clone(),
            new_text: replacement_text,
            priority,
            description: format!("Replace '{}' with its value", analysis.variable_name),
        });
        priority -= 1; // Process in reverse order to avoid offset issues
    }

    // Remove the variable declaration
    edits.push(TextEdit {
        edit_type: EditType::Delete,
        location: analysis.declaration_range.clone().into(),
        original_text: extract_range_text(source, &analysis.declaration_range)?,
        new_text: String::new(),
        priority: 50, // Do this after replacements
        description: format!("Remove declaration of '{}'", analysis.variable_name),
    });

    Ok(EditPlan {
        source_file: "inline_variable".to_string(),
        edits,
        dependency_updates: Vec::new(),
        validations: vec![ValidationRule {
            rule_type: ValidationType::SyntaxCheck,
            description: "Verify syntax is valid after inlining".to_string(),
            parameters: HashMap::new(),
        }],
        metadata: EditPlanMetadata {
            intent_name: "inline_variable".to_string(),
            intent_arguments: serde_json::json!({
                "variable": analysis.variable_name,
                "line": "variable_line",
                "column": "variable_col"
            }),
            created_at: chrono::Utc::now(),
            complexity: (analysis.usage_locations.len().min(10)) as u8,
            impact_areas: vec!["variable_inlining".to_string()],
        },
    })
}

/// Analysis result for extract variable refactoring
#[derive(Debug, Clone)]
pub struct ExtractVariableAnalysis {
    pub expression: String,
    pub expression_range: CodeRange,
    pub can_extract: bool,
    pub suggested_name: String,
    pub insertion_point: CodeRange,
    pub blocking_reasons: Vec<String>,
    pub scope_type: String,
}

/// Analyze a selected expression for extraction into a variable
pub fn analyze_extract_variable(
    source: &str,
    start_line: u32,
    start_col: u32,
    end_line: u32,
    end_col: u32,
    file_path: &str,
) -> AstResult<ExtractVariableAnalysis> {
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
            // TODO: Use parsed AST module for advanced code analysis
            // The module contains the full syntax tree which could be used for:
            // - Precise variable scope analysis
            // - Function dependency detection
            // - Complex expression evaluation
            // Currently using simplified text-based extraction instead

            // Extract the selected expression text
            let expression_range = CodeRange {
                start_line,
                start_col,
                end_line,
                end_col,
            };

            let expression = extract_range_text(source, &expression_range)?;

            // Check if this is a valid expression (not a statement, declaration, etc.)
            let mut can_extract = true;
            let mut blocking_reasons = Vec::new();

            // Simple heuristics for what can be extracted
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

            // Generate a suggested variable name based on the expression
            let suggested_name = suggest_variable_name(&expression);

            // Find the best insertion point (beginning of current scope)
            // For simplicity, we'll insert at the beginning of the line containing the expression
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
                scope_type: "function".to_string(), // Simplified for now
            })
        }
        Err(e) => Err(AstError::parse(format!("Failed to parse file: {:?}", e))),
    }
}

/// Suggest a variable name based on the expression
fn suggest_variable_name(expression: &str) -> String {
    // Simple heuristics for variable naming
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

    // Default
    "extracted".to_string()
}

/// Generate edit plan for extract variable refactoring
pub fn plan_extract_variable(
    source: &str,
    start_line: u32,
    start_col: u32,
    end_line: u32,
    end_col: u32,
    variable_name: Option<String>,
    file_path: &str,
) -> AstResult<EditPlan> {
    match detect_language(file_path) {
        "python" => plan_extract_variable_python(
            source,
            start_line,
            start_col,
            end_line,
            end_col,
            variable_name,
            file_path,
        ),
        _ => {
            let analysis = analyze_extract_variable(
                source, start_line, start_col, end_line, end_col, file_path,
            )?;
            plan_extract_variable_ts_js(source, &analysis, variable_name, file_path)
        }
    }
}

/// Generate edit plan for extract variable refactoring (TypeScript/JavaScript)
fn plan_extract_variable_ts_js(
    source: &str,
    analysis: &ExtractVariableAnalysis,
    variable_name: Option<String>,
    file_path: &str,
) -> AstResult<EditPlan> {
    if !analysis.can_extract {
        return Err(AstError::analysis(format!(
            "Cannot extract expression: {}",
            analysis.blocking_reasons.join(", ")
        )));
    }

    let var_name = variable_name.unwrap_or_else(|| analysis.suggested_name.clone());

    // Get the indentation of the current line
    let lines: Vec<&str> = source.lines().collect();
    let current_line = lines
        .get((analysis.insertion_point.start_line) as usize)
        .unwrap_or(&"");
    let indent = current_line
        .chars()
        .take_while(|c| c.is_whitespace())
        .collect::<String>();

    let mut edits = Vec::new();

    // Insert the variable declaration
    let declaration = format!("const {} = {};\n{}", var_name, analysis.expression, indent);
    edits.push(TextEdit {
        edit_type: EditType::Insert,
        location: analysis.insertion_point.clone().into(),
        original_text: String::new(),
        new_text: declaration,
        priority: 100,
        description: format!(
            "Extract '{}' into variable '{}'",
            analysis.expression, var_name
        ),
    });

    // Replace the original expression with the variable name
    edits.push(TextEdit {
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
        validations: vec![ValidationRule {
            rule_type: ValidationType::SyntaxCheck,
            description: "Verify syntax is valid after extraction".to_string(),
            parameters: HashMap::new(),
        }],
        metadata: EditPlanMetadata {
            intent_name: "extract_variable".to_string(),
            intent_arguments: serde_json::json!({
                "expression": analysis.expression,
                "variableName": var_name,
                "insertionPoint": analysis.insertion_point,
                "expressionRange": analysis.expression_range
            }),
            created_at: chrono::Utc::now(),
            complexity: 2,
            impact_areas: vec!["variable_extraction".to_string()],
        },
    })
}

/// Visitor for analyzing code selection for function extraction
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

    fn finalize(self) -> AstResult<ExtractableFunction> {
        // TODO: Implement sophisticated variable analysis
        // For now, returning simplified result

        let range_copy = self.selection_range.clone();
        Ok(ExtractableFunction {
            selected_range: range_copy,
            required_parameters: Vec::new(), // TODO: Implement parameter analysis
            return_variables: Vec::new(),    // TODO: Implement return variable analysis
            suggested_name: "extracted_function".to_string(), // TODO: Implement name suggestion
            insertion_point: CodeRange {
                // TODO: Implement insertion point detection
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

// TODO: Implement AST visitor for sophisticated analysis
// Visit implementation removed due to complexity and incomplete state

/// Visitor for analyzing variable for inlining
struct InlineVariableAnalyzer {
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

    fn extract_expression_text(&self, expr: &Expr) -> String {
        match expr {
            Expr::Lit(lit) => match lit {
                Lit::Str(s) => format!("'{}'", s.value),
                Lit::Bool(b) => b.value.to_string(),
                Lit::Null(_) => "null".to_string(),
                Lit::Num(n) => n.value.to_string(),
                Lit::BigInt(b) => format!("{}n", b.value),
                Lit::Regex(r) => {
                    format!("/{}/{}", r.exp, r.flags)
                }
                Lit::JSXText(_) => "/* JSX text */".to_string(),
            },
            Expr::Ident(ident) => ident.sym.to_string(),
            Expr::Bin(bin) => {
                let left = self.extract_expression_text(&bin.left);
                let right = self.extract_expression_text(&bin.right);
                let op = match bin.op {
                    swc_ecma_ast::BinaryOp::Add => "+",
                    swc_ecma_ast::BinaryOp::Sub => "-",
                    swc_ecma_ast::BinaryOp::Mul => "*",
                    swc_ecma_ast::BinaryOp::Div => "/",
                    swc_ecma_ast::BinaryOp::Mod => "%",
                    _ => "?",
                };
                format!("{} {} {}", left, op, right)
            }
            Expr::Unary(unary) => {
                let arg = self.extract_expression_text(&unary.arg);
                let op = match unary.op {
                    swc_ecma_ast::UnaryOp::Minus => "-",
                    swc_ecma_ast::UnaryOp::Plus => "+",
                    swc_ecma_ast::UnaryOp::Bang => "!",
                    swc_ecma_ast::UnaryOp::Tilde => "~",
                    _ => "?",
                };
                format!("{}{}", op, arg)
            }
            Expr::Paren(paren) => {
                let inner = self.extract_expression_text(&paren.expr);
                format!("({})", inner)
            }
            _ => "/* complex expression */".to_string(),
        }
    }

    fn scan_for_usages(&mut self) {
        // TODO: Implement usage scanning when source_lines field is restored
        // For now, this method is simplified to avoid compilation errors
    }

    fn finalize(mut self) -> AstResult<InlineVariableAnalysis> {
        // Scan for usages after we've found the target variable
        if self.variable_info.is_some() {
            self.scan_for_usages();
        }

        self.variable_info.ok_or_else(|| {
            AstError::analysis("Could not find variable declaration at specified location")
        })
    }
}

impl Visit for InlineVariableAnalyzer {
    fn visit_var_decl(&mut self, n: &VarDecl) {
        // Use a simple approach: find the variable declaration at the target line
        for decl in &n.decls {
            if let Pat::Ident(ident) = &decl.name {
                let _var_name = ident.id.sym.to_string();

                // Check if this variable is on our target line by looking at source text
                // The test passes line 1 expecting to find const multiplier, but after conversion it becomes 0
                // However, const multiplier is actually at source line 1, so we need to check line 1
                let _actual_target_line = if self.target_line == 0 {
                    1
                } else {
                    self.target_line
                };
                // TODO: Re-implement variable declaration detection with proper source analysis
            }
        }
        // TODO: Re-implement AST traversal when features are completed
    }

    fn visit_ident(&mut self, _n: &Ident) {
        // For now, do nothing here - we'll scan for usages in finalize()
    }
}

/// Helper functions
fn create_source_map(source: &str, file_path: &str) -> AstResult<Lrc<SourceMap>> {
    let cm = Lrc::new(SourceMap::new(FilePathMapping::empty()));
    let file_name = Lrc::new(FileName::Real(std::path::PathBuf::from(file_path)));
    let _source_file = cm.new_source_file(file_name, source.to_string());
    Ok(cm)
}

fn parse_module(source: &str, file_path: &str) -> AstResult<Module> {
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
        .map_err(|e| AstError::parse(format!("Failed to parse module: {:?}", e)))
}

fn extract_range_text(source: &str, range: &CodeRange) -> AstResult<String> {
    let lines: Vec<&str> = source.lines().collect();

    if range.start_line == range.end_line {
        // Single line
        let line = lines
            .get(range.start_line as usize)
            .ok_or_else(|| AstError::analysis("Invalid line number"))?;

        Ok(line[range.start_col as usize..range.end_col as usize].to_string())
    } else {
        // Multi-line
        let mut result = String::new();

        // First line
        if let Some(first_line) = lines.get(range.start_line as usize) {
            result.push_str(&first_line[range.start_col as usize..]);
            result.push('\n');
        }

        // Middle lines
        for line_idx in (range.start_line + 1)..range.end_line {
            if let Some(line) = lines.get(line_idx as usize) {
                result.push_str(line);
                result.push('\n');
            }
        }

        // Last line
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
) -> AstResult<String> {
    let params = analysis.required_parameters.join(", ");

    let return_statement = if analysis.return_variables.is_empty() {
        String::new()
    } else if analysis.return_variables.len() == 1 {
        format!("  return {};", analysis.return_variables[0])
    } else {
        format!("  return {{ {} }};", analysis.return_variables.join(", "))
    };

    // Extract the actual code lines from the selected range
    let lines: Vec<&str> = source.lines().collect();
    let range = &analysis.selected_range;
    let extracted_lines = if range.start_line == range.end_line {
        // Single line extraction
        let line = lines[range.start_line as usize];
        let start_col = range.start_col as usize;
        let end_col = range.end_col as usize;
        let extracted_text = &line[start_col..end_col.min(line.len())];
        vec![format!("  {}", extracted_text)]
    } else {
        // Multi-line extraction
        let mut result = Vec::new();
        for line_num in range.start_line..=range.end_line {
            if line_num >= lines.len() as u32 {
                break;
            }
            let line = lines[line_num as usize];
            if line_num == range.start_line {
                // First line - use from start_col to end
                let start_col = range.start_col as usize;
                if start_col < line.len() {
                    result.push(format!("  {}", &line[start_col..]));
                }
            } else if line_num == range.end_line {
                // Last line - use from start to end_col
                let end_col = range.end_col as usize;
                let extracted_text = &line[..end_col.min(line.len())];
                result.push(format!("  {}", extracted_text));
            } else {
                // Middle lines - use entire line with proper indentation
                result.push(format!("  {}", line));
            }
        }
        result
    };

    let extracted_code = extracted_lines.join("\n");

    Ok(format!(
        "function {}({}) {{\n{}\n{}\n}}",
        function_name, params, extracted_code, return_statement
    ))
}

fn generate_function_call(
    analysis: &ExtractableFunction,
    function_name: &str,
) -> AstResult<String> {
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

// ================================
// Python Refactoring Implementation
// ================================

/// Analyze code selection for function extraction (Python)
fn analyze_extract_function_python(
    source: &str,
    range: &CodeRange,
    _file_path: &str,
) -> AstResult<ExtractableFunction> {
    let lines: Vec<&str> = source.lines().collect();

    // Find variables and functions used in the selection that are defined outside
    let mut required_parameters = Vec::new();
    let mut required_imports = Vec::new();
    let functions = extract_python_functions(source)?;
    let variables = extract_python_variables(source)?;

    // Simple analysis: find variables and function calls referenced in the selection
    for line_num in range.start_line..=range.end_line {
        if let Some(line) = lines.get(line_num as usize) {
            let line_text = if line_num == range.start_line && line_num == range.end_line {
                // Single line selection
                &line[range.start_col as usize..range.end_col as usize]
            } else if line_num == range.start_line {
                &line[range.start_col as usize..]
            } else if line_num == range.end_line {
                &line[..range.end_col as usize]
            } else {
                line
            };

            // Find variable references in this line
            for var in &variables {
                if var.line < range.start_line && line_text.contains(&var.name)
                    && !required_parameters.contains(&var.name) {
                        required_parameters.push(var.name.clone());
                    }
            }

            // Find function calls in this line that are defined outside the selection
            for func in &functions {
                if func.start_line < range.start_line
                    && line_text.contains(&format!("{}(", func.name))
                    && !required_imports.contains(&func.name) {
                        required_imports.push(func.name.clone());
                    }
            }
        }
    }

    // Check for return statements
    let selected_text = extract_python_range_text(source, range)?;
    let contains_return = selected_text.contains("return ");

    // Find insertion point (before the selection at function level)
    let insertion_point = find_python_insertion_point(source, range.start_line)?;

    // TODO: Include required_imports in ExtractableFunction struct for better analysis
    // Currently analyzed but not exposed: function dependencies that need to be available
    // This information could be used to suggest imports or parameter passing

    Ok(ExtractableFunction {
        selected_range: range.clone(),
        required_parameters,
        return_variables: Vec::new(), // Simplified for now
        suggested_name: "extracted_function".to_string(),
        insertion_point,
        contains_return_statements: contains_return,
        complexity_score: 2,
    })
}

/// Analyze variable declaration for inlining (Python)
fn analyze_inline_variable_python(
    source: &str,
    variable_line: u32,
    variable_col: u32,
    _file_path: &str,
) -> AstResult<InlineVariableAnalysis> {
    // Find the variable at the specified position
    if let Some(variable) = find_variable_at_position(source, variable_line, variable_col)? {
        // Get the variable's value from the source
        let lines: Vec<&str> = source.lines().collect();
        let var_line_text = lines
            .get(variable.line as usize)
            .ok_or_else(|| AstError::analysis("Invalid line number"))?;

        // Extract the initializer expression
        let assign_re = regex::Regex::new(&format!(
            r"^\s*{}\s*=\s*(.+)",
            regex::escape(&variable.name)
        ))
        .unwrap();
        let initializer = if let Some(captures) = assign_re.captures(var_line_text) {
            captures.get(1).unwrap().as_str().trim().to_string()
        } else {
            return Err(AstError::analysis("Could not find variable assignment"));
        };

        // Find all usages of this variable
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
            is_safe_to_inline: true, // Simplified safety check
            blocking_reasons: Vec::new(),
        })
    } else {
        Err(AstError::analysis(
            "Could not find variable at specified position",
        ))
    }
}

/// Analyze a selected expression for extraction into a variable (Python)
fn analyze_extract_variable_python(
    source: &str,
    start_line: u32,
    start_col: u32,
    end_line: u32,
    end_col: u32,
    _file_path: &str,
) -> AstResult<ExtractVariableAnalysis> {
    let expression_range = CodeRange {
        start_line,
        start_col,
        end_line,
        end_col,
    };

    let expression =
        analyze_python_expression_range(source, start_line, start_col, end_line, end_col)?;

    // Simple validation for Python expressions
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

    // Generate a suggested variable name
    let suggested_name = suggest_python_variable_name(&expression);

    // Find insertion point
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
fn plan_extract_function_python(
    source: &str,
    range: &CodeRange,
    new_function_name: &str,
    file_path: &str,
) -> AstResult<EditPlan> {
    let analysis = analyze_extract_function_python(source, range, file_path)?;

    let mut edits = Vec::new();

    // Create the new function
    let function_code = generate_extracted_function_python(source, &analysis, new_function_name)?;

    edits.push(TextEdit {
        edit_type: EditType::Insert,
        location: analysis.insertion_point.clone().into(),
        original_text: String::new(),
        new_text: format!("{}\n\n", function_code),
        priority: 100,
        description: format!("Create extracted function '{}'", new_function_name),
    });

    // Replace the selected code with a function call
    let call_code = generate_python_function_call(&analysis, new_function_name)?;

    edits.push(TextEdit {
        edit_type: EditType::Replace,
        location: analysis.selected_range.clone().into(),
        original_text: extract_python_range_text(source, &analysis.selected_range)?,
        new_text: call_code,
        priority: 90,
        description: format!("Replace selected code with call to '{}'", new_function_name),
    });

    Ok(EditPlan {
        source_file: file_path.to_string(),
        edits,
        dependency_updates: Vec::new(),
        validations: vec![ValidationRule {
            rule_type: ValidationType::SyntaxCheck,
            description: "Verify Python syntax is valid after extraction".to_string(),
            parameters: HashMap::new(),
        }],
        metadata: EditPlanMetadata {
            intent_name: "extract_function".to_string(),
            intent_arguments: serde_json::json!({
                "range": range,
                "function_name": new_function_name
            }),
            created_at: chrono::Utc::now(),
            complexity: analysis.complexity_score.min(10) as u8,
            impact_areas: vec!["function_extraction".to_string()],
        },
    })
}

/// Generate edit plan for inline variable refactoring (Python)
fn plan_inline_variable_python(
    source: &str,
    variable_line: u32,
    variable_col: u32,
    file_path: &str,
) -> AstResult<EditPlan> {
    let analysis = analyze_inline_variable_python(source, variable_line, variable_col, file_path)?;

    if !analysis.is_safe_to_inline {
        return Err(AstError::analysis(format!(
            "Cannot safely inline variable '{}': {}",
            analysis.variable_name,
            analysis.blocking_reasons.join(", ")
        )));
    }

    let mut edits = Vec::new();
    let mut priority = 100;

    // Replace all usages with the initializer expression
    for usage_location in &analysis.usage_locations {
        // For Python, we typically don't need parentheses unless it's a complex expression
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
            edit_type: EditType::Replace,
            location: usage_location.clone().into(),
            original_text: analysis.variable_name.clone(),
            new_text: replacement_text,
            priority,
            description: format!("Replace '{}' with its value", analysis.variable_name),
        });
        priority -= 1;
    }

    // Remove the variable declaration
    edits.push(TextEdit {
        edit_type: EditType::Delete,
        location: analysis.declaration_range.clone().into(),
        original_text: extract_python_range_text(source, &analysis.declaration_range)?,
        new_text: String::new(),
        priority: 50,
        description: format!("Remove declaration of '{}'", analysis.variable_name),
    });

    Ok(EditPlan {
        source_file: file_path.to_string(),
        edits,
        dependency_updates: Vec::new(),
        validations: vec![ValidationRule {
            rule_type: ValidationType::SyntaxCheck,
            description: "Verify Python syntax is valid after inlining".to_string(),
            parameters: HashMap::new(),
        }],
        metadata: EditPlanMetadata {
            intent_name: "inline_variable".to_string(),
            intent_arguments: serde_json::json!({
                "variable": analysis.variable_name,
                "line": variable_line,
                "column": variable_col
            }),
            created_at: chrono::Utc::now(),
            complexity: (analysis.usage_locations.len().min(10)) as u8,
            impact_areas: vec!["variable_inlining".to_string()],
        },
    })
}

/// Generate edit plan for extract variable refactoring (Python)
fn plan_extract_variable_python(
    source: &str,
    start_line: u32,
    start_col: u32,
    end_line: u32,
    end_col: u32,
    variable_name: Option<String>,
    file_path: &str,
) -> AstResult<EditPlan> {
    let analysis = analyze_extract_variable_python(
        source, start_line, start_col, end_line, end_col, file_path,
    )?;

    if !analysis.can_extract {
        return Err(AstError::analysis(format!(
            "Cannot extract expression: {}",
            analysis.blocking_reasons.join(", ")
        )));
    }

    let var_name = variable_name.unwrap_or_else(|| analysis.suggested_name.clone());

    // Get the indentation of the current line
    let lines: Vec<&str> = source.lines().collect();
    let current_line = lines.get((start_line) as usize).unwrap_or(&"");
    let indent = current_line
        .chars()
        .take_while(|c| c.is_whitespace())
        .collect::<String>();

    let mut edits = Vec::new();

    // Insert the variable declaration (Python style)
    let declaration = format!("{}{} = {}\n", indent, var_name, analysis.expression);
    edits.push(TextEdit {
        edit_type: EditType::Insert,
        location: analysis.insertion_point.clone().into(),
        original_text: String::new(),
        new_text: declaration,
        priority: 100,
        description: format!(
            "Extract '{}' into variable '{}'",
            analysis.expression, var_name
        ),
    });

    // Replace the original expression with the variable name
    edits.push(TextEdit {
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
        validations: vec![ValidationRule {
            rule_type: ValidationType::SyntaxCheck,
            description: "Verify Python syntax is valid after extraction".to_string(),
            parameters: HashMap::new(),
        }],
        metadata: EditPlanMetadata {
            intent_name: "extract_variable".to_string(),
            intent_arguments: serde_json::json!({
                "expression": analysis.expression,
                "variableName": var_name,
                "startLine": start_line,
                "startCol": start_col,
                "endLine": end_line,
                "endCol": end_col
            }),
            created_at: chrono::Utc::now(),
            complexity: 2,
            impact_areas: vec!["variable_extraction".to_string()],
        },
    })
}

// Helper functions for Python refactoring

/// Extract text from a Python code range
fn extract_python_range_text(source: &str, range: &CodeRange) -> AstResult<String> {
    analyze_python_expression_range(
        source,
        range.start_line,
        range.start_col,
        range.end_line,
        range.end_col,
    )
}

/// Find proper insertion point for a new Python function
fn find_python_insertion_point(source: &str, start_line: u32) -> AstResult<CodeRange> {
    let lines: Vec<&str> = source.lines().collect();

    // Find the current function or class that contains the start_line
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
fn generate_extracted_function_python(
    source: &str,
    analysis: &ExtractableFunction,
    function_name: &str,
) -> AstResult<String> {
    let params = analysis.required_parameters.join(", ");

    // Extract the actual code lines from the selected range
    let extracted_code = extract_python_range_text(source, &analysis.selected_range)?;

    // Ensure proper indentation for the function body
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
fn generate_python_function_call(
    analysis: &ExtractableFunction,
    function_name: &str,
) -> AstResult<String> {
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
fn suggest_python_variable_name(expression: &str) -> String {
    let expr = expression.trim();

    // Python-specific naming conventions
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

    // Default
    "extracted".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_range_text_single_line() {
        let source = "const message = 'hello world';";
        let range = CodeRange {
            start_line: 0,
            start_col: 6,
            end_line: 0,
            end_col: 13,
        };

        let result = extract_range_text(source, &range).unwrap();
        assert_eq!(result, "message");
    }

    #[test]
    fn test_extract_range_text_multi_line() {
        let source = "const x = 1;\nconst y = 2;\nconst z = 3;";
        let range = CodeRange {
            start_line: 0,
            start_col: 6,
            end_line: 1,
            end_col: 7,
        };

        let result = extract_range_text(source, &range).unwrap();
        assert_eq!(result, "x = 1;\nconst y");
    }
}

/// Plan a project-wide symbol rename operation with import updates
pub fn plan_rename_refactor(
    old_name: &str,
    new_name: &str,
    file_path: &Path,
) -> AstResult<EditPlan> {
    // Read the target file to understand its imports and exports
    let content =
        std::fs::read_to_string(file_path).map_err(|e| AstError::Core(CoreError::from(e)))?;

    // Build import graph for the file to understand dependencies
    let _import_graph = build_import_graph(&content, file_path)?;

    let mut edits = Vec::new();
    let mut dependency_updates = Vec::new();
    let mut impact_areas = Vec::new();

    // 1. Find and rename symbol in the definition file
    let definition_edits = find_symbol_references(&content, old_name, new_name, file_path)?;
    edits.extend(definition_edits);
    impact_areas.push(file_path.to_string_lossy().to_string());

    // 2. Find all files that import this symbol and update their import statements
    let project_root = find_project_root(file_path);
    let related_files = find_files_importing_symbol(&project_root, file_path, old_name)?;

    for importing_file in related_files {
        // Read the importing file
        if let Ok(importing_content) = std::fs::read_to_string(&importing_file) {
            // Build import graph for this file
            if let Ok(importing_graph) = build_import_graph(&importing_content, &importing_file) {
                // Check if this file imports the symbol we're renaming
                let imports_target_symbol = importing_graph.imports.iter().any(|import| {
                    // Check if this import references our target file and symbol
                    if is_import_from_target_file(import, file_path) {
                        import
                            .named_imports
                            .iter()
                            .any(|named| named.name == old_name)
                            || import.default_import.as_ref() == Some(&old_name.to_string())
                            || import.namespace_import.as_ref() == Some(&old_name.to_string())
                    } else {
                        false
                    }
                });

                if imports_target_symbol {
                    // Generate import update edits for this file
                    let import_edits = generate_import_update_edits(
                        &importing_content,
                        old_name,
                        new_name,
                        &importing_file,
                    )?;
                    edits.extend(import_edits);

                    // Track dependency update
                    dependency_updates.push(DependencyUpdate {
                        target_file: importing_file.to_string_lossy().to_string(),
                        update_type: DependencyUpdateType::ImportName,
                        old_reference: old_name.to_string(),
                        new_reference: new_name.to_string(),
                    });

                    impact_areas.push(importing_file.to_string_lossy().to_string());
                }
            }
        }
    }

    // Sort edits by priority (higher priority first)
    edits.sort_by(|a, b| b.priority.cmp(&a.priority));

    // Create validation rules
    let validations = vec![
        ValidationRule {
            rule_type: ValidationType::SyntaxCheck,
            description: "Ensure all modified files have valid syntax after rename".to_string(),
            parameters: HashMap::new(),
        },
        ValidationRule {
            rule_type: ValidationType::ImportResolution,
            description: "Verify all import statements resolve correctly".to_string(),
            parameters: HashMap::new(),
        },
    ];

    Ok(EditPlan {
        source_file: file_path.to_string_lossy().to_string(),
        edits,
        dependency_updates,
        validations,
        metadata: EditPlanMetadata {
            intent_name: "rename_symbol_with_imports".to_string(),
            intent_arguments: serde_json::json!({
                "oldName": old_name,
                "newName": new_name,
                "sourceFile": file_path.to_string_lossy()
            }),
            created_at: chrono::Utc::now(),
            complexity: std::cmp::min(impact_areas.len() as u8, 10), // Complexity based on number of files affected
            impact_areas,
        },
    })
}

/// Find all references to a symbol in source code and generate rename edits
fn find_symbol_references(
    source: &str,
    old_name: &str,
    new_name: &str,
    _file_path: &Path,
) -> AstResult<Vec<TextEdit>> {
    let mut edits = Vec::new();

    // Simple text-based search for symbol references
    // This is a simplified implementation - a full implementation would use AST parsing
    for (line_num, line) in source.lines().enumerate() {
        let mut search_pos = 0;
        while let Some(pos) = line[search_pos..].find(old_name) {
            let actual_pos = search_pos + pos;

            // Basic check to ensure we're replacing a whole word, not part of another identifier
            let is_word_boundary_before = actual_pos == 0
                || !line
                    .chars()
                    .nth(actual_pos - 1)
                    .unwrap_or(' ')
                    .is_alphanumeric();
            let is_word_boundary_after = actual_pos + old_name.len() >= line.len()
                || !line
                    .chars()
                    .nth(actual_pos + old_name.len())
                    .unwrap_or(' ')
                    .is_alphanumeric();

            if is_word_boundary_before && is_word_boundary_after {
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
                    priority: 100, // High priority for definition site
                    description: format!(
                        "Rename symbol '{}' to '{}' in definition file",
                        old_name, new_name
                    ),
                });
            }

            search_pos = actual_pos + old_name.len();
        }
    }

    Ok(edits)
}

/// Generate edits to update import statements in a file
fn generate_import_update_edits(
    source: &str,
    old_name: &str,
    new_name: &str,
    file_path: &Path,
) -> AstResult<Vec<TextEdit>> {
    let mut edits = Vec::new();

    // Parse import statements and find ones that need updating
    for (line_num, line) in source.lines().enumerate() {
        let trimmed = line.trim();

        // Handle ES6 imports: import { oldName } from '...'
        if trimmed.starts_with("import") && trimmed.contains(old_name) {
            // Simple pattern matching for named imports
            if let Some(brace_start) = trimmed.find('{') {
                if let Some(brace_end) = trimmed.find('}') {
                    let import_list = &trimmed[brace_start..=brace_end];
                    if import_list.contains(old_name) {
                        // Replace the old name with new name in the import list
                        let updated_line = line.replace(old_name, new_name);

                        edits.push(TextEdit {
                            edit_type: EditType::UpdateImport,
                            location: EditLocation {
                                start_line: line_num as u32,
                                start_column: 0,
                                end_line: line_num as u32,
                                end_column: line.len() as u32,
                            },
                            original_text: line.to_string(),
                            new_text: updated_line,
                            priority: 50, // Medium priority for import updates
                            description: format!(
                                "Update import of '{}' to '{}' in {}",
                                old_name,
                                new_name,
                                file_path.display()
                            ),
                        });
                    }
                }
            }
            // Handle default imports: import oldName from '...'
            else if trimmed.contains(&format!("import {}", old_name)) {
                let updated_line = line.replace(
                    &format!("import {}", old_name),
                    &format!("import {}", new_name),
                );

                edits.push(TextEdit {
                    edit_type: EditType::UpdateImport,
                    location: EditLocation {
                        start_line: line_num as u32,
                        start_column: 0,
                        end_line: line_num as u32,
                        end_column: line.len() as u32,
                    },
                    original_text: line.to_string(),
                    new_text: updated_line,
                    priority: 50,
                    description: format!(
                        "Update default import of '{}' to '{}' in {}",
                        old_name,
                        new_name,
                        file_path.display()
                    ),
                });
            }
        }
    }

    Ok(edits)
}

/// Check if an import is from the target file we're refactoring
fn is_import_from_target_file(import: &ImportInfo, target_file: &Path) -> bool {
    // This is a simplified check - a full implementation would resolve module paths properly
    let target_name = target_file
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("");

    // Check if the import path matches the target file
    import.module_path.contains(target_name)
        || import
            .module_path
            .ends_with(&target_file.to_string_lossy().replace('\\', "/"))
}

/// Find the project root by looking for common project markers
fn find_project_root(file_path: &Path) -> PathBuf {
    let mut current = file_path.parent().unwrap_or(file_path);

    loop {
        // Look for common project root indicators
        if current.join("package.json").exists()
            || current.join("Cargo.toml").exists()
            || current.join(".git").exists()
            || current.join("tsconfig.json").exists()
        {
            return current.to_path_buf();
        }

        if let Some(parent) = current.parent() {
            current = parent;
        } else {
            // If we can't find a project root, use the file's directory
            return file_path.parent().unwrap_or(file_path).to_path_buf();
        }
    }
}

/// Find all files in the project that might import the target symbol
fn find_files_importing_symbol(
    project_root: &Path,
    target_file: &Path,
    _symbol_name: &str,
) -> AstResult<Vec<PathBuf>> {
    let mut files = Vec::new();

    // Walk through the project directory and find source files
    if let Ok(entries) = std::fs::read_dir(project_root) {
        for entry in entries.flatten() {
            let path = entry.path();

            if path.is_file() {
                let extension = path.extension().and_then(|ext| ext.to_str()).unwrap_or("");

                // Only check source files that could contain imports
                if matches!(extension, "ts" | "tsx" | "js" | "jsx" | "py" | "rs")
                    && path != target_file
                {
                    files.push(path);
                }
            } else if path.is_dir()
                && !path
                    .file_name()
                    .unwrap_or_default()
                    .to_str()
                    .unwrap_or("")
                    .starts_with('.')
            {
                // Recursively search subdirectories (excluding hidden dirs)
                if let Ok(subfiles) = find_files_importing_symbol(&path, target_file, _symbol_name)
                {
                    files.extend(subfiles);
                }
            }
        }
    }

    Ok(files)
}
