//! package.json manifest implementation
//!
//! Implements `ManifestOps` trait for TypeScript/JavaScript projects.

use super::{DependencyInfo, ManifestOps};
use mill_foundation::errors::{MillError as ServerError, MillResult as ServerResult};
use serde_json::{Map, Value};
use tracing::debug;

/// package.json manifest wrapper
pub struct PackageJsonManifest {
    json: Value,
}

impl ManifestOps for PackageJsonManifest {
    fn parse(content: &str) -> ServerResult<Self> {
        let json: Value = serde_json::from_str(content)
            .map_err(|e| ServerError::parse(format!("Failed to parse package.json: {}", e)))?;

        if !json.is_object() {
            return Err(ServerError::parse("package.json root must be an object"));
        }

        Ok(Self { json })
    }

    fn sections() -> &'static [&'static str] {
        &[
            "dependencies",
            "devDependencies",
            "peerDependencies",
            "optionalDependencies",
        ]
    }

    fn default_section() -> &'static str {
        "dependencies"
    }

    fn find_dependency(&self, name: &str) -> Option<(&'static str, DependencyInfo)> {
        let obj = self.json.as_object()?;

        // Check each section in order
        for section in Self::sections() {
            if let Some(deps) = obj.get(*section).and_then(|v| v.as_object()) {
                if let Some(value) = deps.get(name) {
                    let info = extract_npm_dep_info(name, value);
                    return Some((section, info));
                }
            }
        }

        None
    }

    fn has_dependency(&self, section: &str, name: &str) -> bool {
        self.json
            .get(section)
            .and_then(|s| s.as_object())
            .and_then(|o| o.get(name))
            .is_some()
    }

    fn add_dependency(
        &mut self,
        section: &str,
        name: &str,
        info: &DependencyInfo,
    ) -> ServerResult<()> {
        let obj = self
            .json
            .as_object_mut()
            .ok_or_else(|| ServerError::internal("package.json root is not an object"))?;

        // Ensure section exists
        if !obj.contains_key(section) {
            obj.insert(section.to_string(), Value::Object(Map::new()));
        }

        let section_obj = obj
            .get_mut(section)
            .and_then(|v| v.as_object_mut())
            .ok_or_else(|| ServerError::parse(format!("'{}' is not an object", section)))?;

        // npm dependencies are just version strings
        section_obj.insert(name.to_string(), Value::String(info.version.clone()));

        debug!(dependency = %name, section = %section, "Added dependency to package.json");
        Ok(())
    }

    fn serialize(&self) -> String {
        // Pretty print with 2 spaces (npm standard) and trailing newline
        serde_json::to_string_pretty(&self.json).unwrap_or_default() + "\n"
    }
}

