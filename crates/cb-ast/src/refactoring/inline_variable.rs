use super::common::detect_language;
use super::{CodeRange, LspRefactoringService};
use crate::error::{AstError, AstResult};
use cb_protocol::EditPlan;
use tracing::debug;

/// Analyze variable declaration for inlining (simplified fallback - prefer LSP)
pub fn analyze_inline_variable(
    _source: &str,
    _variable_line: u32,
    _variable_col: u32,
    _file_path: &str,
) -> AstResult<()> {
    // Language plugins should provide AST-based analysis
    // This is just a stub for LSP fallback
    Err(AstError::analysis(
        "AST-based inline variable analysis requires language plugin support. Use LSP service instead.".to_string()
    ))
}

/// LSP-based inline variable refactoring
async fn lsp_inline_variable(
    lsp_service: &dyn LspRefactoringService,
    file_path: &str,
    variable_line: u32,
    variable_col: u32,
) -> AstResult<EditPlan> {
    debug!(
        file_path = %file_path,
        line = variable_line,
        col = variable_col,
        "Requesting LSP inline variable refactoring"
    );

    let range = CodeRange {
        start_line: variable_line,
        start_col: variable_col,
        end_line: variable_line,
        end_col: variable_col + 1,
    };

    let actions = lsp_service
        .get_code_actions(file_path, &range, Some(vec!["refactor.inline".to_string()]))
        .await?;

    let action = actions
        .as_array()
        .and_then(|arr| {
            arr.iter().find(|a| {
                a.get("kind")
                    .and_then(|k| k.as_str())
                    .map(|k| k.starts_with("refactor.inline"))
                    .unwrap_or(false)
            })
        })
        .ok_or_else(|| {
            AstError::analysis("LSP server returned no inline variable actions".to_string())
        })?;

    let workspace_edit = action
        .get("edit")
        .ok_or_else(|| AstError::analysis("Code action missing edit field".to_string()))?;

    cb_protocol::EditPlan::from_lsp_workspace_edit(workspace_edit, file_path, "inline_variable")
        .map_err(|e| AstError::analysis(format!("Failed to convert LSP edit: {}", e)))
}

/// Generate edit plan for inline variable refactoring
///
/// This function implements an LSP-first approach:
/// 1. If LSP service is provided, try LSP code actions first
/// 2. Fall back to AST-based analysis if LSP is unavailable or fails
pub async fn plan_inline_variable(
    source: &str,
    variable_line: u32,
    variable_col: u32,
    file_path: &str,
    lsp_service: Option<&dyn LspRefactoringService>,
) -> AstResult<EditPlan> {
    // Try LSP first if available
    if let Some(lsp) = lsp_service {
        match lsp_inline_variable(lsp, file_path, variable_line, variable_col).await {
            Ok(plan) => return Ok(plan),
            Err(e) => {
                debug!(
                    error = %e,
                    file_path = %file_path,
                    "LSP inline variable failed, falling back to AST"
                );
            }
        }
    }

    // Fallback to AST-based implementation
    match detect_language(file_path) {
        "python" => ast_inline_variable_python(source, variable_line, variable_col, file_path),
        "rust" => ast_inline_variable_rust(source, variable_line, variable_col, file_path),
        "go" => ast_inline_variable_go(source, variable_line, variable_col, file_path),
        _ => Err(AstError::analysis(format!(
            "Inline variable refactoring requires LSP service for file: {}. Language plugins provide AST fallback.",
            file_path
        ))),
    }
}

/// Generate edit plan for inline variable refactoring (Python) using AST
fn ast_inline_variable_python(
    source: &str,
    variable_line: u32,
    variable_col: u32,
    file_path: &str,
) -> AstResult<EditPlan> {
    cb_lang_python::refactoring::plan_inline_variable(
        source,
        variable_line,
        variable_col,
        file_path,
    )
    .map_err(|e| AstError::analysis(format!("Python refactoring error: {}", e)))
}

/// Generate edit plan for inline variable refactoring (Rust) using AST
fn ast_inline_variable_rust(
    source: &str,
    variable_line: u32,
    variable_col: u32,
    file_path: &str,
) -> AstResult<EditPlan> {
    cb_lang_rust::refactoring::plan_inline_variable(
        source,
        variable_line,
        variable_col,
        file_path,
    )
    .map_err(|e| AstError::analysis(format!("Rust refactoring error: {}", e)))
}

/// Generate edit plan for inline variable refactoring (Go) using AST
fn ast_inline_variable_go(
    source: &str,
    variable_line: u32,
    variable_col: u32,
    file_path: &str,
) -> AstResult<EditPlan> {
    cb_lang_go::refactoring::plan_inline_variable(
        source,
        variable_line,
        variable_col,
        file_path,
    )
    .map_err(|e| AstError::analysis(format!("Go refactoring error: {}", e)))
}