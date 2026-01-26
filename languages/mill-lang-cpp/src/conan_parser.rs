use mill_plugin_api::{Dependency, DependencySource, ManifestData, PluginResult};
use once_cell::sync::Lazy;
use regex::Regex;
use serde_json::json;
use std::path::Path;

static CONANFILE_TXT_REQUIRES_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#"(?ms)\[requires\](.*)"#).unwrap());
static CONANFILE_TXT_DEP_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r#"(\w+)/(\S+)"#).unwrap());
static CONANFILE_PY_REQUIRES_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#"(?s)requires\s*=\s*\[(.*?)\]"#).unwrap());
static CONANFILE_PY_DEP_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r#""([^"]+)/([^"]+)""#).unwrap());

/// Analyzes a Conan manifest file and extracts project metadata.
///
/// Supports both conanfile.txt and conanfile.py formats. Extracts package dependencies
/// with their version requirements.
///
/// # Arguments
/// * `path` - Path to the conanfile.txt or conanfile.py
///
/// # Returns
/// Manifest data containing dependencies extracted from the requires section
pub(crate) fn analyze_conan_manifest(path: &Path) -> PluginResult<ManifestData> {
    let content = std::fs::read_to_string(path).map_err(|e| {
        mill_plugin_api::PluginApiError::manifest(format!("Failed to read manifest: {}", e))
    })?;

    match path.extension().and_then(|s| s.to_str()) {
        Some("txt") => parse_conanfile_txt(&content),
        Some("py") => parse_conanfile_py(&content),
        _ => Err(mill_plugin_api::PluginApiError::manifest(
            "Unsupported conan manifest file type".to_string(),
        )),
    }
}

fn parse_conanfile_txt(content: &str) -> PluginResult<ManifestData> {
    let mut dependencies = vec![];

    if let Some(requires_block) = CONANFILE_TXT_REQUIRES_RE
        .captures(content)
        .and_then(|c| c.get(1))
    {
        for cap in CONANFILE_TXT_DEP_RE.captures_iter(requires_block.as_str()) {
            dependencies.push(Dependency {
                name: cap[1].to_string(),
                source: DependencySource::Version(cap[2].to_string()),
            });
        }
    }

    Ok(ManifestData {
        name: "Unknown".to_string(),
        version: "0.0.0".to_string(),
        dependencies,
        dev_dependencies: vec![],
        raw_data: json!({}),
    })
}

fn parse_conanfile_py(content: &str) -> PluginResult<ManifestData> {
    let mut dependencies = vec![];

    if let Some(requires_block) = CONANFILE_PY_REQUIRES_RE
        .captures(content)
        .and_then(|c| c.get(1))
    {
        for cap in CONANFILE_PY_DEP_RE.captures_iter(requires_block.as_str()) {
            dependencies.push(Dependency {
                name: cap[1].to_string(),
                source: DependencySource::Version(cap[2].to_string()),
            });
        }
    }

    Ok(ManifestData {
        name: "Unknown".to_string(),
        version: "0.0.0".to_string(),
        dependencies,
        dev_dependencies: vec![],
        raw_data: json!({}),
    })
}
