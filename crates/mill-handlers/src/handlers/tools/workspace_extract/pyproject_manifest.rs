//! pyproject.toml manifest implementation
//!
//! Implements `ManifestOps` trait for Python projects using pyproject.toml.
//! Supports both PEP 621 standard format and Poetry format.
//!
//! # Supported Formats
//!
//! ## PEP 621 (Modern Standard)
//! ```toml
//! [project]
//! name = "my-package"
//! version = "1.0.0"
//! dependencies = [
//!     "requests>=2.28.0",
//!     "click>=8.0.0",
//! ]
//!
//! [project.optional-dependencies]
//! dev = ["pytest>=7.0.0", "black"]
//! ```
//!
//! ## Poetry Format
//! ```toml
//! [tool.poetry]
//! name = "my-package"
//! version = "1.0.0"
//!
//! [tool.poetry.dependencies]
//! python = "^3.9"
//! requests = "^2.28.0"
//! click = "^8.0.0"
//!
//! [tool.poetry.dev-dependencies]
//! pytest = "^7.0.0"
//! ```

use super::{DependencyInfo, ManifestOps};
use mill_foundation::errors::{MillError as ServerError, MillResult as ServerResult};
use toml_edit::{Array, DocumentMut, Formatted, Item, Table, Value as TomlValue};
use tracing::debug;

/// pyproject.toml manifest wrapper
///
/// Supports both PEP 621 and Poetry formats for Python dependency management.
pub struct PyProjectManifest {
    doc: DocumentMut,
    format: PyProjectFormat,
}

/// Detected format of the pyproject.toml
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PyProjectFormat {
    /// PEP 621 standard format using [project] section
    Pep621,
    /// Poetry format using [tool.poetry] section
    Poetry,
    /// Unknown or empty format
    Unknown,
}

impl PyProjectManifest {
    /// Detect the format being used in the pyproject.toml
    fn detect_format(doc: &DocumentMut) -> PyProjectFormat {
        // Check for Poetry format first (more specific)
        if doc
            .get("tool")
            .and_then(|t| t.get("poetry"))
            .and_then(|p| p.get("dependencies"))
            .is_some()
        {
            return PyProjectFormat::Poetry;
        }

        // Check for PEP 621 format
        if doc.get("project").is_some() {
            return PyProjectFormat::Pep621;
        }

        PyProjectFormat::Unknown
    }

    /// Get the Poetry dependencies table if it exists
    fn get_poetry_deps(&self) -> Option<&Table> {
        self.doc
            .get("tool")
            .and_then(|t| t.get("poetry"))
            .and_then(|p| p.get("dependencies"))
            .and_then(|d| d.as_table())
    }

    /// Get the Poetry dev-dependencies table if it exists
    fn get_poetry_dev_deps(&self) -> Option<&Table> {
        self.doc
            .get("tool")
            .and_then(|t| t.get("poetry"))
            .and_then(|p| p.get("dev-dependencies"))
            .and_then(|d| d.as_table())
    }

    /// Get the Poetry group dependencies if they exist (Poetry 1.2+ format)
    fn get_poetry_group_deps(&self, group: &str) -> Option<&Table> {
        self.doc
            .get("tool")
            .and_then(|t| t.get("poetry"))
            .and_then(|p| p.get("group"))
            .and_then(|g| g.get(group))
            .and_then(|g| g.get("dependencies"))
            .and_then(|d| d.as_table())
    }

    /// Get the PEP 621 dependencies array if it exists
    fn get_pep621_deps(&self) -> Option<&Array> {
        self.doc
            .get("project")
            .and_then(|p| p.get("dependencies"))
            .and_then(|d| d.as_array())
    }

    /// Get the PEP 621 optional dependencies for a group
    fn get_pep621_optional_deps(&self, group: &str) -> Option<&Array> {
        self.doc
            .get("project")
            .and_then(|p| p.get("optional-dependencies"))
            .and_then(|od| od.get(group))
            .and_then(|d| d.as_array())
    }

