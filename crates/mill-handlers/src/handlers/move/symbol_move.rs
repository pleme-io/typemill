//! Symbol move planning
//!
//! Handles symbol move operations using LSP code actions.
//! Falls back to AST-based approach when LSP is unavailable.

use lsp_types::Position;
use mill_foundation::errors::{MillError as ServerError, MillResult as ServerResult};
use mill_foundation::planning::{MovePlan, PlanMetadata};
use serde_json::{json, Value};
use std::path::Path;
use tracing::{debug, error, info, warn};

use super::validation::analyze_workspace_edit;
use crate::handlers::common::estimate_impact;

/// Generate plan for symbol move using LSP or AST fallback
pub async fn plan_symbol_move(
    target_path: &str,
    destination: &str,
    position: Position,
    context: &mill_handler_api::ToolHandlerContext,
    operation_id: &str,
) -> ServerResult<MovePlan> {
    info!(
        operation_id = %operation_id,
        path = %target_path,
        destination = %destination,
        line = position.line,
        character = position.character,
        "Starting symbol move planning"
    );

    // Get file extension to determine LSP client
    let path = Path::new(target_path);
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .ok_or_else(|| {
            error!(
                operation_id = %operation_id,
                path = %target_path,
                function = "plan_symbol_move",
                "File has no extension"
            );
            ServerError::invalid_request(format!("File has no extension: {}", target_path))
        })?;

    debug!(
        operation_id = %operation_id,
        extension = %extension,
        "Determined file extension, attempting LSP approach"
    );

    // Try LSP approach first
    let lsp_result = try_lsp_symbol_move(
        target_path,
        destination,
        extension,
        position,
        context,
        operation_id,
    )
    .await;

    match lsp_result {
        Ok(plan) => {
            info!(
                operation_id = %operation_id,
                affected_files = plan.summary.affected_files,
                "Symbol move plan completed successfully via LSP"
            );
            Ok(plan)
        }
        Err(e) => {
            // LSP failed, try AST fallback
            warn!(
                operation_id = %operation_id,
                error = %e,
                "LSP symbol move failed, attempting AST fallback"
            );
            ast_symbol_move_fallback(target_path, destination, context, operation_id).await
        }
    }
}

