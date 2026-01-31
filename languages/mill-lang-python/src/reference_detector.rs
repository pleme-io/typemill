//! Python-specific reference detection
//!
//! Handles detection of affected files for Python package renames/moves.
//! Detects references in:
//! - Import statements (`import package`, `from package import X`)
//! - pyproject.toml dependencies
//! - requirements.txt references

use async_trait::async_trait;
use mill_plugin_api::ReferenceDetector;
use once_cell::sync::Lazy;
use regex::Regex;
use std::path::{Path, PathBuf};
use tokio::task::JoinSet;

/// Regex pattern for detecting Python package names in pyproject.toml dependencies
///
/// Matches dependency declarations in both formats:
/// - `dependencies = ["package-name>=1.0"]`
/// - `package-name = "^1.0"` (Poetry style)
static PYPROJECT_DEPENDENCY_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"["']([a-zA-Z0-9_-]+)[><=!~\[\]"'\s]"#)
        .expect("pyproject dependency pattern should be valid")
});

/// Regex pattern for requirements.txt entries
///
/// Matches: `package-name==1.0.0`, `package-name>=1.0`, `package-name`
static REQUIREMENTS_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^([a-zA-Z0-9_-]+)(?:\s*[><=!~\[]|$)")
        .expect("requirements pattern should be valid")
});

/// Python reference detector implementation
///
/// Detects files affected by Python package renames by scanning for:
/// - Python import statements referencing the old package name
/// - pyproject.toml dependency declarations
/// - requirements.txt package references
#[derive(Default)]
pub struct PythonReferenceDetector;

impl PythonReferenceDetector {
    /// Creates a new Python reference detector instance.
    pub fn new() -> Self {
        Self
    }

    /// Extract the Python package name from a directory path.
    ///
    /// For Python packages, the package name is typically:
    /// - The directory name itself (for simple packages)
    /// - The `name` field in pyproject.toml (if present)
    ///
    /// This function checks for pyproject.toml first, then falls back to directory name.
    async fn extract_package_name(path: &Path) -> Option<String> {
        if !path.is_dir() {
            // For files, use the parent directory or file stem
            return path
                .file_stem()
                .and_then(|s| s.to_str())
                .map(String::from);
        }

        // Check for pyproject.toml to get the actual package name
        let pyproject_path = path.join("pyproject.toml");
        if pyproject_path.exists() {
            if let Ok(content) = tokio::fs::read_to_string(&pyproject_path).await {
                // Try to extract name from [project] section
                if let Some(name) = extract_name_from_pyproject(&content) {
                    return Some(name);
                }
            }
        }

        // Fall back to directory name
        path.file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.replace('-', "_")) // Normalize: hyphens to underscores for imports
    }

    /// Check if a Python file imports from a specific package.
    ///
    /// Detects both:
    /// - `import package_name`
    /// - `from package_name import X`
    /// - `from package_name.submodule import Y`
    fn file_imports_package(content: &str, package_name: &str) -> bool {
        for line in content.lines() {
            let trimmed = line.trim();

            // Skip comments
            if trimmed.starts_with('#') {
                continue;
            }

            // Check for "import package_name" or "import package_name.submodule"
            if trimmed.starts_with("import ") {
                let import_part = &trimmed[7..]; // Skip "import "
                let module = import_part
                    .split_whitespace()
                    .next()
                    .unwrap_or("");

                // Check if module starts with package name
                if module == package_name || module.starts_with(&format!("{}.", package_name)) {
                    return true;
                }
            }

            // Check for "from package_name import X" or "from package_name.submodule import Y"
            if trimmed.starts_with("from ") {
                if let Some(rest) = trimmed.strip_prefix("from ") {
                    // Extract module path before "import"
                    if let Some(import_idx) = rest.find(" import") {
                        let module = rest[..import_idx].trim();

                        // Check if module starts with package name
                        if module == package_name || module.starts_with(&format!("{}.", package_name)) {
                            return true;
                        }
                    }
                }
            }
        }

        false
    }

    /// Check if a pyproject.toml file references a package as a dependency.
    fn pyproject_references_package(content: &str, package_name: &str) -> bool {
        // Normalize package name for comparison (Python allows hyphens and underscores interchangeably)
        let normalized_name = package_name.replace('_', "-");
        let alt_normalized = package_name.replace('-', "_");

        // Check for dependency declarations
        for cap in PYPROJECT_DEPENDENCY_PATTERN.captures_iter(content) {
            if let Some(dep_name) = cap.get(1) {
                let dep = dep_name.as_str();
                if dep == package_name || dep == normalized_name || dep == alt_normalized {
                    return true;
                }
            }
        }

        // Also check for direct key-value style: `package-name = "version"`
        let key_pattern = format!(
            r#"(?m)^[\s]*["']?{}["']?\s*="#,
            regex::escape(package_name)
        );
        if let Ok(re) = Regex::new(&key_pattern) {
            if re.is_match(content) {
                return true;
            }
        }

        // Check alternative normalized form
        let alt_key_pattern = format!(
            r#"(?m)^[\s]*["']?{}["']?\s*="#,
            regex::escape(&normalized_name)
        );
        if let Ok(re) = Regex::new(&alt_key_pattern) {
            if re.is_match(content) {
                return true;
            }
        }

        false
    }

    /// Check if a requirements.txt file references a package.
    fn requirements_references_package(content: &str, package_name: &str) -> bool {
        // Normalize package name
        let normalized_name = package_name.replace('_', "-");
        let alt_normalized = package_name.replace('-', "_");

        for line in content.lines() {
            let trimmed = line.trim();

            // Skip comments and empty lines
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            // Extract package name from requirements line
            if let Some(cap) = REQUIREMENTS_PATTERN.captures(trimmed) {
                if let Some(dep_name) = cap.get(1) {
                    let dep = dep_name.as_str();
                    if dep == package_name || dep == normalized_name || dep == alt_normalized {
                        return true;
                    }
                }
            }
        }

        false
    }
}

