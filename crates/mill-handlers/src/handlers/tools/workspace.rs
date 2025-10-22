//! Workspace operations tool handlers
//!
//! Handles: rename_directory, update_dependencies, update_dependency

use super::{ToolHandler, ToolHandlerContext};
use crate::handlers::file_operation_handler::FileOperationHandler;
use crate::handlers::refactoring_handler::RefactoringHandler;
use crate::handlers::system_handler::SystemHandler;
use async_trait::async_trait;
use mill_foundation::core::model::mcp::ToolCall;
use mill_foundation::protocol::ApiResult as ServerResult;
use serde_json::Value;

/// Controls how aggressively imports are updated during rename operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateMode {
    /// Only update top-level import/use statements (current default behavior)
    Conservative,
    /// Update all import/use statements including function-scoped ones
    Standard,
    /// Update all imports and qualified paths (e.g., module::function, module.method)
    /// ⚠️ RISKY: May update code that shouldn't be changed. Use dry_run first!
    Aggressive,
    /// Update everything including string literals
    /// ⚠️ VERY RISKY: Will update strings that may not be import paths. Always preview with dry_run!
    Full,
}

impl UpdateMode {
    /// Convert UpdateMode to mill_plugin_api::ScanScope
    pub fn to_scan_scope(self) -> mill_plugin_api::ScanScope {
        use mill_plugin_api::ScanScope;
        match self {
            UpdateMode::Conservative => ScanScope::TopLevelOnly,
            UpdateMode::Standard => ScanScope::AllUseStatements,
            UpdateMode::Aggressive => ScanScope::QualifiedPaths,
            UpdateMode::Full => ScanScope::All,
        }
    }

    /// Returns true if this mode is risky and requires user confirmation
    pub fn is_risky(self) -> bool {
        matches!(self, UpdateMode::Aggressive | UpdateMode::Full)
    }

    /// Returns a warning message for risky modes
    pub fn warning_message(self) -> Option<&'static str> {
        match self {
            UpdateMode::Aggressive => Some(
                "⚠️ Aggressive mode updates qualified paths (e.g., module::function). This may modify code that shouldn't be changed. Review changes carefully before committing."
            ),
            UpdateMode::Full => Some(
                "⚠️ Full mode updates string literals containing the module name. This is VERY RISKY and may break unrelated code. Always use dry_run=true first to preview changes!"
            ),
            _ => None,
        }
    }
}

pub struct WorkspaceToolsHandler {
    file_op_handler: FileOperationHandler,
    system_handler: SystemHandler,
    #[allow(dead_code)] // Reserved for future workspace-level refactoring operations
    refactoring_handler: RefactoringHandler,
}

impl WorkspaceToolsHandler {
    pub fn new() -> Self {
        Self {
            file_op_handler: FileOperationHandler::new(),
            system_handler: SystemHandler::new(),
            refactoring_handler: RefactoringHandler::new(),
        }
    }
}

#[async_trait]
impl ToolHandler for WorkspaceToolsHandler {
    fn tool_names(&self) -> &[&str] {
        &["move_directory", "update_dependencies", "update_dependency"]
    }

    fn is_internal(&self) -> bool {
        // These tools are internal - used by backend/workflows but not exposed to AI agents.
        // - move_directory: Replaced by move.plan with kind="consolidate"
        // - update_dependencies: Manual package.json/Cargo.toml editing preferred
        // - update_dependency: Manual manifest editing preferred
        true
    }

    async fn handle_tool_call(
        &self,
        context: &ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        // Route to appropriate handler
        let mut call = tool_call.clone();
        if call.name == "move_directory" {
            call.name = "rename_directory".to_string();
        }

        match call.name.as_str() {
            "rename_directory" => {
                // FileOperationHandler now uses the new trait, so delegate directly
                self.file_op_handler.handle_tool_call(context, &call).await
            }
            "update_dependencies" => {
                // SystemHandler now uses the new trait, so pass context directly
                self.system_handler.handle_tool_call(context, &call).await
            }
            "update_dependency" => self.handle_update_dependency(context, &call).await,
            _ => Err(mill_foundation::protocol::ApiError::InvalidRequest(
                format!("Unknown workspace tool: {}", tool_call.name),
            )),
        }
    }
}

