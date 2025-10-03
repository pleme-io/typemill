//! File operations tool handler
//!
//! Handles: rename_file, create_file, delete_file, read_file, write_file, list_files

use super::compat::{ToolContext, ToolHandler};
use cb_core::workspaces::WorkspaceManager;
use cb_protocol::{ApiError as ServerError, ApiResult as ServerResult};
use async_trait::async_trait;
use cb_core::model::mcp::ToolCall;
use serde_json::{json, Value};
use std::path::Path;
use std::time::Duration;
use tracing::{debug, error};

pub struct FileOperationHandler;

impl FileOperationHandler {
    pub fn new() -> Self {
        Self
    }

    /// Escape a shell argument for safe execution
    fn escape_shell_arg(arg: &str) -> String {
        // Replace single quotes with '\'' to safely escape for sh -c
        arg.replace('\'', "'\\''")
    }

    /// Execute a command in a remote workspace via its agent
    async fn execute_remote_command(
        workspace_manager: &WorkspaceManager,
        workspace_id: &str,
        command: &str,
    ) -> ServerResult<String> {
        debug!(
            workspace_id = %workspace_id,
            command = %command,
            "Executing remote command"
        );

        // Look up workspace
        let workspace = workspace_manager.get(workspace_id).ok_or_else(|| {
            ServerError::InvalidRequest(format!("Workspace '{}' not found", workspace_id))
        })?;

        // Build agent URL
        let agent_url = format!("{}/execute", workspace.agent_url);

        // Create HTTP client with timeout
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .map_err(|e| {
                error!(error = %e, "Failed to create HTTP client");
                ServerError::Internal("HTTP client error".into())
            })?;

        // Execute command via agent
        let response = client
            .post(&agent_url)
            .json(&json!({ "command": command }))
            .send()
            .await
            .map_err(|e| {
                error!(
                    workspace_id = %workspace_id,
                    agent_url = %agent_url,
                    error = %e,
                    "Failed to send command to workspace agent"
                );
                ServerError::Internal(format!("Failed to reach workspace agent: {}", e))
            })?;

        // Check response status
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            error!(
                workspace_id = %workspace_id,
                status = %status,
                error = %error_text,
                "Workspace agent returned error"
            );
            return Err(ServerError::Internal(format!(
                "Workspace agent error ({}): {}",
                status, error_text
            )));
        }

        // Parse response
        let result: serde_json::Value = response.json().await.map_err(|e| {
            error!(error = %e, "Failed to parse agent response");
            ServerError::Internal("Failed to parse agent response".into())
        })?;

        // Extract stdout from response
        result
            .get("stdout")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| {
                error!("Agent response missing stdout field");
                ServerError::Internal("Invalid agent response format".into())
            })
    }
}

impl Default for FileOperationHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ToolHandler for FileOperationHandler {
    fn supported_tools(&self) -> Vec<&'static str> {
        vec![
            "rename_file",
            "rename_directory",
            "create_file",
            "delete_file",
            "read_file",
            "write_file",
            "list_files",
        ]
    }

    async fn handle_tool(&self, tool_call: ToolCall, context: &ToolContext) -> ServerResult<Value> {
        debug!(tool_name = %tool_call.name, "Handling file operation");

        match tool_call.name.as_str() {
            "rename_file" => self.handle_rename_file(tool_call, context).await,
            "rename_directory" => self.handle_rename_directory(tool_call, context).await,
            "create_file" => self.handle_create_file(tool_call, context).await,
            "delete_file" => self.handle_delete_file(tool_call, context).await,
            "read_file" => self.handle_read_file(tool_call, context).await,
            "write_file" => self.handle_write_file(tool_call, context).await,
            "list_files" => self.handle_list_files(tool_call, context).await,
            _ => Err(ServerError::Unsupported(format!(
                "Unknown file operation: {}",
                tool_call.name
            ))),
        }
    }
}

