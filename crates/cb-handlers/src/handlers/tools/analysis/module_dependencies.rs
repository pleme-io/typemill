#![allow(dead_code, unused_variables)]

//! Module dependencies analysis handler
//!
//! This module provides analysis of Rust module dependencies to support
//! crate extraction workflows (Proposal 50).
//!
//! ## Purpose
//! Analyzes which Cargo dependencies a module or file needs by parsing imports
//! and cross-referencing with workspace manifest.
//!
//! ## Capabilities
//! - Parse use statements from single files or entire directories
//! - Classify dependencies as external (crates.io) or workspace (internal crates)
//! - Detect standard library usage
//! - Resolve dependency versions from workspace manifest
//! - Identify unresolved imports
//!
//! ## Use Cases
//! - Determine dependencies needed when extracting module to standalone crate
//! - Audit module coupling and dependency usage
//! - Detect missing or unused dependencies

use super::super::{ToolHandler, ToolHandlerContext};
use async_trait::async_trait;
use codebuddy_foundation::core::model::mcp::ToolCall;
use codebuddy_foundation::protocol::{ ApiError as ServerError , ApiResult as ServerResult };
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use tracing::{debug, error, info};

/// Parameters for module dependency analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleDependenciesParams {
    /// Target to analyze (file or directory)
    pub target: TargetSpec,

    /// Optional configuration
    #[serde(default)]
    pub options: ModuleDependenciesOptions,
}

/// Target specification for analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetSpec {
    /// Kind of target (file or directory)
    pub kind: TargetKind,

    /// Path to the target
    pub path: String,
}

/// Target kind enumeration
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TargetKind {
    /// Single file
    File,

    /// Directory (recursive)
    Directory,
}

/// Options for dependency analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleDependenciesOptions {
    /// Include development dependencies (default: false)
    #[serde(default)]
    pub include_dev_dependencies: bool,

    /// Include workspace dependencies (default: true)
    #[serde(default = "default_true")]
    pub include_workspace_deps: bool,

    /// Resolve cargo features (default: true)
    #[serde(default = "default_true")]
    pub resolve_features: bool,
}

fn default_true() -> bool {
    true
}

impl Default for ModuleDependenciesOptions {
    fn default() -> Self {
        Self {
            include_dev_dependencies: false,
            include_workspace_deps: true,
            resolve_features: true,
        }
    }
}

/// Result of module dependency analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleDependenciesResult {
    /// External dependencies (from crates.io)
    pub external_dependencies: HashMap<String, DependencySpec>,

    /// Workspace dependencies (internal crates)
    pub workspace_dependencies: Vec<String>,

    /// Standard library dependencies
    pub std_dependencies: Vec<String>,

    /// Import analysis summary
    pub import_analysis: ImportAnalysisSummary,

    /// List of files analyzed
    pub files_analyzed: Vec<String>,
}

/// Dependency specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencySpec {
    /// Version requirement
    pub version: String,

    /// Required features
    #[serde(skip_serializing_if = "Option::is_none")]
    pub features: Option<Vec<String>>,

    /// Whether dependency is optional
    #[serde(skip_serializing_if = "Option::is_none")]
    pub optional: Option<bool>,

    /// Source of dependency (workspace or direct)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

/// Import analysis summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportAnalysisSummary {
    /// Total imports found
    pub total_imports: usize,

    /// Count of external crates
    pub external_crates: usize,

    /// Count of workspace crates
    pub workspace_crates: usize,

    /// Count of std library imports
    pub std_crates: usize,

    /// Unresolved imports (not found in workspace)
    pub unresolved_imports: Vec<String>,
}

/// Handler for module dependency analysis
pub struct ModuleDependenciesHandler;

impl ModuleDependenciesHandler {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ToolHandler for ModuleDependenciesHandler {
    fn tool_names(&self) -> &[&str] {
        &["analyze.module_dependencies"]
    }

    fn is_internal(&self) -> bool {
        false // PUBLIC tool
    }

