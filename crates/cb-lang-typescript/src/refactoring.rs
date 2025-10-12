//! TypeScript/JavaScript specific refactoring logic.
use cb_plugin_api::{PluginError, PluginResult};
use cb_protocol::{
    EditLocation, EditPlan, EditPlanMetadata, EditType, TextEdit, ValidationRule, ValidationType,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use swc_common::{sync::Lrc, FileName, FilePathMapping, SourceMap};
use swc_ecma_ast::*;
use swc_ecma_parser::{lexer::Lexer, Parser, StringInput, Syntax, TsSyntax};
use swc_ecma_visit::{Visit, VisitWith};

// Note: These structs are moved from cb-ast/src/refactoring.rs
// They might be better in a shared crate in the future.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CodeRange {
    pub start_line: u32,
    pub start_col: u32,
    pub end_line: u32,
    pub end_col: u32,
}

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InlineVariableAnalysis {
    pub variable_name: String,
    pub declaration_range: CodeRange,
    pub initializer_expression: String,
    pub usage_locations: Vec<CodeRange>,
    pub is_safe_to_inline: bool,
    pub blocking_reasons: Vec<String>,
}

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

// Moved from cb-ast/src/refactoring.rs
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

pub fn plan_inline_variable(
    source: &str,
    variable_line: u32,
    variable_col: u32,
    file_path: &str,
) -> PluginResult<EditPlan> {
    let analysis = analyze_inline_variable(source, variable_line, variable_col, file_path)?;
    ast_inline_variable_ts_js(source, &analysis)
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
    let analysis =
        analyze_extract_variable(source, start_line, start_col, end_line, end_col, file_path)?;
    ast_extract_variable_ts_js(source, &analysis, variable_name, file_path)
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
        location: analysis.insertion_point.clone().into(),
        original_text: String::new(),
        new_text: format!("\n{}\n", function_code),
        priority: 100,
        description: format!("Create extracted function '{}'", new_function_name),
    });

    let call_code = generate_function_call(&analysis, new_function_name)?;

    edits.push(TextEdit {
        file_path: None,
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

fn ast_inline_variable_ts_js(
    source: &str,
    analysis: &InlineVariableAnalysis,
) -> PluginResult<EditPlan> {
    if !analysis.is_safe_to_inline {
        return Err(PluginError::internal(format!(
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
            location: usage_location.clone().into(),
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
        location: analysis.declaration_range.clone().into(),
        original_text: extract_range_text(source, &analysis.declaration_range)?,
        new_text: String::new(),
        priority: 50,
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
            }),
            created_at: chrono::Utc::now(),
            complexity: (analysis.usage_locations.len().min(10)) as u8,
            impact_areas: vec!["variable_inlining".to_string()],
        },
    })
}

fn ast_extract_variable_ts_js(
    source: &str,
    analysis: &ExtractVariableAnalysis,
    variable_name: Option<String>,
    file_path: &str,
) -> PluginResult<EditPlan> {
    if !analysis.can_extract {
        return Err(PluginError::internal(format!(
            "Cannot extract expression: {}",
            analysis.blocking_reasons.join(", ")
        )));
    }

    let var_name = variable_name.unwrap_or_else(|| analysis.suggested_name.clone());

    let lines: Vec<&str> = source.lines().collect();
    let current_line = lines
        .get((analysis.insertion_point.start_line) as usize)
        .unwrap_or(&"");
    let indent = current_line
        .chars()
        .take_while(|c| c.is_whitespace())
        .collect::<String>();

    let mut edits = Vec::new();

    let declaration = format!("const {} = {};\n{}", var_name, analysis.expression, indent);
    edits.push(TextEdit {
        file_path: None,
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

    edits.push(TextEdit {
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
            }),
            created_at: chrono::Utc::now(),
            complexity: 2,
            impact_areas: vec!["variable_extraction".to_string()],
        },
    })
}

// --- Analysis Functions (moved from cb-ast) ---

pub fn analyze_extract_function(
    source: &str,
    range: &CodeRange,
    file_path: &str,
) -> PluginResult<ExtractableFunction> {
    let _cm = create_source_map(source, file_path)?;
    let _module = parse_module(source, file_path)?;
    let analyzer = ExtractFunctionAnalyzer::new(source, range.clone());
    analyzer.finalize()
}

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
        Err(e) => Err(PluginError::parse(format!("Failed to parse file: {:?}", e))),
    }
}

// --- Visitors (moved from cb-ast) ---

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
        let range_copy = self.selection_range.clone();
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
            PluginError::internal("Could not find variable declaration at specified location")
        })
    }
}

impl Visit for InlineVariableAnalyzer {
    // Simplified visit implementation
}

// --- Helper Functions (moved from cb-ast) ---

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
        .map_err(|e| PluginError::parse(format!("Failed to parse module: {:?}", e)))
}

fn extract_range_text(source: &str, range: &CodeRange) -> PluginResult<String> {
    let lines: Vec<&str> = source.lines().collect();
    if range.start_line == range.end_line {
        let line = lines
            .get(range.start_line as usize)
            .ok_or_else(|| PluginError::internal("Invalid line number"))?;
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
