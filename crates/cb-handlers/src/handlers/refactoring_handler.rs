//! Refactoring operations tool handler
//!
//! Handles: extract_function, extract_module_to_package, extract_variable, inline_variable

use super::tools::{ToolHandler, ToolHandlerContext};
use super::lsp_adapter::DirectLspAdapter;
use crate::utils::remote_exec;
use async_trait::async_trait;
use cb_ast::refactoring::{CodeRange, LspRefactoringService};
use cb_core::model::mcp::ToolCall;
use cb_protocol::{ApiError as ServerError, ApiResult as ServerResult};
use serde::Deserialize;
use serde_json::{json, Value};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::debug;

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

        // Ensure file is opened in LSP server before requesting code actions
        let content = tokio::fs::read_to_string(file_path).await.map_err(|e| {
            cb_ast::error::AstError::analysis(format!("Failed to read file: {}", e))
        })?;

        let did_open_params = json!({
            "textDocument": {
                "uri": uri,
                "languageId": extension,
                "version": 1,
                "text": content
            }
        });

        // Send didOpen notification (fire and forget)
        let _ = client
            .send_notification("textDocument/didOpen", did_open_params)
            .await;

        // Small delay to let LSP process the file
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

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

        client
            .send_request("textDocument/codeAction", params)
            .await
            .map_err(|e| cb_ast::error::AstError::analysis(format!("LSP request failed: {}", e)))
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

    /// Read file content from local filesystem or remote workspace
    async fn read_file_content(
        workspace_id: Option<&str>,
        user_id: Option<&str>,
        file_path: &str,
        file_service: &cb_services::services::FileService,
        workspace_manager: &cb_core::workspaces::WorkspaceManager,
    ) -> ServerResult<String> {
        if let Some(workspace_id) = workspace_id {
            let user_id = user_id.ok_or_else(|| {
                ServerError::InvalidRequest(
                    "A user_id is required for remote workspace operations".into(),
                )
            })?;
            let command = format!("cat '{}'", Self::escape_shell_arg(file_path));
            remote_exec::execute_in_workspace(
                workspace_manager,
                user_id,
                workspace_id,
                &command,
            )
            .await
        } else {
            file_service.read_file(Path::new(file_path)).await
        }
    }

    /// Create LSP refactoring service wrapper from adapter
    async fn create_lsp_service(
        lsp_adapter: &Arc<Mutex<Option<Arc<DirectLspAdapter>>>>,
    ) -> Option<LspRefactoringServiceWrapper> {
        let adapter_guard = lsp_adapter.lock().await;
        let result = adapter_guard
            .as_ref()
            .map(|adapter| LspRefactoringServiceWrapper::new(adapter.clone()));

        if result.is_none() {
            debug!("LSP adapter is None - refactoring will fall back to AST");
        } else {
            debug!("LSP service created successfully");
        }

        result
    }
}

impl Default for RefactoringHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ToolHandler for RefactoringHandler {
    fn tool_names(&self) -> &[&str] {
        &[
            "extract_function",
            "inline_variable",
            "extract_variable",
            "extract_module_to_package",
        ]
    }

