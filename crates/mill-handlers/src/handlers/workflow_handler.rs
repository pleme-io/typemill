//! Workflow operations tool handler
//!
//! Handles: achieve_intent, apply_edits

use super::tools::{ToolHandler, ToolHandlerContext};
use async_trait::async_trait;
use mill_foundation::core::model::mcp::ToolCall;
use mill_foundation::protocol::{ApiError as ServerError, ApiResult as ServerResult};
use serde_json::{json, Value};
use tracing::{debug, error, info, warn};

pub struct WorkflowHandler;

impl WorkflowHandler {
    pub fn new() -> Self {
        Self
    }
}

impl Default for WorkflowHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ToolHandler for WorkflowHandler {
    fn tool_names(&self) -> &[&str] {
        &["achieve_intent", "apply_edits"]
    }

    async fn handle_tool_call(
        &self,
        context: &ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        debug!(tool_name = %tool_call.name, "Handling workflow operation");

        match tool_call.name.as_str() {
            "achieve_intent" => self.handle_achieve_intent(tool_call.clone(), context).await,
            "apply_edits" => self.handle_apply_edits(tool_call.clone(), context).await,
            _ => Err(ServerError::Unsupported(format!(
                "Unknown workflow operation: {}",
                tool_call.name
            ))),
        }
    }
}

impl WorkflowHandler {
    async fn handle_achieve_intent(
        &self,
        tool_call: ToolCall,
        context: &ToolHandlerContext,
    ) -> ServerResult<Value> {
        debug!(tool_name = %tool_call.name, "Planning or resuming workflow");

        let args = tool_call.arguments.ok_or_else(|| {
            ServerError::InvalidRequest("Missing arguments for achieve_intent".into())
        })?;

        // Check if this is a workflow resume request
        if let Some(workflow_id) = args.get("workflow_id").and_then(|v| v.as_str()) {
            info!(workflow_id = %workflow_id, "Resuming paused workflow");

            let resume_data = args.get("resume_data").cloned();

            return context
                .app_state
                .workflow_executor
                .resume_workflow(workflow_id, resume_data)
                .await;
        }

        // Otherwise, plan a new workflow
        let intent_value = args
            .get("intent")
            .ok_or_else(|| ServerError::InvalidRequest("Missing 'intent' parameter".into()))?;

        let intent: mill_foundation::core::model::workflow::Intent =
            serde_json::from_value(intent_value.clone()).map_err(|e| {
                ServerError::InvalidRequest(format!("Invalid intent format: {}", e))
            })?;

        // Check if we should execute the workflow
        let execute = args
            .get("execute")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // Check if dry-run mode is requested
        let dry_run = args
            .get("dryRun")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        match context.app_state.planner.plan_for_intent(&intent) {
            Ok(workflow) => {
                info!(
                    intent = %intent.name,
                    workflow_name = %workflow.name,
                    steps = workflow.steps.len(),
                    complexity = workflow.metadata.complexity,
                    execute = execute,
                    dry_run = dry_run,
                    "Successfully planned workflow"
                );

                // If execute is true, run the workflow
                if execute {
                    debug!(dry_run = dry_run, "Executing workflow");
                    match context
                        .app_state
                        .workflow_executor
                        .execute_workflow(&workflow, dry_run)
                        .await
                    {
                        Ok(result) => {
                            info!(
                                workflow_name = %workflow.name,
                                dry_run = dry_run,
                                "Workflow executed successfully"
                            );
                            Ok(result)
                        }
                        Err(e) => {
                            error!(
                                workflow_name = %workflow.name,
                                error = %e,
                                "Workflow execution failed"
                            );
                            Err(e)
                        }
                    }
                } else {
                    // Just return the plan
                    Ok(json!({
                        "success": true,
                        "workflow": workflow,
                    }))
                }
            }
            Err(e) => {
                error!(intent = %intent.name, error = %e, "Failed to plan workflow for intent");
                Err(ServerError::Runtime { message: e })
            }
        }
    }

    async fn handle_apply_edits(
        &self,
        tool_call: ToolCall,
        context: &ToolHandlerContext,
    ) -> ServerResult<Value> {
        debug!(tool_name = %tool_call.name, "Handling apply_edits");

        let args = tool_call.arguments.unwrap_or(json!({}));
        let edit_plan_value = args
            .get("edit_plan")
            .ok_or_else(|| ServerError::InvalidRequest("Missing 'edit_plan' parameter".into()))?;

        // Parse the EditPlan from the JSON value
        let edit_plan: mill_foundation::planning::EditPlan =
            serde_json::from_value(edit_plan_value.clone()).map_err(|e| {
                ServerError::InvalidRequest(format!("Invalid edit_plan format: {}", e))
            })?;

        debug!(
            source_file = %edit_plan.source_file,
            edits_count = edit_plan.edits.len(),
            dependency_updates_count = edit_plan.dependency_updates.len(),
            "Applying edit plan"
        );

        // Apply the edit plan using the FileService
        match context
            .app_state
            .file_service
            .apply_edit_plan(&edit_plan)
            .await
        {
            Ok(result) => {
                if result.success {
                    info!(
                        modified_files_count = result.modified_files.len(),
                        "Successfully applied edit plan"
                    );
                    Ok(json!({
                        "success": true,
                        "message": format!("Successfully applied edit plan to {} files",
                                         result.modified_files.len()),
                        "result": result
                    }))
                } else {
                    warn!(errors = ?result.errors, "Edit plan applied with errors");
                    Ok(json!({
                        "success": false,
                        "message": format!("Edit plan completed with errors: {}",
                                         result.errors.as_ref()
                                              .map(|e| e.join("; "))
                                              .unwrap_or_else(|| "Unknown errors".to_string())),
                        "result": result
                    }))
                }
            }
            Err(e) => {
                error!(error = %e, "Failed to apply edit plan");
                Err(ServerError::Runtime {
                    message: format!("Failed to apply edit plan: {}", e),
                })
            }
        }
    }
}