    async fn handle_tool_call(
        &self,
        context: &ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        info!(tool = "analyze.module_dependencies", "Starting module dependencies analysis");

        // Parse parameters
        let args = tool_call.arguments.clone().unwrap_or(json!({}));
        let params: ModuleDependenciesParams = serde_json::from_value(args)
            .map_err(|e| {
                error!(error = %e, "Failed to parse parameters");
                ServerError::InvalidRequest(format!("Invalid parameters: {}", e))
            })?;

        debug!(
            target_kind = ?params.target.kind,
            target_path = %params.target.path,
            "Analyzing module dependencies"
        );

        // Perform analysis
        let result = analyze_module_dependencies(&params, context).await?;

        // Convert to JSON
        let json_result = serde_json::to_value(&result).map_err(|e| {
            error!(error = %e, "Failed to serialize result");
            ServerError::internal(format!("Failed to serialize result: {}", e))
        })?;

        info!(
            external_deps = result.external_dependencies.len(),
            workspace_deps = result.workspace_dependencies.len(),
            std_deps = result.std_dependencies.len(),
            files = result.files_analyzed.len(),
            "Module dependencies analysis complete"
        );

        Ok(json_result)
    }
}

/// Analyze module dependencies
async fn analyze_module_dependencies(
    params: &ModuleDependenciesParams,
    context: &ToolHandlerContext,
) -> ServerResult<ModuleDependenciesResult> {
    // Resolve target path
    let target_path = PathBuf::from(&params.target.path);

    if !target_path.exists() {
        return Err(ServerError::InvalidRequest(format!(
            "Target path does not exist: {}",
            params.target.path
        )));
    }

    // Collect files to analyze
    let files = match params.target.kind {
        TargetKind::File => {
            vec![target_path.clone()]
        }
        TargetKind::Directory => {
            collect_rust_files(&target_path)?
        }
    };

    debug!(files_count = files.len(), "Collected files for analysis");

    // Extract imports from all files
    let mut all_imports = HashSet::new();
    let mut files_analyzed = Vec::new();

    for file_path in &files {
        if let Ok(content) = std::fs::read_to_string(file_path) {
            // Use plugin registry to extract imports
            if let Some(extension) = file_path.extension().and_then(|e| e.to_str()) {
                if let Some(plugin) = context.app_state.language_plugins.get_plugin(extension) {
                    if let Ok(import_graph) = plugin.analyze_detailed_imports(&content, Some(file_path)) {
                        for import_info in &import_graph.imports {
                            // Extract root crate name from module path
                            if let Some(root_crate) = extract_root_crate(&import_info.module_path) {
                                all_imports.insert(root_crate);
                            }
                        }
                    }
                }
            }

            files_analyzed.push(file_path.display().to_string());
        }
    }

    debug!(unique_imports = all_imports.len(), "Extracted unique imports");

    // Classify dependencies
    let (external_deps, workspace_deps, std_deps) =
        classify_dependencies(&all_imports, &target_path, params)?;

    // Build result
    let import_analysis = ImportAnalysisSummary {
        total_imports: all_imports.len(),
        external_crates: external_deps.len(),
        workspace_crates: workspace_deps.len(),
        std_crates: std_deps.len(),
        unresolved_imports: Vec::new(), // TODO: Detect unresolved imports
    };

    Ok(ModuleDependenciesResult {
        external_dependencies: external_deps,
        workspace_dependencies: workspace_deps,
        std_dependencies: std_deps,
        import_analysis,
        files_analyzed,
    })
}

/// Collect all Rust files in a directory recursively
fn collect_rust_files(dir: &Path) -> ServerResult<Vec<PathBuf>> {
    let mut files = Vec::new();

    if !dir.is_dir() {
        return Err(ServerError::InvalidRequest(format!(
            "Path is not a directory: {}",
            dir.display()
        )));
    }

    for entry in walkdir::WalkDir::new(dir)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.is_file() && path.extension().map_or(false, |ext| ext == "rs") {
            files.push(path.to_path_buf());
        }
    }

    Ok(files)
}

/// Extract root crate name from import path
///
/// Examples:
/// - "std::collections::HashMap" -> Some("std")
/// - "serde::Serialize" -> Some("serde")
/// - "crate::utils" -> None (internal)
/// - "self::module" -> None (relative)
/// - "super::parent" -> None (relative)
fn extract_root_crate(import_path: &str) -> Option<String> {
    // Split by :: and get first segment
    let first_segment = import_path.split("::").next()?;

    // Filter out Rust keywords and relative paths
    match first_segment {
        "crate" | "self" | "super" => None,
        other => Some(other.to_string()),
    }
}

