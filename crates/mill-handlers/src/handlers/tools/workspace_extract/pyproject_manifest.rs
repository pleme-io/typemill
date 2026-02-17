//! pyproject.toml manifest implementation
//!
//! Implements `ManifestOps` trait for Python projects using PEP 621 format.
//! Supports [project.dependencies] and [project.optional-dependencies].

use super::{DependencyInfo, ManifestOps};
use mill_foundation::errors::{MillError as ServerError, MillResult as ServerResult};
use regex::Regex;
use toml_edit::{Array, DocumentMut, Item, Value as TomlValue};
use tracing::debug;

/// pyproject.toml manifest wrapper
pub struct PyProjectManifest {
    doc: DocumentMut,
}

impl ManifestOps for PyProjectManifest {
    fn parse(content: &str) -> ServerResult<Self> {
        let doc = content
            .parse::<DocumentMut>()
            .map_err(|e| ServerError::parse(format!("Failed to parse pyproject.toml: {}", e)))?;
        Ok(Self { doc })
    }

    fn sections() -> &'static [&'static str] {
        &[
            "dependencies",
            "optional-dependencies",
            "dev-dependencies", // PDM style
            "dev",              // Poetry dev group
        ]
    }

    fn default_section() -> &'static str {
        "dependencies"
    }

    fn find_dependency(&self, name: &str) -> Option<(&'static str, DependencyInfo)> {
        // Check [project.dependencies] (PEP 621 standard)
        if let Some(deps) = self
            .doc
            .get("project")
            .and_then(|p| p.get("dependencies"))
            .and_then(|d| d.as_array())
        {
            if let Some(info) = find_in_pep_508_array(deps, name) {
                return Some(("dependencies", info));
            }
        }

        // Check [project.optional-dependencies] (all groups)
        if let Some(opt_deps) = self
            .doc
            .get("project")
            .and_then(|p| p.get("optional-dependencies"))
            .and_then(|d| d.as_table())
        {
            for (group_name, group_deps) in opt_deps.iter() {
                if let Some(arr) = group_deps.as_array() {
                    if let Some(mut info) = find_in_pep_508_array(arr, name) {
                        // Tag with the optional group
                        info.features = Some(vec![group_name.to_string()]);
                        return Some(("optional-dependencies", info));
                    }
                }
            }
        }

        // Check [tool.pdm.dev-dependencies] (PDM style)
        if let Some(pdm_dev) = self
            .doc
            .get("tool")
            .and_then(|t| t.get("pdm"))
            .and_then(|p| p.get("dev-dependencies"))
            .and_then(|d| d.as_table())
        {
            for (_group_name, group_deps) in pdm_dev.iter() {
                if let Some(arr) = group_deps.as_array() {
                    if let Some(info) = find_in_pep_508_array(arr, name) {
                        return Some(("dev-dependencies", info));
                    }
                }
            }
        }

        // Check [tool.poetry.dependencies] (Poetry style)
        if let Some(poetry_deps) = self
            .doc
            .get("tool")
            .and_then(|t| t.get("poetry"))
            .and_then(|p| p.get("dependencies"))
            .and_then(|d| d.as_table())
        {
            if let Some(dep) = poetry_deps.get(name) {
                let info = extract_poetry_dep_info(name, dep);
                return Some(("dependencies", info));
            }
        }

        // Check [tool.poetry.dev-dependencies] (Poetry style)
        if let Some(poetry_dev) = self
            .doc
            .get("tool")
            .and_then(|t| t.get("poetry"))
            .and_then(|p| p.get("dev-dependencies"))
            .and_then(|d| d.as_table())
        {
            if let Some(dep) = poetry_dev.get(name) {
                let info = extract_poetry_dep_info(name, dep);
                return Some(("dev-dependencies", info));
            }
        }

        None
    }

    fn has_dependency(&self, section: &str, name: &str) -> bool {
        match section {
            "dependencies" => {
                // Check [project.dependencies]
                if let Some(deps) = self
                    .doc
                    .get("project")
                    .and_then(|p| p.get("dependencies"))
                    .and_then(|d| d.as_array())
                {
                    if find_in_pep_508_array(deps, name).is_some() {
                        return true;
                    }
                }

                // Check [tool.poetry.dependencies]
                if let Some(poetry_deps) = self
                    .doc
                    .get("tool")
                    .and_then(|t| t.get("poetry"))
                    .and_then(|p| p.get("dependencies"))
                    .and_then(|d| d.as_table())
                {
                    if poetry_deps.get(name).is_some() {
                        return true;
                    }
                }

                false
            }
            "optional-dependencies" => {
                if let Some(opt_deps) = self
                    .doc
                    .get("project")
                    .and_then(|p| p.get("optional-dependencies"))
                    .and_then(|d| d.as_table())
                {
                    for (_group_name, group_deps) in opt_deps.iter() {
                        if let Some(arr) = group_deps.as_array() {
                            if find_in_pep_508_array(arr, name).is_some() {
                                return true;
                            }
                        }
                    }
                }
                false
            }
            "dev-dependencies" | "dev" => {
                // Check PDM dev-dependencies
                if let Some(pdm_dev) = self
                    .doc
                    .get("tool")
                    .and_then(|t| t.get("pdm"))
                    .and_then(|p| p.get("dev-dependencies"))
                    .and_then(|d| d.as_table())
                {
                    for (_group_name, group_deps) in pdm_dev.iter() {
                        if let Some(arr) = group_deps.as_array() {
                            if find_in_pep_508_array(arr, name).is_some() {
                                return true;
                            }
                        }
                    }
                }

                // Check Poetry dev-dependencies
                if let Some(poetry_dev) = self
                    .doc
                    .get("tool")
                    .and_then(|t| t.get("poetry"))
                    .and_then(|p| p.get("dev-dependencies"))
                    .and_then(|d| d.as_table())
                {
                    if poetry_dev.get(name).is_some() {
                        return true;
                    }
                }

                false
            }
            _ => false,
        }
    }

    fn add_dependency(
        &mut self,
        section: &str,
        name: &str,
        info: &DependencyInfo,
    ) -> ServerResult<()> {
        match section {
            "dependencies" => {
                // Add to [project.dependencies] (PEP 621 standard)
                ensure_project_section(&mut self.doc)?;

                let project = self.doc["project"]
                    .as_table_mut()
                    .ok_or_else(|| ServerError::parse("[project] is not a table"))?;

                if !project.contains_key("dependencies") {
                    project["dependencies"] = Item::Value(TomlValue::Array(Array::new()));
                }

                let deps = project["dependencies"]
                    .as_array_mut()
                    .ok_or_else(|| ServerError::parse("[project.dependencies] is not an array"))?;

                // Build PEP 508 dependency string
                let dep_string = build_pep508_string(name, info);
                deps.push(dep_string.as_str());

                debug!(dependency = %name, section = "project.dependencies", "Added dependency to pyproject.toml");
            }
            "optional-dependencies" => {
                // Add to [project.optional-dependencies.dev] by default
                ensure_project_section(&mut self.doc)?;

                let project = self.doc["project"]
                    .as_table_mut()
                    .ok_or_else(|| ServerError::parse("[project] is not a table"))?;

                if !project.contains_key("optional-dependencies") {
                    project["optional-dependencies"] = Item::Table(toml_edit::Table::new());
                }

                let opt_deps =
                    project["optional-dependencies"]
                        .as_table_mut()
                        .ok_or_else(|| {
                            ServerError::parse("[project.optional-dependencies] is not a table")
                        })?;

                // Use "dev" as default group or get from features
                let group = info
                    .features
                    .as_ref()
                    .and_then(|f| f.first())
                    .map(|s| s.as_str())
                    .unwrap_or("dev");

                if !opt_deps.contains_key(group) {
                    opt_deps[group] = Item::Value(TomlValue::Array(Array::new()));
                }

                let group_deps = opt_deps[group].as_array_mut().ok_or_else(|| {
                    ServerError::parse(format!(
                        "[project.optional-dependencies.{}] is not an array",
                        group
                    ))
                })?;

                let dep_string = build_pep508_string(name, info);
                group_deps.push(dep_string.as_str());

                debug!(dependency = %name, section = %format!("project.optional-dependencies.{}", group), "Added dependency to pyproject.toml");
            }
            "dev-dependencies" | "dev" => {
                // Try to add to [tool.pdm.dev-dependencies.dev] or create it
                ensure_tool_section(&mut self.doc, "pdm")?;

                let pdm = self.doc["tool"]["pdm"]
                    .as_table_mut()
                    .ok_or_else(|| ServerError::parse("[tool.pdm] is not a table"))?;

                if !pdm.contains_key("dev-dependencies") {
                    pdm["dev-dependencies"] = Item::Table(toml_edit::Table::new());
                }

                let dev_deps = pdm["dev-dependencies"].as_table_mut().ok_or_else(|| {
                    ServerError::parse("[tool.pdm.dev-dependencies] is not a table")
                })?;

                // Use "dev" as the default group
                if !dev_deps.contains_key("dev") {
                    dev_deps["dev"] = Item::Value(TomlValue::Array(Array::new()));
                }

                let dev_group = dev_deps["dev"].as_array_mut().ok_or_else(|| {
                    ServerError::parse("[tool.pdm.dev-dependencies.dev] is not an array")
                })?;

                let dep_string = build_pep508_string(name, info);
                dev_group.push(dep_string.as_str());

                debug!(dependency = %name, section = "tool.pdm.dev-dependencies.dev", "Added dependency to pyproject.toml");
            }
            _ => {
                return Err(ServerError::invalid_request(format!(
                    "Unknown section: {}",
                    section
                )));
            }
        }

        Ok(())
    }

    fn serialize(&self) -> String {
        self.doc.to_string()
    }
}

