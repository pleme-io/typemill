//! Refactoring operations tool handler
//!
//! Handles: extract_function, inline_variable, extract_variable, extract_module_to_package, fix_imports

use super::plugin_dispatcher::DirectLspAdapter;
use super::tool_handler::{ToolContext, ToolHandler};
use crate::workspaces::WorkspaceManager;
use crate::{ServerError, ServerResult};
use async_trait::async_trait;
use cb_ast::refactoring::{CodeRange, LspRefactoringService};
use cb_core::model::mcp::ToolCall;
use cb_plugins::PluginRequest;
use serde::Deserialize;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, error};

/// Parameter structures for refactoring operations
#[derive(Debug, Deserialize)]
struct ExtractFunctionArgs {
    file_path: String,
    start_line: u32,
    end_line: u32,
    function_name: String,
    #[serde(default)]
    dry_run: Option<bool>,
    #[serde(default)]
    workspace_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct InlineVariableArgs {
    file_path: String,
    line: u32,
    #[serde(default)]
    character: Option<u32>,
    #[serde(default)]
    dry_run: Option<bool>,
    #[serde(default)]
    workspace_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ExtractVariableArgs {
    file_path: String,
    start_line: u32,
    start_character: u32,
    end_line: u32,
    end_character: u32,
    variable_name: String,
    #[serde(default)]
    dry_run: Option<bool>,
    #[serde(default)]
    workspace_id: Option<String>,
}

/// LSP service wrapper for refactoring operations
struct LspRefactoringServiceWrapper {
    lsp_adapter: Arc<DirectLspAdapter>,
}

impl LspRefactoringServiceWrapper {
    fn new(lsp_adapter: Arc<DirectLspAdapter>) -> Self {
        Self { lsp_adapter }
    }

    fn get_extension(file_path: &str) -> Option<String> {
        Path::new(file_path)
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|s| s.to_string())
    }
}

#[async_trait]
impl LspRefactoringService for LspRefactoringServiceWrapper {
    async fn get_code_actions(
        &self,
        file_path: &str,
        range: &CodeRange,
        kinds: Option<Vec<String>>,
    ) -> cb_ast::error::AstResult<Value> {
        let uri = format!("file://{}", file_path);

        let params = json!({
            "textDocument": {
                "uri": uri
            },
            "range": {
                "start": {
                    "line": range.start_line,
                    "character": range.start_col
                },
                "end": {
                    "line": range.end_line,
                    "character": range.end_col
                }
            },
            "context": {
                "diagnostics": [],
                "only": kinds.unwrap_or_default()
            }
        });

        let extension = Self::get_extension(file_path).ok_or_else(|| {
            cb_ast::error::AstError::analysis(format!(
                "Could not determine file extension for: {}",
                file_path
            ))
        })?;

        let client = self
            .lsp_adapter
            .get_or_create_client(&extension)
            .await
            .map_err(|e| cb_ast::error::AstError::analysis(format!("LSP client error: {}", e)))?;

        client
            .send_request("textDocument/codeAction", params)
            .await
            .map_err(|e| cb_ast::error::AstError::analysis(format!("LSP request failed: {}", e)))
            .map(|v| v)
    }
}

pub struct RefactoringHandler;

impl RefactoringHandler {
    pub fn new() -> Self {
        Self
    }

    fn escape_shell_arg(arg: &str) -> String {
        arg.replace('\'', "'\\''")
    }