impl FileOperationHandler {
    async fn handle_rename_file(
        &self,
        tool_call: ToolCall,
        context: &ToolContext,
    ) -> ServerResult<Value> {
        let args = tool_call.arguments.ok_or_else(|| {
            ServerError::InvalidRequest("Missing arguments for rename_file".into())
        })?;

        let old_path = args
            .get("old_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ServerError::InvalidRequest("Missing 'old_path' parameter".into()))?;
        let new_path = args
            .get("new_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ServerError::InvalidRequest("Missing 'new_path' parameter".into()))?;
        let dry_run = args
            .get("dry_run")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let result = context
            .app_state
            .file_service
            .rename_file_with_imports(Path::new(old_path), Path::new(new_path), dry_run)
            .await?;

        if result.dry_run {
            // Merge status into the result object instead of nesting
            if let Value::Object(mut obj) = result.result {
                obj.insert("status".to_string(), json!("preview"));
                Ok(Value::Object(obj))
            } else {
                // Fallback for non-object results
                Ok(json!({
                    "status": "preview",
                    "result": result.result
                }))
            }
        } else {
            Ok(result.result)
        }
    }

    async fn handle_rename_directory(
        &self,
        tool_call: ToolCall,
        context: &ToolContext,
    ) -> ServerResult<Value> {
        let args = tool_call.arguments.ok_or_else(|| {
            ServerError::InvalidRequest("Missing arguments for rename_directory".into())
        })?;

        let old_path = args
            .get("old_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ServerError::InvalidRequest("Missing 'old_path' parameter".into()))?;
        let new_path = args
            .get("new_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ServerError::InvalidRequest("Missing 'new_path' parameter".into()))?;
        let dry_run = args
            .get("dry_run")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let result = context
            .app_state
            .file_service
            .rename_directory_with_imports(Path::new(old_path), Path::new(new_path), dry_run)
            .await?;

        if result.dry_run {
            // Merge status into the result object instead of nesting
            if let Value::Object(mut obj) = result.result {
                obj.insert("status".to_string(), json!("preview"));
                Ok(Value::Object(obj))
            } else {
                // Fallback for non-object results
                Ok(json!({
                    "status": "preview",
                    "result": result.result
                }))
            }
        } else {
            Ok(result.result)
        }
    }

    async fn handle_create_file(
        &self,
        tool_call: ToolCall,
        context: &ToolContext,
    ) -> ServerResult<Value> {
        let args = tool_call.arguments.ok_or_else(|| {
            ServerError::InvalidRequest("Missing arguments for create_file".into())
        })?;

        let file_path = args
            .get("file_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ServerError::InvalidRequest("Missing 'file_path' parameter".into()))?;
        let content = args.get("content").and_then(|v| v.as_str());
        let overwrite = args
            .get("overwrite")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let dry_run = args
            .get("dry_run")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let result = context
            .app_state
            .file_service
            .create_file(Path::new(file_path), content, overwrite, dry_run)
            .await?;

        if result.dry_run {
            // Merge status into the result object instead of nesting
            if let Value::Object(mut obj) = result.result {
                obj.insert("status".to_string(), json!("preview"));
                Ok(Value::Object(obj))
            } else {
                // Fallback for non-object results
                Ok(json!({
                    "status": "preview",
                    "result": result.result
                }))
            }
        } else {
            Ok(result.result)
        }
    }

    async fn handle_delete_file(
        &self,
        tool_call: ToolCall,
        context: &ToolContext,
    ) -> ServerResult<Value> {
        let args = tool_call.arguments.ok_or_else(|| {
            ServerError::InvalidRequest("Missing arguments for delete_file".into())
        })?;

        let file_path = args
            .get("file_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ServerError::InvalidRequest("Missing 'file_path' parameter".into()))?;
        let force = args.get("force").and_then(|v| v.as_bool()).unwrap_or(false);
        let dry_run = args
            .get("dry_run")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let result = context
            .app_state
            .file_service
            .delete_file(Path::new(file_path), force, dry_run)
            .await?;

        if result.dry_run {
            // Merge status into the result object instead of nesting
            if let Value::Object(mut obj) = result.result {
                obj.insert("status".to_string(), json!("preview"));
                Ok(Value::Object(obj))
            } else {
                // Fallback for non-object results
                Ok(json!({
                    "status": "preview",
                    "result": result.result
                }))
            }
        } else {
            Ok(result.result)
        }
    }

    async fn handle_read_file(
        &self,
        tool_call: ToolCall,
        context: &ToolContext,
    ) -> ServerResult<Value> {
        let args = tool_call
            .arguments
            .ok_or_else(|| ServerError::InvalidRequest("Missing arguments for read_file".into()))?;

        let file_path = args
            .get("file_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ServerError::InvalidRequest("Missing 'file_path' parameter".into()))?;
        let workspace_id = args.get("workspace_id").and_then(|v| v.as_str());

        // Route to workspace or local filesystem
        let content = if let Some(workspace_id) = workspace_id {
            // Execute in remote workspace
            let command = format!("cat '{}'", Self::escape_shell_arg(file_path));
            Self::execute_remote_command(
                &context.app_state.workspace_manager,
                workspace_id,
                &command,
            )
            .await?
        } else {
            // Use FileService for local operations
            context
                .app_state
                .file_service
                .read_file(Path::new(file_path))
                .await?
        };

        Ok(json!({
            "success": true,
            "file_path": file_path,
            "content": content
        }))
    }

    async fn handle_write_file(
        &self,
        tool_call: ToolCall,
        context: &ToolContext,
    ) -> ServerResult<Value> {
        let args = tool_call.arguments.ok_or_else(|| {
            ServerError::InvalidRequest("Missing arguments for write_file".into())
        })?;

        let file_path = args
            .get("file_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ServerError::InvalidRequest("Missing 'file_path' parameter".into()))?;
        let content = args
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ServerError::InvalidRequest("Missing 'content' parameter".into()))?;
        let dry_run = args
            .get("dry_run")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let workspace_id = args.get("workspace_id").and_then(|v| v.as_str());

        // Route to workspace or local filesystem
        if let Some(workspace_id) = workspace_id {
            // Remote workspace - dry_run not supported for remote operations
            if dry_run {
                return Err(ServerError::InvalidRequest(
                    "dry_run not supported for remote workspace operations".into(),
                ));
            }

            // Use printf for safer writing (avoids issues with echo interpreting backslashes)
            let command = format!(
                "printf '%s' '{}' > '{}'",
                Self::escape_shell_arg(content),
                Self::escape_shell_arg(file_path)
            );
            Self::execute_remote_command(
                &context.app_state.workspace_manager,
                workspace_id,
                &command,
            )
            .await?;

            Ok(json!({
                "success": true,
                "file_path": file_path,
                "message": "File written successfully"
            }))
        } else {
            // Local filesystem
            let result = context
                .app_state
                .file_service
                .write_file(Path::new(file_path), content, dry_run)
                .await?;

            if result.dry_run {
                Ok(json!({
                    "status": "preview",
                    "preview": result.result
                }))
            } else {
                Ok(result.result)
            }
        }
    }

    async fn handle_list_files(
        &self,
        tool_call: ToolCall,
        context: &ToolContext,
    ) -> ServerResult<Value> {
        let args = tool_call.arguments.unwrap_or_else(|| json!({}));

        let directory = args
            .get("directory")
            .and_then(|v| v.as_str())
            .unwrap_or(".");
        let recursive = args
            .get("recursive")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let files = context
            .app_state
            .file_service
            .list_files(Path::new(directory), recursive)
            .await?;

        Ok(json!({
            "success": true,
            "directory": directory,
            "files": files
        }))
    }
}
