use crate::handlers::tools::ToolHandlerContext;
use super::{RenamePlanParams, RenameHandler};
use cb_protocol::{
    refactor_plan::{PlanMetadata, RenamePlan},
    ApiError as ServerError, ApiResult as ServerResult,
};
use lsp_types::WorkspaceEdit;
use serde_json::json;
use std::path::Path;
use tracing::{debug, error};

impl RenameHandler {
    /// Generate plan for symbol rename using LSP
    pub(crate) async fn plan_symbol_rename(
        &self,
        params: &RenamePlanParams,
        context: &ToolHandlerContext,
    ) -> ServerResult<RenamePlan> {
        debug!(path = %params.target.path, "Planning symbol rename via LSP");

        // Extract position from selector
        let position = params
            .target
            .selector
            .as_ref()
            .ok_or_else(|| {
                ServerError::InvalidRequest("Symbol rename requires selector.position".into())
            })?
            .position;

        // Get file extension to determine LSP client
        let path = Path::new(&params.target.path);
        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .ok_or_else(|| {
                ServerError::InvalidRequest(format!(
                    "File has no extension: {}",
                    params.target.path
                ))
            })?;

        // Get LSP adapter
        let lsp_adapter = context.lsp_adapter.lock().await;
        let adapter = lsp_adapter
            .as_ref()
            .ok_or_else(|| ServerError::Internal("LSP adapter not initialized".into()))?;

        // Get or create LSP client for this extension
        let client = adapter.get_or_create_client(extension).await.map_err(|e| {
            ServerError::Unsupported(format!(
                "No LSP server configured for extension {}: {}",
                extension, e
            ))
        })?;

        // Convert path to absolute and create file URI
        let abs_path = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
        let file_uri = url::Url::from_file_path(&abs_path)
            .map_err(|_| {
                ServerError::Internal(format!("Invalid file path: {}", abs_path.display()))
            })?
            .to_string();

        // Build LSP rename request
        let lsp_params = json!({
            "textDocument": {
                "uri": file_uri
            },
            "position": position,
            "newName": params.new_name
        });

        // Send textDocument/rename request to LSP
        debug!(method = "textDocument/rename", "Sending LSP request");
        let lsp_result = client
            .send_request("textDocument/rename", lsp_params)
            .await
            .map_err(|e| {
                error!(error = %e, "LSP rename request failed");
                ServerError::Internal(format!("LSP rename failed: {}", e))
            })?;

        // Parse WorkspaceEdit from LSP response
        let workspace_edit: WorkspaceEdit = serde_json::from_value(lsp_result).map_err(|e| {
            ServerError::Internal(format!("Failed to parse LSP WorkspaceEdit: {}", e))
        })?;

        // Calculate file checksums and summary
        let (file_checksums, summary, warnings) = self
            .analyze_workspace_edit(&workspace_edit, context)
            .await?;

        // Determine language from extension
        let language = super::utils::extension_to_language(extension);

        // Build metadata
        let metadata = PlanMetadata {
            plan_version: "1.0".to_string(),
            kind: "rename".to_string(),
            language,
            estimated_impact: super::utils::estimate_impact(summary.affected_files),
            created_at: chrono::Utc::now().to_rfc3339(),
        };

        Ok(RenamePlan {
            edits: workspace_edit,
            summary,
            warnings,
            metadata,
            file_checksums,
            is_consolidation: false, // Symbol renames are never consolidations
        })
    }
}
