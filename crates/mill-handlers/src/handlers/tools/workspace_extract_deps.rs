//! Workspace dependency extraction tool handler
//!
//! Handles: workspace.extract_dependencies
//!
//! This tool extracts specific dependencies from one Cargo.toml and adds them to another,
//! supporting the crate extraction workflow (Proposal 50).

use super::{ToolHandler, ToolHandlerContext};
use async_trait::async_trait;
use mill_foundation::core::model::mcp::ToolCall;
use mill_foundation::protocol::{ApiError, ApiResult as ServerResult};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::path::Path;
use toml_edit::{DocumentMut, Item, Value as TomlValue};
use tracing::{debug, error, warn};

/// Handler for workspace dependency extraction operations
pub struct WorkspaceExtractDepsHandler;

impl WorkspaceExtractDepsHandler {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ToolHandler for WorkspaceExtractDepsHandler {
    fn tool_names(&self) -> &[&str] {
        &["workspace.extract_dependencies"]
    }

    async fn handle_tool_call(
        &self,
        context: &ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        match tool_call.name.as_str() {
            "workspace.extract_dependencies" => {
                handle_extract_dependencies(context, tool_call).await
            }
            _ => Err(ApiError::InvalidRequest(format!(
                "Unknown workspace extract deps tool: {}",
                tool_call.name
            ))),
        }
    }
}

// Parameter types for MCP interface

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractDependenciesParams {
    pub source_manifest: String,
    pub target_manifest: String,
    pub dependencies: Vec<String>,
    #[serde(default)]
    pub options: ExtractDependenciesOptions,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ExtractDependenciesOptions {
    #[serde(default)]
    pub dry_run: bool,
    #[serde(default = "default_true")]
    pub preserve_versions: bool,
    #[serde(default = "default_true")]
    pub preserve_features: bool,
    #[serde(default)]
    pub section: DependencySection,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum DependencySection {
    #[default]
    Dependencies,
    DevDependencies,
    BuildDependencies,
}

impl DependencySection {
    fn as_str(&self) -> &str {
        match self {
            DependencySection::Dependencies => "dependencies",
            DependencySection::DevDependencies => "dev-dependencies",
            DependencySection::BuildDependencies => "build-dependencies",
        }
    }
}

// Result type for MCP interface

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractDependenciesResult {
    pub dependencies_extracted: usize,
    pub dependencies_added: Vec<DependencyInfo>,
    pub target_manifest_updated: bool,
    pub dry_run: bool,
    pub warnings: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DependencyInfo {
    pub name: String,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub features: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub optional: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub already_exists: Option<bool>,
}

// Handler implementation

async fn handle_extract_dependencies(
    context: &ToolHandlerContext,
    tool_call: &ToolCall,
) -> ServerResult<Value> {
    debug!("Handling workspace.extract_dependencies");

    // Parse parameters
    let params: ExtractDependenciesParams = serde_json::from_value(
        tool_call
            .arguments
            .as_ref()
            .ok_or_else(|| ApiError::InvalidRequest("Missing arguments".to_string()))?
            .clone(),
    )
    .map_err(|e| ApiError::InvalidRequest(format!("Invalid arguments: {}", e)))?;

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
    if !source_path.exists() {
        return Err(ApiError::InvalidRequest(format!(
            "Source manifest not found: {}",
            source_path.display()
        )));
    }

    if !target_path.exists() {
        return Err(ApiError::InvalidRequest(format!(
            "Target manifest not found: {}",
            target_path.display()
        )));
    }

    // Read manifests
    let source_content = fs::read_to_string(&source_path).map_err(|e| {
        error!(error = %e, source_path = %source_path.display(), "Failed to read source manifest");
        ApiError::Internal(format!("Failed to read source manifest: {}", e))
    })?;

    let target_content = fs::read_to_string(&target_path).map_err(|e| {
        error!(error = %e, target_path = %target_path.display(), "Failed to read target manifest");
        ApiError::Internal(format!("Failed to read target manifest: {}", e))
    })?;

    // Extract dependencies
    let extraction_result = extract_dependencies(
        &source_content,
        &target_content,
        &params.dependencies,
        &params.options,
    )?;

    // Write updated target manifest if not dry-run
    let target_updated = if !params.options.dry_run && extraction_result.updated_content.is_some() {
        let updated = extraction_result.updated_content.as_ref().unwrap();
        fs::write(&target_path, updated).map_err(|e| {
            error!(error = %e, target_path = %target_path.display(), "Failed to write target manifest");
            ApiError::Internal(format!("Failed to write target manifest: {}", e))
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

// Helper types

struct ExtractionResult {
    dependencies_added: Vec<DependencyInfo>,
    warnings: Vec<String>,
    updated_content: Option<String>,
}

// Core extraction logic

fn extract_dependencies(
    source_content: &str,
    target_content: &str,
    dep_names: &[String],
    options: &ExtractDependenciesOptions,
) -> ServerResult<ExtractionResult> {
    debug!("Extracting dependencies from source manifest");

    // Parse source manifest
    let source_doc = source_content.parse::<DocumentMut>().map_err(|e| {
        error!(error = %e, "Failed to parse source manifest");
        ApiError::Parse {
            message: format!("Failed to parse source Cargo.toml: {}", e),
        }
    })?;

    // Parse target manifest
    let mut target_doc = target_content.parse::<DocumentMut>().map_err(|e| {
        error!(error = %e, "Failed to parse target manifest");
        ApiError::Parse {
            message: format!("Failed to parse target Cargo.toml: {}", e),
        }
    })?;

    let mut dependencies_added = Vec::new();
    let mut warnings = Vec::new();

    // Process each requested dependency
    for dep_name in dep_names {
        debug!(dependency = %dep_name, "Processing dependency");

        // Look for dependency in source manifest
        let dep_spec = find_dependency_in_manifest(&source_doc, dep_name);

        if dep_spec.is_none() {
            let warning = format!("Dependency '{}' not found in source manifest", dep_name);
            warn!("{}", warning);
            warnings.push(warning);
            continue;
        }

        let (section, dep_item) = dep_spec.unwrap();
        debug!(dependency = %dep_name, section = %section, "Found dependency in source");

        // Check if already exists in target
        let target_section = options.section.as_str();
        let already_exists = check_dependency_exists(&target_doc, target_section, dep_name);

        if already_exists {
            let warning = format!(
                "Dependency '{}' already exists in target, skipped",
                dep_name
            );
            warn!("{}", warning);
            warnings.push(warning);

            // Still add to result with already_exists flag
            let dep_info = extract_dependency_info(dep_name, dep_item, true);
            dependencies_added.push(dep_info);
            continue;
        }

        // Add dependency to target
        add_dependency_to_manifest(&mut target_doc, target_section, dep_name, dep_item, options)?;

        // Add to result
        let dep_info = extract_dependency_info(dep_name, dep_item, false);
        dependencies_added.push(dep_info);
        debug!(dependency = %dep_name, "Added dependency to target");
    }

    // Generate updated content
    let updated_content = if !dependencies_added.is_empty() {
        Some(target_doc.to_string())
    } else {
        None
    };

    Ok(ExtractionResult {
        dependencies_added,
        warnings,
        updated_content,
    })
}

// Helper functions

fn resolve_path(workspace_root: &Path, path: &str) -> ServerResult<std::path::PathBuf> {
    let resolved = if Path::new(path).is_absolute() {
        Path::new(path).to_path_buf()
    } else {
        workspace_root.join(path)
    };

    Ok(resolved)
}

fn find_dependency_in_manifest<'a>(
    doc: &'a DocumentMut,
    dep_name: &str,
) -> Option<(&'static str, &'a Item)> {
    // Check [dependencies]
    if let Some(deps) = doc.get("dependencies").and_then(|d| d.as_table()) {
        if let Some(dep) = deps.get(dep_name) {
            return Some(("dependencies", dep));
        }
    }

    // Check [dev-dependencies]
    if let Some(deps) = doc.get("dev-dependencies").and_then(|d| d.as_table()) {
        if let Some(dep) = deps.get(dep_name) {
            return Some(("dev-dependencies", dep));
        }
    }

    // Check [build-dependencies]
    if let Some(deps) = doc.get("build-dependencies").and_then(|d| d.as_table()) {
        if let Some(dep) = deps.get(dep_name) {
            return Some(("build-dependencies", dep));
        }
    }

    None
}

fn check_dependency_exists(doc: &DocumentMut, section: &str, dep_name: &str) -> bool {
    doc.get(section)
        .and_then(|s| s.as_table())
        .and_then(|t| t.get(dep_name))
        .is_some()
}

fn add_dependency_to_manifest(
    doc: &mut DocumentMut,
    section: &str,
    dep_name: &str,
    dep_item: &Item,
    _options: &ExtractDependenciesOptions,
) -> ServerResult<()> {
    // Ensure section exists
    if !doc.contains_key(section) {
        doc[section] = Item::Table(toml_edit::Table::new());
    }

    let table = doc[section].as_table_mut().ok_or_else(|| ApiError::Parse {
        message: format!("[{}] is not a table", section),
    })?;

    // Clone the dependency item
    table[dep_name] = dep_item.clone();

    Ok(())
}

fn extract_dependency_info(
    dep_name: &str,
    dep_item: &Item,
    already_exists: bool,
) -> DependencyInfo {
    let mut version = String::new();
    let mut features = None;
    let mut optional = None;

    // Parse dependency item
    match dep_item {
        Item::Value(TomlValue::String(v)) => {
            // Simple version string: "1.0"
            version = v.value().to_string();
        }
        Item::Value(TomlValue::InlineTable(table)) => {
            // Inline table: { version = "1.0", features = ["full"] }
            if let Some(v) = table.get("version").and_then(|v| v.as_str()) {
                version = v.to_string();
            } else if table.get("workspace").and_then(|v| v.as_bool()) == Some(true) {
                version = "workspace".to_string();
            } else if let Some(path) = table.get("path").and_then(|v| v.as_str()) {
                version = format!("path = \"{}\"", path);
            } else if let Some(git) = table.get("git").and_then(|v| v.as_str()) {
                version = format!("git = \"{}\"", git);
            }

            if let Some(f) = table.get("features").and_then(|v| v.as_array()) {
                let feat_list: Vec<String> = f
                    .iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect();
                if !feat_list.is_empty() {
                    features = Some(feat_list);
                }
            }

            if let Some(opt) = table.get("optional").and_then(|v| v.as_bool()) {
                optional = Some(opt);
            }
        }
        Item::Table(table) => {
            // Regular table (multi-line)
            if let Some(v) = table.get("version").and_then(|v| v.as_str()) {
                version = v.to_string();
            } else if table.get("workspace").and_then(|v| v.as_bool()) == Some(true) {
                version = "workspace".to_string();
            } else if let Some(path) = table.get("path").and_then(|v| v.as_str()) {
                version = format!("path = \"{}\"", path);
            } else if let Some(git) = table.get("git").and_then(|v| v.as_str()) {
                version = format!("git = \"{}\"", git);
            }

            if let Some(f) = table.get("features").and_then(|v| v.as_array()) {
                let feat_list: Vec<String> = f
                    .iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect();
                if !feat_list.is_empty() {
                    features = Some(feat_list);
                }
            }

            if let Some(opt) = table.get("optional").and_then(|v| v.as_bool()) {
                optional = Some(opt);
            }
        }
        _ => {
            // Unknown format, use placeholder
            version = "unknown".to_string();
        }
    }

    DependencyInfo {
        name: dep_name.to_string(),
        version,
        features,
        optional,
        already_exists: if already_exists { Some(true) } else { None },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_dependency_in_manifest() {
        let content = r#"
[dependencies]
tokio = { version = "1.0", features = ["full"] }
serde = "1.0"

[dev-dependencies]
tempfile = "3.0"
"#;

        let doc = content.parse::<DocumentMut>().unwrap();

        // Find in dependencies
        let result = find_dependency_in_manifest(&doc, "tokio");
        assert!(result.is_some());
        let (section, _) = result.unwrap();
        assert_eq!(section, "dependencies");

        // Find in dev-dependencies
        let result = find_dependency_in_manifest(&doc, "tempfile");
        assert!(result.is_some());
        let (section, _) = result.unwrap();
        assert_eq!(section, "dev-dependencies");

        // Not found
        let result = find_dependency_in_manifest(&doc, "nonexistent");
        assert!(result.is_none());
    }

    #[test]
    fn test_extract_dependency_info_simple() {
        let content = r#"serde = "1.0""#;
        let doc = content.parse::<DocumentMut>().unwrap();
        let item = &doc.as_table()["serde"];

        let info = extract_dependency_info("serde", item, false);
        assert_eq!(info.name, "serde");
        assert_eq!(info.version, "1.0");
        assert_eq!(info.features, None);
        assert_eq!(info.optional, None);
        assert_eq!(info.already_exists, None);
    }

    #[test]
    fn test_extract_dependency_info_with_features() {
        let content = r#"
[dependencies]
tokio = { version = "1.0", features = ["full", "rt"] }
"#;
        let doc = content.parse::<DocumentMut>().unwrap();
        let deps = doc["dependencies"].as_table().unwrap();
        let item = &deps["tokio"];

        let info = extract_dependency_info("tokio", item, false);
        assert_eq!(info.name, "tokio");
        assert_eq!(info.version, "1.0");
        assert_eq!(
            info.features,
            Some(vec!["full".to_string(), "rt".to_string()])
        );
    }

    #[test]
    fn test_extract_dependency_info_workspace() {
        let content = r#"
[dependencies]
my-crate = { workspace = true }
"#;
        let doc = content.parse::<DocumentMut>().unwrap();
        let deps = doc["dependencies"].as_table().unwrap();
        let item = &deps["my-crate"];

        let info = extract_dependency_info("my-crate", item, false);
        assert_eq!(info.name, "my-crate");
        assert_eq!(info.version, "workspace");
    }
}
