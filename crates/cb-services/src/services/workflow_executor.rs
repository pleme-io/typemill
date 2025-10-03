//! The WorkflowExecutor service for executing multi-step workflows.

use cb_protocol::{ApiError as ServerError, ApiResult as ServerResult};
use cb_core::model::workflow::Workflow;
use cb_plugins::{PluginManager, PluginRequest};
use dashmap::DashMap;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{debug, error, info};
use uuid::Uuid;

/// State of a paused workflow waiting for user confirmation.
#[derive(Debug, Clone)]
pub struct PausedWorkflowState {
    /// The workflow being executed
    pub workflow: Workflow,
    /// The index of the step that requires confirmation
    pub step_index: usize,
    /// Results from previously executed steps
    pub step_results: HashMap<usize, Value>,
    /// Execution log up to this point
    pub log: Vec<String>,
    /// Whether this is a dry-run execution
    pub dry_run: bool,
}

/// Defines the contract for a service that can execute workflows.
#[async_trait::async_trait]
pub trait WorkflowExecutor: Send + Sync {
    async fn execute_workflow(&self, workflow: &Workflow, dry_run: bool) -> ServerResult<Value>;
    async fn resume_workflow(
        &self,
        workflow_id: &str,
        resume_data: Option<Value>,
    ) -> ServerResult<Value>;
    fn get_paused_workflow_count(&self) -> usize;
}

/// The default implementation of the WorkflowExecutor service.
pub struct DefaultWorkflowExecutor {
    /// Plugin manager for executing tool calls
    plugin_manager: Arc<PluginManager>,
    /// Cache of paused workflows waiting for confirmation
    paused_workflows: DashMap<String, PausedWorkflowState>,
}

impl DefaultWorkflowExecutor {
    pub fn new(plugin_manager: Arc<PluginManager>) -> Arc<Self> {
        Arc::new(Self {
            plugin_manager,
            paused_workflows: DashMap::new(),
        })
    }

    /// Replace placeholder values in step parameters with actual values from previous steps.
    ///
    /// Supports placeholder format: `$steps.{index}.{path}` where:
    /// - `{index}` is the 0-based step index
    /// - `{path}` is a dot-separated path into the result JSON (e.g., "result.locations")
    ///
    /// Example: `$steps.0.result.locations` retrieves the `locations` field from
    /// the `result` field of step 0's output.
    fn resolve_step_params(
        params: &Value,
        step_results: &HashMap<usize, Value>,
    ) -> ServerResult<Value> {
        match params {
            Value::Object(map) => {
                let mut resolved = serde_json::Map::new();
                for (key, value) in map {
                    resolved.insert(key.clone(), Self::resolve_step_params(value, step_results)?);
                }
                Ok(Value::Object(resolved))
            }
            Value::Array(arr) => {
                let mut resolved = Vec::new();
                for item in arr {
                    resolved.push(Self::resolve_step_params(item, step_results)?);
                }
                Ok(Value::Array(resolved))
            }
            Value::String(s) => {
                // Check if this is a placeholder like "$steps.0.result.locations"
                if s.starts_with("$steps.") {
                    let parts: Vec<&str> = s.split('.').collect();
                    if parts.len() < 3 {
                        return Err(ServerError::Runtime {
                            message: format!(
                                "Invalid placeholder format '{}': expected $steps.INDEX.PATH",
                                s
                            ),
                        });
                    }

                    let step_index =
                        parts[1]
                            .parse::<usize>()
                            .map_err(|_| ServerError::Runtime {
                                message: format!(
                                    "Invalid step index in placeholder '{}': '{}' is not a number",
                                    s, parts[1]
                                ),
                            })?;

                    let step_result =
                        step_results
                            .get(&step_index)
                            .ok_or_else(|| ServerError::Runtime {
                                message: format!(
                                    "Failed to resolve placeholder '{}': step {} has not been executed yet",
                                    s, step_index
                                ),
                            })?;

                    // Navigate the path in the result
                    let mut current = step_result;
                    for (i, part) in parts[2..].iter().enumerate() {
                        current = current.get(part).ok_or_else(|| ServerError::Runtime {
                            message: format!(
                                "Failed to resolve placeholder '{}': field '{}' not found in {}",
                                s,
                                part,
                                if i == 0 {
                                    format!("step {} result", step_index)
                                } else {
                                    format!("path '{}'", parts[2..i + 2].join("."))
                                }
                            ),
                        })?;
                    }
                    Ok(current.clone())
                } else {
                    Ok(Value::String(s.clone()))
                }
            }
            other => Ok(other.clone()),
        }
    }
}

