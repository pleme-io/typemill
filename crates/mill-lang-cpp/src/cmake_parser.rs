use mill_plugin_api::{ManifestData, PluginResult};
use regex::Regex;
use serde_json::json;
use std::path::Path;

pub fn analyze_cmake_manifest(path: &Path) -> PluginResult<ManifestData> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| mill_plugin_api::PluginError::manifest(format!("Failed to read manifest: {}", e)))?;

    let project_re = Regex::new(r#"(?i)project\s*\(\s*(\w+)"#).unwrap();
    let name = project_re
        .captures(&content)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str().to_string())
        .unwrap_or_else(|| "Unknown".to_string());

    let lib_re = Regex::new(r#"(?i)add_library\s*\(\s*(\w+)"#).unwrap();
    let libraries: Vec<_> = lib_re.captures_iter(&content).map(|caps| caps[1].to_string()).collect();

    let exe_re = Regex::new(r#"(?i)add_executable\s*\(\s*(\w+)"#).unwrap();
    let executables: Vec<_> = exe_re.captures_iter(&content).map(|caps| caps[1].to_string()).collect();

    Ok(ManifestData {
        name,
        version: "0.0.0".to_string(),
        dependencies: vec![],
        dev_dependencies: vec![],
        raw_data: json!({
            "libraries": libraries,
            "executables": executables,
        }),
    })
}