/// Classify dependencies into external, workspace, and std
fn classify_dependencies(
    imports: &HashSet<String>,
    target_path: &Path,
    params: &ModuleDependenciesParams,
) -> ServerResult<(
    HashMap<String, DependencySpec>,
    Vec<String>,
    Vec<String>,
)> {
    let mut external_deps = HashMap::new();
    let mut workspace_deps = Vec::new();
    let mut std_deps = Vec::new();

    // Find workspace root (look for Cargo.toml with [workspace])
    let workspace_root = find_workspace_root(target_path)?;

    // Load workspace manifest
    let workspace_manifest = if let Some(root) = &workspace_root {
        load_workspace_manifest(root).ok()
    } else {
        None
    };

    // Load local crate manifest (for the crate containing target_path)
    let local_manifest = find_crate_manifest(target_path)
        .and_then(|p| load_crate_manifest(&p).ok());

    // Get workspace members
    let workspace_members = workspace_manifest
        .as_ref()
        .map(|m| extract_workspace_members(m))
        .unwrap_or_default();

    for import in imports {
        // Classify as std, workspace, or external
        if is_std_crate(import) {
            if params.options.include_workspace_deps {
                std_deps.push(import.clone());
            }
        } else if is_workspace_crate(import, &workspace_members) {
            if params.options.include_workspace_deps {
                workspace_deps.push(import.clone());
            }
        } else {
            // External dependency - try to resolve version
            let dep_spec = resolve_dependency_spec(
                import,
                workspace_manifest.as_ref(),
                local_manifest.as_ref(),
            );
            external_deps.insert(import.clone(), dep_spec);
        }
    }

    Ok((external_deps, workspace_deps, std_deps))
}

/// Check if a crate is a standard library crate
fn is_std_crate(crate_name: &str) -> bool {
    matches!(crate_name, "std" | "core" | "alloc" | "proc_macro")
}

/// Check if a crate is a workspace member
fn is_workspace_crate(crate_name: &str, workspace_members: &[String]) -> bool {
    // Convert crate name to match workspace member package names
    // Workspace members may use underscores while imports use hyphens
    let normalized_name = crate_name.replace('-', "_");

    workspace_members.iter().any(|member| {
        let member_normalized = member.replace('-', "_");
        member_normalized == normalized_name
    })
}

/// Find workspace root by looking for Cargo.toml with [workspace]
fn find_workspace_root(start_path: &Path) -> ServerResult<Option<PathBuf>> {
    let mut current = start_path.to_path_buf();

    // Walk up directory tree
    loop {
        let cargo_toml = current.join("Cargo.toml");

        if cargo_toml.exists() {
            if let Ok(content) = std::fs::read_to_string(&cargo_toml) {
                if content.contains("[workspace]") {
                    return Ok(Some(current));
                }
            }
        }

        if !current.pop() {
            break;
        }
    }

    Ok(None)
}

/// Find Cargo.toml for the crate containing the target path
fn find_crate_manifest(target_path: &Path) -> Option<PathBuf> {
    let mut current = target_path.to_path_buf();

    // If target is a file, start from parent
    if current.is_file() {
        current.pop();
    }

    // Walk up to find Cargo.toml
    loop {
        let cargo_toml = current.join("Cargo.toml");

        if cargo_toml.exists() {
            // Make sure it's a package manifest, not just workspace
            if let Ok(content) = std::fs::read_to_string(&cargo_toml) {
                if content.contains("[package]") {
                    return Some(cargo_toml);
                }
            }
        }

        if !current.pop() {
            break;
        }
    }

    None
}

/// Load workspace manifest as toml_edit::DocumentMut
fn load_workspace_manifest(workspace_root: &Path) -> ServerResult<toml_edit::DocumentMut> {
    let manifest_path = workspace_root.join("Cargo.toml");
    let content = std::fs::read_to_string(&manifest_path).map_err(|e| {
        ServerError::internal(format!("Failed to read workspace manifest: {}", e))
    })?;

    content.parse::<toml_edit::DocumentMut>().map_err(|e| {
        ServerError::internal(format!("Failed to parse workspace manifest: {}", e))
    })
}

/// Load crate manifest as toml_edit::DocumentMut
fn load_crate_manifest(manifest_path: &Path) -> ServerResult<toml_edit::DocumentMut> {
    let content = std::fs::read_to_string(manifest_path).map_err(|e| {
        ServerError::internal(format!("Failed to read crate manifest: {}", e))
    })?;

    content.parse::<toml_edit::DocumentMut>().map_err(|e| {
        ServerError::internal(format!("Failed to parse crate manifest: {}", e))
    })
}