    async fn execute_remote_command(
        workspace_manager: &WorkspaceManager,
        workspace_id: &str,
        command: &str,
    ) -> ServerResult<String> {
        debug!(workspace_id = %workspace_id, command = %command, "Executing remote command");

        let workspace = workspace_manager
            .get(workspace_id)
            .ok_or_else(|| {
                ServerError::InvalidRequest(format!("Workspace '{}' not found", workspace_id))
            })?;

        let agent_url = format!("{}/execute", workspace.agent_url);

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .map_err(|e| {
                error!(error = %e, "Failed to create HTTP client");
                ServerError::Internal("HTTP client error".into())
            })?;

        let response = client
            .post(&agent_url)
            .json(&json!({ "command": command }))
            .send()
            .await
            .map_err(|e| {
                error!(workspace_id = %workspace_id, agent_url = %agent_url, error = %e, "Failed to send command");
                ServerError::Internal(format!("Failed to reach workspace agent: {}", e))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            error!(workspace_id = %workspace_id, status = %status, error = %error_text, "Workspace agent returned error");
            return Err(ServerError::Internal(format!(
                "Workspace agent error ({}): {}",
                status, error_text
            )));
        }

        let result: Value = response
            .json()
            .await
            .map_err(|e| {
                error!(error = %e, "Failed to parse agent response");
                ServerError::Internal("Failed to parse agent response".into())
            })?;

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

impl Default for RefactoringHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ToolHandler for RefactoringHandler {
    fn supported_tools(&self) -> Vec<&'static str> {
        vec![
            "extract_function",
            "inline_variable",
            "extract_variable",
            "extract_module_to_package",
            "fix_imports",
        ]
    }

    async fn handle_tool(
        &self,
        tool_call: ToolCall,
        context: &ToolContext,
    ) -> ServerResult<Value> {
        debug!(tool_name = %tool_call.name, "Handling refactoring operation");

        match tool_call.name.as_str() {
            "extract_function" | "inline_variable" | "extract_variable" | "extract_module_to_package" => {
                self.handle_refactoring_operation(tool_call, context).await
            }
            "fix_imports" => self.handle_fix_imports(tool_call, context).await,
            _ => Err(ServerError::Unsupported(format!(
                "Unknown refactoring operation: {}",
                tool_call.name
            ))),
        }
    }
}

impl RefactoringHandler {
    async fn handle_refactoring_operation(
        &self,
        tool_call: ToolCall,
        context: &ToolContext,
    ) -> ServerResult<Value> {
        debug!(tool_name = %tool_call.name, "Handling refactoring operation");

        let args = tool_call.arguments.ok_or_else(|| {
            ServerError::InvalidRequest(format!("Missing arguments for {}", tool_call.name))
        })?;

        // Parse and execute refactoring based on tool type
        let (file_path, dry_run, workspace_id, edit_plan) = match tool_call.name.as_str() {
            "extract_function" => {
                let parsed: ExtractFunctionArgs = serde_json::from_value(args)
                    .map_err(|e| ServerError::InvalidRequest(format!("Invalid arguments: {}", e)))?;

                let content = if let Some(workspace_id) = &parsed.workspace_id {
                    let command = format!("cat '{}'", Self::escape_shell_arg(&parsed.file_path));
                    Self::execute_remote_command(
                        &context.app_state.workspace_manager,
                        workspace_id,
                        &command,
                    )
                    .await?
                } else {
                    context
                        .app_state
                        .file_service
                        .read_file(Path::new(&parsed.file_path))
                        .await?
                };

                let lines: Vec<&str> = content.lines().collect();
                let end_col = if parsed.end_line > 0 && (parsed.end_line as usize) <= lines.len() {
                    let line = lines[(parsed.end_line as usize) - 1];
                    line.len() as u32
                } else {
                    0
                };

                let range = CodeRange {
                    start_line: parsed.start_line,
                    start_col: 0,
                    end_line: parsed.end_line,
                    end_col,
                };

                let lsp_service: Option<LspRefactoringServiceWrapper> = {
                    let adapter_guard = context.lsp_adapter.lock().await;
                    adapter_guard
                        .as_ref()
                        .map(|adapter| LspRefactoringServiceWrapper::new(adapter.clone()))
                };

                let plan = cb_ast::refactoring::plan_extract_function(
                    &content,
                    &range,
                    &parsed.function_name,
                    &parsed.file_path,
                    lsp_service.as_ref().map(|s| s as &dyn LspRefactoringService),
                )
                .await
                .map_err(|e| ServerError::Runtime {
                    message: format!("Extract function planning failed: {}", e),
                })?;

                (parsed.file_path, parsed.dry_run.unwrap_or(false), parsed.workspace_id, plan)
            }
            "inline_variable" => {
                let parsed: InlineVariableArgs = serde_json::from_value(args)
                    .map_err(|e| ServerError::InvalidRequest(format!("Invalid arguments: {}", e)))?;

                let content = if let Some(workspace_id) = &parsed.workspace_id {
                    let command = format!("cat '{}'", Self::escape_shell_arg(&parsed.file_path));
                    Self::execute_remote_command(
                        &context.app_state.workspace_manager,
                        workspace_id,
                        &command,
                    )
                    .await?
                } else {
                    context
                        .app_state
                        .file_service
                        .read_file(Path::new(&parsed.file_path))
                        .await?
                };

                let lsp_service: Option<LspRefactoringServiceWrapper> = {
                    let adapter_guard = context.lsp_adapter.lock().await;
                    adapter_guard
                        .as_ref()
                        .map(|adapter| LspRefactoringServiceWrapper::new(adapter.clone()))
                };

                let plan = cb_ast::refactoring::plan_inline_variable(
                    &content,
                    parsed.line,
                    parsed.character.unwrap_or(0),
                    &parsed.file_path,
                    lsp_service.as_ref().map(|s| s as &dyn LspRefactoringService),
                )
                .await
                .map_err(|e| ServerError::Runtime {
                    message: format!("Inline variable planning failed: {}", e),
                })?;

                (parsed.file_path, parsed.dry_run.unwrap_or(false), parsed.workspace_id, plan)
            }
            "extract_variable" => {
                let parsed: ExtractVariableArgs = serde_json::from_value(args)
                    .map_err(|e| ServerError::InvalidRequest(format!("Invalid arguments: {}", e)))?;

                let content = if let Some(workspace_id) = &parsed.workspace_id {
                    let command = format!("cat '{}'", Self::escape_shell_arg(&parsed.file_path));
                    Self::execute_remote_command(
                        &context.app_state.workspace_manager,
                        workspace_id,
                        &command,
                    )
                    .await?
                } else {
                    context
                        .app_state
                        .file_service
                        .read_file(Path::new(&parsed.file_path))
                        .await?
                };

                let lsp_service: Option<LspRefactoringServiceWrapper> = {
                    let adapter_guard = context.lsp_adapter.lock().await;
                    adapter_guard
                        .as_ref()
                        .map(|adapter| LspRefactoringServiceWrapper::new(adapter.clone()))
                };

                let plan = cb_ast::refactoring::plan_extract_variable(
                    &content,
                    parsed.start_line,
                    parsed.start_character,
                    parsed.end_line,
                    parsed.end_character,
                    Some(parsed.variable_name.clone()),
                    &parsed.file_path,
                    lsp_service.as_ref().map(|s| s as &dyn LspRefactoringService),
                )
                .await
                .map_err(|e| ServerError::Runtime {
                    message: format!("Extract variable planning failed: {}", e),
                })?;

                (parsed.file_path, parsed.dry_run.unwrap_or(false), parsed.workspace_id, plan)
            }
            "extract_module_to_package" => {
                let parsed: cb_ast::package_extractor::ExtractModuleToPackageParams =
                    serde_json::from_value(args)
                        .map_err(|e| ServerError::InvalidRequest(format!("Invalid arguments: {}", e)))?;

                let plan = cb_ast::package_extractor::plan_extract_module_to_package(parsed)
                    .await
                    .map_err(|e| ServerError::Runtime {
                        message: format!("Extract module to package planning failed: {}", e),
                    })?;

                (
                    plan.source_file.clone(),
                    false,
                    None,
                    plan,
                )
            }
            _ => {
                return Err(ServerError::InvalidRequest(format!(
                    "Unknown refactoring operation: {}",
                    tool_call.name
                )))
            }
        };

        // Apply edits with workspace routing
        if let Some(workspace_id) = &workspace_id {
            if dry_run {
                return Ok(json!({
                    "status": "preview",
                    "operation": tool_call.name,
                    "file_path": file_path,
                    "edit_plan": edit_plan,
                }));
            }

            if let Some(file_edit) = edit_plan.edits.get(0) {
                let command = format!(
                    "printf '%s' '{}' > '{}'",
                    Self::escape_shell_arg(&file_edit.new_content),
                    Self::escape_shell_arg(&file_edit.file_path)
                );
                Self::execute_remote_command(
                    &context.app_state.workspace_manager,
                    workspace_id,
                    &command,
                )
                .await?;

                Ok(json!({
                    "status": "completed",
                    "operation": tool_call.name,
                    "file_path": file_path,
                    "success": true,
                    "modified_files": [file_path],
                }))
            } else {
                Ok(json!({
                    "status": "completed",
                    "operation": tool_call.name,
                    "file_path": file_path,
                    "success": true,
                    "modified_files": [],
                    "message": "No changes needed."
                }))
            }
        } else {
            let dry_run_result = cb_core::execute_with_dry_run(
                dry_run,
                || async {
                    Ok(json!({
                        "status": "preview",
                        "operation": tool_call.name,
                        "file_path": file_path,
                        "edit_plan": edit_plan,
                    }))
                },
                || async {
                    let result = context
                        .app_state
                        .file_service
                        .apply_edit_plan(&edit_plan)
                        .await?;

                    Ok(json!({
                        "status": "completed",
                        "operation": tool_call.name,
                        "file_path": file_path,
                        "success": result.success,
                        "modified_files": result.modified_files,
                        "errors": result.errors
                    }))
                },
            )
            .await
            .map_err(|e| ServerError::Internal(format!("Dry run execution failed: {}", e)))?;

            Ok(dry_run_result.to_json())
        }
    }

    async fn handle_fix_imports(
        &self,
        tool_call: ToolCall,
        context: &ToolContext,
    ) -> ServerResult<Value> {
        let args = tool_call.arguments.unwrap_or(json!({}));

        let file_path = args
            .get("file_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ServerError::InvalidRequest("file_path is required".to_string()))?;

        let dry_run = args.get("dry_run").and_then(|v| v.as_bool()).unwrap_or(false);

        debug!(file_path = %file_path, dry_run = dry_run, "Handling fix_imports via organize_imports");

        if dry_run {
            return Ok(json!({
                "operation": "fix_imports",
                "file_path": file_path,
                "dry_run": true,
                "modified": false,
                "status": "preview",
                "message": "Dry run mode - set dry_run: false to apply import organization"
            }));
        }

        let mut plugin_request = PluginRequest::new(
            "organize_imports".to_string(),
            PathBuf::from(file_path),
        );
        plugin_request.params = json!({
            "file_path": file_path
        });

        match context.plugin_manager.handle_request(plugin_request).await {
            Ok(response) => {
                Ok(json!({
                    "operation": "fix_imports",
                    "file_path": file_path,
                    "dry_run": false,
                    "modified": true,
                    "status": "fixed",
                    "lsp_response": response
                }))
            }
            Err(e) => {
                Err(ServerError::internal(format!(
                    "Failed to organize imports: {}",
                    e
                )))
            }
        }
    }
}
