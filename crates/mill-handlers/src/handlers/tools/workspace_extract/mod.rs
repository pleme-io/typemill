//! Workspace dependency extraction service
//!
//! This module provides a trait-based abstraction for extracting dependencies
//! from one manifest file and adding them to another. Supports both Cargo.toml
//! (Rust) and package.json (TypeScript/JavaScript) manifest formats.
//!
//! # Architecture
//!
//! - `ManifestOps` trait: Defines common operations for manifest parsing and manipulation
//! - `CargoManifest`: Implementation for Cargo.toml files
//! - `PackageJsonManifest`: Implementation for package.json files
//! - Generic `extract_dependencies_generic`: Shared extraction logic

use mill_foundation::errors::{MillError as ServerError, MillResult as ServerResult};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;
use tracing::{debug, error, warn};

mod cargo_manifest;
mod package_json;

pub use cargo_manifest::CargoManifest;
pub use package_json::PackageJsonManifest;

/// Service for workspace dependency extraction operations
pub struct WorkspaceExtractService;

impl WorkspaceExtractService {
    pub fn new() -> Self {
        Self
    }
}

impl Default for WorkspaceExtractService {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Shared Types
// ============================================================================

/// Parameters for extract_dependencies action
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExtractDependenciesParams {
    pub source_manifest: String,
    pub target_manifest: String,
    pub dependencies: Vec<String>,
    #[serde(default)]
    pub options: ExtractDependenciesOptions,
}

/// Options for dependency extraction
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExtractDependenciesOptions {
    #[serde(default = "crate::default_true")]
    pub dry_run: bool,
    #[serde(default = "crate::default_true")]
    #[allow(dead_code)]
    pub preserve_versions: bool,
    #[serde(default = "crate::default_true")]
    #[allow(dead_code)]
    pub preserve_features: bool,
    #[serde(default)]
    pub section: String,
}

impl Default for ExtractDependenciesOptions {
    fn default() -> Self {
        Self {
            dry_run: true, // CRITICAL: Safe default - preview mode
            preserve_versions: true,
            preserve_features: true,
            section: String::new(), // Empty means use default for manifest type
        }
    }
}

/// Result of dependency extraction
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExtractDependenciesResult {
    pub dependencies_extracted: usize,
    pub dependencies_added: Vec<DependencyInfo>,
    pub target_manifest_updated: bool,
    pub dry_run: bool,
    pub warnings: Vec<String>,
}

/// Information about a single dependency
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DependencyInfo {
    pub name: String,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub features: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub optional: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub already_exists: Option<bool>,
}

/// Internal extraction result before writing
pub(crate) struct ExtractionResult {
    pub dependencies_added: Vec<DependencyInfo>,
    pub warnings: Vec<String>,
    pub updated_content: Option<String>,
}

// ============================================================================
// Manifest Operations Trait
// ============================================================================

/// Trait for manifest file operations
///
/// This trait abstracts over different manifest formats (Cargo.toml, package.json)
/// to provide a unified interface for dependency extraction.
pub(crate) trait ManifestOps: Sized {
    /// Parse manifest content into this type
    fn parse(content: &str) -> ServerResult<Self>;

    /// Get the list of valid dependency sections for this manifest type
    fn sections() -> &'static [&'static str];

    /// Get the default section name for adding dependencies
    fn default_section() -> &'static str;