    async fn handle_tool_call(&self, context: &ToolHandlerContext, tool_call: &ToolCall) -> ServerResult<Value> {
        debug!(tool_name = %tool_call.name, "Handling refactoring operation");

        match tool_call.name.as_str() {
            "extract_function"
            | "inline_variable"
            | "extract_variable"
            | "extract_module_to_package" => {
                self.handle_refactoring_operation(tool_call.clone(), context).await
            }
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
        context: &ToolHandlerContext,
    ) -> ServerResult<Value> {
        debug!(tool_name = %tool_call.name, "Handling refactoring operation");

        let args = tool_call.arguments.ok_or_else(|| {
            ServerError::InvalidRequest(format!("Missing arguments for {}", tool_call.name))
        })?;

        // Parse and execute refactoring based on tool type
        let (file_path, dry_run, workspace_id, edit_plan) = match tool_call.name.as_str() {
            "extract_function" => {
                let parsed: ExtractFunctionArgs = serde_json::from_value(args).map_err(|e| {
                    ServerError::InvalidRequest(format!("Invalid arguments: {}", e))
                })?;

                let content = Self::read_file_content(
                    parsed.workspace_id.as_deref(),
                    context.user_id.as_deref(),
                    &parsed.file_path,
                    &context.app_state.file_service,
                    &context.app_state.workspace_manager,
                )
                .await?;

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

                let lsp_service = Self::create_lsp_service(&context.lsp_adapter).await;

                let plan = cb_ast::refactoring::extract_function::plan_extract_function(
                    &content,
                    &range,
                    &parsed.function_name,
                    &parsed.file_path,
                    lsp_service
                        .as_ref()
                        .map(|s| s as &dyn LspRefactoringService),
                )
                .await
                .map_err(|e| ServerError::Runtime {
                    message: format!("Extract function planning failed: {}", e),
                })?;

                (
                    parsed.file_path,
                    parsed.dry_run.unwrap_or(false),
                    parsed.workspace_id,
                    plan,
                )
            }
            "inline_variable" => {
                let parsed: InlineVariableArgs = serde_json::from_value(args).map_err(|e| {
                    ServerError::InvalidRequest(format!("Invalid arguments: {}", e))
                })?;

                let content = Self::read_file_content(
                    parsed.workspace_id.as_deref(),
                    context.user_id.as_deref(),
                    &parsed.file_path,
                    &context.app_state.file_service,
                    &context.app_state.workspace_manager,
                )
                .await?;

                let lsp_service = Self::create_lsp_service(&context.lsp_adapter).await;

                let plan = cb_ast::refactoring::inline_variable::plan_inline_variable(
                    &content,
                    parsed.line,
                    parsed.character.unwrap_or(0),
                    &parsed.file_path,
                    lsp_service
                        .as_ref()
                        .map(|s| s as &dyn LspRefactoringService),
                )
                .await
                .map_err(|e| ServerError::Runtime {
                    message: format!("Inline variable planning failed: {}", e),
                })?;

                (
                    parsed.file_path,
                    parsed.dry_run.unwrap_or(false),
                    parsed.workspace_id,
                    plan,
                )
            }
            "extract_variable" => {
                let parsed: ExtractVariableArgs = serde_json::from_value(args).map_err(|e| {
                    ServerError::InvalidRequest(format!("Invalid arguments: {}", e))
                })?;

                let content = Self::read_file_content(
                    parsed.workspace_id.as_deref(),
                    context.user_id.as_deref(),
                    &parsed.file_path,
                    &context.app_state.file_service,
                    &context.app_state.workspace_manager,
                )
                .await?;

                let lsp_service = Self::create_lsp_service(&context.lsp_adapter).await;

                let plan = cb_ast::refactoring::extract_variable::plan_extract_variable(
                    &content,
                    parsed.start_line,
                    parsed.start_character,
                    parsed.end_line,
                    parsed.end_character,
                    Some(parsed.variable_name.clone()),
                    &parsed.file_path,
                    lsp_service
                        .as_ref()
                        .map(|s| s as &dyn LspRefactoringService),
                )
                .await
                .map_err(|e| ServerError::Runtime {
                    message: format!("Extract variable planning failed: {}", e),
                })?;

                (
                    parsed.file_path,
                    parsed.dry_run.unwrap_or(false),
                    parsed.workspace_id,
                    plan,
                )
            }
            "extract_module_to_package" => {
                let parsed: cb_ast::package_extractor::ExtractModuleToPackageParams =
                    serde_json::from_value(args).map_err(|e| {
                        ServerError::InvalidRequest(format!("Invalid arguments: {}", e))
                    })?;

                // Use centralized language plugin registry
                let registry = context.app_state.language_plugins.inner.clone();

                let plan = cb_ast::package_extractor::plan_extract_module_to_package_with_registry(
                    parsed, &registry,
                )
                .await
                .map_err(|e| ServerError::Runtime {
                    message: format!("Extract module to package planning failed: {}", e),
                })?;

                (plan.source_file.clone(), false, None, plan)
            }
            _ => {
                return Err(ServerError::InvalidRequest(format!(
                    "Unknown refactoring operation: {}",
                    tool_call.name
                )))
            }
        };

        // Apply edits with workspace routing
        if let Some(workspace_id) = workspace_id.as_deref() {
            if dry_run {
                return Ok(json!({
                    "status": "preview",
                    "operation": tool_call.name,
                    "file_path": file_path,
                    "edit_plan": edit_plan,
                }));
            }

            if let Some(file_edit) = edit_plan.edits.first() {
                let user_id = context.user_id.as_deref().ok_or_else(|| {
                    ServerError::InvalidRequest(
                        "A user_id is required for remote workspace operations".into(),
                    )
                })?;
                let target_file = file_edit
                    .file_path
                    .as_ref()
                    .unwrap_or(&edit_plan.source_file);
                let command = format!(
                    "printf '%s' '{}' > '{}'",
                    Self::escape_shell_arg(&file_edit.new_text),
                    Self::escape_shell_arg(target_file)
                );
                remote_exec::execute_in_workspace(
                    &context.app_state.workspace_manager,
                    user_id,
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
}