//! Project/Package creation support
//!
//! This module defines the trait for creating new projects/packages in a workspace.
//! Language plugins can implement this to provide language-specific initialization.

use crate::PluginResult;
use serde::{Deserialize, Serialize};

/// Project factory capability for language plugins
///
/// This trait allows language plugins to create new packages/crates/projects
/// in their respective ecosystems (e.g., `cargo init` for Rust, `npm init` for TypeScript).
pub trait ProjectFactory: Send + Sync {
    /// Create a new package/crate in the workspace
    ///
    /// # Arguments
    ///
    /// * `config` - Configuration for the new package
    ///
    /// # Returns
    ///
    /// Result containing details of what was created
    fn create_package(&self, config: &CreatePackageConfig) -> PluginResult<CreatePackageResult>;
}

/// Configuration for creating a new package
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePackageConfig {
    /// Path where the package should be created (relative to workspace root)
    pub package_path: String,

    /// Type of package to create (library, binary, etc.)
    pub package_type: PackageType,

    /// Template to use (minimal, full, etc.)
    pub template: Template,

    /// Whether to add to workspace automatically
    pub add_to_workspace: bool,

    /// Workspace root path (for calculating relative paths)
    pub workspace_root: String,
}

/// Type of package to create
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PackageType {
    /// Library package (e.g., lib crate in Rust, library in TypeScript)
    Library,
    /// Binary/executable package
    Binary,
}

/// Template for package scaffolding
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Template {
    /// Minimal scaffolding (manifest + entry point)
    Minimal,
    /// Full scaffolding (includes tests, examples, README, etc.)
    Full,
}

impl Default for Template {
    fn default() -> Self {
        Template::Minimal
    }
}

/// Result of package creation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePackageResult {
    /// List of files that were created
    pub created_files: Vec<String>,

    /// Whether workspace was updated
    pub workspace_updated: bool,

    /// Package manifest information
    pub package_info: PackageInfo,
}

/// Information about the created package
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageInfo {
    /// Package name (derived from path)
    pub name: String,

    /// Initial version
    pub version: String,

    /// Path to package manifest (Cargo.toml, package.json, etc.)
    pub manifest_path: String,
}