/// Ensure [project] section exists
fn ensure_project_section(doc: &mut DocumentMut) -> ServerResult<()> {
    if !doc.contains_key("project") {
        doc["project"] = Item::Table(toml_edit::Table::new());
    }
    Ok(())
}

/// Ensure [tool.<name>] section exists
fn ensure_tool_section(doc: &mut DocumentMut, tool_name: &str) -> ServerResult<()> {
    if !doc.contains_key("tool") {
        doc["tool"] = Item::Table(toml_edit::Table::new());
    }

    let tool = doc["tool"]
        .as_table_mut()
        .ok_or_else(|| ServerError::parse("[tool] is not a table"))?;

    if !tool.contains_key(tool_name) {
        tool[tool_name] = Item::Table(toml_edit::Table::new());
    }

    Ok(())
}

/// Find a dependency in a PEP 508 style array (list of strings like "requests>=2.0")
fn find_in_pep_508_array(arr: &Array, name: &str) -> Option<DependencyInfo> {
    // Regex to parse PEP 508 dependency specifier
    // Handles: name, name>=version, name[extra1,extra2]>=version, etc.
    let re = Regex::new(r"^([a-zA-Z0-9][-a-zA-Z0-9._]*)(?:\[([^\]]+)\])?(.*)$").ok()?;

    for item in arr.iter() {
        if let Some(dep_str) = item.as_str() {
            if let Some(caps) = re.captures(dep_str) {
                let dep_name = caps.get(1)?.as_str();

                // Normalize name comparison (PEP 503: case-insensitive, underscores = hyphens)
                if normalize_package_name(dep_name) == normalize_package_name(name) {
                    let extras = caps.get(2).map(|m| {
                        m.as_str()
                            .split(',')
                            .map(|s| s.trim().to_string())
                            .collect::<Vec<_>>()
                    });

                    let version_spec = caps.get(3).map(|m| m.as_str().trim()).unwrap_or("");

                    return Some(DependencyInfo {
                        name: dep_name.to_string(),
                        version: if version_spec.is_empty() {
                            "*".to_string()
                        } else {
                            version_spec.to_string()
                        },
                        features: extras,
                        optional: None,
                        already_exists: None,
                    });
                }
            }
        }
    }

    None
}

