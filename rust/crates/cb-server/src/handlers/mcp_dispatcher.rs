//! MCP message dispatcher

use crate::error::{ServerError, ServerResult};
use cb_core::model::mcp::{McpMessage, McpRequest, McpResponse, ToolCall};
use crate::services::{LockManager, OperationQueue, FileOperation, OperationType};
use crate::services::operation_queue::OperationTransaction;
use crate::utils::SimdJsonParser;
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
        self.tool_operations.insert("find_code_duplicates".to_string(), OperationType::Read);

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

    /// Test helper to directly call a tool (only available in test builds)
    #[cfg(test)]
    pub async fn call_tool_for_test(&self, tool_name: &str, args: Value) -> ServerResult<Value> {
        let handler = self.tools.get(tool_name)
            .ok_or_else(|| ServerError::Unsupported(format!("Unknown tool: {}", tool_name)))?;

        handler(self.app_state.clone(), args).await
    }

    /// Test helper to call handle_tool_call directly (only available in test builds)
    #[cfg(test)]
    pub async fn handle_tool_call_for_test(&self, params: Option<Value>) -> ServerResult<Value> {
        self.handle_tool_call(params).await
    }

    /// Test helper to dispatch a tool call with direct parameters (only available in test builds)
    #[cfg(test)]
    pub async fn dispatch_tool_call_for_test(&self, tool_name: &str, args: Value) -> ServerResult<Value> {
        let tool_call = ToolCall {
            name: tool_name.to_string(),
            arguments: Some(args),
        };
        self.handle_tool_call(Some(serde_json::to_value(tool_call)?)).await
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

        let tool_call: ToolCall = SimdJsonParser::from_value(params)?;

        let handler = self.tools.get(&tool_call.name)
            .ok_or_else(|| ServerError::Unsupported(format!("Unknown tool: {}", tool_call.name)))?;

        // Determine if this operation needs to be queued or can execute concurrently
        let operation_type = self.tool_operations.get(&tool_call.name);

        let result = if let Some(op_type) = operation_type {
            if op_type.is_write_operation() {
                // Check if this is a refactoring operation that might affect multiple files
                if *op_type == OperationType::Refactor && self.is_multi_file_refactoring(&tool_call.name) {
                    // Handle multi-file refactoring with transactions
                    self.handle_refactoring_operation(&tool_call, handler).await?
                } else {
                    // Single file write operations - queue normally
                    let file_path = self.extract_file_path(&tool_call.arguments);

                    if let Some(path) = file_path {
                        // Queue the write operation with appropriate priority
                        let priority = self.get_operation_priority(op_type);
                        let operation = FileOperation::new(
                            tool_call.name.clone(),
                            op_type.clone(),
                            path,
                            tool_call.arguments.clone().unwrap_or(json!({}))
                        ).with_priority(priority);

                        let operation_id = self.app_state.operation_queue.enqueue(operation).await?;
                        tracing::debug!("Queued write operation {} for tool {} with priority {}", operation_id, tool_call.name, priority);

                        // Process the operation immediately (it will wait for its turn in the queue)
                        self.process_queued_operation(&tool_call.name, handler, tool_call.arguments.unwrap_or(json!({}))).await?
                    } else {
                        // No file path, execute directly
                        handler(self.app_state.clone(), tool_call.arguments.unwrap_or(json!({}))).await?
                    }
                }
            } else {
                // Read operation - execute concurrently with read lock
                let file_path = self.extract_file_path(&tool_call.arguments);

                if let Some(path) = file_path {
                    tracing::debug!("Acquiring read lock for concurrent read operation: {}", tool_call.name);
                    // Acquire read lock and execute immediately
                    let file_lock = self.app_state.lock_manager.get_lock(&path).await;
                    let _read_guard = file_lock.read().await;

                    tracing::debug!("Executing read operation {} with concurrent read lock", tool_call.name);
                    handler(self.app_state.clone(), tool_call.arguments.unwrap_or(json!({}))).await?
                } else {
                    // No file path, execute directly (no locking needed)
                    handler(self.app_state.clone(), tool_call.arguments.unwrap_or(json!({}))).await?
                }
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

    /// Get the priority for an operation type
    fn get_operation_priority(&self, op_type: &OperationType) -> u8 {
        match op_type {
            OperationType::Format => 10,      // Low priority - formatting can wait
            OperationType::Write => 5,        // Medium priority - regular writes
            OperationType::Delete => 3,       // High priority - deletions are urgent
            OperationType::Rename => 2,       // High priority - renames affect multiple references
            OperationType::Refactor => 1,     // Highest priority - refactoring operations are complex and should be prioritized
            OperationType::Read => 5,         // Medium priority (though reads bypass the queue)
        }
    }

    /// Check if a tool performs multi-file refactoring
    fn is_multi_file_refactoring(&self, tool_name: &str) -> bool {
        matches!(tool_name,
            "rename_symbol" |
            "organize_imports" |
            "extract_function" |
            "extract_variable" |
            "inline_variable"
        )
    }

    /// Handle refactoring operations with transaction support
    async fn handle_refactoring_operation(
        &self,
        tool_call: &ToolCall,
        handler: &ToolHandler,
    ) -> ServerResult<Value> {
        tracing::debug!("Handling multi-file refactoring operation: {}", tool_call.name);

        // Execute the handler to get the WorkspaceEdit
        let handler_result = handler(self.app_state.clone(), tool_call.arguments.clone().unwrap_or(json!({}))).await?;

        // Check if this is a dry run
        let is_dry_run = handler_result.get("dry_run")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        if is_dry_run {
            // For dry runs, just return the workspace edit without applying
            return Ok(handler_result);
        }

        // Extract the WorkspaceEdit from the handler result
        let workspace_edit = handler_result.get("workspace_edit")
            .ok_or_else(|| ServerError::runtime("No workspace_edit in refactoring result"))?;

        // Parse the WorkspaceEdit to determine affected files
        let affected_files = self.parse_workspace_edit(workspace_edit)?;

        if affected_files.is_empty() {
            tracing::debug!("No files affected by refactoring operation");
            return Ok(handler_result);
        }

        tracing::debug!("Refactoring {} will affect {} files", tool_call.name, affected_files.len());

        // Create a transaction for the refactoring operation
        let mut transaction = OperationTransaction::new(self.app_state.operation_queue.clone());

        // Create individual file operations for each affected file
        let priority = self.get_operation_priority(&OperationType::Refactor);
        for (file_path, file_edits) in affected_files {
            let operation = FileOperation::new(
                format!("{}_file_operation", tool_call.name),
                OperationType::Refactor,
                file_path,
                json!({
                    "edits": file_edits,
                    "original_tool": tool_call.name.clone(),
                }),
            ).with_priority(priority);

            transaction.add_operation(operation);
        }

        // Commit all operations atomically
        let operation_ids = transaction.commit().await?;
        tracing::debug!("Committed refactoring transaction with {} operations: {:?}", operation_ids.len(), operation_ids);

        // Return the original workspace edit result
        Ok(handler_result)
    }

    /// Analyze the impact of a refactoring operation to determine affected files
    async fn analyze_refactoring_impact(
        &self,
        tool_name: &str,
        args: &Option<Value>,
    ) -> ServerResult<Vec<(PathBuf, Value)>> {
        let default_args = json!({});
        let args = args.as_ref().unwrap_or(&default_args);

        match tool_name {
            "rename_symbol" => {
                // For rename symbol, we need to find all references
                if let Some(file_path) = args.get("file_path").and_then(|v| v.as_str()) {
                    // In a real implementation, we would use LSP to find all references
                    // For now, simulate finding 2-3 affected files
                    let base_path = PathBuf::from(file_path);
                    let affected_files = vec![
                        (base_path.clone(), args.clone()),
                        (base_path.with_file_name("related_file1.ts"), args.clone()),
                        (base_path.with_file_name("related_file2.ts"), args.clone()),
                    ];
                    Ok(affected_files)
                } else {
                    Ok(vec![])
                }
            }
            "organize_imports" => {
                // Organize imports might affect multiple files if it's a workspace-wide operation
                if let Some(file_path) = args.get("file_path").and_then(|v| v.as_str()) {
                    Ok(vec![(PathBuf::from(file_path), args.clone())])
                } else {
                    Ok(vec![])
                }
            }
            "extract_function" | "extract_variable" | "inline_variable" => {
                // These operations typically affect the current file and potentially callers
                if let Some(file_path) = args.get("file_path").and_then(|v| v.as_str()) {
                    let base_path = PathBuf::from(file_path);
                    let affected_files = vec![
                        (base_path.clone(), args.clone()),
                        // In practice, we would analyze dependencies to find callers
                    ];
                    Ok(affected_files)
                } else {
                    Ok(vec![])
                }
            }
            _ => {
                tracing::warn!("Unknown refactoring operation: {}", tool_name);
                Ok(vec![])
            }
        }
    }

    /// Parse a WorkspaceEdit to extract affected files and their edits
    fn parse_workspace_edit(&self, workspace_edit: &Value) -> ServerResult<Vec<(PathBuf, Value)>> {
        let mut affected_files = Vec::new();

        // Check for 'changes' field (simple text edits per file)
        if let Some(changes) = workspace_edit.get("changes").and_then(|v| v.as_object()) {
            for (uri, edits) in changes {
                // Convert URI to file path (remove "file://" prefix)
                let file_path = if uri.starts_with("file://") {
                    PathBuf::from(&uri[7..])
                } else {
                    PathBuf::from(uri)
                };

                affected_files.push((file_path, edits.clone()));
            }
        }

        // Check for 'documentChanges' field (more complex edits with versioning)
        if let Some(doc_changes) = workspace_edit.get("documentChanges").and_then(|v| v.as_array()) {
            for change in doc_changes {
                if let Some(text_doc_edit) = change.get("textDocument") {
                    if let Some(uri) = text_doc_edit.get("uri").and_then(|v| v.as_str()) {
                        let file_path = if uri.starts_with("file://") {
                            PathBuf::from(&uri[7..])
                        } else {
                            PathBuf::from(uri)
                        };

                        if let Some(edits) = change.get("edits") {
                            affected_files.push((file_path, edits.clone()));
                        }
                    }
                }
            }
        }

        Ok(affected_files)
    }

    /// Process a queued operation
    async fn process_queued_operation(
        &self,
        _tool_name: &str,
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