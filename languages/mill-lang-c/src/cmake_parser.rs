use mill_plugin_api::{ManifestData, PluginResult};
use regex::Regex;
use std::fs;
use std::path::Path;

/// Analyzes a CMakeLists.txt file and extracts project metadata.
///
/// Parses CMake project files to extract the project name, executable targets,
/// and library targets using regex-based pattern matching.
///
/// # Arguments
/// * `path` - Path to the CMakeLists.txt file
///
/// # Returns
/// Manifest data containing project name, executables, and libraries
pub(crate) fn analyze_cmake_manifest(path: &Path) -> PluginResult<ManifestData> {
    let content = fs::read_to_string(path).unwrap_or_default();

    let name = extract_project_name(&content).unwrap_or_default();
    let executables = extract_targets(&content, "add_executable");
    let libraries = extract_targets(&content, "add_library");

    Ok(ManifestData {
        name,
        version: "".to_string(),
        dependencies: vec![],
        dev_dependencies: vec![],
        raw_data: serde_json::json!({
            "executables": executables,
            "libraries": libraries,
        }),
    })
}

fn extract_project_name(content: &str) -> Option<String> {
    let re = Regex::new(r#"project\(([^)]+)\)"#).unwrap();
    re.captures(content)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str().trim().to_string())
}

fn extract_targets(content: &str, command: &str) -> Vec<String> {
    let re = Regex::new(&format!(r#"{}\(([^ ]+)"#, command)).unwrap();
    re.captures_iter(content)
        .map(|cap| cap[1].to_string())
        .collect()
}