    /// Parse a PEP 508 dependency specification string
    /// Examples: "requests>=2.28.0", "click[cli]>=8.0,<9.0", "pytest"
    fn parse_pep508_spec(spec: &str) -> (String, String) {
        let spec = spec.trim();

        // Find the start of version specifier
        // Look for operators: ==, >=, <=, ~=, !=, <, >
        let version_start = spec
            .find(|c: char| c == '=' || c == '<' || c == '>' || c == '!' || c == '~')
            .or_else(|| spec.find('[').map(|_| spec.len())); // extras without version

        if let Some(pos) = version_start {
            // Handle extras (e.g., "package[extra1,extra2]>=1.0")
            let name_end = spec.find('[').unwrap_or(pos);
            let name = spec[..name_end].trim().to_string();
            let version = spec[pos..].trim().to_string();

            // If we found extras but no version, use "*"
            let version = if version.is_empty() || version.starts_with('[') {
                "*".to_string()
            } else {
                version
            };

            (name, version)
        } else {
            // No version specifier, just the package name
            (spec.to_string(), "*".to_string())
        }
    }

    /// Convert a version string to PEP 508 format for a given package name
    fn to_pep508_spec(name: &str, version: &str) -> String {
        if version == "*" || version.is_empty() {
            name.to_string()
        } else if version.starts_with('=')
            || version.starts_with('<')
            || version.starts_with('>')
            || version.starts_with('!')
            || version.starts_with('~')
        {
            format!("{}{}", name, version)
        } else {
            // Assume it's a bare version number, use >=
            format!("{}=={}", name, version)
        }
    }
}

impl ManifestOps for PyProjectManifest {
    fn parse(content: &str) -> ServerResult<Self> {
        let doc = content
            .parse::<DocumentMut>()
            .map_err(|e| ServerError::parse(format!("Failed to parse pyproject.toml: {}", e)))?;

        let format = Self::detect_format(&doc);

        Ok(Self { doc, format })
    }

