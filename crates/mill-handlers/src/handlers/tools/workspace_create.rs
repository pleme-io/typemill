//! Workspace package creation service
//!
//! This is a thin service that delegates to language plugins for package creation.

use mill_foundation::errors::{MillError as ServerError, MillResult as ServerResult};
use mill_plugin_api::{CreatePackageConfig, PackageType, Template};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;
use tracing::{debug, error, info};

/// Service for workspace package creation operations
pub struct WorkspaceCreateService;

impl WorkspaceCreateService {
    pub fn new() -> Self {
        Self
    }
}

// Parameter types for MCP interface

/// Language/package manager type (npm, cargo, etc.)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LanguageType {
    /// TypeScript/JavaScript (npm package.json)
    Npm,
    /// Rust (Cargo.toml)
    #[default]
    Cargo,
    /// Python (pyproject.toml)
    Python,
}

impl LanguageType {
    /// Get the file extension for this language type
    pub fn extension(&self) -> &'static str {
        match self {
            LanguageType::Npm => "ts",
            LanguageType::Cargo => "rs",
            LanguageType::Python => "py",
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreatePackageParams {
    pub package_path: String,
    #[serde(default = "default_lib")]
    pub package_type: PackageType,
    /// Language/package manager type (npm, cargo, python)
    #[serde(default, alias = "type")]
    pub language: LanguageType,
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
#[serde(rename_all = "camelCase")]
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
    info!("Handling workspace create_package action");
    info!(args = %serde_json::to_string_pretty(&args).unwrap_or_default(), "Received args");

    // Parse parameters
    let params: CreatePackageParams = serde_json::from_value(args)
        .map_err(|e| ServerError::invalid_request(format!("Invalid arguments: {}", e)))?;

    info!(
        package_path = %params.package_path,
        package_type = ?params.package_type,
        language = ?params.language,
        dry_run = params.options.dry_run,
        "Parsed create_package parameters"
    );

    let workspace_root = &context.app_state.project_root;

    // Handle dry-run mode - preview what would be created
    if params.options.dry_run {
        return preview_create_package(workspace_root, &params);
    }

    // Get language extension based on the specified language type
    let language_ext = params.language.extension();

    info!(language_ext = %language_ext, "Using language plugin");

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

/// Preview what files would be created for a package without actually creating them.
/// This implements dry-run mode for create_package.
fn preview_create_package(
    workspace_root: &Path,
    params: &CreatePackageParams,
) -> ServerResult<Value> {
    debug!(
        package_path = %params.package_path,
        language = ?params.language,
        package_type = ?params.package_type,
        template = ?params.options.template,
        "Previewing package creation"
    );

    // Resolve package path (relative to workspace or absolute)
    let package_path = if Path::new(&params.package_path).is_absolute() {
        std::path::PathBuf::from(&params.package_path)
    } else {
        workspace_root.join(&params.package_path)
    };

    // Validate package path doesn't already exist
    if package_path.exists() {
        return Err(ServerError::invalid_request(format!(
            "Package path already exists: {}",
            package_path.display()
        )));
    }

    // Derive package name from the path (last component)
    let package_name = package_path
        .file_name()
        .and_then(|n| n.to_str())
        .map(|n| n.replace('-', "_")) // Normalize for Rust/Python conventions
        .ok_or_else(|| {
            ServerError::invalid_request(format!(
                "Cannot derive package name from path: {}",
                params.package_path
            ))
        })?;

    // Calculate what files would be created based on language type
    let (created_files, manifest_path) = predict_created_files(
        &package_path,
        &package_name,
        params.language,
        params.package_type,
        params.options.template,
    );

    let mcp_result = CreatePackageResult {
        created_files,
        workspace_updated: params.options.add_to_workspace,
        package_info: PackageInfo {
            name: package_name,
            version: "0.1.0".to_string(),
            manifest_path,
        },
        dry_run: true,
    };

    Ok(serde_json::to_value(mcp_result).unwrap())
}

/// Predict what files would be created for a given language, package type, and template.
fn predict_created_files(
    package_path: &Path,
    package_name: &str,
    language: LanguageType,
    package_type: PackageType,
    template: Template,
) -> (Vec<String>, String) {
    let mut files = Vec::new();
    let manifest_path: String;

    match language {
        LanguageType::Cargo => {
            // Cargo.toml
            let cargo_toml = package_path.join("Cargo.toml");
            manifest_path = cargo_toml.display().to_string();
            files.push(manifest_path.clone());

            // Entry file
            match package_type {
                PackageType::Library => {
                    files.push(package_path.join("src/lib.rs").display().to_string());
                }
                PackageType::Binary => {
                    files.push(package_path.join("src/main.rs").display().to_string());
                }
            }

            // Full template extras
            if matches!(template, Template::Full) {
                files.push(package_path.join("README.md").display().to_string());
                files.push(
                    package_path
                        .join("tests/integration_test.rs")
                        .display()
                        .to_string(),
                );
                files.push(package_path.join("examples/basic.rs").display().to_string());
            }
        }
        LanguageType::Npm => {
            // package.json
            let package_json = package_path.join("package.json");
            manifest_path = package_json.display().to_string();
            files.push(manifest_path.clone());

            // tsconfig.json
            files.push(package_path.join("tsconfig.json").display().to_string());

            // Entry file
            match package_type {
                PackageType::Library => {
                    files.push(package_path.join("src/index.ts").display().to_string());
                }
                PackageType::Binary => {
                    files.push(package_path.join("src/main.ts").display().to_string());
                }
            }

            // Baseline files (always included for npm)
            files.push(package_path.join("README.md").display().to_string());
            files.push(package_path.join(".gitignore").display().to_string());
            files.push(
                package_path
                    .join("tests/index.test.ts")
                    .display()
                    .to_string(),
            );

            // Full template extras
            if matches!(template, Template::Full) {
                files.push(package_path.join(".eslintrc.json").display().to_string());
            }
        }
        LanguageType::Python => {
            // pyproject.toml
            let pyproject = package_path.join("pyproject.toml");
            manifest_path = pyproject.display().to_string();
            files.push(manifest_path.clone());

            // Entry file (in src/<package_name>/)
            match package_type {
                PackageType::Library => {
                    files.push(
                        package_path
                            .join(format!("src/{}/__init__.py", package_name))
                            .display()
                            .to_string(),
                    );
                }
                PackageType::Binary => {
                    files.push(
                        package_path
                            .join(format!("src/{}/main.py", package_name))
                            .display()
                            .to_string(),
                    );
                }
            }

            // Baseline files (always included for Python)
            files.push(package_path.join("README.md").display().to_string());
            files.push(package_path.join(".gitignore").display().to_string());
            files.push(
                package_path
                    .join("tests/test_basic.py")
                    .display()
                    .to_string(),
            );

            // Full template extras
            if matches!(template, Template::Full) {
                files.push(package_path.join("setup.py").display().to_string());
            }
        }
    }

    (files, manifest_path)
}
