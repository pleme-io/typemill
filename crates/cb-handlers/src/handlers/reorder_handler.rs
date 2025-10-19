//! Reorder handler for Unified Refactoring API
//!
//! Implements `reorder.plan` command for:
//! - Reordering function parameters
//! - Reordering struct fields
//! - Reordering imports
//! - Reordering statements

use crate::handlers::tools::{ToolHandler, ToolHandlerContext};
use async_trait::async_trait;
use codebuddy_core::model::mcp::ToolCall;
use codebuddy_foundation::protocol::{
    refactor_plan::{PlanMetadata, PlanSummary, ReorderPlan},
    ApiError as ServerError, ApiResult as ServerResult, RefactorPlan,
};
use lsp_types::{Position, WorkspaceEdit};
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::Path;
use tracing::{debug, error, info};

/// Handler for reorder.plan operations
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

#[derive(Debug, Deserialize, Default)]
#[allow(dead_code)] // Reserved for future configuration
struct ReorderOptions {
    #[serde(default)]
    preserve_formatting: Option<bool>,
    #[serde(default)]
    update_call_sites: Option<bool>, // For parameter reordering
}

#[async_trait]
impl ToolHandler for ReorderHandler {
    fn tool_names(&self) -> &[&str] {
        &["reorder.plan"]
    }

    fn is_internal(&self) -> bool {
        false // Public tool
    }

    async fn handle_tool_call(
        &self,
        context: &ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        info!(tool_name = %tool_call.name, "Handling reorder.plan");

        // Parse parameters
        let args = tool_call.arguments.clone().ok_or_else(|| {
            ServerError::InvalidRequest("Missing arguments for reorder.plan".into())
        })?;

        let params: ReorderPlanParams = serde_json::from_value(args).map_err(|e| {
            ServerError::InvalidRequest(format!("Invalid reorder.plan parameters: {}", e))
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
                return Err(ServerError::InvalidRequest(format!(
                    "Unsupported reorder kind: {}. Must be one of: parameters, fields, imports, statements",
                    kind
                )));
            }
        };

        // Wrap in RefactorPlan enum for discriminant, then serialize for MCP protocol
        let refactor_plan = RefactorPlan::ReorderPlan(plan);
        let plan_json = serde_json::to_value(&refactor_plan).map_err(|e| {
            ServerError::Internal(format!("Failed to serialize reorder plan: {}", e))
        })?;

        Ok(json!({
            "content": plan_json
        }))
    }
}

impl ReorderHandler {
    /// Generate plan for reordering function parameters
    async fn plan_reorder_parameters(
        &self,
        params: &ReorderPlanParams,
        context: &ToolHandlerContext,
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
                Err(ServerError::Unsupported(
                    "Parameter reordering requires LSP server support. Consider using AST-based approach.".into()
                ))
            }
        }
    }

    /// Generate plan for reordering struct fields
    async fn plan_reorder_fields(
        &self,
        params: &ReorderPlanParams,
        context: &ToolHandlerContext,
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
                Err(ServerError::Unsupported(
                    "Field reordering requires LSP server support. Consider using AST-based approach.".into()
                ))
            }
        }
    }

    /// Generate plan for reordering imports
    async fn plan_reorder_imports(
        &self,
        params: &ReorderPlanParams,
        context: &ToolHandlerContext,
    ) -> ServerResult<ReorderPlan> {
        debug!(file_path = %params.target.file_path, "Planning import reorder");

        // Get file extension to determine LSP client
        let path = Path::new(&params.target.file_path);
        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .ok_or_else(|| {
                ServerError::InvalidRequest(format!(
                    "File has no extension: {}",
                    params.target.file_path
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
                ServerError::Internal(format!("LSP organize imports failed: {}", e))
            })?;

        // Parse WorkspaceEdit from LSP response
        let workspace_edit: WorkspaceEdit = serde_json::from_value(lsp_result).map_err(|e| {
            ServerError::Internal(format!("Failed to parse LSP WorkspaceEdit: {}", e))
        })?;

        // Read file content for checksum
        let content = context
            .app_state
            .file_service
            .read_file(&abs_path)
            .await
            .map_err(|e| {
                ServerError::Internal(format!("Failed to read file for checksum: {}", e))
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

        // Determine language from extension
        let language = self.extension_to_language(extension);

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
        context: &ToolHandlerContext,
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
                Err(ServerError::Unsupported(
                    "Statement reordering requires LSP server support. Consider using AST-based approach.".into()
                ))
            }
        }
    }

    /// Try to reorder using LSP code actions
    async fn try_lsp_reorder(
        &self,
        params: &ReorderPlanParams,
        context: &ToolHandlerContext,
        code_action_kind: &str,
    ) -> ServerResult<ReorderPlan> {
        // Get file extension to determine LSP client
        let path = Path::new(&params.target.file_path);
        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .ok_or_else(|| {
                ServerError::InvalidRequest(format!(
                    "File has no extension: {}",
                    params.target.file_path
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
                ServerError::Internal(format!("LSP reorder failed: {}", e))
            })?;

        // Parse code actions from response
        let code_actions: Vec<Value> = serde_json::from_value(lsp_result).map_err(|e| {
            ServerError::Internal(format!("Failed to parse LSP code actions: {}", e))
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
                ServerError::Unsupported(format!(
                    "No {} code action available from LSP",
                    code_action_kind
                ))
            })?;

        // Extract WorkspaceEdit from code action
        let workspace_edit: WorkspaceEdit = serde_json::from_value(
            reorder_action
                .get("edit")
                .cloned()
                .ok_or_else(|| ServerError::Internal("Code action missing edit field".into()))?,
        )
        .map_err(|e| ServerError::Internal(format!("Failed to parse WorkspaceEdit: {}", e)))?;

        // Read file content for checksum
        let content = context
            .app_state
            .file_service
            .read_file(&abs_path)
            .await
            .map_err(|e| {
                ServerError::Internal(format!("Failed to read file for checksum: {}", e))
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

        // Determine language from extension
        let language = self.extension_to_language(extension);

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

    /// Map file extension to language name
    fn extension_to_language(&self, extension: &str) -> String {
        match extension {
            "rs" => "rust",
            "ts" | "tsx" => "typescript",
            "js" | "jsx" => "javascript",
            "py" | "pyi" => "python",
            "go" => "go",
            "java" => "java",
            "swift" => "swift",
            "cs" => "csharp",
            _ => "unknown",
        }
        .to_string()
    }
}

/// Calculate SHA-256 checksum of file content
fn calculate_checksum(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}