/// Normalize package name per PEP 503
fn normalize_package_name(name: &str) -> String {
    name.to_lowercase().replace(['-', '_', '.'], "-")
}

/// Extract dependency info from Poetry-style table entry
fn extract_poetry_dep_info(name: &str, item: &Item) -> DependencyInfo {
    let mut version = String::new();
    let mut features = None;
    let optional = None;

    match item {
        Item::Value(TomlValue::String(v)) => {
            // Simple version string: "^1.0"
            version = v.value().to_string();
        }
        Item::Value(TomlValue::InlineTable(table)) => {
            // Inline table: { version = "1.0", extras = ["full"] }
            if let Some(v) = table.get("version").and_then(|v| v.as_str()) {
                version = v.to_string();
            } else if let Some(path) = table.get("path").and_then(|v| v.as_str()) {
                version = format!("path:{}", path);
            } else if let Some(git) = table.get("git").and_then(|v| v.as_str()) {
                version = format!("git:{}", git);
            }

            if let Some(extras) = table.get("extras").and_then(|v| v.as_array()) {
                let extras_list: Vec<String> = extras
                    .iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect();
                if !extras_list.is_empty() {
                    features = Some(extras_list);
                }
            }
        }
        Item::Table(table) => {
            // Regular table (multi-line)
            if let Some(v) = table.get("version").and_then(|v| v.as_str()) {
                version = v.to_string();
            } else if let Some(path) = table.get("path").and_then(|v| v.as_str()) {
                version = format!("path:{}", path);
            } else if let Some(git) = table.get("git").and_then(|v| v.as_str()) {
                version = format!("git:{}", git);
            }

            if let Some(extras) = table.get("extras").and_then(|v| v.as_array()) {
                let extras_list: Vec<String> = extras
                    .iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect();
                if !extras_list.is_empty() {
                    features = Some(extras_list);
                }
            }
        }
        _ => {
            version = "*".to_string();
        }
    }

    DependencyInfo {
        name: name.to_string(),
        version,
        features,
        optional,
        already_exists: None,
    }
}

