//! package.json manifest file handling
//!
//! This module provides functionality for parsing and manipulating package.json
//! manifest files, extracting dependency information, and updating dependencies.
//!
//! # package.json Format
//!
//! A package.json file contains:
//! - Package metadata: name, version, description, author
//! - Dependencies: dependencies, devDependencies, peerDependencies, optionalDependencies
//! - Scripts: npm/yarn script commands
//! - Configuration: type (module/commonjs), main, types, exports
//! - Workspace: workspace configuration for monorepos
//!
//! # Example
//!
//! ```json
//! {
//!   "name": "my-package",
//!   "version": "1.0.0",
//!   "dependencies": {
//!     "react": "^18.0.0",
//!     "lodash": "~4.17.0"
//!   },
//!   "devDependencies": {
//!     "typescript": "^5.0.0"
//!   }
//! }
//! ```

use cb_lang_common::read_manifest;
use mill_plugin_api::{ Dependency , DependencySource , ManifestData , PluginError , PluginResult };
use serde_json::{Map, Value};
use std::path::Path;
use tracing::{debug, warn};

/// Parse a package.json file and extract manifest information
pub fn parse_package_json(content: &str) -> PluginResult<ManifestData> {
    debug!("Parsing package.json content");

    let json: Value = serde_json::from_str(content)
        .map_err(|e| PluginError::manifest(format!("Failed to parse package.json: {}", e)))?;

    let obj = json
        .as_object()
        .ok_or_else(|| PluginError::manifest("package.json root must be an object"))?;

    // Extract package information
    let name = obj
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| PluginError::manifest("Missing 'name' field in package.json"))?
        .to_string();

    let version = obj
        .get("version")
        .and_then(|v| v.as_str())
        .unwrap_or("0.0.0")
        .to_string();

    // Extract dependencies
    let dependencies = extract_dependencies(obj, "dependencies");
    let dev_dependencies = extract_dependencies(obj, "devDependencies");

    debug!(
        package = %name,
        version = %version,
        dependencies_count = dependencies.len(),
        dev_dependencies_count = dev_dependencies.len(),
        "Parsed package.json successfully"
    );

    Ok(ManifestData {
        name,
        version,
        dependencies,
        dev_dependencies,
        raw_data: json,
    })
}

/// Extract dependencies from a specific field in the package.json
fn extract_dependencies(obj: &Map<String, Value>, field_name: &str) -> Vec<Dependency> {
    let mut deps = Vec::new();

    if let Some(deps_obj) = obj.get(field_name).and_then(|v| v.as_object()) {
        for (name, value) in deps_obj.iter() {
            let source = parse_dependency_source(value);
            deps.push(Dependency {
                name: name.clone(),
                source,
            });
        }
    }

    deps
}

/// Parse dependency source from package.json value
fn parse_dependency_source(value: &Value) -> DependencySource {
    let value_str = match value.as_str() {
        Some(s) => s,
        None => return DependencySource::Version("*".to_string()),
    };

    // Git dependency: "git://..." or "git+https://..." or "github:user/repo"
    if value_str.starts_with("git://")
        || value_str.starts_with("git+")
        || value_str.contains("github.com")
        || value_str.starts_with("github:")
        || value_str.starts_with("bitbucket:")
        || value_str.starts_with("gitlab:")
    {
        // Extract revision if specified with #
        let parts: Vec<&str> = value_str.split('#').collect();
        let url = parts[0].to_string();
        let rev = parts.get(1).map(|s| s.to_string());

        return DependencySource::Git { url, rev };
    }

    // Local path dependency: "file:..." or relative/absolute paths
    if value_str.starts_with("file:")
        || value_str.starts_with("./")
        || value_str.starts_with("../")
        || value_str.starts_with('/')
    {
        let path = value_str.strip_prefix("file:").unwrap_or(value_str);
        return DependencySource::Path(path.to_string());
    }

    // Workspace dependency: "workspace:*" or "workspace:^1.0.0"
    if value_str.starts_with("workspace:") {
        let version = value_str.strip_prefix("workspace:").unwrap_or("*");
        return DependencySource::Version(format!("workspace:{}", version));
    }

    // URL dependency: "http://..." or "https://..."
    if value_str.starts_with("http://") || value_str.starts_with("https://") {
        return DependencySource::Git {
            url: value_str.to_string(),
            rev: None,
        };
    }

    // Default: version specifier (^1.0.0, ~1.0.0, >=1.0.0, 1.0.0, etc.)
    DependencySource::Version(value_str.to_string())
}

