//! Python manifest parsing and manipulation
//!
//! Supports multiple Python dependency specification formats:
//! - requirements.txt (pip)
//! - pyproject.toml (Poetry, PDM, setuptools)
//! - setup.py (legacy setuptools)
//! - Pipfile (Pipenv)
use mill_lang_common::read_manifest;
use mill_plugin_api::{Dependency, DependencySource, ManifestData, PluginError, PluginResult};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::path::Path;
use tracing::{debug, warn};
/// Parse requirements.txt file
///
/// Format: package==version or package>=version or package
pub async fn parse_requirements_txt(path: &Path) -> PluginResult<ManifestData> {
    let content = read_manifest(path).await?;
    let mut dependencies = Vec::new();
    let dev_dependencies = Vec::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((name, version)) = parse_requirement_line(line) {
            dependencies.push(Dependency {
                name,
                source: DependencySource::Version(version),
            });
        }
    }
    debug!(
        dependencies_count = dependencies.len(), path = ? path, "Parsed requirements.txt"
    );
    Ok(ManifestData {
        name: path
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("python-project")
            .to_string(),
        version: "0.1.0".to_string(),
        dependencies,
        dev_dependencies,
        raw_data: json!({ "format" : "requirements.txt" }),
    })
}
/// Parse a single requirement line
///
/// Supports: package==1.0.0, package>=1.0.0, package, package[extras]
fn parse_requirement_line(line: &str) -> Option<(String, String)> {
    let line = line.trim();
    let clean_line = if let Some(bracket_pos) = line.find('[') {
        if let Some(bracket_end) = line.find(']') {
            format!("{}{}", &line[..bracket_pos], &line[bracket_end + 1..])
        } else {
            line.to_string()
        }
    } else {
        line.to_string()
    };
    for sep in &["==", ">=", "<=", "~=", ">", "<", "!="] {
        if let Some(pos) = clean_line.find(sep) {
            let name = clean_line[..pos].trim().to_string();
            let version = clean_line[pos + sep.len()..].trim().to_string();
            return Some((name, version));
        }
    }
    if !clean_line.is_empty() {
        Some((clean_line.trim().to_string(), "*".to_string()))
    } else {
        None
    }
}
/// Parse pyproject.toml file
///
/// Supports both Poetry and PDM formats
pub async fn parse_pyproject_toml(path: &Path) -> PluginResult<ManifestData> {
    let content = read_manifest(path).await?;
    let toml: PyProjectToml = toml::from_str(&content)
        .map_err(|e| PluginError::parse(format!("Failed to parse pyproject.toml: {}", e)))?;
    let name = toml
        .project
        .as_ref()
        .and_then(|p| p.name.clone())
        .or_else(|| {
            toml.tool
                .as_ref()
                .and_then(|t| t.poetry.as_ref().and_then(|p| p.name.clone()))
        })
        .unwrap_or_else(|| "python-project".to_string());
    let version = toml
        .project
        .as_ref()
        .and_then(|p| p.version.clone())
        .or_else(|| {
            toml.tool
                .as_ref()
                .and_then(|t| t.poetry.as_ref().and_then(|p| p.version.clone()))
        })
        .unwrap_or_else(|| "0.1.0".to_string());
    let mut dependencies = Vec::new();
    let mut dev_dependencies = Vec::new();
    if let Some(tool) = &toml.tool {
        if let Some(poetry) = &tool.poetry {
            if let Some(deps) = &poetry.dependencies {
                for (name, spec) in deps {
                    if name != "python" {
                        let version = dependency_spec_to_version(spec);
                        dependencies.push(Dependency {
                            name: name.clone(),
                            source: DependencySource::Version(version),
                        });
                    }
                }
            }
            if let Some(dev_deps) = &poetry.dev_dependencies {
                for (name, spec) in dev_deps {
                    let version = dependency_spec_to_version(spec);
                    dev_dependencies.push(Dependency {
                        name: name.clone(),
                        source: DependencySource::Version(version),
                    });
                }
            }
        }
    }
    if dependencies.is_empty() {
        if let Some(project) = &toml.project {
            if let Some(deps) = &project.dependencies {
                for dep in deps {
                    if let Some((name, version)) = parse_requirement_line(dep) {
                        dependencies.push(Dependency {
                            name,
                            source: DependencySource::Version(version),
                        });
                    }
                }
            }
        }
    }
    debug!(
        name = % name, version = % version, dependencies_count = dependencies.len(),
        dev_dependencies_count = dev_dependencies.len(), "Parsed pyproject.toml"
    );
    Ok(ManifestData {
        name,
        version,
        dependencies,
        dev_dependencies,
        raw_data: json!({ "format" : "pyproject.toml" }),
    })
}
/// Convert Poetry/PDM dependency spec to version string
fn dependency_spec_to_version(spec: &DependencySpec) -> String {
    match spec {
        DependencySpec::Simple(version) => version.clone(),
        DependencySpec::Detailed(details) => {
            details.version.clone().unwrap_or_else(|| "*".to_string())
        }
    }
}
/// Parse setup.py file
///
/// Extracts basic metadata and dependencies from setup.py
/// Note: This is a best-effort parser for common setup.py patterns
pub async fn parse_setup_py(path: &Path) -> PluginResult<ManifestData> {
    let content = read_manifest(path).await?;
    let mut name = "python-project".to_string();
    let mut version = "0.1.0".to_string();
    let mut dependencies = Vec::new();
    let mut dev_dependencies = Vec::new();
    if let Some(name_match) = regex::Regex::new(r#"name\s*=\s*["']([^"']+)["']"#)
        .unwrap()
        .captures(&content)
    {
        if let Some(n) = name_match.get(1) {
            name = n.as_str().to_string();
        }
    }
    if let Some(version_match) = regex::Regex::new(r#"version\s*=\s*["']([^"']+)["']"#)
        .unwrap()
        .captures(&content)
    {
        if let Some(v) = version_match.get(1) {
            version = v.as_str().to_string();
        }
    }
    if let Some(install_requires) = extract_list_from_setup(&content, "install_requires") {
        for dep in install_requires {
            if let Some((dep_name, dep_version)) = parse_requirement_line(&dep) {
                dependencies.push(Dependency {
                    name: dep_name,
                    source: DependencySource::Version(dep_version),
                });
            }
        }
    }
    if let Some(extras_require) = extract_list_from_setup(&content, "extras_require") {
        for dep in extras_require {
            if let Some((dep_name, dep_version)) = parse_requirement_line(&dep) {
                dev_dependencies.push(Dependency {
                    name: dep_name,
                    source: DependencySource::Version(dep_version),
                });
            }
        }
    }
    debug!(
        name = % name, version = % version, dependencies_count = dependencies.len(),
        dev_dependencies_count = dev_dependencies.len(), "Parsed setup.py"
    );
    Ok(ManifestData {
        name,
        version,
        dependencies,
        dev_dependencies,
        raw_data: json!({ "format" : "setup.py" }),
    })
}
/// Extract list values from setup() call
fn extract_list_from_setup(content: &str, key: &str) -> Option<Vec<String>> {
    let pattern = format!(r#"{}\s*=\s*\[(.*?)\]"#, regex::escape(key));
    let re = regex::Regex::new(&pattern).ok()?;
    let captures = re.captures(content)?;
    let list_content = captures.get(1)?.as_str();
    let mut items = Vec::new();
    for item in list_content.split(',') {
        let item = item.trim();
        let item = item.trim_matches(|c| c == '"' || c == '\'');
        if !item.is_empty() {
            items.push(item.to_string());
        }
    }
    Some(items)
}
/// Parse Pipfile
///
/// Pipfile uses TOML-like format for dependency management
pub async fn parse_pipfile(path: &Path) -> PluginResult<ManifestData> {
    let content = read_manifest(path).await?;
    let pipfile: PipfileFormat = toml::from_str(&content)
        .map_err(|e| PluginError::parse(format!("Failed to parse Pipfile: {}", e)))?;
    let name = path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("python-project")
        .to_string();
    let version = "0.1.0".to_string();
    let mut dependencies = Vec::new();
    let mut dev_dependencies = Vec::new();
    if let Some(packages) = pipfile.packages {
        for (name, spec) in packages {
            let version = pipfile_spec_to_version(&spec);
            dependencies.push(Dependency {
                name,
                source: DependencySource::Version(version),
            });
        }
    }
    if let Some(dev_packages) = pipfile.dev_packages {
        for (name, spec) in dev_packages {
            let version = pipfile_spec_to_version(&spec);
            dev_dependencies.push(Dependency {
                name,
                source: DependencySource::Version(version),
            });
        }
    }
    debug!(
        name = % name, version = % version, dependencies_count = dependencies.len(),
        dev_dependencies_count = dev_dependencies.len(), "Parsed Pipfile"
    );
    Ok(ManifestData {
        name,
        version,
        dependencies,
        dev_dependencies,
        raw_data: json!({ "format" : "Pipfile" }),
    })
}
/// Convert Pipfile dependency spec to version string
fn pipfile_spec_to_version(spec: &PipfileSpec) -> String {
    match spec {
        PipfileSpec::Simple(version) => version.clone(),
        PipfileSpec::Detailed(details) => {
            if let Some(version) = &details.version {
                version.clone()
            } else {
                "*".to_string()
            }
        }
    }
}
/// Update a dependency in requirements.txt
pub async fn update_requirements_txt(
    path: &Path,
    old_name: &str,
    new_name: &str,
    new_version: Option<&str>,
) -> PluginResult<String> {
    let content = read_manifest(path).await?;
    let mut lines = Vec::new();
    let mut updated = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some((name, _version)) = parse_requirement_line(trimmed) {
            if name == old_name {
                let new_line = if let Some(ver) = new_version {
                    format!("{}=={}", new_name, ver)
                } else {
                    new_name.to_string()
                };
                lines.push(new_line);
                updated = true;
                continue;
            }
        }
        lines.push(line.to_string());
    }
    if !updated {
        warn!(
            old_name = % old_name, path = ? path,
            "Dependency not found in requirements.txt"
        );
    }
    Ok(lines.join("\n") + "\n")
}
/// Update a dependency in pyproject.toml
pub async fn update_pyproject_toml(
    path: &Path,
    old_name: &str,
    new_name: &str,
    new_version: Option<&str>,
) -> PluginResult<String> {
    let content = read_manifest(path).await?;
    let mut toml: toml::Value = toml::from_str(&content)
        .map_err(|e| PluginError::parse(format!("Failed to parse pyproject.toml: {}", e)))?;
    let mut updated = false;
    if let Some(tool) = toml.get_mut("tool") {
        if let Some(poetry) = tool.get_mut("poetry") {
            if let Some(deps) = poetry.get_mut("dependencies") {
                if let Some(deps_table) = deps.as_table_mut() {
                    if deps_table.contains_key(old_name) {
                        deps_table.remove(old_name);
                        let new_value = if let Some(ver) = new_version {
                            toml::Value::String(format!("^{}", ver))
                        } else {
                            toml::Value::String("*".to_string())
                        };
                        deps_table.insert(new_name.to_string(), new_value);
                        updated = true;
                    }
                }
            }
            if let Some(dev_deps) = poetry.get_mut("dev-dependencies") {
                if let Some(dev_deps_table) = dev_deps.as_table_mut() {
                    if dev_deps_table.contains_key(old_name) {
                        dev_deps_table.remove(old_name);
                        let new_value = if let Some(ver) = new_version {
                            toml::Value::String(format!("^{}", ver))
                        } else {
                            toml::Value::String("*".to_string())
                        };
                        dev_deps_table.insert(new_name.to_string(), new_value);
                        updated = true;
                    }
                }
            }
        }
    }
    if !updated {
        if let Some(project) = toml.get_mut("project") {
            if let Some(deps) = project.get_mut("dependencies") {
                if let Some(deps_array) = deps.as_array_mut() {
                    for dep in deps_array.iter_mut() {
                        if let Some(dep_str) = dep.as_str() {
                            if let Some((name, _)) = parse_requirement_line(dep_str) {
                                if name == old_name {
                                    *dep = if let Some(ver) = new_version {
                                        toml::Value::String(format!("{}=={}", new_name, ver))
                                    } else {
                                        toml::Value::String(new_name.to_string())
                                    };
                                    updated = true;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    if !updated {
        warn!(
            old_name = % old_name, path = ? path,
            "Dependency not found in pyproject.toml"
        );
    }
    toml::to_string(&toml)
        .map_err(|e| PluginError::internal(format!("Failed to serialize pyproject.toml: {}", e)))
}

/// Update a dependency in pyproject.toml
pub fn update_dependency_in_pyproject(
    manifest_path: &Path,
    dep_name: &str,
    new_version: &str,
    section: Option<&str>,
) -> Result<String, String> {
    use std::fs;

    let content = fs::read_to_string(manifest_path)
        .map_err(|e| format!("Failed to read manifest: {}", e))?;

    let mut doc: toml_edit::DocumentMut = content
        .parse()
        .map_err(|e| format!("Failed to parse TOML: {}", e))?;

    // Determine which section to update
    let section_name = section.unwrap_or("dependencies");

    // Navigate to [project.dependencies] or [project.dev-dependencies]
    if let Some(project) = doc.get_mut("project").and_then(|p| p.as_table_mut()) {
        if let Some(deps) = project.get_mut(section_name).and_then(|d| d.as_table_mut()) {
            // Update the dependency version
            deps.insert(dep_name, toml_edit::value(new_version));
        } else {
            // Section doesn't exist, create it
            let mut new_deps = toml_edit::Table::new();
            new_deps.insert(dep_name, toml_edit::value(new_version));
            project.insert(section_name, toml_edit::Item::Table(new_deps));
        }
    } else {
        return Err("pyproject.toml missing [project] section".to_string());
    }

    Ok(doc.to_string())
}

/// Generate a basic pyproject.toml manifest
pub fn generate_pyproject_toml(package_name: &str, dependencies: &[String]) -> String {
    let mut doc = toml_edit::DocumentMut::new();

    // [project] section
    let mut project = toml_edit::Table::new();
    project.insert("name", toml_edit::value(package_name));
    project.insert("version", toml_edit::value("0.1.0"));
    project.insert("description", toml_edit::value(""));

    // [project.dependencies]
    let mut deps = toml_edit::Table::new();
    for dep in dependencies {
        // Parse "name==version" or just "name"
        if let Some((name, version)) = dep.split_once("==") {
            deps.insert(name, toml_edit::value(version));
        } else {
            deps.insert(dep, toml_edit::value("*")); // Latest version
        }
    }
    project.insert("dependencies", toml_edit::Item::Table(deps));

    doc.insert("project", toml_edit::Item::Table(project));

    // [build-system]
    let mut build_system = toml_edit::Table::new();
    build_system.insert(
        "requires",
        toml_edit::value(toml_edit::Array::from_iter(vec!["setuptools", "wheel"])),
    );
    build_system.insert("build-backend", toml_edit::value("setuptools.build_meta"));
    doc.insert("build-system", toml_edit::Item::Table(build_system));

    doc.to_string()
}

#[derive(Debug, Deserialize, Serialize)]
struct PyProjectToml {
    #[serde(skip_serializing_if = "Option::is_none")]
    project: Option<ProjectMetadata>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool: Option<ToolConfig>,
}
#[derive(Debug, Deserialize, Serialize)]
struct ProjectMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    dependencies: Option<Vec<String>>,
}
#[derive(Debug, Deserialize, Serialize)]
struct ToolConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    poetry: Option<PoetryConfig>,
}
#[derive(Debug, Deserialize, Serialize)]
struct PoetryConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    dependencies: Option<HashMap<String, DependencySpec>>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "dev-dependencies")]
    dev_dependencies: Option<HashMap<String, DependencySpec>>,
}
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(untagged)]
enum DependencySpec {
    Simple(String),
    Detailed(DetailedDependency),
}
#[derive(Debug, Deserialize, Serialize, Clone)]
struct DetailedDependency {
    #[serde(skip_serializing_if = "Option::is_none")]
    version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    git: Option<String>,
}
#[derive(Debug, Deserialize, Serialize)]
struct PipfileFormat {
    #[serde(skip_serializing_if = "Option::is_none")]
    packages: Option<HashMap<String, PipfileSpec>>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "dev-packages")]
    dev_packages: Option<HashMap<String, PipfileSpec>>,
}
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(untagged)]
enum PipfileSpec {
    Simple(String),
    Detailed(PipfileDetailedDependency),
}
#[derive(Debug, Deserialize, Serialize, Clone)]
struct PipfileDetailedDependency {
    #[serde(skip_serializing_if = "Option::is_none")]
    version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    git: Option<String>,
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;
    #[test]
    fn test_parse_requirement_line() {
        assert_eq!(
            parse_requirement_line("django==3.2.0"),
            Some(("django".to_string(), "3.2.0".to_string()))
        );
        assert_eq!(
            parse_requirement_line("requests>=2.25.1"),
            Some(("requests".to_string(), "2.25.1".to_string()))
        );
        assert_eq!(
            parse_requirement_line("flask"),
            Some(("flask".to_string(), "*".to_string()))
        );
        assert_eq!(
            parse_requirement_line("pytest[testing]==7.0.0"),
            Some(("pytest".to_string(), "7.0.0".to_string()))
        );
    }
    #[tokio::test]
    async fn test_parse_requirements_txt() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            "# Test requirements\ndjango==3.2.0\nrequests>=2.25.1\nflask"
        )
        .unwrap();
        let manifest = parse_requirements_txt(file.path()).await.unwrap();
        assert!(manifest.dependencies.iter().any(|d| d.name == "django"));
        let django_dep = manifest
            .dependencies
            .iter()
            .find(|d| d.name == "django")
            .unwrap();
        match &django_dep.source {
            DependencySource::Version(v) => assert_eq!(v, "3.2.0"),
            _ => panic!("Expected version source"),
        }
        assert!(manifest.dependencies.iter().any(|d| d.name == "requests"));
        assert!(manifest.dependencies.iter().any(|d| d.name == "flask"));
    }
    #[tokio::test]
    async fn test_parse_pyproject_toml_poetry() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            r#"
