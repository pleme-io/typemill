//! Advanced operations tool handlers
//!
//! Handles: apply_edits, achieve_intent, batch_execute

use super::{ToolHandler, ToolHandlerContext};
use crate::handlers::tool_handler::{ToolContext, ToolHandler as LegacyToolHandler};
use crate::handlers::workflow_handler::WorkflowHandler as LegacyWorkflowHandler;
use crate::ServerResult;
use async_trait::async_trait;
use cb_core::model::mcp::ToolCall;
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
    fn supported_tools(&self) -> &[&'static str] {
        &["apply_edits", "achieve_intent", "batch_execute"]
    }

    async fn handle(
        &self,
        tool_name: &str,
        params: Value,
        context: &ToolHandlerContext,
    ) -> ServerResult<Value> {
        match tool_name {
            "apply_edits" | "achieve_intent" => {
                // Convert to ToolCall for legacy handler
                let tool_call = ToolCall {
                    name: tool_name.to_string(),
                    arguments: Some(params),
                };

                // Convert new context to legacy context
                let legacy_context = ToolContext {
                    app_state: context.app_state.clone(),
                    plugin_manager: context.plugin_manager.clone(),
                    lsp_adapter: context.lsp_adapter.clone(),
                };

                self.workflow_handler
                    .handle_tool(tool_call, &legacy_context)
                    .await
            }
            "batch_execute" => {
                use serde::Deserialize;
                use crate::services::OperationType;
                use uuid::Uuid;
                use std::path::PathBuf;
                use serde_json::json;

                // Define the structure for individual operations within the batch
                #[derive(Deserialize, Debug)]
                #[serde(tag = "type", rename_all = "snake_case")]
                enum BatchOperation {
                    CreateFile { path: String, content: Option<String> },
                    DeleteFile { path: String },
                    WriteFile { path: String, content: String },
                    RenameFile { old_path: String, new_path: String },
                }

                // Define the structure for the overall batch_execute parameters
                #[derive(Deserialize, Debug)]
                struct BatchExecuteParams {
                    operations: Vec<BatchOperation>,
                }

                // 1. Deserialize the incoming parameters
                let batch_params: BatchExecuteParams = serde_json::from_value(params)
                    .map_err(|e| crate::ServerError::runtime(format!("Failed to parse batch_execute params: {}", e)))?;

                // 2. Get the operation queue from the context
                let operation_queue = &context.app_state.operation_queue;

                let batch_id = Uuid::new_v4().to_string(); // A single ID for the entire batch
                let mut queued_count = 0;

                // 3. Iterate, convert to internal FileOperation, and enqueue
                for operation in batch_params.operations.into_iter() {
                    let (operation_type, file_path, operation_params) = match operation {
                        BatchOperation::CreateFile { path, content } => {
                            let params = json!({
                                "file_path": path,
                                "content": content.unwrap_or_default()
                            });
                            (OperationType::Write, PathBuf::from(path), params)
                        },
                        BatchOperation::DeleteFile { path } => {
                            let params = json!({
                                "file_path": path
                            });
                            (OperationType::Delete, PathBuf::from(path), params)
                        },
                        BatchOperation::WriteFile { path, content } => {
                            let params = json!({
                                "file_path": path,
                                "content": content
                            });
                            (OperationType::Write, PathBuf::from(path), params)
                        },
                        BatchOperation::RenameFile { old_path, new_path } => {
                            let params = json!({
                                "old_path": old_path,
                                "new_path": new_path.clone()
                            });
                            (OperationType::Rename, PathBuf::from(old_path), params)
                        },
                    };

                    let file_op = crate::services::FileOperation::new(
                        "batch_execute".to_string(),
                        operation_type,
                        file_path,
                        operation_params,
                    );

                    operation_queue.enqueue(file_op).await.map_err(|e| {
                        crate::ServerError::runtime(format!("Failed to enqueue batch operation: {}", e))
                    })?;

                    queued_count += 1;
                }

                // 4. Return a success response
                let response = json!({
                    "status": "success",
                    "message": format!("Queued {} operations for execution.", queued_count),
                    "batch_id": batch_id
                });
                Ok(response)
            }
            _ => Err(crate::ServerError::InvalidRequest(format!(
                "Unknown advanced tool: {}",
                tool_name
            ))),
        }
    }
}
