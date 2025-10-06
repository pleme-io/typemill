//! Manifest file generation templates
//!
//! Provides language-agnostic manifest generation. Individual language plugins
//! can use these as starting points or implement their own specific logic.

/// Trait for manifest template generators
pub trait ManifestTemplate {
    /// Generate a manifest file content
    ///
    /// # Arguments
    ///
    /// * `name` - Package/project name
    /// * `version` - Initial version
    /// * `dependencies` - List of dependency names (without versions)
    ///
    /// # Returns
    ///
    /// Complete manifest file content as a string
    fn generate(&self, name: &str, version: &str, dependencies: &[String]) -> String;

    /// Generate a workspace manifest
    ///
    /// For package managers that support monorepo workspaces
    fn generate_workspace(&self, members: &[String]) -> Option<String> {
        let _ = members;
        None // Default: not supported
    }
}

/// Generate a basic TOML-based manifest (Cargo.toml style)
pub struct TomlManifestTemplate {
    package_section: String,
    include_dev_deps: bool,
}

impl TomlManifestTemplate {
    pub fn new(package_section: &str) -> Self {
        Self {
            package_section: package_section.to_string(),
            include_dev_deps: false,
        }
    }

    pub fn with_dev_dependencies(mut self, include: bool) -> Self {
        self.include_dev_deps = include;
        self
    }
}

impl ManifestTemplate for TomlManifestTemplate {
    fn generate(&self, name: &str, version: &str, dependencies: &[String]) -> String {
        let mut lines = vec![
            format!("[{}]", self.package_section),
            format!("name = \"{}\"", name),
            format!("version = \"{}\"", version),
            String::new(),
        ];

        if !dependencies.is_empty() {
            lines.push("[dependencies]".to_string());
            for dep in dependencies {
                lines.push(format!("{} = \"*\"", dep));
            }
            lines.push(String::new());
        }

        if self.include_dev_deps {
            lines.push("[dev-dependencies]".to_string());
            lines.push(String::new());
        }

        lines.join("\n")
    }

    fn generate_workspace(&self, members: &[String]) -> Option<String> {
        let mut lines = vec!["[workspace]".to_string(), "members = [".to_string()];

        for member in members {
            lines.push(format!("    \"{}\",", member));
        }

        lines.push("]".to_string());
        Some(lines.join("\n"))
    }
}

/// Generate a JSON-based manifest (package.json style)
pub struct JsonManifestTemplate {
    include_scripts: bool,
    package_type: Option<String>, // "module" or "commonjs"
}

impl JsonManifestTemplate {
    pub fn new() -> Self {
        Self {
            include_scripts: false,
            package_type: None,
        }
    }

    pub fn with_scripts(mut self, include: bool) -> Self {
        self.include_scripts = include;
        self
    }

    pub fn with_type(mut self, package_type: &str) -> Self {
        self.package_type = Some(package_type.to_string());
        self
    }
}

impl Default for JsonManifestTemplate {
    fn default() -> Self {
        Self::new()
    }
}

impl ManifestTemplate for JsonManifestTemplate {
    fn generate(&self, name: &str, version: &str, dependencies: &[String]) -> String {
        let mut obj = serde_json::json!({
            "name": name,
            "version": version,
            "description": "",
            "main": "index.js",
        });

        if let Some(ref pkg_type) = self.package_type {
            obj["type"] = serde_json::json!(pkg_type);
        }

        if self.include_scripts {
            obj["scripts"] = serde_json::json!({
                "test": "echo \"Error: no test specified\" && exit 1"
            });
        }

        if !dependencies.is_empty() {
            let mut deps = serde_json::Map::new();
            for dep in dependencies {
                deps.insert(dep.clone(), serde_json::json!("*"));
            }
            obj["dependencies"] = serde_json::json!(deps);
        }

        serde_json::to_string_pretty(&obj).unwrap_or_else(|_| "{}".to_string())
    }

    fn generate_workspace(&self, members: &[String]) -> Option<String> {
        let obj = serde_json::json!({
            "private": true,
            "workspaces": members
        });

        Some(serde_json::to_string_pretty(&obj).unwrap_or_else(|_| "{}".to_string()))
    }
}

/// Generate a simple line-based manifest (requirements.txt style)
pub struct LineBasedManifestTemplate {
    include_version: bool,
    comment_header: Option<String>,
}

impl LineBasedManifestTemplate {
    pub fn new() -> Self {
        Self {
            include_version: false,
            comment_header: None,
        }
    }

    pub fn with_versions(mut self, include: bool) -> Self {
        self.include_version = include;
        self
    }

    pub fn with_header(mut self, header: &str) -> Self {
        self.comment_header = Some(header.to_string());
        self
    }
}

impl Default for LineBasedManifestTemplate {
    fn default() -> Self {
        Self::new()
    }
}

impl ManifestTemplate for LineBasedManifestTemplate {
    fn generate(&self, _name: &str, _version: &str, dependencies: &[String]) -> String {
        let mut lines = Vec::new();

        if let Some(ref header) = self.comment_header {
            for line in header.lines() {
                lines.push(format!("# {}", line));
            }
            lines.push(String::new());
        }

        for dep in dependencies {
            if self.include_version {
                lines.push(format!("{}>=0.0.0", dep));
            } else {
                lines.push(dep.clone());
            }
        }

        lines.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_toml_manifest() {
        let template = TomlManifestTemplate::new("package");
        let manifest = template.generate("test-package", "1.0.0", &["dep1".to_string()]);

        assert!(manifest.contains("[package]"));
        assert!(manifest.contains("name = \"test-package\""));
        assert!(manifest.contains("version = \"1.0.0\""));
        assert!(manifest.contains("[dependencies]"));
        assert!(manifest.contains("dep1 = \"*\""));
    }

    #[test]
    fn test_toml_workspace() {
        let template = TomlManifestTemplate::new("package");
        let workspace = template.generate_workspace(&["crate1".to_string(), "crate2".to_string()]);

        assert!(workspace.is_some());
        let content = workspace.unwrap();
        assert!(content.contains("[workspace]"));
        assert!(content.contains("crate1"));
        assert!(content.contains("crate2"));
    }

    #[test]
    fn test_json_manifest() {
        let template = JsonManifestTemplate::new();
        let manifest = template.generate("my-package", "2.0.0", &["express".to_string()]);

        assert!(manifest.contains("\"name\": \"my-package\""));
        assert!(manifest.contains("\"version\": \"2.0.0\""));
        assert!(manifest.contains("\"dependencies\""));
        assert!(manifest.contains("\"express\""));
    }

    #[test]
    fn test_json_with_type() {
        let template = JsonManifestTemplate::new().with_type("module");
        let manifest = template.generate("esm-package", "1.0.0", &[]);

        assert!(manifest.contains("\"type\": \"module\""));
    }

    #[test]
    fn test_line_based_manifest() {
        let template = LineBasedManifestTemplate::new().with_versions(true);
        let manifest = template.generate("", "", &["django".to_string(), "requests".to_string()]);

        assert!(manifest.contains("django>=0.0.0"));
        assert!(manifest.contains("requests>=0.0.0"));
    }

    #[test]
    fn test_line_based_with_header() {
        let template = LineBasedManifestTemplate::new()
            .with_header("Auto-generated requirements\nDo not edit");
        let manifest = template.generate("", "", &["flask".to_string()]);

        assert!(manifest.contains("# Auto-generated requirements"));
        assert!(manifest.contains("# Do not edit"));
        assert!(manifest.contains("flask"));
    }
}
