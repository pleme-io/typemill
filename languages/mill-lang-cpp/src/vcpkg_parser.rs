use mill_plugin_api::{Dependency, DependencySource, ManifestData, PluginResult};
use serde_json::Value;

/// Analyzes a vcpkg.json manifest file and extracts project metadata.
///
/// Parses JSON-formatted vcpkg manifests to extract package name, version,
/// and dependency information.
///
/// # Arguments
/// * `content` - Raw JSON content of the vcpkg.json file
///
/// # Returns
/// Manifest data containing name, version, and dependencies
pub(crate) fn analyze_vcpkg_manifest(content: &str) -> PluginResult<ManifestData> {
    let v: Value = serde_json::from_str(content).map_err(|e| {
        mill_plugin_api::PluginApiError::manifest(format!("Failed to parse vcpkg.json: {}", e))
    })?;

    let name = v["name"].as_str().unwrap_or("Unknown").to_string();
    let version = v["version-string"].as_str().unwrap_or("0.0.0").to_string();

    let mut dependencies = vec![];
    if let Some(deps) = v["dependencies"].as_array() {
        for dep in deps {
            if let Some(dep_name) = dep.as_str() {
                dependencies.push(Dependency {
                    name: dep_name.to_string(),
                    source: DependencySource::Version("*".to_string()),
                });
            }
        }
    }

    Ok(ManifestData {
        name,
        version,
        dependencies,
        dev_dependencies: vec![],
        raw_data: v,
    })
}