/// Try to move symbol using LSP
async fn try_lsp_symbol_move(
    target_path: &str,
    destination: &str,
    extension: &str,
    position: Position,
    context: &mill_handler_api::ToolHandlerContext,
    operation_id: &str,
) -> ServerResult<MovePlan> {
    debug!(
        operation_id = %operation_id,
        "Getting LSP adapter for symbol move"
    );

    // Get LSP adapter
    let lsp_adapter = context.lsp_adapter.lock().await;
    let adapter = lsp_adapter.as_ref().ok_or_else(|| {
        error!(
            operation_id = %operation_id,
            function = "try_lsp_symbol_move",
            "LSP adapter not initialized"
        );
        ServerError::internal("LSP adapter not initialized")
    })?;

    debug!(
        operation_id = %operation_id,
        extension = %extension,
        "Getting or creating LSP client for extension"
    );

    // Get or create LSP client for this extension
    let client = adapter.get_or_create_client(extension).await.map_err(|e| {
        error!(
            operation_id = %operation_id,
            error = %e,
            extension = %extension,
            function = "try_lsp_symbol_move",
            "No LSP server configured for extension"
        );
        ServerError::not_supported(format!(
            "No LSP server configured for extension {}: {}",
            extension, e
        ))
    })?;

    // Convert source path to absolute and create file URI
    let path = Path::new(target_path);
    let abs_path = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let file_uri = url::Url::from_file_path(&abs_path)
        .map_err(|_| {
            error!(
                operation_id = %operation_id,
                path = %abs_path.display(),
                function = "try_lsp_symbol_move",
                "Invalid file path for URI conversion"
            );
            ServerError::internal(format!("Invalid file path: {}", abs_path.display()))
        })?
        .to_string();

    debug!(
        operation_id = %operation_id,
        file_uri = %file_uri,
        "Converted source path to URI for LSP request"
    );

    // Convert destination path to absolute and create destination URI
    let dest_path = Path::new(destination);
    let abs_dest_path =
        std::fs::canonicalize(dest_path).unwrap_or_else(|_| dest_path.to_path_buf());
    let destination_uri = url::Url::from_file_path(&abs_dest_path)
        .map_err(|_| {
            error!(
                operation_id = %operation_id,
                destination = %abs_dest_path.display(),
                function = "try_lsp_symbol_move",
                "Invalid destination path for URI conversion"
            );
            ServerError::internal(format!(
                "Invalid destination path: {}",
                abs_dest_path.display()
            ))
        })?
        .to_string();

    debug!(
        operation_id = %operation_id,
        destination_uri = %destination_uri,
        "Converted destination path to URI for LSP request"
    );

    // Build LSP code action request for move refactoring
    // Include destination URI in context.data to inform the LSP where to move the symbol
    let lsp_params = json!({
        "textDocument": {
            "uri": file_uri
        },
        "range": {
            "start": position,
            "end": position
        },
        "context": {
            "diagnostics": [],
            "only": ["refactor.move"],
            "data": {
                "destinationUri": destination_uri
            }
        }
    });

    // Send textDocument/codeAction request to LSP
    debug!(
        operation_id = %operation_id,
        method = "textDocument/codeAction",
        request = ?lsp_params,
        "Sending textDocument/codeAction request to LSP"
    );

    let lsp_result = client
        .send_request("textDocument/codeAction", lsp_params)
        .await
        .map_err(|e| {
            error!(
                operation_id = %operation_id,
                error = %e,
                method = "textDocument/codeAction",
                function = "try_lsp_symbol_move",
                "LSP textDocument/codeAction request failed"
            );
            ServerError::internal(format!("LSP move failed: {}", e))
        })?;

    debug!(
        operation_id = %operation_id,
        response = ?lsp_result,
        "Received response from LSP textDocument/codeAction"
    );

    // Parse code actions from response
    let code_actions: Vec<Value> = serde_json::from_value(lsp_result).map_err(|e| {
        error!(
            operation_id = %operation_id,
            error = %e,
            function = "try_lsp_symbol_move",
            "Failed to parse LSP code actions from response"
        );
        ServerError::internal(format!("Failed to parse LSP code actions: {}", e))
    })?;

    info!(
        operation_id = %operation_id,
        code_actions_count = code_actions.len(),
        "Parsed code actions from LSP response"
    );

    // Find the appropriate move action
    let move_action = code_actions
        .into_iter()
        .find(|action| {
            action
                .get("kind")
                .and_then(|k| k.as_str())
                .map(|k| k.starts_with("refactor.move"))
                .unwrap_or(false)
        })
        .ok_or_else(|| {
            error!(
                operation_id = %operation_id,
                function = "try_lsp_symbol_move",
                "No move code action found in LSP response"
            );
            ServerError::not_supported("No move code action available from LSP")
        })?;

    debug!(
        operation_id = %operation_id,
        "Found refactor.move code action, extracting WorkspaceEdit"
    );

    // Extract WorkspaceEdit from code action
    // Handle two cases: direct edit or command execution
    let workspace_edit = if move_action.get("edit").is_some() && !move_action["edit"].is_null() {
        // Case A: Edit is directly available
        info!(
            operation_id = %operation_id,
            "Found direct WorkspaceEdit in CodeAction"
        );
        serde_json::from_value(move_action["edit"].clone()).map_err(|e| {
            error!(
                operation_id = %operation_id,
                error = %e,
                function = "try_lsp_symbol_move",
                "Failed to parse WorkspaceEdit from code action"
            );
            ServerError::internal(format!("Failed to parse WorkspaceEdit: {}", e))
        })?
    } else if move_action.get("command").is_some() && !move_action["command"].is_null() {
        // Case B: A command needs to be executed
        info!(
            operation_id = %operation_id,
            "Found Command in CodeAction, sending workspace/executeCommand"
        );

        let command: lsp_types::Command = serde_json::from_value(move_action["command"].clone())
            .map_err(|e| {
                error!(
                    operation_id = %operation_id,
                    error = %e,
                    function = "try_lsp_symbol_move",
                    "Failed to parse Command from code action"
                );
                ServerError::internal(format!("Failed to parse Command: {}", e))
            })?;

        debug!(
            operation_id = %operation_id,
            command = %command.command,
            arguments_count = command.arguments.as_ref().map(|a| a.len()).unwrap_or(0),
            "Parsed Command, preparing workspace/executeCommand request"
        );

        let params = lsp_types::ExecuteCommandParams {
            command: command.command,
            arguments: command.arguments.unwrap_or_default(),
            work_done_progress_params: Default::default(),
        };

        // Serialize params to JSON Value for send_request
        let params_value = serde_json::to_value(&params).map_err(|e| {
            error!(
                operation_id = %operation_id,
                error = %e,
                function = "try_lsp_symbol_move",
                "Failed to serialize ExecuteCommandParams"
            );
            ServerError::internal(format!("Failed to serialize ExecuteCommandParams: {}", e))
        })?;

        // Send the command and try to interpret the result as a WorkspaceEdit
        let result_value = client
            .send_request("workspace/executeCommand", params_value)
            .await
            .map_err(|e| {
                error!(
                    operation_id = %operation_id,
                    error = %e,
                    method = "workspace/executeCommand",
                    function = "try_lsp_symbol_move",
                    "LSP workspace/executeCommand request failed"
                );
                ServerError::internal(format!("executeCommand failed: {}", e))
            })?;

        debug!(
            operation_id = %operation_id,
            response = ?result_value,
            "Received response from workspace/executeCommand"
        );

        serde_json::from_value(result_value).map_err(|e| {
            error!(
                operation_id = %operation_id,
                error = %e,
                function = "try_lsp_symbol_move",
                "Failed to parse WorkspaceEdit from executeCommand result"
            );
            ServerError::internal(format!("Failed to parse WorkspaceEdit: {}", e))
        })?
    } else {
        // No actionable information found
        error!(
            operation_id = %operation_id,
            function = "try_lsp_symbol_move",
            "CodeAction contained neither an edit nor a command"
        );
        return Err(ServerError::not_supported(
            "CodeAction contained neither an edit nor a command.",
        ));
    };

    // Calculate file checksums and summary
    debug!(
        operation_id = %operation_id,
        "Analyzing WorkspaceEdit to calculate checksums and summary"
    );

    let (file_checksums, summary, warnings) =
        analyze_workspace_edit(&workspace_edit, context).await?;

    info!(
        operation_id = %operation_id,
        affected_files = summary.affected_files,
        checksums_count = file_checksums.len(),
        warnings_count = warnings.len(),
        "Analyzed WorkspaceEdit successfully"
    );

    // Determine language from extension via plugin registry
    let language = context
        .app_state
        .language_plugins
        .get_plugin(extension)
        .map(|p| p.metadata().name.to_string())
        .unwrap_or_else(|| "unknown".to_string());

    // Build metadata
    let metadata = PlanMetadata {
        plan_version: "1.0".to_string(),
        kind: "move".to_string(),
        language,
        estimated_impact: estimate_impact(summary.affected_files),
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    debug!(
        operation_id = %operation_id,
        language = %metadata.language,
        impact = %metadata.estimated_impact,
        "Built metadata for MovePlan"
    );

    Ok(MovePlan {
        edits: workspace_edit,
        summary,
        warnings,
        metadata,
        file_checksums,
    })
}

/// AST-based fallback for symbol move
async fn ast_symbol_move_fallback(
    _target_path: &str,
    _destination: &str,
    _context: &mill_handler_api::ToolHandlerContext,
    operation_id: &str,
) -> ServerResult<MovePlan> {
    // For now, return unsupported error
    // Full AST-based symbol move would require extensive analysis
    error!(
        operation_id = %operation_id,
        function = "ast_symbol_move_fallback",
        "AST-based symbol move not yet implemented"
    );
    Err(ServerError::not_supported(
        "AST-based symbol move not yet implemented. LSP server required.",
    ))
}
