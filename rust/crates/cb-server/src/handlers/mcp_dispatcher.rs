//! MCP message dispatcher

use crate::error::{ServerError, ServerResult};
use cb_core::model::mcp::{McpMessage, McpRequest, McpResponse, ToolCall};
use crate::services::{LockManager, OperationQueue, FileOperation, OperationType};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::path::PathBuf;

/// Application state containing services
#[derive(Clone)]
pub struct AppState {
    /// LSP service for code intelligence
    pub lsp: Arc<dyn crate::interfaces::LspService>,
    /// File service for file operations with import awareness
    pub file_service: Arc<crate::services::FileService>,
    /// Project root directory
    pub project_root: std::path::PathBuf,
    /// Lock manager for file-level locking
    pub lock_manager: Arc<LockManager>,
    /// Operation queue for serializing file operations
    pub operation_queue: Arc<OperationQueue>,
}

/// Tool handler function type that receives app state
pub type ToolHandler = Box<
    dyn Fn(Arc<AppState>, Value) -> Pin<Box<dyn Future<Output = ServerResult<Value>> + Send>> + Send + Sync
>;

/// MCP message dispatcher
pub struct McpDispatcher {
    tools: HashMap<String, ToolHandler>,
    app_state: Arc<AppState>,
    /// Map of tool names to their operation types
    tool_operations: HashMap<String, OperationType>,
}

impl McpDispatcher {
    /// Create a new dispatcher with app state
    pub fn new(app_state: Arc<AppState>) -> Self {
        let mut dispatcher = Self {
            tools: HashMap::new(),
            app_state,
            tool_operations: HashMap::new(),
        };
        dispatcher.initialize_tool_operations();
        dispatcher
    }

    /// Initialize the mapping of tools to operation types
    fn initialize_tool_operations(&mut self) {
        // Read operations
        self.tool_operations.insert("find_definition".to_string(), OperationType::Read);
        self.tool_operations.insert("find_references".to_string(), OperationType::Read);
        self.tool_operations.insert("find_type_definition".to_string(), OperationType::Read);
        self.tool_operations.insert("find_implementations".to_string(), OperationType::Read);
        self.tool_operations.insert("get_symbols".to_string(), OperationType::Read);
        self.tool_operations.insert("get_hover".to_string(), OperationType::Read);
        self.tool_operations.insert("get_completions".to_string(), OperationType::Read);
        self.tool_operations.insert("get_signature_help".to_string(), OperationType::Read);
        self.tool_operations.insert("get_diagnostics".to_string(), OperationType::Read);
        self.tool_operations.insert("get_inlay_hints".to_string(), OperationType::Read);
        self.tool_operations.insert("find_unused_imports".to_string(), OperationType::Read);
        self.tool_operations.insert("get_test_locations".to_string(), OperationType::Read);
        self.tool_operations.insert("get_all_symbols".to_string(), OperationType::Read);
        self.tool_operations.insert("get_outline".to_string(), OperationType::Read);
        self.tool_operations.insert("find_workspace_symbols".to_string(), OperationType::Read);

        // Write operations
        self.tool_operations.insert("apply_code_action".to_string(), OperationType::Write);
        self.tool_operations.insert("rename_symbol".to_string(), OperationType::Refactor);
        self.tool_operations.insert("format_document".to_string(), OperationType::Format);
        self.tool_operations.insert("format_selection".to_string(), OperationType::Format);
        self.tool_operations.insert("organize_imports".to_string(), OperationType::Refactor);
        self.tool_operations.insert("add_missing_imports".to_string(), OperationType::Write);
        self.tool_operations.insert("remove_unused_imports".to_string(), OperationType::Write);
        self.tool_operations.insert("extract_function".to_string(), OperationType::Refactor);
        self.tool_operations.insert("extract_variable".to_string(), OperationType::Refactor);
        self.tool_operations.insert("inline_variable".to_string(), OperationType::Refactor);
        self.tool_operations.insert("fix_all".to_string(), OperationType::Write);
    }

    /// Register a tool handler
    pub fn register_tool<F, Fut>(&mut self, name: String, handler: F)
    where
        F: Fn(Arc<AppState>, Value) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ServerResult<Value>> + Send + 'static,
    {
        self.tools.insert(
            name,
            Box::new(move |app_state, args| Box::pin(handler(app_state, args))),
        );
    }

    /// Dispatch an MCP message
    pub async fn dispatch(&self, message: McpMessage) -> ServerResult<McpMessage> {
        match message {
            McpMessage::Request(request) => self.handle_request(request).await,
            McpMessage::Response(response) => Ok(McpMessage::Response(response)),
            McpMessage::Notification(notification) => {
                tracing::debug!("Received notification: {:?}", notification);
                Ok(McpMessage::Response(McpResponse {
                    id: None,
                    result: Some(json!({"status": "ok"})),
                    error: None,
                }))
            }
            _ => {
                // Handle any future variants
                Err(ServerError::Unsupported("Unknown message type".into()))
            }
        }
    }

    /// Handle an MCP request
    async fn handle_request(&self, request: McpRequest) -> ServerResult<McpMessage> {
        tracing::debug!("Handling request: {:?}", request.method);

        let response = match request.method.as_str() {
            "tools/list" => self.handle_list_tools(),
            "tools/call" => self.handle_tool_call(request.params).await?,
            _ => {
                return Err(ServerError::Unsupported(format!(
                    "Unknown method: {}",
                    request.method
                )))
            }
        };

        Ok(McpMessage::Response(McpResponse {
            id: request.id,
            result: Some(response),
            error: None,
        }))
    }

