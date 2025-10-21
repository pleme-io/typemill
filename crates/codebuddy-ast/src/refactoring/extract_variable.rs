use super::common::{detect_language, extract_range_text};
use super::{CodeRange, ExtractVariableAnalysis, LspRefactoringService};
use crate::error::{AstError, AstResult};
use codebuddy_foundation::protocol::{ EditPlan , EditPlanMetadata , EditType , TextEdit , ValidationRule , ValidationType };
use std::collections::HashMap;
use tracing::debug;

/// Analyze a selected expression for extraction into a variable (simplified fallback)
pub fn analyze_extract_variable(
    source: &str,
    start_line: u32,
    start_col: u32,
    end_line: u32,
    end_col: u32,
    _file_path: &str,
) -> AstResult<ExtractVariableAnalysis> {
    // Simplified text-based analysis (language plugins should provide AST-based analysis)
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

/// LSP-based extract variable refactoring
async fn lsp_extract_variable(
    lsp_service: &dyn LspRefactoringService,
    file_path: &str,
    range: &CodeRange,
    _variable_name: &str,
) -> AstResult<EditPlan> {
    debug!(
        file_path = %file_path,
        start_line = range.start_line,
        end_line = range.end_line,
        "Requesting LSP extract variable refactoring"
    );

    let actions = lsp_service
        .get_code_actions(
            file_path,
            range,
            Some(vec!["refactor.extract.constant".to_string()]),
        )
        .await?;

    let action = actions
        .as_array()
        .and_then(|arr| {
            arr.iter().find(|a| {
                a.get("kind")
                    .and_then(|k| k.as_str())
                    .map(|k| k.contains("extract"))
                    .unwrap_or(false)
            })
        })
        .ok_or_else(|| {
            AstError::analysis("LSP server returned no extract variable actions".to_string())
        })?;

    let workspace_edit = action
        .get("edit")
        .ok_or_else(|| AstError::analysis("Code action missing edit field".to_string()))?;

    codebuddy_foundation::protocol::EditPlan::from_lsp_workspace_edit(workspace_edit, file_path, "extract_variable")
        .map_err(|e| AstError::analysis(format!("Failed to convert LSP edit: {}", e)))
}

/// Generate edit plan for extract variable refactoring
///
/// This function implements an LSP-first approach:
/// 1. If LSP service is provided, try LSP code actions first
/// 2. Fall back to AST-based analysis if LSP is unavailable or fails
#[allow(clippy::too_many_arguments)]
pub async fn plan_extract_variable(
    source: &str,
    start_line: u32,
    start_col: u32,
    end_line: u32,
    end_col: u32,
    variable_name: Option<String>,
    file_path: &str,
    lsp_service: Option<&dyn LspRefactoringService>,
    _language_plugins: Option<&cb_plugin_api::PluginRegistry>,
) -> AstResult<EditPlan> {
    let range = CodeRange {
        start_line,
        start_col,
        end_line,
        end_col,
    };

    // Try LSP first if available
    if let Some(lsp) = lsp_service {
        let var_name = variable_name.as_deref().unwrap_or("extracted");
        debug!(file_path = %file_path, "Attempting LSP extract variable");
        match lsp_extract_variable(lsp, file_path, &range, var_name).await {
            Ok(plan) => {
                debug!(file_path = %file_path, "LSP extract variable succeeded");
                return Ok(plan);
            }
            Err(e) => {
                debug!(
                    error = %e,
                    file_path = %file_path,
                    "LSP extract variable failed, falling back to AST"
                );
            }
        }
    } else {
        debug!(file_path = %file_path, "No LSP service provided, using AST fallback");
    }

    // Fallback to AST-based implementation (only TypeScript and Rust supported after language reduction)
    match detect_language(file_path) {
        #[cfg(feature = "lang-typescript")]
        "typescript" | "javascript" => ast_extract_variable_ts_js(
            source,
            &analyze_extract_variable(source, start_line, start_col, end_line, end_col, file_path)?,
            variable_name,
            file_path,
        ),
        #[cfg(feature = "lang-rust")]
        "rust" => ast_extract_variable_rust(
            source,
            start_line,
            start_col,
            end_line,
            end_col,
            variable_name,
            file_path,
        ),
        _ => Err(AstError::analysis(format!(
            "Language not supported (only TypeScript and Rust). LSP server may provide this via code actions for: {}",
            file_path
        ))),
    }
}

/// Generate edit plan for extract variable refactoring (TypeScript/JavaScript)
#[allow(dead_code)]
fn ast_extract_variable_ts_js(
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

    // Replace the original expression with the variable name
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
                "insertionPoint": analysis.insertion_point,
                "expressionRange": analysis.expression_range
            }),
            created_at: chrono::Utc::now(),
            complexity: 2,
            impact_areas: vec!["variable_extraction".to_string()],
                consolidation: None,
        },
    })
}

/// Generate edit plan for extract variable refactoring (Rust) using AST
#[cfg(feature = "lang-rust")]
#[allow(clippy::too_many_arguments)]
fn ast_extract_variable_rust(
    source: &str,
    start_line: u32,
    start_col: u32,
    end_line: u32,
    end_col: u32,
    variable_name: Option<String>,
    file_path: &str,
) -> AstResult<EditPlan> {
    cb_lang_rust::refactoring::plan_extract_variable(
        source,
        start_line,
        start_col,
        end_line,
        end_col,
        variable_name,
        file_path,
    )
    .map_err(|e| AstError::analysis(format!("Rust refactoring error: {}", e)))
}