    /// Find a dependency by name, returning (section_name, dependency_info)
    fn find_dependency(&self, name: &str) -> Option<(&'static str, DependencyInfo)>;

    /// Check if a dependency exists in a specific section
    fn has_dependency(&self, section: &str, name: &str) -> bool;

    /// Add a dependency to a specific section
    fn add_dependency(
        &mut self,
        section: &str,
        name: &str,
        info: &DependencyInfo,
    ) -> ServerResult<()>;

    /// Serialize the manifest back to string
    fn serialize(&self) -> String;
}

// ============================================================================
// Generic Extraction Logic
// ============================================================================

/// Extract dependencies using the trait-based abstraction
pub(crate) fn extract_dependencies_generic<M: ManifestOps>(
    source_content: &str,
    target_content: &str,
    dep_names: &[String],
    options: &ExtractDependenciesOptions,
) -> ServerResult<ExtractionResult> {
    debug!("Extracting dependencies from source manifest");

    // Parse manifests
    let source = M::parse(source_content)?;
    let mut target = M::parse(target_content)?;

    let mut dependencies_added = Vec::new();
    let mut warnings = Vec::new();

    // Determine target section
    let target_section = if options.section.is_empty() {
        M::default_section()
    } else {
        // Validate section name
        if !M::sections().contains(&options.section.as_str()) {
            return Err(ServerError::invalid_request(format!(
                "Invalid section '{}'. Valid sections: {:?}",
                options.section,
                M::sections()
            )));
        }
        options.section.as_str()
    };

    // Process each requested dependency
    for dep_name in dep_names {
        debug!(dependency = %dep_name, "Processing dependency");

        // Look for dependency in source manifest
        let dep_spec = source.find_dependency(dep_name);

        if dep_spec.is_none() {
            let warning = format!("Dependency '{}' not found in source manifest", dep_name);
            warn!("{}", warning);
            warnings.push(warning);
            continue;
        }

        let (source_section, mut dep_info) = dep_spec.unwrap();
        debug!(dependency = %dep_name, section = %source_section, "Found dependency in source");

        // Check if already exists in target
        let already_exists = target.has_dependency(target_section, dep_name);

        if already_exists {
            let warning = format!(
                "Dependency '{}' already exists in target, skipped",
                dep_name
            );
            warn!("{}", warning);
            warnings.push(warning);

            // Still add to result with already_exists flag
            dep_info.already_exists = Some(true);
            dependencies_added.push(dep_info);
            continue;
        }

        // Add dependency to target
        target.add_dependency(target_section, dep_name, &dep_info)?;

        // Add to result
        dependencies_added.push(dep_info);
        debug!(dependency = %dep_name, "Added dependency to target");
    }

    // Generate updated content
    let updated_content = if dependencies_added
        .iter()
        .any(|d| d.already_exists != Some(true))
    {
        Some(target.serialize())
    } else {
        None
    };

    Ok(ExtractionResult {
        dependencies_added,
        warnings,
        updated_content,
    })
}

// ============================================================================
// Manifest Type Detection
// ============================================================================

/// Detect manifest type from file path
fn detect_manifest_type(path: &str) -> ManifestType {
    let path_lower = path.to_lowercase();
    if path_lower.ends_with("cargo.toml") {
        ManifestType::Cargo
    } else if path_lower.ends_with("package.json") {
        ManifestType::PackageJson
    } else {
        ManifestType::Unknown
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ManifestType {
    Cargo,
    PackageJson,
    Unknown,
}

// ============================================================================
// MCP Handler
// ============================================================================

/// Handle extract_dependencies tool call
///
/// Auto-detects manifest type and dispatches to appropriate implementation.
pub async fn handle_extract_dependencies(
    context: &mill_handler_api::ToolHandlerContext,
    args: Value,
) -> ServerResult<Value> {
    debug!("Handling workspace extract_dependencies action");

    // Parse parameters
    let params: ExtractDependenciesParams = serde_json::from_value(args)
        .map_err(|e| ServerError::invalid_request(format!("Invalid arguments: {}", e)))?;

    debug!(
        source_manifest = %params.source_manifest,
        target_manifest = %params.target_manifest,
        dependencies_count = params.dependencies.len(),
        dry_run = params.options.dry_run,
        "Parsed extract_dependencies parameters"
    );

    // Resolve paths relative to workspace root
    let workspace_root = &context.app_state.project_root;
    let source_path = resolve_path(workspace_root, &params.source_manifest)?;
    let target_path = resolve_path(workspace_root, &params.target_manifest)?;

    // Validate files exist
    if !tokio::fs::try_exists(&source_path)
        .await
        .map_err(|e| ServerError::internal(format!("Failed to check path existence: {}", e)))?
    {
        return Err(ServerError::invalid_request(format!(
            "Source manifest not found: {}",
            source_path.display()
        )));
    }

    if !tokio::fs::try_exists(&target_path)
        .await
        .map_err(|e| ServerError::internal(format!("Failed to check path existence: {}", e)))?
    {
        return Err(ServerError::invalid_request(format!(
            "Target manifest not found: {}",
            target_path.display()
        )));
    }

    // Read manifests
    let source_content = tokio::fs::read_to_string(&source_path).await.map_err(|e| {
        error!(error = %e, source_path = %source_path.display(), "Failed to read source manifest");
        ServerError::internal(format!("Failed to read source manifest: {}", e))
    })?;

    let target_content = tokio::fs::read_to_string(&target_path).await.map_err(|e| {
        error!(error = %e, target_path = %target_path.display(), "Failed to read target manifest");
        ServerError::internal(format!("Failed to read target manifest: {}", e))
    })?;

    // Detect manifest type from source (should match target)
    let source_type = detect_manifest_type(&params.source_manifest);
    let target_type = detect_manifest_type(&params.target_manifest);

    if source_type != target_type {
        return Err(ServerError::invalid_request(format!(
            "Source and target manifests must be the same type. Source: {:?}, Target: {:?}",
            source_type, target_type
        )));
    }

    // Extract dependencies using appropriate implementation
    let extraction_result = match source_type {
        ManifestType::Cargo => extract_dependencies_generic::<CargoManifest>(
            &source_content,
            &target_content,
            &params.dependencies,
            &params.options,
        )?,
        ManifestType::PackageJson => extract_dependencies_generic::<PackageJsonManifest>(
            &source_content,
            &target_content,
            &params.dependencies,
            &params.options,
        )?,
        ManifestType::Unknown => {
            return Err(ServerError::invalid_request(format!(
                "Unknown manifest type. Supported: Cargo.toml, package.json. Got: {}",
                params.source_manifest
            )));
        }
    };

    // Write updated target manifest if not dry-run
    let target_updated = if let (false, Some(updated)) =
        (params.options.dry_run, &extraction_result.updated_content)
    {
        tokio::fs::write(&target_path, updated).await.map_err(|e| {
            error!(error = %e, target_path = %target_path.display(), "Failed to write target manifest");
            ServerError::internal(format!("Failed to write target manifest: {}", e))
        })?;
        debug!(target_path = %target_path.display(), "Wrote updated target manifest");
        true
    } else {
        false
    };

    // Build result
    let result = ExtractDependenciesResult {
        dependencies_extracted: extraction_result.dependencies_added.len(),
        dependencies_added: extraction_result.dependencies_added,
        target_manifest_updated: target_updated,
        dry_run: params.options.dry_run,
        warnings: extraction_result.warnings,
    };

    Ok(serde_json::to_value(result).unwrap())
}

// ============================================================================
// Helpers
// ============================================================================

fn resolve_path(workspace_root: &Path, path: &str) -> ServerResult<std::path::PathBuf> {
    let resolved = if Path::new(path).is_absolute() {
        Path::new(path).to_path_buf()
    } else {
        workspace_root.join(path)
    };

    Ok(resolved)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_manifest_type() {
        assert_eq!(detect_manifest_type("Cargo.toml"), ManifestType::Cargo);
        assert_eq!(
            detect_manifest_type("crates/foo/Cargo.toml"),
            ManifestType::Cargo
        );
        assert_eq!(
            detect_manifest_type("package.json"),
            ManifestType::PackageJson
        );
        assert_eq!(
            detect_manifest_type("packages/ui/package.json"),
            ManifestType::PackageJson
        );
        assert_eq!(detect_manifest_type("unknown.txt"), ManifestType::Unknown);
    }

    #[test]
    fn test_options_default() {
        let options = ExtractDependenciesOptions::default();
        assert!(options.dry_run);
        assert!(options.preserve_versions);
        assert!(options.preserve_features);
        assert!(options.section.is_empty());
    }
}