#[async_trait::async_trait]
impl WorkflowExecutor for DefaultWorkflowExecutor {
    async fn execute_workflow(&self, workflow: &Workflow, dry_run: bool) -> ServerResult<Value> {
        info!(
            workflow_name = %workflow.name,
            steps_count = workflow.steps.len(),
            dry_run = dry_run,
            "Starting workflow execution"
        );

        let mut step_results: HashMap<usize, Value> = HashMap::new();
        let mut final_result = json!({});
        let mut log: Vec<String> = Vec::new();

        if dry_run {
            log.push(format!(
                "[DRY RUN MODE] Executing workflow '{}' without modifying files",
                workflow.name
            ));
        }

        for (step_index, step) in workflow.steps.iter().enumerate() {
            // Check if this step requires user confirmation
            if step.requires_confirmation == Some(true) {
                info!(
                    step_index = step_index,
                    workflow_name = %workflow.name,
                    "Step requires confirmation - pausing workflow"
                );

                // Generate a unique workflow ID
                let workflow_id = Uuid::new_v4().to_string();

                // Store the paused workflow state
                let paused_state = PausedWorkflowState {
                    workflow: workflow.clone(),
                    step_index,
                    step_results: step_results.clone(),
                    log: log.clone(),
                    dry_run,
                };

                self.paused_workflows
                    .insert(workflow_id.clone(), paused_state);

                log.push(format!(
                    "[Step {}/{}] PAUSED: {} - {}. Awaiting user confirmation.",
                    step_index + 1,
                    workflow.steps.len(),
                    step.tool,
                    step.description
                ));

                return Ok(json!({
                    "status": "awaiting_confirmation",
                    "workflow_id": workflow_id,
                    "workflow": workflow.name,
                    "step_index": step_index,
                    "step_description": step.description,
                    "log": log
                }));
            }

            debug!(
                step_index = step_index,
                tool = %step.tool,
                description = %step.description,
                "Executing workflow step"
            );

            // Resolve parameters using generic placeholder substitution
            let mut resolved_params = Self::resolve_step_params(&step.params, &step_results)?;

            // If dry_run is enabled, add it to the parameters for all tools
            if dry_run {
                if let Value::Object(ref mut map) = resolved_params {
                    map.insert("dry_run".to_string(), Value::Bool(true));
                }
            }

            debug!(params = ?resolved_params, dry_run = dry_run, "Resolved step parameters");

            // Create plugin request
            let file_path = resolved_params
                .get("file_path")
                .and_then(|v| v.as_str())
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("."));

            let plugin_request = PluginRequest {
                method: step.tool.clone(),
                file_path,
                position: None,
                range: None,
                params: resolved_params,
                request_id: None,
            };

            // Execute the step
            match self.plugin_manager.handle_request(plugin_request).await {
                Ok(response) => {
                    let step_result = response.data.unwrap_or(json!({}));
                    debug!(
                        step_index = step_index,
                        result = ?step_result,
                        "Step completed successfully"
                    );

                    // Log successful step completion
                    log.push(format!(
                        "[Step {}/{}] SUCCESS: {} - {}",
                        step_index + 1,
                        workflow.steps.len(),
                        step.tool,
                        step.description
                    ));

                    step_results.insert(step_index, step_result.clone());
                    final_result = step_result;
                }
                Err(e) => {
                    error!(
                        step_index = step_index,
                        step_description = %step.description,
                        tool = %step.tool,
                        workflow = %workflow.name,
                        error = %e,
                        "Step execution failed - halting workflow"
                    );

                    // Log the failure
                    log.push(format!(
                        "[Step {}/{}] FAILED: {} - {}. Error: {}",
                        step_index + 1,
                        workflow.steps.len(),
                        step.tool,
                        step.description,
                        e
                    ));

                    return Err(ServerError::Runtime {
                        message: format!(
                            "Workflow '{}' failed at step {}/{} ({}): {}. Error: {}",
                            workflow.name,
                            step_index + 1,
                            workflow.steps.len(),
                            step.tool,
                            step.description,
                            e
                        ),
                    });
                }
            }
        }

        info!(
            workflow_name = %workflow.name,
            dry_run = dry_run,
            "Workflow execution completed successfully"
        );

        log.push(format!(
            "[COMPLETE] Workflow '{}' finished successfully ({} steps executed)",
            workflow.name,
            workflow.steps.len()
        ));