    /// Handle tools/list request
    fn handle_list_tools(&self) -> Value {
        let tools: Vec<Value> = self.tools.keys().map(|name| {
            json!({
                "name": name,
                "description": format!("{} tool", name),
                "parameters": {
                    "type": "object",
                    "properties": {}
                }
            })
        }).collect();

        json!({ "tools": tools })
    }

    /// Handle tools/call request
    async fn handle_tool_call(&self, params: Option<Value>) -> ServerResult<Value> {
        let params = params.ok_or_else(|| ServerError::InvalidRequest("Missing params".into()))?;

        let tool_call: ToolCall = serde_json::from_value(params)
            .map_err(|e| ServerError::InvalidRequest(format!("Invalid tool call: {}", e)))?;

        let handler = self.tools.get(&tool_call.name)
            .ok_or_else(|| ServerError::Unsupported(format!("Unknown tool: {}", tool_call.name)))?;

        // Determine if this operation needs to be queued
        let operation_type = self.tool_operations.get(&tool_call.name);

        let result = if let Some(op_type) = operation_type {
            if op_type.is_write_operation() {
                // Extract file path from arguments if available
                let file_path = self.extract_file_path(&tool_call.arguments);

                if let Some(path) = file_path {
                    // Queue the operation
                    let operation = FileOperation::new(
                        tool_call.name.clone(),
                        op_type.clone(),
                        path,
                        tool_call.arguments.clone().unwrap_or(json!({}))
                    );

                    let operation_id = self.app_state.operation_queue.enqueue(operation).await?;
                    tracing::debug!("Queued operation {} for tool {}", operation_id, tool_call.name);

                    // Process the operation immediately (it will wait for its turn in the queue)
                    self.process_queued_operation(&tool_call.name, handler, tool_call.arguments.unwrap_or(json!({}))).await?
                } else {
                    // No file path, execute directly
                    handler(self.app_state.clone(), tool_call.arguments.unwrap_or(json!({}))).await?
                }
            } else {
                // Read operation, can execute directly with read lock
                handler(self.app_state.clone(), tool_call.arguments.unwrap_or(json!({}))).await?
            }
        } else {
            // Unknown operation type, execute directly
            handler(self.app_state.clone(), tool_call.arguments.unwrap_or(json!({}))).await?
        };

        Ok(json!({
            "content": result
        }))
    }

    /// Extract file path from tool arguments
    fn extract_file_path(&self, args: &Option<Value>) -> Option<PathBuf> {
        args.as_ref()
            .and_then(|v| v.get("file_path"))
            .or_else(|| args.as_ref().and_then(|v| v.get("path")))
            .and_then(|v| v.as_str())
            .map(PathBuf::from)
    }

    /// Process a queued operation
    async fn process_queued_operation(
        &self,
        tool_name: &str,
        handler: &ToolHandler,
        args: Value,
    ) -> ServerResult<Value> {
        // The operation is already in the queue, just execute it
        // The queue system will handle locking
        handler(self.app_state.clone(), args).await
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::systems::LspManager;
    use cb_core::config::LspConfig;

    fn create_test_app_state() -> Arc<AppState> {
        let lsp_config = LspConfig::default();
        let lsp_manager = Arc::new(LspManager::new(lsp_config));
        let file_service = Arc::new(crate::services::FileService::new(std::path::PathBuf::from("/tmp")));
        let project_root = std::path::PathBuf::from("/tmp");
        let lock_manager = Arc::new(LockManager::new());
        let operation_queue = Arc::new(OperationQueue::new(lock_manager.clone()));

        Arc::new(AppState {
            lsp: lsp_manager,
            file_service,
            project_root,
            lock_manager,
            operation_queue,
        })
    }

    #[tokio::test]
    async fn test_dispatcher_list_tools() {
        let app_state = create_test_app_state();
        let mut dispatcher = McpDispatcher::new(app_state);

        // Register a test tool
        dispatcher.register_tool("test_tool".to_string(), |_app_state, _args| async move {
            Ok(json!({"result": "success"}))
        });

        let request = McpRequest {
            id: Some(json!(1)),
            method: "tools/list".to_string(),
            params: None,
        };

        let response = dispatcher.dispatch(McpMessage::Request(request)).await.unwrap();

        if let McpMessage::Response(resp) = response {
            assert!(resp.result.is_some());
            let result = resp.result.unwrap();
            assert!(result["tools"].is_array());
        } else {
            panic!("Expected Response message");
        }
    }

    #[tokio::test]
    async fn test_dispatcher_call_tool() {
        let app_state = create_test_app_state();
        let mut dispatcher = McpDispatcher::new(app_state);

        // Register a test tool that echoes its input
        dispatcher.register_tool("echo".to_string(), |_app_state, args| async move {
            Ok(json!({
                "echoed": args
            }))
        });

        let request = McpRequest {
            id: Some(json!(1)),
            method: "tools/call".to_string(),
            params: Some(json!({
                "name": "echo",
                "arguments": {
                    "message": "hello"
                }
            })),
        };

        let response = dispatcher.dispatch(McpMessage::Request(request)).await.unwrap();

        if let McpMessage::Response(resp) = response {
            assert!(resp.result.is_some());
            let result = resp.result.unwrap();
            assert!(result["content"]["echoed"]["message"] == "hello");
        } else {
            panic!("Expected Response message");
        }
    }
}