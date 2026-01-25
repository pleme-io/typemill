//! Reorder handler for Unified Refactoring API
//!
//! Implements `reorder` command with dryRun option for:
//! - Reordering function parameters
//! - Reordering struct fields
//! - Reordering imports
//! - Reordering statements

use crate::handlers::common::calculate_checksum;
use crate::handlers::tools::ToolHandler;
use async_trait::async_trait;
use lsp_types::{Position, WorkspaceEdit};
use mill_foundation::core::model::mcp::ToolCall;
use mill_foundation::errors::{MillError as ServerError, MillResult as ServerResult};
use mill_foundation::planning::{PlanMetadata, PlanSummary, RefactorPlan, ReorderPlan};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::Path;
use tracing::{debug, error, info};

/// Handler for reorder operations
pub struct ReorderHandler;

impl ReorderHandler {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ReorderHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)] // Reserved for future implementation
struct ReorderPlanParams {
    target: ReorderTarget,
    new_order: Vec<String>,
    #[serde(default)]
    options: ReorderOptions,
}

#[derive(Debug, Deserialize)]
struct ReorderTarget {
    kind: String, // "parameters" | "fields" | "imports" | "statements"
    file_path: String,
    position: Position,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)] // Reserved for future configuration
struct ReorderOptions {
    /// Preview mode - don't actually apply changes (default: true for safety)
    #[serde(default = "crate::default_true")]
    dry_run: bool,
    #[serde(default)]
    preserve_formatting: Option<bool>,
    #[serde(default)]
    update_call_sites: Option<bool>, // For parameter reordering
}

impl Default for ReorderOptions {
    fn default() -> Self {
        Self {
            dry_run: true, // Safe default: preview mode
            preserve_formatting: None,
            update_call_sites: None,
        }
    }
}

#[async_trait]
impl ToolHandler for ReorderHandler {
    fn tool_names(&self) -> &[&str] {
        &["reorder"]
    }

    fn is_internal(&self) -> bool {
        false // Public tool
    }

    async fn handle_tool_call(
        &self,
        context: &mill_handler_api::ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        info!(tool_name = %tool_call.name, "Handling reorder");

        // Parse parameters
        let args = tool_call
            .arguments
            .clone()
            .ok_or_else(|| ServerError::invalid_request("Missing arguments for reorder"))?;

        let params: ReorderPlanParams = serde_json::from_value(args).map_err(|e| {
            ServerError::invalid_request(format!("Invalid reorder parameters: {}", e))
        })?;

        debug!(
            kind = %params.target.kind,
            file_path = %params.target.file_path,
            "Generating reorder plan"
        );

        // Dispatch based on target kind
        let plan = match params.target.kind.as_str() {
            "parameters" => self.plan_reorder_parameters(&params, context).await?,
            "fields" => self.plan_reorder_fields(&params, context).await?,
            "imports" => self.plan_reorder_imports(&params, context).await?,
            "statements" => self.plan_reorder_statements(&params, context).await?,
            kind => {
                return Err(ServerError::invalid_request(format!(
                    "Unsupported reorder kind: {}. Must be one of: parameters, fields, imports, statements",
                    kind
                )));
            }
        };

        // Wrap in RefactorPlan enum for discriminant
        let refactor_plan = RefactorPlan::ReorderPlan(plan);

        // Check if we should execute or just return plan
        if params.options.dry_run {
            // Return plan only (preview mode)
            let plan_json = serde_json::to_value(&refactor_plan).map_err(|e| {
                ServerError::internal(format!("Failed to serialize reorder plan: {}", e))
            })?;

            info!(
                operation = "reorder",
                dry_run = true,
                "Returning reorder plan (preview mode)"
            );

            Ok(json!({"content": plan_json}))
        } else {
            // Execute the plan
            info!(
                operation = "reorder",
                dry_run = false,
                "Executing reorder plan"
            );

            use crate::handlers::tools::extensions::get_concrete_app_state;
            use mill_services::services::{ExecutionOptions, PlanExecutor};

            // Get concrete AppState to access concrete FileService
            let concrete_state = get_concrete_app_state(&context.app_state)?;
            let executor = PlanExecutor::new(concrete_state.file_service.clone());
            let result = executor
                .execute_plan(refactor_plan, ExecutionOptions::default())
                .await?;

            let result_json = serde_json::to_value(&result).map_err(|e| {
                ServerError::internal(format!("Failed to serialize execution result: {}", e))
            })?;

            info!(
                operation = "reorder",
                success = result.success,
                applied_files = result.applied_files.len(),
                "Reorder execution completed"
            );

            Ok(json!({"content": result_json}))
        }
    }
}