/// Update a dependency version in package.json content
pub fn update_dependency(content: &str, dep_name: &str, new_version: &str) -> PluginResult<String> {
    debug!(
        dependency = %dep_name,
        version = %new_version,
        "Updating dependency in package.json"
    );

    let mut json: Value = serde_json::from_str(content)
        .map_err(|e| PluginError::manifest(format!("Failed to parse package.json: {}", e)))?;

    let obj = json
        .as_object_mut()
        .ok_or_else(|| PluginError::manifest("package.json root must be an object"))?;

    let mut found = false;

    // Update in dependencies
    if let Some(deps) = obj.get_mut("dependencies").and_then(|v| v.as_object_mut()) {
        if deps.contains_key(dep_name) {
            deps.insert(dep_name.to_string(), Value::String(new_version.to_string()));
            found = true;
        }
    }

    // Update in devDependencies
    if let Some(dev_deps) = obj
        .get_mut("devDependencies")
        .and_then(|v| v.as_object_mut())
    {
        if dev_deps.contains_key(dep_name) {
            dev_deps.insert(dep_name.to_string(), Value::String(new_version.to_string()));
            found = true;
        }
    }

    // Update in peerDependencies
    if let Some(peer_deps) = obj
        .get_mut("peerDependencies")
        .and_then(|v| v.as_object_mut())
    {
        if peer_deps.contains_key(dep_name) {
            peer_deps.insert(dep_name.to_string(), Value::String(new_version.to_string()));
            found = true;
        }
    }

    // Update in optionalDependencies
    if let Some(opt_deps) = obj
        .get_mut("optionalDependencies")
        .and_then(|v| v.as_object_mut())
    {
        if opt_deps.contains_key(dep_name) {
            opt_deps.insert(dep_name.to_string(), Value::String(new_version.to_string()));
            found = true;
        }
    }

    if !found {
        warn!(dependency = %dep_name, "Dependency not found in package.json");
        return Err(PluginError::manifest(format!(
            "Dependency {} not found in package.json",
            dep_name
        )));
    }

    // Serialize with pretty formatting (2 spaces, like npm)
    serde_json::to_string_pretty(&json)
        .map(|s| s + "\n") // Add trailing newline like npm
        .map_err(|e| PluginError::manifest(format!("Failed to serialize package.json: {}", e)))
}

/// Generate a new package.json file
pub fn generate_manifest(package_name: &str, dependencies: &[String]) -> String {
    let mut manifest = serde_json::json!({
        "name": package_name,
        "version": "0.1.0",
        "private": true,
        "type": "module"
    });

    if !dependencies.is_empty() {
        let deps: Map<String, Value> = dependencies
            .iter()
            .map(|dep| (dep.clone(), Value::String("*".to_string())))
            .collect();

        if let Some(obj) = manifest.as_object_mut() {
            obj.insert("dependencies".to_string(), Value::Object(deps));
        }
    }

    serde_json::to_string_pretty(&manifest).unwrap() + "\n"
}

