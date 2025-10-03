//! Workspace operations tool handlers
//!
//! Handles: rename_directory, analyze_imports, find_dead_code, update_dependencies, extract_module_to_package

use super::{ToolHandler, ToolHandlerContext};
use crate::handlers::compat::{ToolContext, ToolHandler as LegacyToolHandler};
use crate::handlers::file_operation_handler::FileOperationHandler as LegacyFileHandler;
use crate::handlers::refactoring_handler::RefactoringHandler as LegacyRefactoringHandler;
use crate::handlers::system_handler::SystemHandler as LegacySystemHandler;
use cb_protocol::ApiResult as ServerResult;
use async_trait::async_trait;
use cb_core::model::mcp::ToolCall;
use serde_json::Value;

pub struct WorkspaceHandler {
    file_handler: LegacyFileHandler,
    system_handler: LegacySystemHandler,
    refactoring_handler: LegacyRefactoringHandler,
}

impl WorkspaceHandler {
    pub fn new() -> Self {
        Self {
            file_handler: LegacyFileHandler::new(),
            system_handler: LegacySystemHandler::new(),
            refactoring_handler: LegacyRefactoringHandler::new(),
        }
    }
}

#[async_trait]
impl ToolHandler for WorkspaceHandler {
    fn tool_names(&self) -> &[&str] {
        &[
            "rename_directory",
            "analyze_imports",
            "find_dead_code",
            "update_dependencies",
            "extract_module_to_package",
            "update_dependency",
        ]
    }

    async fn handle_tool_call(
        &self,
        context: &ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        // Convert new context to legacy context
        let legacy_context = ToolContext {
            app_state: context.app_state.clone(),
            plugin_manager: context.plugin_manager.clone(),
            lsp_adapter: context.lsp_adapter.clone(),
        };

        // Route to appropriate legacy handler
        match tool_call.name.as_str() {
            "rename_directory" => {
                self.file_handler
                    .handle_tool(tool_call.clone(), &legacy_context)
                    .await
            }
            "analyze_imports" | "find_dead_code" | "update_dependencies" => {
                self.system_handler
                    .handle_tool(tool_call.clone(), &legacy_context)
                    .await
            }
            "extract_module_to_package" => {
                self.refactoring_handler
                    .handle_tool(tool_call.clone(), &legacy_context)
                    .await
            }
            "update_dependency" => self.handle_update_dependency(tool_call).await,
            _ => Err(cb_protocol::ApiError::InvalidRequest(format!(
                "Unknown workspace tool: {}",
                tool_call.name
            ))),
        }
    }
}

impl WorkspaceHandler {
    /// Handle update_dependency tool call
    /// Updates a dependency in any supported manifest file (Cargo.toml, package.json, etc.)
    /// This is language-agnostic and works across all supported package managers.
    async fn handle_update_dependency(&self, tool_call: &ToolCall) -> ServerResult<Value> {
        use serde_json::json;
        use std::path::Path;
        use tokio::fs;

        // Parse arguments
        let args = tool_call
            .arguments
            .as_ref()
            .and_then(|v| v.as_object())
            .ok_or_else(|| {
                cb_protocol::ApiError::InvalidRequest("Arguments must be an object".to_string())
            })?;

        let manifest_path = args
            .get("manifest_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                cb_protocol::ApiError::InvalidRequest(
                    "Missing required parameter: manifest_path".to_string(),
                )
            })?;

        let old_dep_name = args
            .get("old_dep_name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                cb_protocol::ApiError::InvalidRequest(
                    "Missing required parameter: old_dep_name".to_string(),
                )
            })?;

        let new_dep_name = args
            .get("new_dep_name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                cb_protocol::ApiError::InvalidRequest(
                    "Missing required parameter: new_dep_name".to_string(),
                )
            })?;

        // new_path is optional - if not provided, only rename the dependency
        let new_path = args.get("new_path").and_then(|v| v.as_str());

        // Read the manifest file
        let content = fs::read_to_string(manifest_path)
            .await
            .map_err(|e| {
                cb_protocol::ApiError::Internal(format!(
                    "Failed to read manifest file at {}: {}",
                    manifest_path, e
                ))
            })?;

        // Use the manifest factory to get the correct handler
        let path = Path::new(manifest_path);
        let mut manifest = cb_ast::manifest::load_manifest(path, &content).map_err(|e| {
            cb_protocol::ApiError::Internal(format!("Failed to load manifest: {}", e))
        })?;

        // Update the dependency using the generic trait method
        manifest
            .rename_dependency(old_dep_name, new_dep_name, new_path)
            .map_err(|e| {
                cb_protocol::ApiError::Internal(format!("Failed to update dependency: {}", e))
            })?;

        // Write the updated content back
        let updated_content = manifest.to_string().map_err(|e| {
            cb_protocol::ApiError::Internal(format!("Failed to serialize manifest: {}", e))
        })?;

        fs::write(manifest_path, updated_content)
            .await
            .map_err(|e| {
                cb_protocol::ApiError::Internal(format!(
                    "Failed to write manifest file at {}: {}",
                    manifest_path, e
                ))
            })?;

        Ok(json!({
            "success": true,
            "message": format!(
                "Updated dependency '{}' to '{}'{} in {}",
                old_dep_name,
                new_dep_name,
                new_path.map(|p| format!(" with path '{}'", p)).unwrap_or_default(),
                manifest_path
            ),
            "file": manifest_path,
            "old_dep_name": old_dep_name,
            "new_dep_name": new_dep_name,
            "new_path": new_path,
        }))
    }
}