impl WorkspaceToolsHandler {
    /// Helper to update a manifest dependency with the given parameters
    /// Returns the updated manifest content as a string
    ///
    /// This now uses the language plugin registry to dynamically route to the
    /// appropriate language plugin based on the manifest filename, and uses
    /// FileService for all file operations (respecting caching, locking, and
    /// virtual workspaces).
    async fn update_manifest_dependency(
        context: &ToolHandlerContext,
        manifest_path: &str,
        old_dep_name: &str,
        new_dep_name: &str,
        new_path: Option<&str>,
    ) -> ServerResult<String> {
        use std::path::Path;

        let path = Path::new(manifest_path);

        // Get the manifest filename (e.g., "Cargo.toml")
        let filename = path.file_name().and_then(|s| s.to_str()).ok_or_else(|| {
            mill_foundation::protocol::ApiError::InvalidRequest(format!(
                "Invalid manifest path: {}",
                manifest_path
            ))
        })?;

        // Find the appropriate language plugin for this manifest
        let plugin = context
            .app_state
            .language_plugins
            .get_plugin_for_manifest(filename)
            .ok_or_else(|| {
                mill_foundation::protocol::ApiError::Unsupported(format!(
                    "No language plugin found for manifest file: {}",
                    filename
                ))
            })?;

        // Use the plugin to update the dependency
        // Note: The plugin will read the file directly for now. In the future, we could
        // refactor the plugin API to accept content instead of paths, which would allow
        // us to use FileService for reading and benefit from caching/locking.

        // Use manifest updater capability - no downcasting or cfg guards needed!
        let manifest_updater = plugin.manifest_updater().ok_or_else(|| {
            mill_foundation::protocol::ApiError::Unsupported(format!(
                "Plugin '{}' does not support manifest updates",
                plugin.metadata().name
            ))
        })?;

        let updated_content = manifest_updater
            .update_dependency(path, old_dep_name, new_dep_name, new_path)
            .await
            .map_err(|e| {
                mill_foundation::protocol::ApiError::Internal(format!(
                    "Failed to update dependency: {}",
                    e
                ))
            })?;

        context
            .app_state
            .file_service
            .write_file(path, &updated_content, false)
            .await
            .map_err(|e| {
                mill_foundation::protocol::ApiError::Internal(format!(
                    "Failed to write manifest file at {}: {}",
                    manifest_path, e
                ))
            })?;

        Ok(updated_content)
    }

    /// Handle update_dependency tool call
    /// Updates a dependency in any supported manifest file (Cargo.toml, package.json, etc.)
    /// This is language-agnostic and works across all supported package managers.
    async fn handle_update_dependency(
        &self,
        context: &ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        use serde_json::json;

        // Parse arguments
        let args = tool_call
            .arguments
            .as_ref()
            .and_then(|v| v.as_object())
            .ok_or_else(|| {
                mill_foundation::protocol::ApiError::InvalidRequest(
                    "Arguments must be an object".to_string(),
                )
            })?;

        let manifest_path = args
            .get("manifest_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                mill_foundation::protocol::ApiError::InvalidRequest(
                    "Missing required parameter: manifest_path".to_string(),
                )
            })?;

        let old_dep_name = args
            .get("old_dep_name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                mill_foundation::protocol::ApiError::InvalidRequest(
                    "Missing required parameter: old_dep_name".to_string(),
                )
            })?;

        let new_dep_name = args
            .get("new_dep_name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                mill_foundation::protocol::ApiError::InvalidRequest(
                    "Missing required parameter: new_dep_name".to_string(),
                )
            })?;

        // new_path is optional - if not provided, only rename the dependency
        let new_path = args.get("new_path").and_then(|v| v.as_str());

        // Use the helper to perform the update
        Self::update_manifest_dependency(
            context,
            manifest_path,
            old_dep_name,
            new_dep_name,
            new_path,
        )
        .await?;

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