/// Extract dependency info from a JSON value
fn extract_npm_dep_info(dep_name: &str, value: &Value) -> DependencyInfo {
    let version = match value {
        Value::String(s) => s.clone(),
        _ => "*".to_string(),
    };

    // npm doesn't have features or optional in the version string
    // (optionalDependencies are in a separate section)
    DependencyInfo {
        name: dep_name.to_string(),
        version,
        features: None,
        optional: None,
        already_exists: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple() {
        let content = r#"{
  "name": "test-package",
  "version": "1.0.0",
  "dependencies": {
    "react": "^18.0.0"
  }
}"#;
        let manifest = PackageJsonManifest::parse(content).unwrap();
        assert!(manifest.json.is_object());
    }

    #[test]
    fn test_find_dependency() {
        let content = r#"{
  "name": "test-package",
  "dependencies": {
    "react": "^18.0.0",
    "lodash": "~4.17.0"
  },
  "devDependencies": {
    "typescript": "^5.0.0"
  }
}"#;
        let manifest = PackageJsonManifest::parse(content).unwrap();

        // Find in dependencies
        let result = manifest.find_dependency("react");
        assert!(result.is_some());
        let (section, info) = result.unwrap();
        assert_eq!(section, "dependencies");
        assert_eq!(info.name, "react");
        assert_eq!(info.version, "^18.0.0");

        // Find in devDependencies
        let result = manifest.find_dependency("typescript");
        assert!(result.is_some());
        let (section, _) = result.unwrap();
        assert_eq!(section, "devDependencies");

        // Not found
        let result = manifest.find_dependency("nonexistent");
        assert!(result.is_none());
    }

    #[test]
    fn test_find_dependency_peer() {
        let content = r#"{
  "name": "test-package",
  "peerDependencies": {
    "react": ">=17.0.0"
  }
}"#;
        let manifest = PackageJsonManifest::parse(content).unwrap();
        let result = manifest.find_dependency("react");
        assert!(result.is_some());
        let (section, info) = result.unwrap();
        assert_eq!(section, "peerDependencies");
        assert_eq!(info.version, ">=17.0.0");
    }

    #[test]
    fn test_find_dependency_optional() {
        let content = r#"{
  "name": "test-package",
  "optionalDependencies": {
    "fsevents": "^2.0.0"
  }
}"#;
        let manifest = PackageJsonManifest::parse(content).unwrap();
        let result = manifest.find_dependency("fsevents");
        assert!(result.is_some());
        let (section, _) = result.unwrap();
        assert_eq!(section, "optionalDependencies");
    }

    #[test]
    fn test_has_dependency() {
        let content = r#"{
  "name": "test-package",
  "dependencies": {
    "react": "^18.0.0"
  }
}"#;
        let manifest = PackageJsonManifest::parse(content).unwrap();
        assert!(manifest.has_dependency("dependencies", "react"));
        assert!(!manifest.has_dependency("dependencies", "vue"));
        assert!(!manifest.has_dependency("devDependencies", "react"));
    }

    #[test]
    fn test_add_dependency() {
        let content = r#"{
  "name": "test-package",
  "dependencies": {
    "react": "^18.0.0"
  }
}"#;
        let mut manifest = PackageJsonManifest::parse(content).unwrap();
        let info = DependencyInfo {
            name: "vue".to_string(),
            version: "^3.0.0".to_string(),
            features: None,
            optional: None,
            already_exists: None,
        };
        manifest
            .add_dependency("dependencies", "vue", &info)
            .unwrap();

        let serialized = manifest.serialize();
        assert!(serialized.contains("\"vue\": \"^3.0.0\""));
    }

    #[test]
    fn test_add_dependency_new_section() {
        let content = r#"{
  "name": "test-package",
  "dependencies": {
    "react": "^18.0.0"
  }
}"#;
        let mut manifest = PackageJsonManifest::parse(content).unwrap();
        let info = DependencyInfo {
            name: "typescript".to_string(),
            version: "^5.0.0".to_string(),
            features: None,
            optional: None,
            already_exists: None,
        };
        manifest
            .add_dependency("devDependencies", "typescript", &info)
            .unwrap();

        let serialized = manifest.serialize();
        assert!(serialized.contains("\"devDependencies\""));
        assert!(serialized.contains("\"typescript\": \"^5.0.0\""));
    }

    #[test]
    fn test_sections() {
        assert_eq!(
            PackageJsonManifest::sections(),
            &[
                "dependencies",
                "devDependencies",
                "peerDependencies",
                "optionalDependencies"
            ]
        );
    }

    #[test]
    fn test_default_section() {
        assert_eq!(PackageJsonManifest::default_section(), "dependencies");
    }

    #[test]
    fn test_serialize_formatting() {
        let content = r#"{"name":"test","dependencies":{"a":"1.0"}}"#;
        let manifest = PackageJsonManifest::parse(content).unwrap();
        let serialized = manifest.serialize();

        // Should be pretty-printed with 2 spaces
        assert!(serialized.contains("  "));
        // Should have trailing newline
        assert!(serialized.ends_with('\n'));
    }

    #[test]
    fn test_various_version_formats() {
        let content = r#"{
  "name": "test-package",
  "dependencies": {
    "caret": "^1.0.0",
    "tilde": "~1.0.0",
    "exact": "1.0.0",
    "range": ">=1.0.0 <2.0.0",
    "latest": "latest",
    "git": "git+https://github.com/user/repo.git#v1.0.0",
    "local": "file:../local-lib",
    "workspace": "workspace:*"
  }
}"#;
        let manifest = PackageJsonManifest::parse(content).unwrap();

        // All should be found and preserve their version strings
        for name in &[
            "caret",
            "tilde",
            "exact",
            "range",
            "latest",
            "git",
            "local",
            "workspace",
        ] {
            let result = manifest.find_dependency(name);
            assert!(result.is_some(), "Should find dependency: {}", name);
        }

        // Check specific values
        let (_, info) = manifest.find_dependency("git").unwrap();
        assert_eq!(info.version, "git+https://github.com/user/repo.git#v1.0.0");

        let (_, info) = manifest.find_dependency("workspace").unwrap();
        assert_eq!(info.version, "workspace:*");
    }
}