/// Extract workspace member package names
fn extract_workspace_members(manifest: &toml_edit::DocumentMut) -> Vec<String> {
    let mut members = Vec::new();

    if let Some(workspace) = manifest.get("workspace").and_then(|w| w.as_table()) {
        if let Some(members_array) = workspace.get("members").and_then(|m| m.as_array()) {
            for member in members_array.iter() {
                if let Some(path) = member.as_str() {
                    // Extract crate name from path (e.g., "crates/cb-core" -> "cb-core")
                    if let Some(name) = path.split('/').last() {
                        members.push(name.to_string());
                    }
                }
            }
        }
    }

    members
}

/// Resolve dependency specification from manifests
fn resolve_dependency_spec(
    crate_name: &str,
    workspace_manifest: Option<&toml_edit::DocumentMut>,
    local_manifest: Option<&toml_edit::DocumentMut>,
) -> DependencySpec {
    // Try local manifest first
    if let Some(spec) = local_manifest.and_then(|m| extract_dependency_spec(m, crate_name)) {
        return spec;
    }

    // Fall back to workspace manifest
    if let Some(spec) = workspace_manifest.and_then(|m| extract_dependency_spec(m, crate_name)) {
        return spec;
    }

    // Default spec if not found
    DependencySpec {
        version: "*".to_string(),
        features: None,
        optional: None,
        source: None,
    }
}

/// Extract dependency spec from a manifest
fn extract_dependency_spec(
    manifest: &toml_edit::DocumentMut,
    crate_name: &str,
) -> Option<DependencySpec> {
    // Check [dependencies] section
    if let Some(deps) = manifest.get("dependencies").and_then(|d| d.as_table()) {
        if let Some(dep) = deps.get(crate_name) {
            return parse_dependency_value(dep, "direct");
        }
    }

    // Check [workspace.dependencies] section
    if let Some(workspace) = manifest.get("workspace").and_then(|w| w.as_table()) {
        if let Some(deps) = workspace.get("dependencies").and_then(|d| d.as_table()) {
            if let Some(dep) = deps.get(crate_name) {
                return parse_dependency_value(dep, "workspace");
            }
        }
    }

    None
}

/// Parse dependency value from TOML
fn parse_dependency_value(value: &toml_edit::Item, source: &str) -> Option<DependencySpec> {
    match value {
        toml_edit::Item::Value(v) if v.is_str() => {
            // Simple version string: dep = "1.0"
            Some(DependencySpec {
                version: v.as_str()?.to_string(),
                features: None,
                optional: None,
                source: Some(source.to_string()),
            })
        }
        toml_edit::Item::Value(v) if v.is_inline_table() => {
            // Inline table: dep = { version = "1.0", features = [...] }
            let table = v.as_inline_table()?;

            let version = table
                .get("version")
                .and_then(|v| v.as_str())
                .unwrap_or("*")
                .to_string();

            let features = table
                .get("features")
                .and_then(|f| f.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                });

            let optional = table
                .get("optional")
                .and_then(|o| o.as_bool());

            Some(DependencySpec {
                version,
                features,
                optional,
                source: Some(source.to_string()),
            })
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_root_crate() {
        assert_eq!(extract_root_crate("std::collections::HashMap"), Some("std".to_string()));
        assert_eq!(extract_root_crate("serde::Serialize"), Some("serde".to_string()));
        assert_eq!(extract_root_crate("tokio::runtime::Runtime"), Some("tokio".to_string()));
        assert_eq!(extract_root_crate("crate::utils"), None);
        assert_eq!(extract_root_crate("self::module"), None);
        assert_eq!(extract_root_crate("super::parent"), None);
    }

    #[test]
    fn test_is_std_crate() {
        assert!(is_std_crate("std"));
        assert!(is_std_crate("core"));
        assert!(is_std_crate("alloc"));
        assert!(is_std_crate("proc_macro"));
        assert!(!is_std_crate("tokio"));
        assert!(!is_std_crate("serde"));
    }

    #[test]
    fn test_is_workspace_crate() {
        let members = vec!["cb-core".to_string(), "cb-handlers".to_string()];

        assert!(is_workspace_crate("cb_core", &members));
        assert!(is_workspace_crate("cb-core", &members));
        assert!(is_workspace_crate("cb_handlers", &members));
        assert!(!is_workspace_crate("tokio", &members));
    }
}