//! Advanced operations tool handlers
//!
//! Handles: apply_edits, achieve_intent, batch_execute

use super::{ToolHandler, ToolHandlerContext};
use crate::handlers::workflow_handler::WorkflowHandler as LegacyWorkflowHandler;
use async_trait::async_trait;
use cb_core::model::mcp::ToolCall;
use cb_protocol::ApiResult as ServerResult;
use serde_json::Value;

pub struct AdvancedHandler {
    workflow_handler: LegacyWorkflowHandler,
}

impl AdvancedHandler {
    pub fn new() -> Self {
        Self {
            workflow_handler: LegacyWorkflowHandler::new(),
        }
    }
}

#[async_trait]
impl ToolHandler for AdvancedHandler {
    fn tool_names(&self) -> &[&str] {
        &["execute_edits", "execute_batch"]
    }

    async fn handle_tool_call(
        &self,
        context: &ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        let tool_name = &tool_call.name;
        let params = tool_call
            .arguments
            .clone()
            .unwrap_or_else(|| serde_json::json!({}));

        match tool_name.as_str() {
            "execute_edits" => {
                // The legacy tool was `apply_edits`, so we need to clone and modify the call
                let mut legacy_tool_call = tool_call.clone();
                legacy_tool_call.name = "apply_edits".to_string();

                // WorkflowHandler now uses the new trait, so delegate directly
                self.workflow_handler
                    .handle_tool_call(context, &legacy_tool_call)
                    .await
            }
            "execute_batch" => {
                use cb_services::services::OperationType;
                use serde::Deserialize;
                use serde_json::json;
                use std::path::PathBuf;
                use uuid::Uuid;

                // Define the structure for individual operations within the batch
                #[derive(Deserialize, Debug)]
                #[serde(tag = "type", rename_all = "snake_case")]
                #[allow(clippy::enum_variant_names)]
                enum BatchOperation {
                    CreateFile {
                        path: String,
                        content: Option<String>,
                        dry_run: Option<bool>,
                    },
                    DeleteFile {
                        path: String,
                        dry_run: Option<bool>,
                    },
                    WriteFile {
                        path: String,
                        content: String,
                        dry_run: Option<bool>,
                    },
                    RenameFile {
                        old_path: String,
                        new_path: String,
                        dry_run: Option<bool>,
                    },
                    UpdateDependency {
                        manifest_path: Option<String>,
                        dependency_name: String,
                        version: String,
                        dry_run: Option<bool>,
                    },
                }

                // Define the structure for the overall execute_batch parameters
                #[derive(Deserialize, Debug)]
                struct ExecuteBatchParams {
                    operations: Vec<BatchOperation>,
                }

                // 1. Deserialize the incoming parameters
                let batch_params: ExecuteBatchParams =
                    serde_json::from_value(params).map_err(|e| {
                        cb_protocol::ApiError::runtime(format!(
                            "Failed to parse execute_batch params: {}",
                            e
                        ))
                    })?;

                // 2. Get the operation queue from the context
                let operation_queue = &context.app_state.operation_queue;

                let batch_id = Uuid::new_v4().to_string(); // A single ID for the entire batch
                let mut queued_count = 0;
                let mut results = Vec::new();

                // 3. Iterate, convert to internal FileOperation, and enqueue or execute with dry_run
                for operation in batch_params.operations.into_iter() {
                    // Check if this operation is a dry run
                    let is_dry_run = match &operation {
                        BatchOperation::CreateFile { dry_run, .. }
                        | BatchOperation::DeleteFile { dry_run, .. }
                        | BatchOperation::WriteFile { dry_run, .. }
                        | BatchOperation::RenameFile { dry_run, .. }
                        | BatchOperation::UpdateDependency { dry_run, .. } => {
                            dry_run.unwrap_or(false)
                        }
                    };

                    // If dry_run, execute directly and collect results
                    if is_dry_run {
                        let result = match operation {
                            BatchOperation::CreateFile { path, content, .. } => {
                                let file_service = &context.app_state.file_service;
                                file_service
                                    .write_file(
                                        &PathBuf::from(&path),
                                        &content.unwrap_or_default(),
                                        true, // dry_run
                                    )
                                    .await
                                    .map(|dry_result| dry_result.result)
                                    .map_err(|e| {
                                        cb_protocol::ApiError::runtime(format!(
                                            "Dry run failed for create_file {}: {}",
                                            path, e
                                        ))
                                    })?
                            }
                            BatchOperation::WriteFile { path, content, .. } => {
                                let file_service = &context.app_state.file_service;
                                file_service
                                    .write_file(
                                        &PathBuf::from(&path),
                                        &content,
                                        true, // dry_run
                                    )
                                    .await
                                    .map(|dry_result| dry_result.result)
                                    .map_err(|e| {
                                        cb_protocol::ApiError::runtime(format!(
                                            "Dry run failed for write_file {}: {}",
                                            path, e
                                        ))
                                    })?
                            }
                            BatchOperation::DeleteFile { path, .. } => {
                                let file_service = &context.app_state.file_service;
                                file_service
                                    .delete_file(&PathBuf::from(&path), false, true)
                                    .await
                                    .map(|dry_result| dry_result.result)
                                    .map_err(|e| {
                                        cb_protocol::ApiError::runtime(format!(
                                            "Dry run failed for delete_file {}: {}",
                                            path, e
                                        ))
                                    })?
                            }
                            BatchOperation::RenameFile {
                                old_path, new_path, ..
                            } => {
                                let file_service = &context.app_state.file_service;
                                file_service
                                    .rename_file_with_imports(
                                        &PathBuf::from(&old_path),
                                        &PathBuf::from(&new_path),
                                        true,
                                        None,
                                    )
                                    .await
                                    .map(|dry_result| dry_result.result)
                                    .map_err(|e| {
                                        cb_protocol::ApiError::runtime(format!(
                                            "Dry run failed for rename_file {} -> {}: {}",
                                            old_path, new_path, e
                                        ))
                                    })?
                            }
                            BatchOperation::UpdateDependency {
                                manifest_path,
                                dependency_name,
                                version,
                                ..
                            } => {
                                use cb_plugins::protocol::PluginRequest;
                                let plugin_manager = &context.plugin_manager;
                                let params = json!({
                                    "manifest_path": manifest_path.clone(),
                                    "dependency_name": dependency_name,
                                    "version": version,
                                    "dry_run": true,
                                });
                                let file_path = PathBuf::from(
                                    manifest_path.unwrap_or_else(|| ".".to_string()),
                                );
                                let request = PluginRequest::new("update_dependency", file_path)
                                    .with_params(params);
                                plugin_manager
                                    .handle_request(request)
                                    .await
                                    .map(|response| response.data.unwrap_or_default())
                                    .map_err(|e| cb_protocol::ApiError::Plugin(e.to_string()))?
                            }
                        };
                        results.push(result);
                        continue;
                    }

                    let (operation_type, file_path, operation_params) = match operation {
                        BatchOperation::CreateFile {
                            path,
                            content,
                            dry_run,
                        } => {
                            let params = json!({
                                "file_path": path,
                                "content": content.unwrap_or_default(),
                                "dry_run": dry_run.unwrap_or(false)
                            });
                            (OperationType::Write, PathBuf::from(path), params)
                        }
                        BatchOperation::DeleteFile { path, dry_run } => {
                            let params = json!({
                                "file_path": path,
                                "dry_run": dry_run.unwrap_or(false)
                            });
                            (OperationType::Delete, PathBuf::from(path), params)
                        }
                        BatchOperation::WriteFile {
                            path,
                            content,
                            dry_run,
                        } => {
                            let params = json!({
                                "file_path": path,
                                "content": content,
                                "dry_run": dry_run.unwrap_or(false)
                            });
                            (OperationType::Write, PathBuf::from(path), params)
                        }
                        BatchOperation::RenameFile {
                            old_path,
                            new_path,
                            dry_run,
                        } => {
                            let params = json!({
                                "old_path": old_path,
                                "new_path": new_path.clone(),
                                "dry_run": dry_run.unwrap_or(false)
                            });
                            (OperationType::Rename, PathBuf::from(old_path), params)
                        }
                        BatchOperation::UpdateDependency {
                            manifest_path,
                            dependency_name,
                            version,
                            dry_run,
                        } => {
                            let params = json!({
                                "manifest_path": manifest_path.clone(),
                                "dependency_name": dependency_name,
                                "version": version,
                                "dry_run": dry_run.unwrap_or(false)
                            });
                            let file_path =
                                PathBuf::from(manifest_path.unwrap_or_else(|| ".".to_string()));
                            (OperationType::UpdateDependency, file_path, params)
                        }
                    };

                    let file_op = cb_services::services::FileOperation::new(
                        "execute_batch".to_string(),
                        operation_type,
                        file_path,
                        operation_params,
                    );

                    operation_queue.enqueue(file_op).await.map_err(|e| {
                        cb_protocol::ApiError::runtime(format!(
                            "Failed to enqueue batch operation: {}",
                            e
                        ))
                    })?;

                    queued_count += 1;
                }

                // 4. Return a success response
                let response = if !results.is_empty() {
                    // Dry run results
                    json!({
                        "status": "preview",
                        "message": format!("Dry run completed for {} operations.", results.len()),
                        "results": results,
                        "batch_id": batch_id
                    })
                } else {
                    // Normal queued operations
                    json!({
                        "status": "success",
                        "message": format!("Queued {} operations for execution.", queued_count),
                        "batch_id": batch_id
                    })
                };
                Ok(response)
            }
            _ => Err(cb_protocol::ApiError::InvalidRequest(format!(
                "Unknown advanced tool: {}",
                tool_name
            ))),
        }
    }
}
