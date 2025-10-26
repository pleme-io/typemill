//! Transform handler for Unified Refactoring API
//!
//! Implements `transform` command with dryRun option for:
//! - Converting between types (e.g., if-else to match)
//! - Adding/removing async/await
//! - Converting function to closure
//! - Other syntax transformations

use crate::handlers::tools::{ToolHandler, ToolHandlerContext};
use async_trait::async_trait;
use mill_foundation::core::model::mcp::ToolCall;
use mill_foundation::protocol::{ refactor_plan::{ PlanMetadata , PlanSummary , TransformPlan } , ApiError as ServerError , ApiResult as ServerResult , RefactorPlan , };
use lsp_types::{Range, WorkspaceEdit};
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::Path;
use tracing::{debug, error, info};

/// Handler for transform operations
pub struct TransformHandler;

impl TransformHandler {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TransformHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)] // Reserved for future options support
struct TransformPlanParams {
    transformation: Transformation,
    #[serde(default)]
    options: TransformOptions,
}

#[derive(Debug, Deserialize)]
struct Transformation {
    kind: String, // "if_to_match" | "add_async" | "remove_async" | "fn_to_closure" | etc.
    file_path: String,
    range: Range,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)] // Reserved for future configuration
struct TransformOptions {
    /// Preview mode - don't actually apply changes (default: true for safety)
    #[serde(default = "default_true")]
    dry_run: bool,
    #[serde(default)]
    preserve_formatting: Option<bool>,
    #[serde(default)]
    preserve_comments: Option<bool>,
}

fn default_true() -> bool {
    true
}

#[async_trait]
impl ToolHandler for TransformHandler {
    fn tool_names(&self) -> &[&str] {
        &["transform"]
    }

    fn is_internal(&self) -> bool {
        false // Public tool
    }

    async fn handle_tool_call(
        &self,
        context: &ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        info!(tool_name = %tool_call.name, "Handling transform");

        // Parse parameters
        let args = tool_call.arguments.clone().ok_or_else(|| {
            ServerError::InvalidRequest("Missing arguments for transform".into())
        })?;

        let params: TransformPlanParams = serde_json::from_value(args).map_err(|e| {
            ServerError::InvalidRequest(format!("Invalid transform parameters: {}", e))
        })?;

        debug!(
            kind = %params.transformation.kind,
            file_path = %params.transformation.file_path,
            "Generating transform plan"
        );

        // Dispatch based on transformation kind
        let plan = match params.transformation.kind.as_str() {
            "if_to_match" => self.plan_if_to_match(&params, context).await?,
            "match_to_if" => self.plan_match_to_if(&params, context).await?,
            "add_async" => self.plan_add_async(&params, context).await?,
            "remove_async" => self.plan_remove_async(&params, context).await?,
            "fn_to_closure" => self.plan_fn_to_closure(&params, context).await?,
            "closure_to_fn" => self.plan_closure_to_fn(&params, context).await?,
            kind => {
                return Err(ServerError::InvalidRequest(format!(
                    "Unsupported transform kind: {}. Must be one of: if_to_match, match_to_if, add_async, remove_async, fn_to_closure, closure_to_fn",
                    kind
                )));
            }
        };

        // Wrap in RefactorPlan enum for discriminant
        let refactor_plan = RefactorPlan::TransformPlan(plan);

        // Check if we should execute or just return plan
        if params.options.dry_run {
            // Return plan only (preview mode)
            let plan_json = serde_json::to_value(&refactor_plan).map_err(|e| {
                ServerError::Internal(format!("Failed to serialize transform plan: {}", e))
            })?;

            info!(
                operation = "transform",
                dry_run = true,
                "Returning transform plan (preview mode)"
            );

            Ok(json!({"content": plan_json}))
        } else {
            // Execute the plan
            info!(
                operation = "transform",
                dry_run = false,
                "Executing transform plan"
            );

            use mill_services::services::{ExecutionOptions, PlanExecutor};

            let executor = PlanExecutor::new(context.app_state.file_service.clone());
            let result = executor
                .execute_plan(refactor_plan, ExecutionOptions::default())
                .await?;

            let result_json = serde_json::to_value(&result).map_err(|e| {
                ServerError::Internal(format!("Failed to serialize execution result: {}", e))
            })?;

            info!(
                operation = "transform",
                success = result.success,
                applied_files = result.applied_files.len(),
                "Transform execution completed"
            );

            Ok(json!({"content": result_json}))
        }
    }
}

