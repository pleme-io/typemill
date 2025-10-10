//! File operations tool handler
//!
//! Handles: rename_file, create_file, delete_file, read_file, write_file, list_files

use super::tools::{ToolHandler, ToolHandlerContext};
use crate::utils::{dry_run::wrap_dry_run_result, remote_exec};
use async_trait::async_trait;
use cb_core::model::mcp::ToolCall;
use cb_protocol::{ApiError as ServerError, ApiResult as ServerResult};
use serde_json::{json, Value};
use std::path::Path;
use tracing::debug;

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
}

impl Default for FileOperationHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ToolHandler for FileOperationHandler {
    fn tool_names(&self) -> &[&str] {
        &[
            "rename_file",
            "rename_directory",
            "create_file",
            "delete_file",
            "read_file",
            "write_file",
            "list_files",
        ]
    }

    async fn handle_tool_call(
        &self,
        context: &ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        debug!(tool_name = %tool_call.name, "Handling file operation");

        match tool_call.name.as_str() {
            "rename_file" => self.handle_rename_file(tool_call.clone(), context).await,
            "rename_directory" => {
                self.handle_rename_directory(tool_call.clone(), context)
                    .await
            }
            "create_file" => self.handle_create_file(tool_call.clone(), context).await,
            "delete_file" => self.handle_delete_file(tool_call.clone(), context).await,
            "read_file" => self.handle_read_file(tool_call.clone(), context).await,
            "write_file" => self.handle_write_file(tool_call.clone(), context).await,
            "list_files" => self.handle_list_files(tool_call.clone(), context).await,
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
        context: &ToolHandlerContext,
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
            .rename_file_with_imports(Path::new(old_path), Path::new(new_path), dry_run, None)
            .await?;

        wrap_dry_run_result(result)
    }

    async fn handle_rename_directory(
        &self,
        tool_call: ToolCall,
        context: &ToolHandlerContext,
    ) -> ServerResult<Value> {
        let args = tool_call.arguments.ok_or_else(|| {
            ServerError::InvalidRequest("Missing arguments for rename_directory".into())
        })?;

        // Deserialize into strongly-typed parameters
        let params: cb_protocol::RenameDirectoryParams =
            serde_json::from_value(args).map_err(|e| {
                ServerError::InvalidRequest(format!("Invalid rename_directory parameters: {}", e))
            })?;

        // Parse update_mode parameter (optional, defaults to Conservative)
        let update_mode = params
            .update_mode
            .as_ref()
            .and_then(|s| match s.to_lowercase().as_str() {
                "conservative" => Some(crate::handlers::tools::workspace::UpdateMode::Conservative),
                "standard" => Some(crate::handlers::tools::workspace::UpdateMode::Standard),
                "aggressive" => Some(crate::handlers::tools::workspace::UpdateMode::Aggressive),
                "full" => Some(crate::handlers::tools::workspace::UpdateMode::Full),
                _ => None,
            })
            .unwrap_or(crate::handlers::tools::workspace::UpdateMode::Conservative);

        // Require dry_run=true for risky modes if not already in dry_run mode
        if update_mode.is_risky() && !params.dry_run {
            return Err(ServerError::InvalidRequest(format!(
                "⚠️ {} mode requires dry_run=true for safety. Please run with dry_run=true first to preview changes, then re-run without dry_run if the changes look correct. {}",
                match update_mode {
                    crate::handlers::tools::workspace::UpdateMode::Aggressive => "Aggressive",
                    crate::handlers::tools::workspace::UpdateMode::Full => "Full",
                    _ => unreachable!()
                },
                update_mode.warning_message().unwrap_or("")
            )));
        }

        let result = context
            .app_state
            .file_service
            .rename_directory_with_imports(
                &params.old_path,
                &params.new_path,
                params.dry_run,
                params.consolidate,
                Some(update_mode.to_scan_scope()),
            )
            .await?;

        // Wrap with dry-run status, then add warning if present
        let mut response = wrap_dry_run_result(result)?;
        if let Some(warning) = update_mode.warning_message() {
            if let Value::Object(ref mut obj) = response {
                obj.insert("warning".to_string(), json!(warning));
            }
        }

        Ok(response)
    }

    async fn handle_create_file(
        &self,
        tool_call: ToolCall,
        context: &ToolHandlerContext,
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

        wrap_dry_run_result(result)
    }

    async fn handle_delete_file(
        &self,
        tool_call: ToolCall,
        context: &ToolHandlerContext,
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

        wrap_dry_run_result(result)
    }

    async fn handle_read_file(
        &self,
        tool_call: ToolCall,
        context: &ToolHandlerContext,
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
            let user_id = context.user_id.as_deref().ok_or_else(|| {
                ServerError::InvalidRequest(
                    "A user_id is required for remote workspace operations".into(),
                )
            })?;
            // Execute in remote workspace
            let command = format!("cat '{}'", Self::escape_shell_arg(file_path));
            remote_exec::execute_in_workspace(
                &context.app_state.workspace_manager,
                user_id,
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
        context: &ToolHandlerContext,
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
            let user_id = context.user_id.as_deref().ok_or_else(|| {
                ServerError::InvalidRequest(
                    "A user_id is required for remote workspace operations".into(),
                )
            })?;

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
            remote_exec::execute_in_workspace(
                &context.app_state.workspace_manager,
                user_id,
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

            wrap_dry_run_result(result)
        }
    }

    async fn handle_list_files(
        &self,
        tool_call: ToolCall,
        context: &ToolHandlerContext,
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
        let pattern = args.get("pattern").and_then(|v| v.as_str());

        let files = context
            .app_state
            .file_service
            .list_files_with_pattern(Path::new(directory), recursive, pattern)
            .await?;

        Ok(json!({
            "success": true,
            "directory": directory,
            "pattern": pattern,
            "files": files
        }))
    }
}