impl ReorderHandler {
    /// Generate plan for reordering function parameters
    async fn plan_reorder_parameters(
        &self,
        params: &ReorderPlanParams,
        context: &mill_handler_api::ToolHandlerContext,
    ) -> ServerResult<ReorderPlan> {
        debug!(file_path = %params.target.file_path, "Planning parameter reorder");

        // Try LSP-based code action approach
        let lsp_result = self
            .try_lsp_reorder(params, context, "refactor.reorder.parameters")
            .await;

        match lsp_result {
            Ok(plan) => Ok(plan),
            Err(e) => {
                // LSP failed, fall back to unsupported
                debug!(error = %e, "LSP parameter reorder failed");
                Err(ServerError::not_supported(
                    "Parameter reordering requires LSP server support. Consider using AST-based approach."
                ))
            }
        }
    }

    /// Generate plan for reordering struct fields
    async fn plan_reorder_fields(
        &self,
        params: &ReorderPlanParams,
        context: &mill_handler_api::ToolHandlerContext,
    ) -> ServerResult<ReorderPlan> {
        debug!(file_path = %params.target.file_path, "Planning field reorder");

        // Try LSP-based code action approach
        let lsp_result = self
            .try_lsp_reorder(params, context, "refactor.reorder.fields")
            .await;

        match lsp_result {
            Ok(plan) => Ok(plan),
            Err(e) => {
                // LSP failed, fall back to unsupported
                debug!(error = %e, "LSP field reorder failed");
                Err(ServerError::not_supported(
                    "Field reordering requires LSP server support. Consider using AST-based approach."
                ))
            }
        }
    }

