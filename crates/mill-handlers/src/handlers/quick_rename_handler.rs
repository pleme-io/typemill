//! Quick Rename Handler - One-step rename (plan + execute)
//!
//! This handler combines `rename.plan` and `workspace.apply_edit` into a single
//! operation for ease of use. It provides the `rename` tool which:
//! 1. Generates a rename plan (using RenameHandler)
//! 2. Automatically applies it (using WorkspaceApplyHandler)
//! 3. Returns the result
//!
//! Supports all rename types:
//! - Symbol (functions, classes, variables, etc.)
//! - File
//! - Directory

use super::rename_handler::RenameHandler;
use super::tools::{ToolHandler, ToolHandlerContext};
use super::workspace_apply_handler::WorkspaceApplyHandler;
use async_trait::async_trait;
use codebuddy_foundation::core::model::mcp::ToolCall;
use codebuddy_foundation::protocol::{ApiError as ServerError, ApiResult as ServerResult};
use serde_json::{json, Value};
use tracing::{debug, info};

/// Handler for one-step rename operations (plan + execute)
pub struct QuickRenameHandler {
    rename_handler: RenameHandler,
    apply_handler: WorkspaceApplyHandler,
}

impl QuickRenameHandler {
    pub fn new() -> Self {
        Self {
            rename_handler: RenameHandler::new(),
            apply_handler: WorkspaceApplyHandler::new(),
        }
    }
}

impl Default for QuickRenameHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ToolHandler for QuickRenameHandler {
    fn tool_names(&self) -> &[&str] {
        &["rename"]
    }

    fn is_internal(&self) -> bool {
        false // Public tool - simple one-step rename for users
    }

    async fn handle_tool_call(
        &self,
        context: &ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        info!(
            tool_name = %tool_call.name,
            "Handling one-step rename (plan + execute)"
        );

        // Step 1: Generate the rename plan using RenameHandler
        debug!("Step 1: Generating rename plan");
        let plan_call = ToolCall {
            name: "rename.plan".to_string(),
            arguments: tool_call.arguments.clone(),
        };

        let plan_result = self
            .rename_handler
            .handle_tool_call(context, &plan_call)
            .await
            .map_err(|e| ServerError::Internal(format!("Failed to generate rename plan: {}", e)))?;

        debug!("Step 1: Rename plan generated successfully");

        // Step 2: Automatically apply the plan using WorkspaceApplyHandler
        debug!("Step 2: Applying rename plan");
        let plan_content = plan_result
            .get("content")
            .ok_or_else(|| ServerError::Internal("Plan result missing 'content' field".into()))?;

        let apply_call = ToolCall {
            name: "workspace.apply_edit".to_string(),
            arguments: Some(json!({
                "plan": plan_content,
                "options": {
                    "dry_run": false
                }
            })),
        };

        let apply_result = self
            .apply_handler
            .handle_tool_call(context, &apply_call)
            .await
            .map_err(|e| ServerError::Internal(format!("Failed to apply rename plan: {}", e)))?;

        info!("Rename completed successfully (plan + execute)");

        Ok(apply_result)
    }
}