/// Load and parse a package.json file from a path
pub async fn load_package_json(path: &Path) -> PluginResult<ManifestData> {
    let content = read_manifest(path).await?;
    parse_package_json(&content)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_package_json() {
        let content = r#"{
  "name": "my-package",
  "version": "1.0.0",
  "dependencies": {
    "react": "^18.0.0",
    "lodash": "~4.17.0"
  },
  "devDependencies": {
    "typescript": "^5.0.0"
  }
}"#;

        let manifest = parse_package_json(content).unwrap();
        assert_eq!(manifest.name, "my-package");
        assert_eq!(manifest.version, "1.0.0");
        assert_eq!(manifest.dependencies.len(), 2);
        assert_eq!(manifest.dev_dependencies.len(), 1);

        assert!(manifest.dependencies.iter().any(|d| {
            d.name == "react" && matches!(&d.source, DependencySource::Version(v) if v == "^18.0.0")
        }));

        assert!(manifest.dev_dependencies.iter().any(|d| {
            d.name == "typescript"
                && matches!(&d.source, DependencySource::Version(v) if v == "^5.0.0")
        }));
    }

    #[test]
    fn test_parse_git_dependency() {
        let content = r#"{
  "name": "test-package",
  "version": "1.0.0",
  "dependencies": {
    "my-lib": "git+https://github.com/user/repo.git#v1.0.0",
    "other-lib": "github:user/other"
  }
}"#;

        let manifest = parse_package_json(content).unwrap();
        assert_eq!(manifest.dependencies.len(), 2);

        let git_dep = manifest
            .dependencies
            .iter()
            .find(|d| d.name == "my-lib")
            .unwrap();
        assert!(matches!(
            &git_dep.source,
            DependencySource::Git { url, rev }
            if url == "git+https://github.com/user/repo.git" && rev.as_deref() == Some("v1.0.0")
        ));

        let github_dep = manifest
            .dependencies
            .iter()
            .find(|d| d.name == "other-lib")
            .unwrap();
        assert!(matches!(&github_dep.source, DependencySource::Git { .. }));
    }

    #[test]
    fn test_parse_local_path_dependency() {
        let content = r#"{
  "name": "test-package",
  "version": "1.0.0",
  "dependencies": {
    "local-lib": "file:../local-lib",
    "relative-lib": "./libs/my-lib"
  }
}"#;

        let manifest = parse_package_json(content).unwrap();
        assert_eq!(manifest.dependencies.len(), 2);

        let local_dep = manifest
            .dependencies
            .iter()
            .find(|d| d.name == "local-lib")
            .unwrap();
        assert!(matches!(&local_dep.source, DependencySource::Path(p) if p == "../local-lib"));

        let relative_dep = manifest
            .dependencies
            .iter()
            .find(|d| d.name == "relative-lib")
            .unwrap();
        assert!(matches!(&relative_dep.source, DependencySource::Path(p) if p == "./libs/my-lib"));
    }

    #[test]
    fn test_parse_workspace_dependency() {
        let content = r#"{
  "name": "test-package",
  "version": "1.0.0",
  "dependencies": {
    "workspace-lib": "workspace:*",
    "workspace-lib2": "workspace:^1.0.0"
  }
}"#;

        let manifest = parse_package_json(content).unwrap();
        assert_eq!(manifest.dependencies.len(), 2);

        let ws_dep = manifest
            .dependencies
            .iter()
            .find(|d| d.name == "workspace-lib")
            .unwrap();
        assert!(matches!(&ws_dep.source, DependencySource::Version(v) if v == "workspace:*"));
    }

    #[test]
    fn test_update_dependency() {
        let content = r#"{
  "name": "test-package",
  "version": "1.0.0",
  "dependencies": {
    "react": "^17.0.0",
    "lodash": "^4.0.0"
  },
  "devDependencies": {
    "typescript": "^4.0.0"
  }
}"#;

        let result = update_dependency(content, "react", "^18.0.0").unwrap();
        assert!(result.contains("\"react\": \"^18.0.0\""));
        assert!(!result.contains("^17.0.0"));

        // Test dev dependency
        let result2 = update_dependency(content, "typescript", "^5.0.0").unwrap();
        assert!(result2.contains("\"typescript\": \"^5.0.0\""));
    }

    #[test]
    fn test_update_nonexistent_dependency() {
        let content = r#"{
  "name": "test-package",
  "version": "1.0.0",
  "dependencies": {
    "react": "^17.0.0"
  }
}"#;

        let result = update_dependency(content, "nonexistent", "^1.0.0");
        assert!(result.is_err());
    }

    #[test]
    fn test_generate_manifest() {
        let result = generate_manifest("my-new-package", &["react".to_string(), "vue".to_string()]);

        assert!(result.contains("\"name\": \"my-new-package\""));
        assert!(result.contains("\"version\": \"0.1.0\""));
        assert!(result.contains("\"react\""));
        assert!(result.contains("\"vue\""));
        assert!(result.contains("\"type\": \"module\""));
    }

    #[test]
    fn test_generate_manifest_without_dependencies() {
        let result = generate_manifest("empty-package", &[]);

        assert!(result.contains("\"name\": \"empty-package\""));
        assert!(!result.contains("dependencies"));
    }

    #[test]
    fn test_parse_version_ranges() {
        let content = r#"{
  "name": "test-package",
  "version": "1.0.0",
  "dependencies": {
    "caret": "^1.0.0",
    "tilde": "~1.0.0",
    "exact": "1.0.0",
    "gte": ">=1.0.0",
    "range": "1.0.0 - 2.0.0",
    "latest": "latest"
  }
}"#;

        let manifest = parse_package_json(content).unwrap();
        assert_eq!(manifest.dependencies.len(), 6);

        // All should parse as version dependencies
        for dep in &manifest.dependencies {
            assert!(matches!(&dep.source, DependencySource::Version(_)));
        }
    }
}