    fn sections() -> &'static [&'static str] {
        &[
            "dependencies",
            "dev-dependencies",
            "optional-dependencies.dev",
            "optional-dependencies.test",
        ]
    }

    fn default_section() -> &'static str {
        "dependencies"
    }

    fn find_dependency(&self, name: &str) -> Option<(&'static str, DependencyInfo)> {
        match self.format {
            PyProjectFormat::Poetry => {
                // Check [tool.poetry.dependencies]
                if let Some(deps) = self.get_poetry_deps() {
                    if let Some(dep) = deps.get(name) {
                        let info = extract_poetry_dep_info(name, dep);
                        return Some(("dependencies", info));
                    }
                }

                // Check [tool.poetry.dev-dependencies]
                if let Some(deps) = self.get_poetry_dev_deps() {
                    if let Some(dep) = deps.get(name) {
                        let info = extract_poetry_dep_info(name, dep);
                        return Some(("dev-dependencies", info));
                    }
                }

                // Check [tool.poetry.group.dev.dependencies] (Poetry 1.2+ format)
                if let Some(deps) = self.get_poetry_group_deps("dev") {
                    if let Some(dep) = deps.get(name) {
                        let info = extract_poetry_dep_info(name, dep);
                        return Some(("dev-dependencies", info));
                    }
                }

                None
            }
            PyProjectFormat::Pep621 => {
                // Check [project.dependencies]
                if let Some(deps) = self.get_pep621_deps() {
                    for item in deps.iter() {
                        if let Some(spec_str) = item.as_str() {
                            let (dep_name, version) = Self::parse_pep508_spec(spec_str);
                            if dep_name.to_lowercase() == name.to_lowercase() {
                                return Some((
                                    "dependencies",
                                    DependencyInfo {
                                        name: dep_name,
                                        version,
                                        features: None,
                                        optional: None,
                                        already_exists: None,
                                    },
                                ));
                            }
                        }
                    }
                }

                // Check [project.optional-dependencies.dev]
                if let Some(deps) = self.get_pep621_optional_deps("dev") {
                    for item in deps.iter() {
                        if let Some(spec_str) = item.as_str() {
                            let (dep_name, version) = Self::parse_pep508_spec(spec_str);
                            if dep_name.to_lowercase() == name.to_lowercase() {
                                return Some((
                                    "optional-dependencies.dev",
                                    DependencyInfo {
                                        name: dep_name,
                                        version,
                                        features: None,
                                        optional: Some(true),
                                        already_exists: None,
                                    },
                                ));
                            }
                        }
                    }
                }

                // Check [project.optional-dependencies.test]
                if let Some(deps) = self.get_pep621_optional_deps("test") {
                    for item in deps.iter() {
                        if let Some(spec_str) = item.as_str() {
                            let (dep_name, version) = Self::parse_pep508_spec(spec_str);
                            if dep_name.to_lowercase() == name.to_lowercase() {
                                return Some((
                                    "optional-dependencies.test",
                                    DependencyInfo {
                                        name: dep_name,
                                        version,
                                        features: None,
                                        optional: Some(true),
                                        already_exists: None,
                                    },
                                ));
                            }
                        }
                    }
                }

                None
            }
            PyProjectFormat::Unknown => None,
        }
    }

    fn has_dependency(&self, section: &str, name: &str) -> bool {
        match self.format {
            PyProjectFormat::Poetry => {
                let table = match section {
                    "dependencies" => self.get_poetry_deps(),
                    "dev-dependencies" => self
                        .get_poetry_dev_deps()
                        .or_else(|| self.get_poetry_group_deps("dev")),
                    _ => None,
                };
                table.map(|t| t.get(name).is_some()).unwrap_or(false)
            }
            PyProjectFormat::Pep621 => {
                let array = match section {
                    "dependencies" => self.get_pep621_deps(),
                    "optional-dependencies.dev" => self.get_pep621_optional_deps("dev"),
                    "optional-dependencies.test" => self.get_pep621_optional_deps("test"),
                    _ => None,
                };

                if let Some(arr) = array {
                    for item in arr.iter() {
                        if let Some(spec_str) = item.as_str() {
                            let (dep_name, _) = Self::parse_pep508_spec(spec_str);
                            if dep_name.to_lowercase() == name.to_lowercase() {
                                return true;
                            }
                        }
                    }
                }
                false
            }
            PyProjectFormat::Unknown => false,
        }
    }

    fn add_dependency(
        &mut self,
        section: &str,
        name: &str,
        info: &DependencyInfo,
    ) -> ServerResult<()> {
        match self.format {
            PyProjectFormat::Poetry => {
                add_poetry_dependency(&mut self.doc, section, name, info)
            }
            PyProjectFormat::Pep621 => {
                add_pep621_dependency(&mut self.doc, section, name, info)
            }
            PyProjectFormat::Unknown => {
                // Default to PEP 621 format for unknown/empty pyproject.toml
                self.format = PyProjectFormat::Pep621;
                ensure_pep621_structure(&mut self.doc)?;
                add_pep621_dependency(&mut self.doc, section, name, info)
            }
        }
    }

    fn serialize(&self) -> String {
        self.doc.to_string()
    }
}

/// Extract dependency info from Poetry-style TOML value
fn extract_poetry_dep_info(dep_name: &str, dep_item: &Item) -> DependencyInfo {
    let mut features = None;
    let mut optional = None;
    let mut version = String::new();

    match dep_item {
        Item::Value(TomlValue::String(v)) => {
            // Simple version string: "^1.0" or ">=1.0,<2.0"
            version = v.value().to_string();
        }
        Item::Value(TomlValue::InlineTable(table)) => {
            // Inline table: { version = "^1.0", optional = true, extras = ["full"] }
            if let Some(v) = table.get("version").and_then(|v| v.as_str()) {
                version = v.to_string();
            } else if let Some(path) = table.get("path").and_then(|v| v.as_str()) {
                version = format!("path:{}", path);
            } else if let Some(git) = table.get("git").and_then(|v| v.as_str()) {
                version = format!("git:{}", git);
            } else {
                version = "*".to_string();
            }

            // Extract extras
            if let Some(extras) = table.get("extras").and_then(|v| v.as_array()) {
                let extras_list: Vec<String> = extras
                    .iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect();
                if !extras_list.is_empty() {
                    features = Some(extras_list);
                }
            }

            optional = table.get("optional").and_then(|v| v.as_bool());
        }
        Item::Table(table) => {
            // Regular table (multi-line)
            if let Some(v) = table.get("version").and_then(|v| v.as_str()) {
                version = v.to_string();
            } else if let Some(path) = table.get("path").and_then(|v| v.as_str()) {
                version = format!("path:{}", path);
            } else if let Some(git) = table.get("git").and_then(|v| v.as_str()) {
                version = format!("git:{}", git);
            } else {
                version = "*".to_string();
            }

            if let Some(extras) = table.get("extras").and_then(|v| v.as_array()) {
                let extras_list: Vec<String> = extras
                    .iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect();
                if !extras_list.is_empty() {
                    features = Some(extras_list);
                }
            }

            optional = table.get("optional").and_then(|v| v.as_bool());
        }
        _ => {
            version = "*".to_string();
        }
    }

    DependencyInfo {
        name: dep_name.to_string(),
        version,
        features,
        optional,
        already_exists: None,
    }
}