    /// Generate plan for reordering imports
    async fn plan_reorder_imports(
        &self,
        params: &ReorderPlanParams,
        context: &mill_handler_api::ToolHandlerContext,
    ) -> ServerResult<ReorderPlan> {
        debug!(file_path = %params.target.file_path, "Planning import reorder");

        // Get file extension to determine LSP client
        let path = Path::new(&params.target.file_path);
        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .ok_or_else(|| {
                ServerError::invalid_request(format!(
                    "File has no extension: {}",
                    params.target.file_path
                ))
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
        let abs_path = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
        let file_uri = url::Url::from_file_path(&abs_path)
            .map_err(|_| {
                ServerError::internal(format!("Invalid file path: {}", abs_path.display()))
            })?
            .to_string();

        // Build LSP organize imports request
        // Note: organize imports is a common LSP capability
        let lsp_params = json!({
            "textDocument": {
                "uri": file_uri
            }
        });

        // Send textDocument/organizeImports request to LSP
        debug!(
            method = "textDocument/organizeImports",
            "Sending LSP request"
        );
        let lsp_result = client
            .send_request("textDocument/organizeImports", lsp_params)
            .await
            .map_err(|e| {
                error!(error = %e, "LSP organize imports request failed");
                ServerError::internal(format!("LSP organize imports failed: {}", e))
            })?;

        // Parse WorkspaceEdit from LSP response
        let workspace_edit: WorkspaceEdit = serde_json::from_value(lsp_result).map_err(|e| {
            ServerError::internal(format!("Failed to parse LSP WorkspaceEdit: {}", e))
        })?;

        // Read file content for checksum
        let content = context
            .app_state
            .file_service
            .read_file(&abs_path)
            .await
            .map_err(|e| {
                ServerError::internal(format!("Failed to read file for checksum: {}", e))
            })?;

        let mut file_checksums = HashMap::new();
        file_checksums.insert(
            abs_path.to_string_lossy().to_string(),
            calculate_checksum(&content),
        );

        // Build summary
        let summary = PlanSummary {
            affected_files: 1,
            created_files: 0,
            deleted_files: 0,
        };

        let warnings = Vec::new();

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
            kind: "reorder".to_string(),
            language,
            estimated_impact: "low".to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
        };

        Ok(ReorderPlan {
            edits: workspace_edit,
            summary,
            warnings,
            metadata,
            file_checksums,
        })
    }

    /// Generate plan for reordering statements
    async fn plan_reorder_statements(
        &self,
        params: &ReorderPlanParams,
        context: &mill_handler_api::ToolHandlerContext,
    ) -> ServerResult<ReorderPlan> {
        debug!(file_path = %params.target.file_path, "Planning statement reorder");

        // Try LSP-based code action approach
        let lsp_result = self
            .try_lsp_reorder(params, context, "refactor.reorder.statements")
            .await;

        match lsp_result {
            Ok(plan) => Ok(plan),
            Err(e) => {
                // LSP failed, fall back to unsupported
                debug!(error = %e, "LSP statement reorder failed");
                Err(ServerError::not_supported(
                    "Statement reordering requires LSP server support. Consider using AST-based approach."
                ))
            }
        }
    }

    /// Try to reorder using LSP code actions
    async fn try_lsp_reorder(
        &self,
        params: &ReorderPlanParams,
        context: &mill_handler_api::ToolHandlerContext,
        code_action_kind: &str,
    ) -> ServerResult<ReorderPlan> {
        // Get file extension to determine LSP client
        let path = Path::new(&params.target.file_path);
        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .ok_or_else(|| {
                ServerError::invalid_request(format!(
                    "File has no extension: {}",
                    params.target.file_path
                ))
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
        let abs_path = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
        let file_uri = url::Url::from_file_path(&abs_path)
            .map_err(|_| {
                ServerError::internal(format!("Invalid file path: {}", abs_path.display()))
            })?
            .to_string();

        // Build LSP code action request for reorder refactoring
        let lsp_params = json!({
            "textDocument": {
                "uri": file_uri
            },
            "range": {
                "start": params.target.position,
                "end": params.target.position
            },
            "context": {
                "diagnostics": [],
                "only": [code_action_kind]
            }
        });

        // Send textDocument/codeAction request to LSP
        debug!(
            method = "textDocument/codeAction",
            kind = code_action_kind,
            "Sending LSP request"
        );
        let lsp_result = client
            .send_request("textDocument/codeAction", lsp_params)
            .await
            .map_err(|e| {
                error!(error = %e, "LSP reorder request failed");
                ServerError::internal(format!("LSP reorder failed: {}", e))
            })?;

        // Parse code actions from response
        let code_actions: Vec<Value> = serde_json::from_value(lsp_result).map_err(|e| {
            ServerError::internal(format!("Failed to parse LSP code actions: {}", e))
        })?;

        // Find the appropriate reorder action
        let reorder_action = code_actions
            .into_iter()
            .find(|action| {
                action
                    .get("kind")
                    .and_then(|k| k.as_str())
                    .map(|k| k.starts_with(code_action_kind))
                    .unwrap_or(false)
            })
            .ok_or_else(|| {
                ServerError::not_supported(format!(
                    "No {} code action available from LSP",
                    code_action_kind
                ))
            })?;

        // Extract WorkspaceEdit from code action
        let workspace_edit: WorkspaceEdit = serde_json::from_value(
            reorder_action
                .get("edit")
                .cloned()
                .ok_or_else(|| ServerError::internal("Code action missing edit field"))?,
        )
        .map_err(|e| ServerError::internal(format!("Failed to parse WorkspaceEdit: {}", e)))?;

        // Read file content for checksum
        let content = context
            .app_state
            .file_service
            .read_file(&abs_path)
            .await
            .map_err(|e| {
                ServerError::internal(format!("Failed to read file for checksum: {}", e))
            })?;

        let mut file_checksums = HashMap::new();
        file_checksums.insert(
            abs_path.to_string_lossy().to_string(),
            calculate_checksum(&content),
        );

        // Build summary
        let summary = PlanSummary {
            affected_files: 1,
            created_files: 0,
            deleted_files: 0,
        };

        let warnings = Vec::new();

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
            kind: "reorder".to_string(),
            language,
            estimated_impact: "low".to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
        };

        Ok(ReorderPlan {
            edits: workspace_edit,
            summary,
            warnings,
            metadata,
            file_checksums,
        })
    }

    // Removed extension_to_language() - use plugin registry instead:
    // context.app_state.language_plugins.get_plugin(ext)?.metadata().name
}
