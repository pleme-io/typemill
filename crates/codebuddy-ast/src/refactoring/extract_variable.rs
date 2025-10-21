use super::common::extract_range_text;
use super::{CodeRange, ExtractVariableAnalysis, LspRefactoringService};
use crate::error::{AstError, AstResult};
use codebuddy_foundation::protocol::EditPlan;
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

    codebuddy_foundation::protocol::EditPlan::from_lsp_workspace_edit(
        workspace_edit,
        file_path,
        "extract_variable",
    )
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
    language_plugins: Option<&cb_plugin_api::PluginRegistry>,
) -> AstResult<EditPlan> {
    let range = CodeRange {
        start_line,
        start_col,
        end_line,
        end_col,
    };

    // Try language plugin capability first (faster, more reliable, under our control)
    if let Some(plugins) = language_plugins {
        if let Some(provider) = plugins.refactoring_provider_for_file(file_path) {
            if provider.supports_extract_variable() {
                debug!(
                    file_path = %file_path,
                    "Using language plugin for extract variable"
                );
                match provider
                    .plan_extract_variable(
                        source,
                        start_line,
                        start_col,
                        end_line,
                        end_col,
                        variable_name.clone(),
                        file_path,
                    )
                    .await
                {
                    Ok(plan) => return Ok(plan),
                    Err(e) => {
                        debug!(
                            error = ?e,
                            file_path = %file_path,
                            "Language plugin extract variable failed, trying LSP fallback"
                        );
                    }
                }
            }
        }
    }

    // Fallback to LSP if plugin not available or failed
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
                    "LSP extract variable also failed"
                );
            }
        }
    }

    // Both plugin and LSP failed
    Err(AstError::analysis(format!(
        "Extract variable not supported for: {}. Neither language plugin nor LSP implementation succeeded.",
        file_path
    )))
}
