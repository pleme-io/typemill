//! Workspace package creation tool handler
//!
//! Handles: workspace.create_package
//!
//! This is a thin handler that delegates to language plugins for package creation.

use super::{ToolHandler, ToolHandlerContext};
use async_trait::async_trait;
use codebuddy_core::model::mcp::ToolCall;
use cb_plugin_api::{CreatePackageConfig, PackageType, Template};
use codebuddy_foundation::protocol::{ ApiError , ApiResult as ServerResult };
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::{debug, error};

/// Handler for workspace package creation operations
pub struct WorkspaceCreateHandler;

impl WorkspaceCreateHandler {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ToolHandler for WorkspaceCreateHandler {
    fn tool_names(&self) -> &[&str] {
        &["workspace.create_package"]
    }

    async fn handle_tool_call(
        &self,
        context: &ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        match tool_call.name.as_str() {
            "workspace.create_package" => handle_create_package(context, tool_call).await,
            _ => Err(ApiError::InvalidRequest(format!(
                "Unknown workspace create tool: {}",
                tool_call.name
            ))),
        }
    }
}

// Parameter types for MCP interface

#[derive(Debug, Deserialize)]
pub struct CreatePackageParams {
    pub package_path: String,
    #[serde(default = "default_lib")]
    pub package_type: PackageType,
    #[serde(default)]
    pub options: CreatePackageOptions,
}

fn default_lib() -> PackageType {
    PackageType::Library
}

#[derive(Debug, Deserialize, Default)]
pub struct CreatePackageOptions {
    #[serde(default)]
    pub dry_run: bool,
    #[serde(default = "default_true")]
    pub add_to_workspace: bool,
    #[serde(default)]
    pub template: Template,
}

fn default_true() -> bool {
    true
}

// Result type for MCP interface

#[derive(Debug, Serialize)]
pub struct CreatePackageResult {
    pub created_files: Vec<String>,
    pub workspace_updated: bool,
    pub package_info: PackageInfo,
    pub dry_run: bool,
}

#[derive(Debug, Serialize)]
pub struct PackageInfo {
    pub name: String,
    pub version: String,
    pub manifest_path: String,
}

// Handler implementation

async fn handle_create_package(
    context: &ToolHandlerContext,
    tool_call: &ToolCall,
) -> ServerResult<Value> {
    debug!("Handling workspace.create_package");

    // Parse parameters
    let params: CreatePackageParams = serde_json::from_value(
        tool_call
            .arguments
            .as_ref()
            .ok_or_else(|| ApiError::InvalidRequest("Missing arguments".to_string()))?
            .clone(),
    )
    .map_err(|e| ApiError::InvalidRequest(format!("Invalid arguments: {}", e)))?;

    debug!(
        package_path = %params.package_path,
        package_type = ?params.package_type,
        dry_run = params.options.dry_run,
        "Parsed create_package parameters"
    );

    // Dry-run mode not yet supported - requires non-mutable plugin operations
    if params.options.dry_run {
        return Err(ApiError::InvalidRequest(
            "dry_run mode not yet supported for workspace.create_package".to_string(),
        ));
    }

    // Detect language from package path extension or default to Rust
    // For now, we only support Rust (manifest: Cargo.toml)
    let language_ext = "rs"; // Default to Rust

    // Get language plugin
    let plugin = context
        .app_state
        .language_plugins
        .get_plugin(language_ext)
        .ok_or_else(|| {
            ApiError::Unsupported(format!(
                "No language plugin found for extension: {}",
                language_ext
            ))
        })?;

    // Get project factory capability
    let project_factory = plugin.project_factory().ok_or_else(|| {
        ApiError::Unsupported(format!(
            "{} language plugin does not support package creation",
            plugin.metadata().name
        ))
    })?;

    // Build plugin configuration
    let config = CreatePackageConfig {
        package_path: params.package_path.clone(),
        package_type: params.package_type,
        template: params.options.template,
        add_to_workspace: params.options.add_to_workspace,
        workspace_root: context
            .app_state
            .project_root
            .to_string_lossy()
            .to_string(),
    };

    // Delegate to plugin
    let result = project_factory.create_package(&config).map_err(|e| {
        error!(error = ?e, "Failed to create package");
        // Convert PluginError to ApiError
        match e {
            cb_plugin_api::PluginError::Parse { message, .. } => ApiError::Parse { message },
            cb_plugin_api::PluginError::Manifest { message } => ApiError::Parse { message },
            cb_plugin_api::PluginError::NotSupported { operation } => {
                ApiError::Unsupported(operation)
            }
            cb_plugin_api::PluginError::InvalidInput { message } => {
                ApiError::InvalidRequest(message)
            }
            cb_plugin_api::PluginError::Internal { message } => ApiError::Internal(message),
        }
    })?;

    // Convert plugin result to MCP result
    let mcp_result = CreatePackageResult {
        created_files: result.created_files,
        workspace_updated: result.workspace_updated,
        package_info: PackageInfo {
            name: result.package_info.name,
            version: result.package_info.version,
            manifest_path: result.package_info.manifest_path,
        },
        dry_run: params.options.dry_run,
    };

    Ok(serde_json::to_value(mcp_result).unwrap())
}