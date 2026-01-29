//! Workspace package creation service
//!
//! This is a thin service that delegates to language plugins for package creation.

use mill_foundation::errors::{MillError as ServerError, MillResult as ServerResult};
use mill_plugin_api::{CreatePackageConfig, PackageType, Template};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::{debug, error};

/// Service for workspace package creation operations
pub struct WorkspaceCreateService;

impl WorkspaceCreateService {
    pub fn new() -> Self {
        Self
    }
}

// Parameter types for MCP interface

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreatePackageParams {
    pub package_path: String,
    #[serde(default = "default_lib")]
    pub package_type: PackageType,
    #[serde(default)]
    pub options: CreatePackageOptions,
}

fn default_lib() -> PackageType {
    PackageType::Library
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreatePackageOptions {
    #[serde(default = "crate::default_true")]
    pub dry_run: bool,
    #[serde(default = "crate::default_true")]
    pub add_to_workspace: bool,
    #[serde(default)]
    pub template: Template,
}

impl Default for CreatePackageOptions {
    fn default() -> Self {
        Self {
            dry_run: true, // CRITICAL: Safe default - preview mode
            add_to_workspace: true,
            template: Template::default(),
        }
    }
}

// Result type for MCP interface

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreatePackageResult {
    pub created_files: Vec<String>,
    pub workspace_updated: bool,
    pub package_info: PackageInfo,
    pub dry_run: bool,
}

#[derive(Debug, Serialize)]
pub(crate) struct PackageInfo {
    pub name: String,
    pub version: String,
    pub manifest_path: String,
}

// Handler implementation

pub async fn handle_create_package(
    context: &mill_handler_api::ToolHandlerContext,
    args: Value,
) -> ServerResult<Value> {
    debug!("Handling workspace create_package action");

    // Parse parameters
    let params: CreatePackageParams = serde_json::from_value(args)
    .map_err(|e| ServerError::invalid_request(format!("Invalid arguments: {}", e)))?;

    debug!(
        package_path = %params.package_path,
        package_type = ?params.package_type,
        dry_run = params.options.dry_run,
        "Parsed create_package parameters"
    );

    // Dry-run mode not yet supported - requires non-mutable plugin operations
    if params.options.dry_run {
        return Err(ServerError::invalid_request(
            "dry_run mode not yet supported for workspace create_package action".to_string(),
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
            ServerError::not_supported(format!(
                "No language plugin found for extension: {}",
                language_ext
            ))
        })?;

    // Get project factory capability
    let project_factory = plugin.project_factory().ok_or_else(|| {
        ServerError::not_supported(format!(
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
        workspace_root: context.app_state.project_root.to_string_lossy().to_string(),
    };

    // Delegate to plugin
    let result = project_factory.create_package(&config).map_err(|e| {
        error!(error = ?e, "Failed to create package");
        // Convert PluginApiError to ServerError
        match e {
            mill_plugin_api::PluginApiError::Parse { message, .. } => ServerError::parse(message),
            mill_plugin_api::PluginApiError::Manifest { message } => ServerError::parse(message),
            mill_plugin_api::PluginApiError::NotSupported { operation } => {
                ServerError::not_supported(operation)
            }
            mill_plugin_api::PluginApiError::InvalidInput { message } => {
                ServerError::invalid_request(message)
            }
            mill_plugin_api::PluginApiError::Internal { message } => ServerError::internal(message),
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