[tool.poetry]
name = "my-project"
version = "1.0.0"

[tool.poetry.dependencies]
python = "^3.9"
django = "^3.2.0"
requests = ">=2.25.1"

[tool.poetry.dev-dependencies]
pytest = "^7.0.0"
"#
        )
        .unwrap();
        let manifest = parse_pyproject_toml(file.path()).await.unwrap();
        assert_eq!(manifest.name, "my-project");
        assert_eq!(manifest.version, "1.0.0");
        assert!(manifest.dependencies.iter().any(|d| d.name == "django"));
        assert!(manifest.dependencies.iter().any(|d| d.name == "requests"));
        assert!(manifest.dev_dependencies.iter().any(|d| d.name == "pytest"));
    }
    #[tokio::test]
    async fn test_update_requirements_txt() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "django==3.2.0\nrequests>=2.25.1").unwrap();
        let updated = update_requirements_txt(file.path(), "django", "django", Some("4.0.0"))
            .await
            .unwrap();
        assert!(updated.contains("django==4.0.0"));
        assert!(updated.contains("requests>=2.25.1"));
    }
    #[tokio::test]
    async fn test_update_pyproject_toml() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            r#"
[tool.poetry.dependencies]
django = "^3.2.0"
requests = "^2.25.1"
"#
        )
        .unwrap();
        let updated = update_pyproject_toml(file.path(), "django", "django", Some("4.0.0"))
            .await
            .unwrap();
        assert!(updated.contains("django"));
        assert!(updated.contains("4.0"));
    }
}
