use super::{CodeRange, LspRefactoringService};
use crate::error::{AstError, AstResult};
use mill_foundation::protocol::EditPlan;
use tracing::debug;

/// LSP-based symbol move refactoring
async fn lsp_symbol_move(
    lsp_service: &dyn LspRefactoringService,
    file_path: &str,
    symbol_line: u32,
    symbol_col: u32,
    destination: &str,
) -> AstResult<EditPlan> {
    debug!(
        file_path = %file_path,
        line = symbol_line,
        col = symbol_col,
        destination = %destination,
        "Requesting LSP symbol move refactoring"
    );

    let range = CodeRange {
        start_line: symbol_line,
        start_col: symbol_col,
        end_line: symbol_line,
        end_col: symbol_col + 1,
    };

    let actions = lsp_service
        .get_code_actions(file_path, &range, Some(vec!["refactor.move".to_string()]))
        .await?;

    let action = actions
        .as_array()
        .and_then(|arr| {
            arr.iter().find(|a| {
                a.get("kind")
                    .and_then(|k| k.as_str())
                    .map(|k| k.starts_with("refactor.move"))
                    .unwrap_or(false)
            })
        })
        .ok_or_else(|| {
            AstError::analysis("LSP server returned no symbol move actions".to_string())
        })?;

    let workspace_edit = action
        .get("edit")
        .ok_or_else(|| AstError::analysis("Code action missing edit field".to_string()))?;

    mill_foundation::protocol::EditPlan::from_lsp_workspace_edit(
        workspace_edit,
        file_path,
        "move_symbol",
    )
    .map_err(|e| AstError::analysis(format!("Failed to convert LSP edit: {}", e)))
}

/// Generate edit plan for symbol move refactoring
///
/// This function implements an LSP-first approach:
/// 1. If language plugin supports symbol move, use it
/// 2. If LSP service is provided, try LSP code actions
/// 3. Fall back to AST-based analysis if both fail
pub async fn plan_symbol_move(
    source: &str,
    symbol_line: u32,
    symbol_col: u32,
    file_path: &str,
    destination: &str,
    language_plugins: Option<&mill_plugin_api::PluginDiscovery>,
) -> AstResult<EditPlan> {
    // Try language plugin capability first (faster, more reliable, under our control)
    if let Some(plugins) = language_plugins {
        if let Some(provider) = plugins.refactoring_provider_for_file(file_path) {
            if provider.supports_symbol_move() {
                debug!(
                    file_path = %file_path,
                    destination = %destination,
                    "Using language plugin for symbol move"
                );
                match provider
                    .plan_symbol_move(source, symbol_line, symbol_col, file_path, destination)
                    .await
                {
                    Ok(plan) => return Ok(plan),
                    Err(e) => {
                        debug!(
                            error = ?e,
                            file_path = %file_path,
                            "Language plugin symbol move failed"
                        );
                    }
                }
            }
        }
    }

    // Both plugin and LSP failed/unavailable
    Err(AstError::analysis(format!(
        "Symbol move not supported for: {}. Language plugin does not implement symbol move.",
        file_path
    )))
}
