//! Dependency file handlers for different languages

pub mod nodejs;
pub mod rust_lang;
pub mod python;
pub mod golang;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;
use anyhow::Result;

/// Supported dependency file types
#[derive(Debug, Clone, PartialEq)]
pub enum DependencyFileType {
    PackageJson,     // package.json (Node.js)
    Requirements,    // requirements.txt (Python)
    PyProject,       // pyproject.toml (Python)
    GoMod,          // go.mod (Go)
    CargoToml,      // Cargo.toml (Rust)
}

/// Arguments for update_dependencies tool
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UpdateDependenciesArgs {
    pub file_path: String,
    pub add_dependencies: Option<serde_json::Map<String, Value>>,
    pub add_dev_dependencies: Option<serde_json::Map<String, Value>>,
    pub remove_dependencies: Option<Vec<String>>,

    // Language-specific options
    pub add_scripts: Option<serde_json::Map<String, Value>>, // Node.js
    pub remove_scripts: Option<Vec<String>>, // Node.js
    pub update_version: Option<String>, // Universal
    pub workspace_config: Option<WorkspaceConfig>, // Node.js

    pub dry_run: Option<bool>,
}

/// Node.js workspace configuration
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct WorkspaceConfig {
    pub workspaces: Option<Vec<String>>,
}

/// Result of dependency update operation
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DependencyUpdateResult {
    pub success: bool,
    pub file_type: String,
    pub file_path: String,
    pub changes_made: Vec<String>,
    pub dry_run: bool,
    pub error: Option<String>,
}

/// Detect dependency file type from file path
pub fn detect_dependency_file_type(file_path: &str) -> Result<DependencyFileType> {
    let path = Path::new(file_path);

    match path.file_name().and_then(|n| n.to_str()) {
        Some("package.json") => Ok(DependencyFileType::PackageJson),
        Some("requirements.txt") => Ok(DependencyFileType::Requirements),
        Some("pyproject.toml") => Ok(DependencyFileType::PyProject),
        Some("go.mod") => Ok(DependencyFileType::GoMod),
        Some("Cargo.toml") => Ok(DependencyFileType::CargoToml),
        _ => Err(anyhow::anyhow!("Unsupported dependency file: {}", file_path))
    }
}

/// Main handler for update_dependencies tool
pub async fn handle_update_dependencies(args: UpdateDependenciesArgs) -> Result<DependencyUpdateResult> {
    let file_type = detect_dependency_file_type(&args.file_path)?;

    match file_type {
        DependencyFileType::PackageJson => nodejs::handle_package_json_update(args).await,
        DependencyFileType::CargoToml => rust_lang::handle_cargo_toml_update(args).await,
        DependencyFileType::Requirements => python::handle_requirements_update(args).await,
        DependencyFileType::PyProject => python::handle_pyproject_update(args).await,
        DependencyFileType::GoMod => golang::handle_go_mod_update(args).await,
    }
}