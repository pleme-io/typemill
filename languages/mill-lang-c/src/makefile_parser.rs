use mill_plugin_api::{ManifestData, PluginResult};
use regex::Regex;
use std::fs;
use std::path::Path;

/// Analyzes a Makefile and extracts project metadata.
///
/// Parses Makefiles to extract the TARGET variable (project name), SRCS variable
/// (source files), and CFLAGS variable (compiler flags) using regex-based pattern matching.
///
/// # Arguments
/// * `path` - Path to the Makefile
///
/// # Returns
/// Manifest data containing target name, source files, and compiler flags
pub(crate) fn analyze_makefile_manifest(path: &Path) -> PluginResult<ManifestData> {
    let content = fs::read_to_string(path).unwrap_or_default();

    let name = extract_var(&content, "TARGET").unwrap_or_default();
    let srcs = extract_list(&content, "SRCS");

    Ok(ManifestData {
        name,
        version: "".to_string(),
        dependencies: vec![],
        dev_dependencies: vec![],
        raw_data: serde_json::json!({
            "source_files": srcs,
            "cflags": extract_var(&content, "CFLAGS").unwrap_or_default(),
        }),
    })
}

fn extract_var(content: &str, var_name: &str) -> Option<String> {
    let re = Regex::new(&format!(r#"{}\s*=\s*(.*)"#, var_name)).unwrap();
    re.captures(content)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str().trim().to_string())
}

fn extract_list(content: &str, var_name: &str) -> Vec<String> {
    extract_var(content, var_name)
        .map(|s| s.split_whitespace().map(String::from).collect())
        .unwrap_or_default()
}