/// Add a dependency to Poetry-format pyproject.toml
fn add_poetry_dependency(
    doc: &mut DocumentMut,
    section: &str,
    name: &str,
    info: &DependencyInfo,
) -> ServerResult<()> {
    // Ensure [tool.poetry] structure exists
    if !doc.contains_key("tool") {
        doc["tool"] = Item::Table(Table::new());
    }
    let tool = doc["tool"]
        .as_table_mut()
        .ok_or_else(|| ServerError::parse("[tool] is not a table"))?;

    if !tool.contains_key("poetry") {
        tool["poetry"] = Item::Table(Table::new());
    }
    let poetry = tool["poetry"]
        .as_table_mut()
        .ok_or_else(|| ServerError::parse("[tool.poetry] is not a table"))?;

    // Determine which section to add to
    let section_key = match section {
        "dependencies" => "dependencies",
        "dev-dependencies" | "optional-dependencies.dev" => "dev-dependencies",
        _ => "dependencies",
    };

    if !poetry.contains_key(section_key) {
        poetry[section_key] = Item::Table(Table::new());
    }

    let deps = poetry[section_key]
        .as_table_mut()
        .ok_or_else(|| ServerError::parse(format!("[tool.poetry.{}] is not a table", section_key)))?;

    // Build the dependency value
    let dep_value = build_poetry_dep_value(info);
    deps[name] = dep_value;

    debug!(dependency = %name, section = %section_key, "Added dependency to pyproject.toml (Poetry)");
    Ok(())
}

