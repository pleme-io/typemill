use mill_plugin_api::{ManifestData, PluginResult};
use regex::Regex;
use serde_json::json;
use std::path::Path;

use mill_plugin_api::{Dependency, DependencySource};

pub fn analyze_cmake_manifest(path: &Path) -> PluginResult<ManifestData> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| mill_plugin_api::PluginError::manifest(format!("Failed to read manifest: {}", e)))?;

    let project_re = Regex::new(r#"(?i)project\s*\(\s*(\w+)"#).unwrap();
    let name = project_re
        .captures(&content)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str().to_string())
        .unwrap_or_else(|| "Unknown".to_string());

    let mut libraries = vec![];
    let mut executables = vec![];
    let mut source_files = vec![];

    let target_re =
        Regex::new(r#"(?i)(add_library|add_executable)\s*\(([\w\s./]+)\)"#).unwrap();

    for caps in target_re.captures_iter(&content) {
        let command = caps.get(1).unwrap().as_str();
        let args_str = caps.get(2).unwrap().as_str();
        let mut args = args_str.split_whitespace();

        if let Some(target_name) = args.next() {
            let sources: Vec<String> = args.map(|s| s.to_string()).collect();
            if command.eq_ignore_ascii_case("add_library") {
                libraries.push(target_name.to_string());
            } else {
                executables.push(target_name.to_string());
            }
            source_files.extend(sources);
        }
    }

    // NOTE: This is a best-effort regex-based parser. It does not handle
    // complex CMake syntax like variables, generator expressions, or multi-line
    // commands. A full-fledged CMake parser would be required for complete accuracy.
    let link_re = Regex::new(r#"(?i)target_link_libraries\s*\(\s*(\w+)\s+(?:(PUBLIC|PRIVATE|INTERFACE)\s+)?([\w\s]+)\)"#).unwrap();
    let mut dependencies: Vec<Dependency> = vec![];
    let mut linked_libraries: Vec<serde_json::Value> = vec![];

    for caps in link_re.captures_iter(&content) {
        let target = caps.get(1).map_or("", |m| m.as_str()).to_string();
        let linkage = caps.get(2).map_or("PRIVATE", |m| m.as_str()).to_string();
        let libs_str = caps.get(3).map_or("", |m| m.as_str());

        for lib in libs_str.split_whitespace() {
            dependencies.push(Dependency {
                name: lib.to_string(),
                source: DependencySource::Version("".to_string()),
            });
            linked_libraries.push(json!({
                "target": target,
                "library": lib,
                "linkage": linkage,
            }));
        }
    }

    Ok(ManifestData {
        name,
        version: "0.0.0".to_string(),
        dependencies,
        dev_dependencies: vec![],
        raw_data: json!({
            "libraries": libraries,
            "executables": executables,
            "linked_libraries": linked_libraries,
            "source_files": source_files,
        }),
    })
}