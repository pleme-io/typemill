use mill_plugin_api::{Dependency, DependencySource, ManifestData, PluginResult};
use regex::Regex;
use serde_json::json;
use std::path::Path;

pub fn analyze_conan_manifest(path: &Path) -> PluginResult<ManifestData> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| mill_plugin_api::PluginError::manifest(format!("Failed to read manifest: {}", e)))?;

    let requires_re = Regex::new(r#"(?ms)\[requires\](.*?)\["#).unwrap();
    let dep_re = Regex::new(r#"(\w+)/(\S+)"#).unwrap();
    let mut dependencies = vec![];

    let content_with_guard = format!("{}\n[", content);

    if let Some(requires_block) = requires_re.captures(&content_with_guard).and_then(|c| c.get(1)) {
        for cap in dep_re.captures_iter(requires_block.as_str()) {
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