use super::{RenameOptions, RenameService, RenameTarget};
use crate::handlers::tools::cross_file_references;
use lsp_types::WorkspaceEdit;
use mill_foundation::errors::{MillError as ServerError, MillResult as ServerResult};
use mill_foundation::planning::{PlanMetadata, RenamePlan};
use serde_json::json;
use std::path::Path;
use tracing::{debug, error};

impl RenameService {
    /// Generate plan for symbol rename using LSP
    pub(crate) async fn plan_symbol_rename(
        &self,
        target: &RenameTarget,
        new_name: &str,
        _options: &RenameOptions,
        context: &mill_handler_api::ToolHandlerContext,
    ) -> ServerResult<RenamePlan> {
        debug!(path = %target.path, new_name = %new_name, "Planning symbol rename via LSP");

        // Extract position from selector
        let position = target
            .selector
            .as_ref()
            .ok_or_else(|| {
                ServerError::invalid_request("Symbol rename requires selector.position")
            })?
            .position;

        // Get file extension to determine LSP client
        let path = Path::new(&target.path);
        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .ok_or_else(|| {
                ServerError::invalid_request(format!("File has no extension: {}", target.path))
            })?;

        // Get LSP adapter
        let lsp_adapter = context.lsp_adapter.lock().await;
        let adapter = lsp_adapter
            .as_ref()
            .ok_or_else(|| ServerError::internal("LSP adapter not initialized"))?;

        // Get or create LSP client for this extension
        let client = adapter.get_or_create_client(extension).await.map_err(|e| {
            ServerError::not_supported(format!(
                "No LSP server configured for extension {}: {}",
                extension, e
            ))
        })?;

        // Convert path to absolute and create file URI
        let abs_path = tokio::fs::canonicalize(path)
            .await
            .unwrap_or_else(|_| path.to_path_buf());
        let file_uri = url::Url::from_file_path(&abs_path)
            .map_err(|_| {
                ServerError::internal(format!("Invalid file path: {}", abs_path.display()))
            })?
            .to_string();

        // Build LSP rename request
        let lsp_params = json!({
            "textDocument": {
                "uri": file_uri
            },
            "position": position,
            "newName": new_name
        });

        // Send textDocument/rename request to LSP
        debug!(method = "textDocument/rename", "Sending LSP request");
        let lsp_result = client
            .send_request("textDocument/rename", lsp_params)
            .await
            .map_err(|e| {
                error!(error = %e, "LSP rename request failed");
                ServerError::internal(format!("LSP rename failed: {}", e))
            })?;

        // Parse WorkspaceEdit from LSP response
        let workspace_edit: WorkspaceEdit = serde_json::from_value(lsp_result).map_err(|e| {
            ServerError::internal(format!("Failed to parse LSP WorkspaceEdit: {}", e))
        })?;

        // Get the old symbol name by reading the source file directly
        // (We use tokio::fs instead of std::fs to avoid blocking the async runtime,
        // since the path was already validated by LSP)
        let old_symbol_name = tokio::fs::read_to_string(&abs_path)
            .await
            .ok()
            .and_then(|content| {
                cross_file_references::extract_symbol_at_position_public(
                    &content,
                    position.line,
                    position.character,
                )
            });

        // Enhance with cross-file edits if we have the old symbol name
        let workspace_edit = if let Some(old_name) = old_symbol_name {
            // Clone the original edit so we can fall back to it on error
            let original_edit = workspace_edit.clone();
            cross_file_references::enhance_symbol_rename(
                workspace_edit,
                &abs_path,
                position.line,
                position.character,
                &old_name,
                new_name,
                context,
            )
            .await
            .unwrap_or_else(|e| {
                debug!(error = %e, "Cross-file symbol rename enhancement failed, using LSP-only result");
                // Return the original LSP result (not empty!)
                original_edit
            })
        } else {
            debug!("Could not extract old symbol name, skipping cross-file enhancement");
            workspace_edit
        };

        // Calculate file checksums and summary
        let (file_checksums, summary, warnings) = self
            .analyze_workspace_edit(&workspace_edit, context)
            .await?;

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
