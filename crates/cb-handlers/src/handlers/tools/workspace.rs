//! Workspace operations tool handlers
//!
//! Handles: rename_directory, analyze_imports, find_dead_code, update_dependencies, extract_module_to_package

use super::{ToolHandler, ToolHandlerContext};
use crate::handlers::compat::{ToolContext, ToolHandler as LegacyToolHandler};
use crate::handlers::file_operation_handler::FileOperationHandler as LegacyFileHandler;
use crate::handlers::refactoring_handler::RefactoringHandler as LegacyRefactoringHandler;
use crate::handlers::system_handler::SystemHandler as LegacySystemHandler;
use async_trait::async_trait;
use cb_core::model::mcp::ToolCall;
use cb_protocol::ApiResult as ServerResult;
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
            "batch_update_dependencies",
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
            "batch_update_dependencies" => self.handle_batch_update_dependencies(tool_call).await,
            _ => Err(cb_protocol::ApiError::InvalidRequest(format!(
                "Unknown workspace tool: {}",
                tool_call.name
            ))),
        }
    }
}

impl WorkspaceHandler {
    /// Discover all manifest files in the workspace (Cargo.toml, package.json, etc.)
    async fn discover_manifests(&self, root_path: &str) -> ServerResult<Vec<String>> {
        use std::path::PathBuf;
        use tokio::fs;

        let root = PathBuf::from(root_path);
        let mut manifests = Vec::new();

        // Walk the directory tree to find manifest files
        let mut dirs = vec![root.clone()];

        while let Some(dir) = dirs.pop() {
            let mut entries = fs::read_dir(&dir).await.map_err(|e| {
                cb_protocol::ApiError::Internal(format!(
                    "Failed to read directory {}: {}",
                    dir.display(),
                    e
                ))
            })?;

            while let Some(entry) = entries.next_entry().await.map_err(|e| {
                cb_protocol::ApiError::Internal(format!("Failed to read directory entry: {}", e))
            })? {
                let path = entry.path();

                if path.is_dir() {
                    // Skip common directories that don't contain manifests
                    let dir_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    if !matches!(
                        dir_name,
                        "target" | "node_modules" | ".git" | "dist" | "build"
                    ) {
                        dirs.push(path);
                    }
                } else if path.is_file() {
                    let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    if matches!(file_name, "Cargo.toml" | "package.json") {
                        if let Some(path_str) = path.to_str() {
                            manifests.push(path_str.to_string());
                        }
                    }
                }
            }
        }

        Ok(manifests)
    }

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
        let content = fs::read_to_string(manifest_path).await.map_err(|e| {
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

    /// Handle batch_update_dependencies tool call
    /// Updates multiple dependencies across multiple manifest files in a single operation.
    async fn handle_batch_update_dependencies(&self, tool_call: &ToolCall) -> ServerResult<Value> {
        use serde_json::json;

        // Parse arguments
        let args = tool_call
            .arguments
            .as_ref()
            .and_then(|v| v.as_object())
            .ok_or_else(|| {
                cb_protocol::ApiError::InvalidRequest("Arguments must be an object".to_string())
            })?;

        // Parse updates array
        let updates = args
            .get("updates")
            .and_then(|v| v.as_array())
            .ok_or_else(|| {
                cb_protocol::ApiError::InvalidRequest(
                    "Missing required parameter: updates (array)".to_string(),
                )
            })?;

        // Parse optional manifest_paths or discover them
        let manifest_paths =
            if let Some(paths) = args.get("manifest_paths").and_then(|v| v.as_array()) {
                paths
                    .iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect::<Vec<_>>()
            } else {
                // Auto-discover manifests in current workspace
                self.discover_manifests(".").await?
            };

        let mut results = Vec::new();
        let mut success_count = 0;
        let mut failure_count = 0;

        // Process each manifest
        for manifest_path in &manifest_paths {
            // Process each update for this manifest
            for update in updates {
                let update_obj = update.as_object().ok_or_else(|| {
                    cb_protocol::ApiError::InvalidRequest(
                        "Each update must be an object".to_string(),
                    )
                })?;

                let old_dep_name = update_obj
                    .get("old_dep_name")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        cb_protocol::ApiError::InvalidRequest(
                            "Each update must have 'old_dep_name'".to_string(),
                        )
                    })?;

                let new_dep_name = update_obj
                    .get("new_dep_name")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        cb_protocol::ApiError::InvalidRequest(
                            "Each update must have 'new_dep_name'".to_string(),
                        )
                    })?;

                let new_path = update_obj.get("new_path").and_then(|v| v.as_str());

                // Try to update this manifest
                match self
                    .update_single_dependency(manifest_path, old_dep_name, new_dep_name, new_path)
                    .await
                {
                    Ok(_) => {
                        success_count += 1;
                        results.push(json!({
                            "file": manifest_path,
                            "old_dep_name": old_dep_name,
                            "new_dep_name": new_dep_name,
                            "status": "updated"
                        }));
                    }
                    Err(e) => {
                        failure_count += 1;
                        results.push(json!({
                            "file": manifest_path,
                            "old_dep_name": old_dep_name,
                            "new_dep_name": new_dep_name,
                            "status": "failed",
                            "error": e.to_string()
                        }));
                    }
                }
            }
        }

        Ok(json!({
            "success": failure_count == 0,
            "updated": success_count,
            "failed": failure_count,
            "total_manifests": manifest_paths.len(),
            "total_operations": results.len(),
            "details": results
        }))
    }

    /// Helper function to update a single dependency in a manifest
    async fn update_single_dependency(
        &self,
        manifest_path: &str,
        old_dep_name: &str,
        new_dep_name: &str,
        new_path: Option<&str>,
    ) -> ServerResult<()> {
        use std::path::Path;
        use tokio::fs;

        // Read the manifest file
        let content = fs::read_to_string(manifest_path).await.map_err(|e| {
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

        Ok(())
    }
}