        Ok(json!({
            "success": true,
            "workflow": workflow.name,
            "steps_executed": workflow.steps.len(),
            "dry_run": dry_run,
            "log": log,
            "result": final_result
        }))
    }

    async fn resume_workflow(
        &self,
        workflow_id: &str,
        _resume_data: Option<Value>,
    ) -> ServerResult<Value> {
        info!(workflow_id = %workflow_id, "Resuming paused workflow");

        // Retrieve the paused workflow state
        let paused_state = self
            .paused_workflows
            .remove(workflow_id)
            .ok_or_else(|| ServerError::Runtime {
                message: format!("Workflow '{}' not found or already completed", workflow_id),
            })?
            .1;

        let workflow = paused_state.workflow;
        let mut step_results = paused_state.step_results;
        let mut log = paused_state.log;
        let dry_run = paused_state.dry_run;
        let mut final_result = json!({});

        log.push(format!(
            "[RESUMED] Continuing workflow '{}' from step {}",
            workflow.name,
            paused_state.step_index + 1
        ));

        // Continue execution from the paused step
        for (step_index, step) in workflow
            .steps
            .iter()
            .enumerate()
            .skip(paused_state.step_index)
        {
            debug!(
                step_index = step_index,
                tool = %step.tool,
                description = %step.description,
                "Executing workflow step after resume"
            );

            // Resolve parameters using generic placeholder substitution
            let mut resolved_params = Self::resolve_step_params(&step.params, &step_results)?;

            // If dry_run is enabled, add it to the parameters for all tools
            if dry_run {
                if let Value::Object(ref mut map) = resolved_params {
                    map.insert("dry_run".to_string(), Value::Bool(true));
                }
            }

            debug!(params = ?resolved_params, dry_run = dry_run, "Resolved step parameters");

            // Create plugin request
            let file_path = resolved_params
                .get("file_path")
                .and_then(|v| v.as_str())
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("."));

            let plugin_request = PluginRequest {
                method: step.tool.clone(),
                file_path,
                position: None,
                range: None,
                params: resolved_params,
                request_id: None,
            };

            // Execute the step
            match self.plugin_manager.handle_request(plugin_request).await {
                Ok(response) => {
                    let step_result = response.data.unwrap_or(json!({}));
                    debug!(
                        step_index = step_index,
                        result = ?step_result,
                        "Step completed successfully"
                    );

                    // Log successful step completion
                    log.push(format!(
                        "[Step {}/{}] SUCCESS: {} - {}",
                        step_index + 1,
                        workflow.steps.len(),
                        step.tool,
                        step.description
                    ));

                    step_results.insert(step_index, step_result.clone());
                    final_result = step_result;
                }
                Err(e) => {
                    error!(
                        step_index = step_index,
                        step_description = %step.description,
                        tool = %step.tool,
                        workflow = %workflow.name,
                        error = %e,
                        "Step execution failed - halting workflow"
                    );

                    // Log the failure
                    log.push(format!(
                        "[Step {}/{}] FAILED: {} - {}. Error: {}",
                        step_index + 1,
                        workflow.steps.len(),
                        step.tool,
                        step.description,
                        e
                    ));

                    return Err(ServerError::Runtime {
                        message: format!(
                            "Workflow '{}' failed at step {}/{} ({}): {}. Error: {}",
                            workflow.name,
                            step_index + 1,
                            workflow.steps.len(),
                            step.tool,
                            step.description,
                            e
                        ),
                    });
                }
            }
        }

        info!(
            workflow_name = %workflow.name,
            dry_run = dry_run,
            "Resumed workflow execution completed successfully"
        );

        log.push(format!(
            "[COMPLETE] Workflow '{}' finished successfully ({} steps executed)",
            workflow.name,
            workflow.steps.len()
        ));

        Ok(json!({
            "success": true,
            "workflow": workflow.name,
            "steps_executed": workflow.steps.len(),
            "dry_run": dry_run,
            "log": log,
            "result": final_result
        }))
    }

    fn get_paused_workflow_count(&self) -> usize {
        self.paused_workflows.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test the placeholder resolution logic
    #[test]
    fn test_resolve_simple_placeholder() {
        let plugin_manager = Arc::new(PluginManager::new());
        let executor = DefaultWorkflowExecutor::new(plugin_manager);

        let mut step_results = HashMap::new();
        step_results.insert(
            0,
            json!({
                "locations": [
                    { "file": "test.ts", "line": 10 }
                ]
            }),
        );

        let params = json!({
            "file_path": "test.ts",
            "locations": "$steps.0.locations"
        });

        let resolved = DefaultWorkflowExecutor::resolve_step_params(&params, &step_results);
        assert!(resolved.is_ok());

        let result = resolved.unwrap();
        assert_eq!(
            result.get("file_path").unwrap().as_str().unwrap(),
            "test.ts"
        );
        assert!(result.get("locations").unwrap().is_array());
        let locations = result.get("locations").unwrap().as_array().unwrap();
        assert_eq!(locations.len(), 1);
    }

    #[test]
    fn test_resolve_nested_placeholders() {
        let plugin_manager = Arc::new(PluginManager::new());
        let executor = DefaultWorkflowExecutor::new(plugin_manager);

        let mut step_results = HashMap::new();
        step_results.insert(
            0,
            json!({
                "data": {
                    "nested": {
                        "value": 42
                    }
                }
            }),
        );

        let params = json!({
            "number": "$steps.0.data.nested.value"
        });

        let resolved = DefaultWorkflowExecutor::resolve_step_params(&params, &step_results);
        assert!(resolved.is_ok());

        let result = resolved.unwrap();
        assert_eq!(result.get("number").unwrap().as_u64().unwrap(), 42);
    }

    #[test]
    fn test_resolve_multiple_placeholders() {
        let plugin_manager = Arc::new(PluginManager::new());
        let executor = DefaultWorkflowExecutor::new(plugin_manager);

        let mut step_results = HashMap::new();
        step_results.insert(
            0,
            json!({
                "range": {
                    "start": { "line": 5, "character": 10 }
                }
            }),
        );

        let params = json!({
            "line": "$steps.0.range.start.line",
            "character": "$steps.0.range.start.character"
        });

        let resolved = DefaultWorkflowExecutor::resolve_step_params(&params, &step_results);
        assert!(resolved.is_ok());

        let result = resolved.unwrap();
        assert_eq!(result.get("line").unwrap().as_u64().unwrap(), 5);
        assert_eq!(result.get("character").unwrap().as_u64().unwrap(), 10);
    }

    #[test]
    fn test_resolve_placeholder_not_executed() {
        let plugin_manager = Arc::new(PluginManager::new());
        let executor = DefaultWorkflowExecutor::new(plugin_manager);

        let step_results = HashMap::new();
        let params = json!({
            "value": "$steps.0.result"
        });

        let resolved = DefaultWorkflowExecutor::resolve_step_params(&params, &step_results);
        assert!(resolved.is_err());

        let error = resolved.unwrap_err();
        match error {
            ServerError::Runtime { message } => {
                assert!(message.contains("step 0 has not been executed"));
            }
            _ => panic!("Expected Runtime error"),
        }
    }

    #[test]
    fn test_resolve_placeholder_missing_field() {
        let plugin_manager = Arc::new(PluginManager::new());
        let executor = DefaultWorkflowExecutor::new(plugin_manager);

        let mut step_results = HashMap::new();
        step_results.insert(
            0,
            json!({
                "data": { "value": 10 }
            }),
        );

        let params = json!({
            "value": "$steps.0.data.nonexistent"
        });

        let resolved = DefaultWorkflowExecutor::resolve_step_params(&params, &step_results);
        assert!(resolved.is_err());

        let error = resolved.unwrap_err();
        match error {
            ServerError::Runtime { message } => {
                assert!(message.contains("field 'nonexistent' not found"));
            }
            _ => panic!("Expected Runtime error"),
        }
    }

    #[test]
    fn test_resolve_invalid_placeholder_format() {
        let plugin_manager = Arc::new(PluginManager::new());
        let executor = DefaultWorkflowExecutor::new(plugin_manager);

        let step_results = HashMap::new();
        let params = json!({
            "value": "$steps.invalid"
        });

        let resolved = DefaultWorkflowExecutor::resolve_step_params(&params, &step_results);
        assert!(resolved.is_err());

        let error = resolved.unwrap_err();
        match error {
            ServerError::Runtime { message } => {
                // The error should be about invalid format - either missing PATH or invalid index
                assert!(
                    message.contains("Invalid placeholder format")
                        || message.contains("Invalid step index")
                        || message.contains("is not a number")
                        || message.contains("expected $steps.INDEX.PATH")
                );
            }
            _ => panic!("Expected Runtime error"),
        }
    }

    #[test]
    fn test_resolve_placeholder_in_array() {
        let plugin_manager = Arc::new(PluginManager::new());
        let executor = DefaultWorkflowExecutor::new(plugin_manager);

        let mut step_results = HashMap::new();
        step_results.insert(
            0,
            json!({
                "file": "test.ts"
            }),
        );

        let params = json!({
            "files": ["$steps.0.file", "other.ts"]
        });

        let resolved = DefaultWorkflowExecutor::resolve_step_params(&params, &step_results);
        assert!(resolved.is_ok());

        let result = resolved.unwrap();
        let files = result.get("files").unwrap().as_array().unwrap();
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].as_str().unwrap(), "test.ts");
        assert_eq!(files[1].as_str().unwrap(), "other.ts");
    }
}