impl TransformHandler {
    /// Generate plan for converting if-else to match
    async fn plan_if_to_match(
        &self,
        params: &TransformPlanParams,
        context: &ToolHandlerContext,
    ) -> ServerResult<TransformPlan> {
        debug!(file_path = %params.transformation.file_path, "Planning if-to-match transform");

        // Try LSP-based code action approach
        self.try_lsp_transform(params, context, "refactor.rewrite.if-to-match")
            .await
    }

    /// Generate plan for converting match to if-else
    async fn plan_match_to_if(
        &self,
        params: &TransformPlanParams,
        context: &ToolHandlerContext,
    ) -> ServerResult<TransformPlan> {
        debug!(file_path = %params.transformation.file_path, "Planning match-to-if transform");

        // Try LSP-based code action approach
        self.try_lsp_transform(params, context, "refactor.rewrite.match-to-if")
            .await
    }

    /// Generate plan for adding async/await
    async fn plan_add_async(
        &self,
        params: &TransformPlanParams,
        context: &ToolHandlerContext,
    ) -> ServerResult<TransformPlan> {
        debug!(file_path = %params.transformation.file_path, "Planning add-async transform");

        // Try LSP-based code action approach
        self.try_lsp_transform(params, context, "refactor.rewrite.add-async")
            .await
    }

    /// Generate plan for removing async/await
    async fn plan_remove_async(
        &self,
        params: &TransformPlanParams,
        context: &ToolHandlerContext,
    ) -> ServerResult<TransformPlan> {
        debug!(file_path = %params.transformation.file_path, "Planning remove-async transform");

        // Try LSP-based code action approach
        self.try_lsp_transform(params, context, "refactor.rewrite.remove-async")
            .await
    }

    /// Generate plan for converting function to closure
    async fn plan_fn_to_closure(
        &self,
        params: &TransformPlanParams,
        context: &ToolHandlerContext,
    ) -> ServerResult<TransformPlan> {
        debug!(file_path = %params.transformation.file_path, "Planning fn-to-closure transform");

        // Try LSP-based code action approach
        self.try_lsp_transform(params, context, "refactor.rewrite.function-to-closure")
            .await
    }

    /// Generate plan for converting closure to function
    async fn plan_closure_to_fn(
        &self,
        params: &TransformPlanParams,
        context: &ToolHandlerContext,
    ) -> ServerResult<TransformPlan> {
        debug!(file_path = %params.transformation.file_path, "Planning closure-to-fn transform");

        // Try LSP-based code action approach
        self.try_lsp_transform(params, context, "refactor.rewrite.closure-to-function")
            .await
    }

    /// Try to transform using LSP code actions
    async fn try_lsp_transform(
        &self,
        params: &TransformPlanParams,
        context: &ToolHandlerContext,
        code_action_kind: &str,
    ) -> ServerResult<TransformPlan> {
        // Get file extension to determine LSP client
        let path = Path::new(&params.transformation.file_path);
        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .ok_or_else(|| {
                ServerError::InvalidRequest(format!(
                    "File has no extension: {}",
                    params.transformation.file_path
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

        // Build LSP code action request for transform refactoring
        let lsp_params = json!({
            "textDocument": {
                "uri": file_uri
            },
            "range": params.transformation.range,
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
                error!(error = %e, "LSP transform request failed");
                ServerError::Internal(format!("LSP transform failed: {}", e))
            })?;

        // Parse code actions from response
        let code_actions: Vec<Value> = serde_json::from_value(lsp_result).map_err(|e| {
            ServerError::Internal(format!("Failed to parse LSP code actions: {}", e))
        })?;

        // Find the appropriate transform action
        let transform_action = code_actions
            .into_iter()
            .find(|action| {
                action
                    .get("kind")
                    .and_then(|k| k.as_str())
                    .map(|k| k.starts_with(code_action_kind) || k.starts_with("refactor.rewrite"))
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
            transform_action
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
            kind: "transform".to_string(),
            language,
            estimated_impact: "low".to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
        };

        Ok(TransformPlan {
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