/// Build a PEP 508 dependency string
fn build_pep508_string(name: &str, info: &DependencyInfo) -> String {
    let mut dep_string = name.to_string();

    // Add extras if present
    if let Some(features) = &info.features {
        if !features.is_empty() {
            dep_string.push('[');
            dep_string.push_str(&features.join(","));
            dep_string.push(']');
        }
    }

    // Add version specifier if present and not "*"
    if !info.version.is_empty() && info.version != "*" {
        // If version doesn't start with an operator, assume >=
        let version = &info.version;
        if version.starts_with(">=")
            || version.starts_with("<=")
            || version.starts_with("==")
            || version.starts_with("!=")
            || version.starts_with("~=")
            || version.starts_with('^')
            || version.starts_with('>')
            || version.starts_with('<')
        {
            dep_string.push_str(version);
        } else {
            dep_string.push_str(">=");
            dep_string.push_str(version);
        }
    }

    dep_string
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_dependency_pep621() {
        let content = r#"
[project]
dependencies = [
    "requests>=2.28.0",
    "click>=8.0",
]
"#;
        let manifest = PyProjectManifest::parse(content).unwrap();
        let result = manifest.find_dependency("requests");
        assert!(result.is_some());
        let (section, info) = result.unwrap();
        assert_eq!(section, "dependencies");
        assert_eq!(info.name, "requests");
        assert_eq!(info.version, ">=2.28.0");
    }

    #[test]
    fn test_find_dependency_with_extras() {
        let content = r#"
[project]
dependencies = [
    "httpx[http2,socks]>=0.24.0",
]
"#;
        let manifest = PyProjectManifest::parse(content).unwrap();
        let result = manifest.find_dependency("httpx");
        assert!(result.is_some());
        let (_, info) = result.unwrap();
        assert_eq!(info.version, ">=0.24.0");
        assert_eq!(
            info.features,
            Some(vec!["http2".to_string(), "socks".to_string()])
        );
    }

    #[test]
    fn test_find_dependency_optional() {
        let content = r#"
[project.optional-dependencies]
dev = [
    "pytest>=7.0",
    "black>=23.0",
]
"#;
        let manifest = PyProjectManifest::parse(content).unwrap();
        let result = manifest.find_dependency("pytest");
        assert!(result.is_some());
        let (section, info) = result.unwrap();
        assert_eq!(section, "optional-dependencies");
        assert_eq!(info.features, Some(vec!["dev".to_string()]));
    }

    #[test]
    fn test_find_dependency_poetry() {
        let content = r#"
[tool.poetry.dependencies]
python = "^3.9"
requests = "^2.28"
"#;
        let manifest = PyProjectManifest::parse(content).unwrap();
        let result = manifest.find_dependency("requests");
        assert!(result.is_some());
        let (section, info) = result.unwrap();
        assert_eq!(section, "dependencies");
        assert_eq!(info.version, "^2.28");
    }

    #[test]
    fn test_add_dependency() {
        let content = r#"
[project]
name = "my-project"
dependencies = [
    "existing>=1.0",
]
"#;
        let mut manifest = PyProjectManifest::parse(content).unwrap();
        let info = DependencyInfo {
            name: "new-dep".to_string(),
            version: ">=2.0".to_string(),
            features: None,
            optional: None,
            already_exists: None,
        };
        manifest
            .add_dependency("dependencies", "new-dep", &info)
            .unwrap();
        let serialized = manifest.serialize();
        assert!(serialized.contains("new-dep>=2.0"));
    }

    #[test]
    fn test_has_dependency() {
        let content = r#"
[project]
dependencies = [
    "requests>=2.0",
]
"#;
        let manifest = PyProjectManifest::parse(content).unwrap();
        assert!(manifest.has_dependency("dependencies", "requests"));
        assert!(!manifest.has_dependency("dependencies", "click"));
        assert!(!manifest.has_dependency("dev-dependencies", "requests"));
    }

    #[test]
    fn test_normalize_package_name() {
        assert_eq!(normalize_package_name("Requests"), "requests");
        assert_eq!(normalize_package_name("my_package"), "my-package");
        assert_eq!(normalize_package_name("My.Package"), "my-package");
        assert_eq!(
            normalize_package_name("some-package"),
            normalize_package_name("some_package")
        );
    }

    #[test]
    fn test_build_pep508_string() {
        let info = DependencyInfo {
            name: "requests".to_string(),
            version: ">=2.28.0".to_string(),
            features: None,
            optional: None,
            already_exists: None,
        };
        assert_eq!(build_pep508_string("requests", &info), "requests>=2.28.0");

        let info_with_extras = DependencyInfo {
            name: "httpx".to_string(),
            version: ">=0.24".to_string(),
            features: Some(vec!["http2".to_string(), "socks".to_string()]),
            optional: None,
            already_exists: None,
        };
        assert_eq!(
            build_pep508_string("httpx", &info_with_extras),
            "httpx[http2,socks]>=0.24"
        );
    }

    #[test]
    fn test_sections() {
        assert!(PyProjectManifest::sections().contains(&"dependencies"));
        assert!(PyProjectManifest::sections().contains(&"optional-dependencies"));
        assert!(PyProjectManifest::sections().contains(&"dev-dependencies"));
    }

    #[test]
    fn test_default_section() {
        assert_eq!(PyProjectManifest::default_section(), "dependencies");
    }
}