/// Extract the package name from pyproject.toml content.
///
/// Looks for the `name` field in `[project]` or `[tool.poetry]` sections.
fn extract_name_from_pyproject(content: &str) -> Option<String> {
    // Try [project] section first (PEP 621 standard)
    let project_name_re = Regex::new(r#"(?m)^\s*\[project\][\s\S]*?name\s*=\s*["']([^"']+)["']"#)
        .ok()?;
    if let Some(cap) = project_name_re.captures(content) {
        if let Some(name) = cap.get(1) {
            return Some(name.as_str().to_string());
        }
    }

    // Try [tool.poetry] section (Poetry)
    let poetry_name_re =
        Regex::new(r#"(?m)^\s*\[tool\.poetry\][\s\S]*?name\s*=\s*["']([^"']+)["']"#).ok()?;
    if let Some(cap) = poetry_name_re.captures(content) {
        if let Some(name) = cap.get(1) {
            return Some(name.as_str().to_string());
        }
    }

    // Simple fallback: look for any `name = "..."` at the start of a line
    let simple_name_re = Regex::new(r#"(?m)^name\s*=\s*["']([^"']+)["']"#).ok()?;
    if let Some(cap) = simple_name_re.captures(content) {
        if let Some(name) = cap.get(1) {
            return Some(name.as_str().to_string());
        }
    }

    None
}

#[async_trait]
impl ReferenceDetector for PythonReferenceDetector {
    /// Find Python files affected by a package rename or move.
    ///
    /// This method scans the project for:
    /// 1. Python files (.py) that import from the old package
    /// 2. pyproject.toml files that declare the package as a dependency
    /// 3. requirements.txt files that reference the package
    ///
    /// # Arguments
    ///
    /// * `old_path` - The current path of the package being renamed/moved
    /// * `new_path` - The target path after the rename/move
    /// * `project_root` - Root directory of the project
    /// * `project_files` - List of all files in the project (for scanning)
    ///
    /// # Returns
    ///
    /// List of file paths that contain references needing updates
    async fn find_affected_files(
        &self,
        old_path: &Path,
        new_path: &Path,
        project_root: &Path,
        project_files: &[PathBuf],
    ) -> Vec<PathBuf> {
        let mut affected = Vec::new();

        tracing::info!(
            project_root = %project_root.display(),
            old_path = %old_path.display(),
            new_path = %new_path.display(),
            old_is_dir = old_path.is_dir(),
            "Starting Python reference detection"
        );

        // Extract package names from paths
        let old_package_name = match Self::extract_package_name(old_path).await {
            Some(name) => name,
            None => {
                tracing::warn!(
                    old_path = %old_path.display(),
                    "Could not extract package name from old path"
                );
                return affected;
            }
        };

        let new_package_name = match Self::extract_package_name(new_path).await {
            Some(name) => name,
            None => {
                tracing::warn!(
                    new_path = %new_path.display(),
                    "Could not extract package name from new path"
                );
                // Continue anyway - we can still detect references to old package
                old_package_name.clone()
            }
        };

        tracing::info!(
            old_package = %old_package_name,
            new_package = %new_package_name,
            "Extracted Python package names"
        );

        // Only scan if the package names differ (or for directory renames)
        if old_package_name == new_package_name && !old_path.is_dir() {
            tracing::info!("Package names are identical - no affected files");
            return affected;
        }

        // Parallelize scanning using JoinSet
        let mut set = JoinSet::new();

        for file in project_files {
            // Skip files inside the renamed package itself
            if file.starts_with(old_path) {
                continue;
            }

            let file_path = file.clone();
            let package_name = old_package_name.clone();

            // Determine file type and check accordingly
            let extension = file.extension().and_then(|e| e.to_str());
            let filename = file.file_name().and_then(|n| n.to_str());

            match (extension, filename) {
                // Python source files
                (Some("py"), _) => {
                    set.spawn(async move {
                        if let Ok(content) = tokio::fs::read_to_string(&file_path).await {
                            if Self::file_imports_package(&content, &package_name) {
                                tracing::debug!(
                                    file = %file_path.display(),
                                    package = %package_name,
                                    "Found Python file importing from old package"
                                );
                                return Some(file_path);
                            }
                        }
                        None
                    });
                }

                // pyproject.toml files
                (Some("toml"), Some("pyproject.toml")) | (_, Some("pyproject.toml")) => {
                    set.spawn(async move {
                        if let Ok(content) = tokio::fs::read_to_string(&file_path).await {
                            if Self::pyproject_references_package(&content, &package_name) {
                                tracing::debug!(
                                    file = %file_path.display(),
                                    package = %package_name,
                                    "Found pyproject.toml referencing old package"
                                );
                                return Some(file_path);
                            }
                        }
                        None
                    });
                }

                // requirements.txt files (including variations like requirements-dev.txt)
                (Some("txt"), Some(name)) if name.starts_with("requirements") => {
                    set.spawn(async move {
                        if let Ok(content) = tokio::fs::read_to_string(&file_path).await {
                            if Self::requirements_references_package(&content, &package_name) {
                                tracing::debug!(
                                    file = %file_path.display(),
                                    package = %package_name,
                                    "Found requirements file referencing old package"
                                );
                                return Some(file_path);
                            }
                        }
                        None
                    });
                }

                _ => {
                    // Skip other file types
                }
            }
        }

        // Collect results
        while let Some(res) = set.join_next().await {
            if let Ok(Some(file)) = res {
                if !affected.contains(&file) {
                    affected.push(file);
                }
            }
        }

        tracing::info!(
            affected_count = affected.len(),
            affected_files = ?affected.iter().map(|p| p.display().to_string()).collect::<Vec<_>>(),
            "Found Python files affected by package rename"
        );

        affected
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_file_imports_package_basic_import() {
        let content = r#"
import os
import mypackage
import json
"#;
        assert!(PythonReferenceDetector::file_imports_package(content, "mypackage"));
        assert!(!PythonReferenceDetector::file_imports_package(content, "other"));
    }

    #[test]
    fn test_file_imports_package_from_import() {
        let content = r#"
from mypackage import utils
from mypackage.submodule import helper
from other import thing
"#;
        assert!(PythonReferenceDetector::file_imports_package(content, "mypackage"));
        assert!(PythonReferenceDetector::file_imports_package(content, "other"));
        assert!(!PythonReferenceDetector::file_imports_package(content, "missing"));
    }

    #[test]
    fn test_file_imports_package_submodule() {
        let content = r#"
import mypackage.utils
import mypackage.submodule.helper
"#;
        assert!(PythonReferenceDetector::file_imports_package(content, "mypackage"));
        assert!(!PythonReferenceDetector::file_imports_package(content, "mypack")); // Partial match should fail
    }

    #[test]
    fn test_file_imports_package_with_alias() {
        let content = r#"
import mypackage as mp
from mypackage import utils as u
"#;
        assert!(PythonReferenceDetector::file_imports_package(content, "mypackage"));
    }

    #[test]
    fn test_file_imports_ignores_comments() {
        let content = r#"
# import mypackage
# from mypackage import utils
import other
"#;
        assert!(!PythonReferenceDetector::file_imports_package(content, "mypackage"));
        assert!(PythonReferenceDetector::file_imports_package(content, "other"));
    }

    #[test]
    fn test_pyproject_references_package_array_style() {
        let content = r#"
[project]
name = "myapp"
dependencies = [
    "mypackage>=1.0.0",
    "requests>=2.0",
]
"#;
        assert!(PythonReferenceDetector::pyproject_references_package(content, "mypackage"));
        assert!(PythonReferenceDetector::pyproject_references_package(content, "requests"));
        assert!(!PythonReferenceDetector::pyproject_references_package(content, "missing"));
    }

    #[test]
    fn test_pyproject_references_package_poetry_style() {
        let content = r#"
[tool.poetry.dependencies]
python = "^3.9"
mypackage = "^1.0.0"
requests = ">=2.0"
"#;
        assert!(PythonReferenceDetector::pyproject_references_package(content, "mypackage"));
        assert!(PythonReferenceDetector::pyproject_references_package(content, "requests"));
    }

    #[test]
    fn test_pyproject_references_package_normalized_names() {
        let content = r#"
[project]
dependencies = [
    "my-package>=1.0.0",
]
"#;
        // Should match both hyphenated and underscored versions
        assert!(PythonReferenceDetector::pyproject_references_package(content, "my-package"));
        assert!(PythonReferenceDetector::pyproject_references_package(content, "my_package"));
    }

    #[test]
    fn test_requirements_references_package() {
        let content = r#"
# Requirements file
mypackage==1.0.0
requests>=2.25.0
flask
django[extras]>=3.0
"#;
        assert!(PythonReferenceDetector::requirements_references_package(content, "mypackage"));
        assert!(PythonReferenceDetector::requirements_references_package(content, "requests"));
        assert!(PythonReferenceDetector::requirements_references_package(content, "flask"));
        assert!(PythonReferenceDetector::requirements_references_package(content, "django"));
        assert!(!PythonReferenceDetector::requirements_references_package(content, "missing"));
    }

    #[test]
    fn test_requirements_references_normalized_names() {
        let content = r#"
my-package==1.0.0
"#;
        assert!(PythonReferenceDetector::requirements_references_package(content, "my-package"));
        assert!(PythonReferenceDetector::requirements_references_package(content, "my_package"));
    }

    #[test]
    fn test_requirements_ignores_comments() {
        let content = r#"
# mypackage==1.0.0
other==2.0.0
"#;
        assert!(!PythonReferenceDetector::requirements_references_package(content, "mypackage"));
        assert!(PythonReferenceDetector::requirements_references_package(content, "other"));
    }

    #[test]
    fn test_extract_name_from_pyproject_pep621() {
        let content = r#"
[project]
name = "my-awesome-package"
version = "1.0.0"
"#;
        assert_eq!(
            extract_name_from_pyproject(content),
            Some("my-awesome-package".to_string())
        );
    }

    #[test]
    fn test_extract_name_from_pyproject_poetry() {
        let content = r#"
[tool.poetry]
name = "poetry-package"
version = "0.1.0"
"#;
        assert_eq!(
            extract_name_from_pyproject(content),
            Some("poetry-package".to_string())
        );
    }

    #[tokio::test]
    async fn test_find_affected_files_integration() {
        // Setup: Create a temporary directory with Python project structure
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();

        // Create old package directory
        let old_package = project_root.join("old_package");
        tokio::fs::create_dir(&old_package).await.unwrap();
        tokio::fs::write(old_package.join("__init__.py"), "").await.unwrap();

        // Create pyproject.toml
        tokio::fs::write(
            old_package.join("pyproject.toml"),
            r#"[project]
name = "old_package"
version = "1.0.0"
"#,
        )
        .await
        .unwrap();

        // Create app that imports from old_package
        let app_dir = project_root.join("app");
        tokio::fs::create_dir(&app_dir).await.unwrap();
        tokio::fs::write(
            app_dir.join("main.py"),
            r#"from old_package import utils

def main():
    utils.do_something()
"#,
        )
        .await
        .unwrap();

        // Create requirements.txt that references old_package
        tokio::fs::write(
            project_root.join("requirements.txt"),
            r#"old_package==1.0.0
requests>=2.0
"#,
        )
        .await
        .unwrap();

        // Define paths
        let new_package = project_root.join("new_package");

        // Project files list
        let project_files = vec![
            app_dir.join("main.py"),
            project_root.join("requirements.txt"),
        ];

        // Test: Run the detector
        let detector = PythonReferenceDetector::new();
        let affected = detector
            .find_affected_files(&old_package, &new_package, project_root, &project_files)
            .await;

        // Verify: app/main.py should be affected (imports from old_package)
        assert!(
            affected.contains(&app_dir.join("main.py")),
            "app/main.py should be detected as affected. Affected files: {:?}",
            affected
        );

        // Verify: requirements.txt should be affected
        assert!(
            affected.contains(&project_root.join("requirements.txt")),
            "requirements.txt should be detected as affected. Affected files: {:?}",
            affected
        );
    }

    #[tokio::test]
    async fn test_extract_package_name_from_pyproject() {
        let temp_dir = TempDir::new().unwrap();
        let pkg_dir = temp_dir.path().join("my-package");
        tokio::fs::create_dir(&pkg_dir).await.unwrap();
        tokio::fs::write(
            pkg_dir.join("pyproject.toml"),
            r#"[project]
name = "my-package"
version = "1.0.0"
"#,
        )
        .await
        .unwrap();

        let name = PythonReferenceDetector::extract_package_name(&pkg_dir).await;
        assert_eq!(name, Some("my-package".to_string()));
    }

    #[tokio::test]
    async fn test_extract_package_name_fallback_to_dirname() {
        let temp_dir = TempDir::new().unwrap();
        let pkg_dir = temp_dir.path().join("my_package");
        tokio::fs::create_dir(&pkg_dir).await.unwrap();

        let name = PythonReferenceDetector::extract_package_name(&pkg_dir).await;
        assert_eq!(name, Some("my_package".to_string()));
    }

    #[tokio::test]
    async fn test_extract_package_name_normalizes_hyphens() {
        let temp_dir = TempDir::new().unwrap();
        let pkg_dir = temp_dir.path().join("my-package-name");
        tokio::fs::create_dir(&pkg_dir).await.unwrap();

        let name = PythonReferenceDetector::extract_package_name(&pkg_dir).await;
        // Directory name with hyphens should be normalized to underscores
        assert_eq!(name, Some("my_package_name".to_string()));
    }
}