/// Add a dependency to PEP 621-format pyproject.toml
fn add_pep621_dependency(
    doc: &mut DocumentMut,
    section: &str,
    name: &str,
    info: &DependencyInfo,
) -> ServerResult<()> {
    // Ensure [project] structure exists
    ensure_pep621_structure(doc)?;

    let project = doc["project"]
        .as_table_mut()
        .ok_or_else(|| ServerError::parse("[project] is not a table"))?;

    // Build the PEP 508 dependency specification
    let dep_spec = PyProjectManifest::to_pep508_spec(name, &info.version);

    match section {
        "dependencies" => {
            if !project.contains_key("dependencies") {
                project["dependencies"] = Item::Value(TomlValue::Array(Array::new()));
            }

            let deps = project["dependencies"]
                .as_array_mut()
                .ok_or_else(|| ServerError::parse("[project.dependencies] is not an array"))?;

            deps.push(dep_spec.as_str());
            debug!(dependency = %name, section = "dependencies", "Added dependency to pyproject.toml (PEP 621)");
        }
        "optional-dependencies.dev" | "dev-dependencies" => {
            if !project.contains_key("optional-dependencies") {
                project["optional-dependencies"] = Item::Table(Table::new());
            }

            let opt_deps = project["optional-dependencies"]
                .as_table_mut()
                .ok_or_else(|| ServerError::parse("[project.optional-dependencies] is not a table"))?;

            if !opt_deps.contains_key("dev") {
                opt_deps["dev"] = Item::Value(TomlValue::Array(Array::new()));
            }

            let dev_deps = opt_deps["dev"]
                .as_array_mut()
                .ok_or_else(|| ServerError::parse("[project.optional-dependencies.dev] is not an array"))?;

            dev_deps.push(dep_spec.as_str());
            debug!(dependency = %name, section = "optional-dependencies.dev", "Added dependency to pyproject.toml (PEP 621)");
        }
        "optional-dependencies.test" => {
            if !project.contains_key("optional-dependencies") {
                project["optional-dependencies"] = Item::Table(Table::new());
            }

            let opt_deps = project["optional-dependencies"]
                .as_table_mut()
                .ok_or_else(|| ServerError::parse("[project.optional-dependencies] is not a table"))?;

            if !opt_deps.contains_key("test") {
                opt_deps["test"] = Item::Value(TomlValue::Array(Array::new()));
            }

            let test_deps = opt_deps["test"]
                .as_array_mut()
                .ok_or_else(|| ServerError::parse("[project.optional-dependencies.test] is not an array"))?;

            test_deps.push(dep_spec.as_str());
            debug!(dependency = %name, section = "optional-dependencies.test", "Added dependency to pyproject.toml (PEP 621)");
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

/// Ensure the [project] section exists for PEP 621 format
fn ensure_pep621_structure(doc: &mut DocumentMut) -> ServerResult<()> {
    if !doc.contains_key("project") {
        doc["project"] = Item::Table(Table::new());
    }
    Ok(())
}

/// Build a TOML value for a Poetry dependency
fn build_poetry_dep_value(info: &DependencyInfo) -> Item {
    // If no extras/features and not optional, and it's a simple version, use string
    if info.features.is_none() && info.optional.is_none() && !info.version.contains(':') {
        return Item::Value(TomlValue::String(Formatted::new(info.version.clone())));
    }

    // Build inline table for complex dependencies
    let mut table = toml_edit::InlineTable::new();

    // Handle special version formats
    if info.version.starts_with("path:") {
        let path = info.version.strip_prefix("path:").unwrap_or(&info.version);
        table.insert("path", TomlValue::String(Formatted::new(path.to_string())));
    } else if info.version.starts_with("git:") {
        let git = info.version.strip_prefix("git:").unwrap_or(&info.version);
        table.insert("git", TomlValue::String(Formatted::new(git.to_string())));
    } else {
        table.insert(
            "version",
            TomlValue::String(Formatted::new(info.version.clone())),
        );
    }

    // Add extras if present
    if let Some(extras) = &info.features {
        let mut arr = Array::new();
        for extra in extras {
            arr.push(extra.as_str());
        }
        table.insert("extras", TomlValue::Array(arr));
    }

    // Add optional flag if present
    if let Some(opt) = info.optional {
        table.insert("optional", TomlValue::Boolean(Formatted::new(opt)));
    }

    Item::Value(TomlValue::InlineTable(table))
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // PEP 621 Format Tests
    // ========================================================================

    #[test]
    fn test_parse_pep621_simple() {
        let content = r#"
[project]
name = "my-package"
version = "1.0.0"
dependencies = [
    "requests>=2.28.0",
    "click>=8.0.0",
]
"#;
        let manifest = PyProjectManifest::parse(content).unwrap();
        assert_eq!(manifest.format, PyProjectFormat::Pep621);
    }

    #[test]
    fn test_find_dependency_pep621() {
        let content = r#"
[project]
name = "my-package"
dependencies = [
    "requests>=2.28.0",
    "click>=8.0.0",
]

[project.optional-dependencies]
dev = ["pytest>=7.0.0", "black"]
"#;
        let manifest = PyProjectManifest::parse(content).unwrap();

        // Find in dependencies
        let result = manifest.find_dependency("requests");
        assert!(result.is_some());
        let (section, info) = result.unwrap();
        assert_eq!(section, "dependencies");
        assert_eq!(info.name, "requests");
        assert_eq!(info.version, ">=2.28.0");

        // Find in optional-dependencies.dev
        let result = manifest.find_dependency("pytest");
        assert!(result.is_some());
        let (section, info) = result.unwrap();
        assert_eq!(section, "optional-dependencies.dev");
        assert_eq!(info.name, "pytest");
        assert_eq!(info.version, ">=7.0.0");

        // Not found
        let result = manifest.find_dependency("nonexistent");
        assert!(result.is_none());
    }

    #[test]
    fn test_has_dependency_pep621() {
        let content = r#"
[project]
name = "my-package"
dependencies = [
    "requests>=2.28.0",
]

[project.optional-dependencies]
dev = ["pytest>=7.0.0"]
"#;
        let manifest = PyProjectManifest::parse(content).unwrap();
        assert!(manifest.has_dependency("dependencies", "requests"));
        assert!(!manifest.has_dependency("dependencies", "click"));
        assert!(manifest.has_dependency("optional-dependencies.dev", "pytest"));
    }

    #[test]
    fn test_add_dependency_pep621() {
        let content = r#"
[project]
name = "my-package"
dependencies = [
    "requests>=2.28.0",
]
"#;
        let mut manifest = PyProjectManifest::parse(content).unwrap();
        let info = DependencyInfo {
            name: "click".to_string(),
            version: ">=8.0.0".to_string(),
            features: None,
            optional: None,
            already_exists: None,
        };
        manifest
            .add_dependency("dependencies", "click", &info)
            .unwrap();

        let serialized = manifest.serialize();
        assert!(serialized.contains("click>=8.0.0"));
    }

    #[test]
    fn test_add_dev_dependency_pep621() {
        let content = r#"
[project]
name = "my-package"
dependencies = ["requests>=2.28.0"]
"#;
        let mut manifest = PyProjectManifest::parse(content).unwrap();
        let info = DependencyInfo {
            name: "pytest".to_string(),
            version: ">=7.0.0".to_string(),
            features: None,
            optional: None,
            already_exists: None,
        };
        manifest
            .add_dependency("optional-dependencies.dev", "pytest", &info)
            .unwrap();

        let serialized = manifest.serialize();
        assert!(serialized.contains("[project.optional-dependencies]"));
        assert!(serialized.contains("pytest>=7.0.0"));
    }

    // ========================================================================
    // Poetry Format Tests
    // ========================================================================

    #[test]
    fn test_parse_poetry_simple() {
        let content = r#"
[tool.poetry]
name = "my-package"
version = "1.0.0"

[tool.poetry.dependencies]
python = "^3.9"
requests = "^2.28.0"
"#;
        let manifest = PyProjectManifest::parse(content).unwrap();
        assert_eq!(manifest.format, PyProjectFormat::Poetry);
    }

    #[test]
    fn test_find_dependency_poetry() {
        let content = r#"
[tool.poetry]
name = "my-package"

[tool.poetry.dependencies]
python = "^3.9"
requests = "^2.28.0"
click = { version = "^8.0.0", optional = true }

[tool.poetry.dev-dependencies]
pytest = "^7.0.0"
"#;
        let manifest = PyProjectManifest::parse(content).unwrap();

        // Find in dependencies
        let result = manifest.find_dependency("requests");
        assert!(result.is_some());
        let (section, info) = result.unwrap();
        assert_eq!(section, "dependencies");
        assert_eq!(info.name, "requests");
        assert_eq!(info.version, "^2.28.0");

        // Find optional dependency
        let result = manifest.find_dependency("click");
        assert!(result.is_some());
        let (_, info) = result.unwrap();
        assert_eq!(info.optional, Some(true));

        // Find in dev-dependencies
        let result = manifest.find_dependency("pytest");
        assert!(result.is_some());
        let (section, _) = result.unwrap();
        assert_eq!(section, "dev-dependencies");

        // Not found (python is skipped by convention)
        let result = manifest.find_dependency("nonexistent");
        assert!(result.is_none());
    }

    #[test]
    fn test_find_dependency_poetry_with_extras() {
        let content = r#"
[tool.poetry.dependencies]
requests = { version = "^2.28.0", extras = ["socks", "security"] }
"#;
        let manifest = PyProjectManifest::parse(content).unwrap();
        let result = manifest.find_dependency("requests");
        assert!(result.is_some());
        let (_, info) = result.unwrap();
        assert_eq!(
            info.features,
            Some(vec!["socks".to_string(), "security".to_string()])
        );
    }

    #[test]
    fn test_find_dependency_poetry_path() {
        let content = r#"
[tool.poetry.dependencies]
my-local-lib = { path = "../local-lib" }
"#;
        let manifest = PyProjectManifest::parse(content).unwrap();
        let result = manifest.find_dependency("my-local-lib");
        assert!(result.is_some());
        let (_, info) = result.unwrap();
        assert_eq!(info.version, "path:../local-lib");
    }

    #[test]
    fn test_find_dependency_poetry_git() {
        let content = r#"
[tool.poetry.dependencies]
my-git-lib = { git = "https://github.com/user/repo.git" }
"#;
        let manifest = PyProjectManifest::parse(content).unwrap();
        let result = manifest.find_dependency("my-git-lib");
        assert!(result.is_some());
        let (_, info) = result.unwrap();
        assert_eq!(info.version, "git:https://github.com/user/repo.git");
    }

    #[test]
    fn test_has_dependency_poetry() {
        let content = r#"
[tool.poetry.dependencies]
requests = "^2.28.0"

[tool.poetry.dev-dependencies]
pytest = "^7.0.0"
"#;
        let manifest = PyProjectManifest::parse(content).unwrap();
        assert!(manifest.has_dependency("dependencies", "requests"));
        assert!(!manifest.has_dependency("dependencies", "click"));
        assert!(manifest.has_dependency("dev-dependencies", "pytest"));
    }

    #[test]
    fn test_add_dependency_poetry() {
        let content = r#"
[tool.poetry]
name = "my-package"

[tool.poetry.dependencies]
python = "^3.9"
"#;
        let mut manifest = PyProjectManifest::parse(content).unwrap();
        let info = DependencyInfo {
            name: "requests".to_string(),
            version: "^2.28.0".to_string(),
            features: None,
            optional: None,
            already_exists: None,
        };
        manifest
            .add_dependency("dependencies", "requests", &info)
            .unwrap();

        let serialized = manifest.serialize();
        assert!(serialized.contains("requests"));
        assert!(serialized.contains("^2.28.0"));
    }

    #[test]
    fn test_add_dev_dependency_poetry() {
        let content = r#"
[tool.poetry]
name = "my-package"

[tool.poetry.dependencies]
python = "^3.9"
"#;
        let mut manifest = PyProjectManifest::parse(content).unwrap();
        let info = DependencyInfo {
            name: "pytest".to_string(),
            version: "^7.0.0".to_string(),
            features: None,
            optional: None,
            already_exists: None,
        };
        manifest
            .add_dependency("dev-dependencies", "pytest", &info)
            .unwrap();

        let serialized = manifest.serialize();
        assert!(serialized.contains("[tool.poetry.dev-dependencies]"));
        assert!(serialized.contains("pytest"));
    }

    // ========================================================================
    // PEP 508 Parsing Tests
    // ========================================================================

    #[test]
    fn test_parse_pep508_simple() {
        let (name, version) = PyProjectManifest::parse_pep508_spec("requests");
        assert_eq!(name, "requests");
        assert_eq!(version, "*");
    }

    #[test]
    fn test_parse_pep508_with_version() {
        let (name, version) = PyProjectManifest::parse_pep508_spec("requests>=2.28.0");
        assert_eq!(name, "requests");
        assert_eq!(version, ">=2.28.0");
    }

    #[test]
    fn test_parse_pep508_with_extras() {
        let (name, version) = PyProjectManifest::parse_pep508_spec("requests[security]>=2.28.0");
        assert_eq!(name, "requests");
        assert_eq!(version, ">=2.28.0");
    }

    #[test]
    fn test_parse_pep508_complex_version() {
        let (name, version) = PyProjectManifest::parse_pep508_spec("click>=8.0.0,<9.0.0");
        assert_eq!(name, "click");
        assert_eq!(version, ">=8.0.0,<9.0.0");
    }

    #[test]
    fn test_parse_pep508_tilde() {
        let (name, version) = PyProjectManifest::parse_pep508_spec("django~=3.2");
        assert_eq!(name, "django");
        assert_eq!(version, "~=3.2");
    }

    #[test]
    fn test_parse_pep508_not_equal() {
        let (name, version) = PyProjectManifest::parse_pep508_spec("package!=1.0.0");
        assert_eq!(name, "package");
        assert_eq!(version, "!=1.0.0");
    }

    // ========================================================================
    // Format Detection Tests
    // ========================================================================

    #[test]
    fn test_detect_format_pep621() {
        let content = r#"
[project]
name = "my-package"
"#;
        let manifest = PyProjectManifest::parse(content).unwrap();
        assert_eq!(manifest.format, PyProjectFormat::Pep621);
    }

    #[test]
    fn test_detect_format_poetry() {
        let content = r#"
[tool.poetry]
name = "my-package"

[tool.poetry.dependencies]
python = "^3.9"
"#;
        let manifest = PyProjectManifest::parse(content).unwrap();
        assert_eq!(manifest.format, PyProjectFormat::Poetry);
    }

    #[test]
    fn test_detect_format_unknown() {
        let content = r#"
[build-system]
requires = ["setuptools"]
"#;
        let manifest = PyProjectManifest::parse(content).unwrap();
        assert_eq!(manifest.format, PyProjectFormat::Unknown);
    }

    // ========================================================================
    // Serialization Tests
    // ========================================================================

    #[test]
    fn test_serialize_preserves_structure() {
        let content = r#"[project]
name = "my-package"
version = "1.0.0"

[project.dependencies]
"#;
        let manifest = PyProjectManifest::parse(content).unwrap();
        let serialized = manifest.serialize();
        assert!(serialized.contains("[project]"));
        assert!(serialized.contains("name = \"my-package\""));
    }

    #[test]
    fn test_sections() {
        assert!(PyProjectManifest::sections().contains(&"dependencies"));
        assert!(PyProjectManifest::sections().contains(&"dev-dependencies"));
        assert!(PyProjectManifest::sections().contains(&"optional-dependencies.dev"));
    }

    #[test]
    fn test_default_section() {
        assert_eq!(PyProjectManifest::default_section(), "dependencies");
    }

    // ========================================================================
    // Poetry 1.2+ Group Format Tests
    // ========================================================================

    #[test]
    fn test_find_dependency_poetry_group() {
        let content = r#"
[tool.poetry]
name = "my-package"

[tool.poetry.dependencies]
python = "^3.9"

[tool.poetry.group.dev.dependencies]
pytest = "^7.0.0"
black = "^23.0.0"
"#;
        let manifest = PyProjectManifest::parse(content).unwrap();

        // Should find in group.dev.dependencies
        let result = manifest.find_dependency("pytest");
        assert!(result.is_some());
        let (section, info) = result.unwrap();
        assert_eq!(section, "dev-dependencies");
        assert_eq!(info.name, "pytest");
        assert_eq!(info.version, "^7.0.0");
    }

    // ========================================================================
    // Edge Case Tests
    // ========================================================================

    #[test]
    fn test_case_insensitive_package_names() {
        let content = r#"
[project]
dependencies = ["Requests>=2.28.0"]
"#;
        let manifest = PyProjectManifest::parse(content).unwrap();

        // Should find regardless of case
        assert!(manifest.find_dependency("requests").is_some());
        assert!(manifest.find_dependency("Requests").is_some());
        assert!(manifest.find_dependency("REQUESTS").is_some());
    }

    #[test]
    fn test_empty_pyproject() {
        let content = "";
        let manifest = PyProjectManifest::parse(content).unwrap();
        assert_eq!(manifest.format, PyProjectFormat::Unknown);
    }

    #[test]
    fn test_add_to_empty_pyproject() {
        let content = "[build-system]\nrequires = [\"setuptools\"]";
        let mut manifest = PyProjectManifest::parse(content).unwrap();

        let info = DependencyInfo {
            name: "requests".to_string(),
            version: ">=2.28.0".to_string(),
            features: None,
            optional: None,
            already_exists: None,
        };

        // Should create [project] section and add dependency
        manifest
            .add_dependency("dependencies", "requests", &info)
            .unwrap();

        let serialized = manifest.serialize();
        assert!(serialized.contains("[project]"));
        assert!(serialized.contains("requests>=2.28.0"));
    }
}
