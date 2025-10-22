//! Generic Quick Refactoring Handler - One-step refactoring operations
//!
//! This handler provides a generic wrapper that combines `*.plan` and `workspace.apply_edit`
//! into single-step operations for ease of use. It eliminates duplication by handling
//! all refactoring types with the same logic.
//!
//! Supports all refactoring operations:
//! - `rename` (wraps `rename.plan`)
//! - `delete` (wraps `delete.plan`)
//! - `extract` (wraps `extract.plan`)
//! - `inline` (wraps `inline.plan`)
//! - `move` (wraps `move.plan`)
//! - `transform` (wraps `transform.plan`)
//! - `reorder` (wraps `reorder.plan`)

use super::tools::{ToolHandler, ToolHandlerContext};
use super::workspace_apply_handler::WorkspaceApplyHandler;
use async_trait::async_trait;
use codebuddy_foundation::core::model::mcp::ToolCall;
use codebuddy_foundation::protocol::{ApiError as ServerError, ApiResult as ServerResult};
use serde_json::{json, Value};
use std::sync::Arc;
use tracing::{debug, info};

/// Configuration for a quick refactoring operation
#[derive(Debug, Clone)]
struct RefactoringConfig {
    /// The quick tool name (e.g., "rename", "delete")
    quick_name: &'static str,
    /// The plan tool name (e.g., "rename.plan", "delete.plan")
    plan_name: &'static str,
}

/// Generic handler for one-step refactoring operations (plan + execute)
///
/// This handler wraps any `*.plan` tool and automatically applies the result.
/// It eliminates code duplication by using a configuration-driven approach.
pub struct QuickRefactoringHandler {
    /// Configuration for the refactoring operation this handler manages
    config: RefactoringConfig,
    /// The handler for applying workspace edits
    apply_handler: WorkspaceApplyHandler,
    /// The actual plan handler (dynamically resolved)
    plan_handler: Arc<dyn ToolHandler>,
}

impl QuickRefactoringHandler {
    /// Create a new quick refactoring handler for a specific operation
    ///
    /// # Arguments
    /// * `quick_name` - The quick tool name (e.g., "rename", "delete")
    /// * `plan_name` - The corresponding plan tool name (e.g., "rename.plan")
    /// * `plan_handler` - The handler that implements the plan operation
    pub fn new(
        quick_name: &'static str,
        plan_name: &'static str,
        plan_handler: Arc<dyn ToolHandler>,
    ) -> Self {
        Self {
            config: RefactoringConfig {
                quick_name,
                plan_name,
            },
            apply_handler: WorkspaceApplyHandler::new(),
            plan_handler,
        }
    }

    /// Execute the two-step refactoring: plan → apply
    async fn execute_refactoring(
        &self,
        context: &ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        info!(
            quick_tool = %self.config.quick_name,
            plan_tool = %self.config.plan_name,
            "Executing one-step refactoring (plan + apply)"
        );

        // Step 1: Generate the refactoring plan
        debug!(
            step = 1,
            plan_tool = %self.config.plan_name,
            "Generating refactoring plan"
        );

        let plan_call = ToolCall {
            name: self.config.plan_name.to_string(),
            arguments: tool_call.arguments.clone(),
        };

        let plan_result = self
            .plan_handler
            .handle_tool_call(context, &plan_call)
            .await
            .map_err(|e| {
                ServerError::Internal(format!(
                    "Failed to generate {} plan: {}",
                    self.config.quick_name, e
                ))
            })?;

        debug!(
            step = 1,
            plan_tool = %self.config.plan_name,
            "Plan generated successfully"
        );

        // Step 2: Automatically apply the plan
        debug!(step = 2, "Applying refactoring plan");

        let plan_content = plan_result.get("content").ok_or_else(|| {
            ServerError::Internal("Plan result missing 'content' field".into())
        })?;

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
            .map_err(|e| {
                ServerError::Internal(format!(
                    "Failed to apply {} plan: {}",
                    self.config.quick_name, e
                ))
            })?;

        info!(
            quick_tool = %self.config.quick_name,
            "Refactoring completed successfully (plan + apply)"
        );

        Ok(apply_result)
    }
}

#[async_trait]
impl ToolHandler for QuickRefactoringHandler {
    fn tool_names(&self) -> &[&str] {
        std::slice::from_ref(&self.config.quick_name)
    }

    fn is_internal(&self) -> bool {
        false // All quick refactoring tools are public
    }

    async fn handle_tool_call(
        &self,
        context: &ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        self.execute_refactoring(context, tool_call).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_refactoring_config() {
        let config = RefactoringConfig {
            quick_name: "rename",
            plan_name: "rename.plan",
        };

        assert_eq!(config.quick_name, "rename");
        assert_eq!(config.plan_name, "rename.plan");
    }
}
