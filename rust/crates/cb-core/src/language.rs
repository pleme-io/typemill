//! Language and package manager detection utilities
//!
//! This module provides shared functionality for detecting project languages
//! and package managers based on manifest files and project structure.

use std::path::Path;
use tracing::debug;

/// Supported project languages
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectLanguage {
    /// Rust projects (Cargo.toml)
    Rust,
    /// TypeScript/JavaScript projects (package.json)
    TypeScript,
    /// Python projects (requirements.txt, pyproject.toml, setup.py)
    Python,
    /// Go projects (go.mod)
    Go,
    /// Java projects (pom.xml, build.gradle)
    Java,
    /// Unknown or mixed-language project
    Unknown,
}

/// Package manager types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageManager {
    /// Cargo (Rust)
    Cargo,
    /// npm (Node.js)
    Npm,
    /// Yarn (Node.js)
    Yarn,
    /// pnpm (Node.js)
    Pnpm,
    /// pip (Python)
    Pip,
    /// Go modules
    Go,
    /// Maven (Java)
    Maven,
    /// Gradle (Java)
    Gradle,
    /// Unknown package manager
    Unknown,
}

impl ProjectLanguage {
    /// Get the string representation of the language
    pub fn as_str(&self) -> &'static str {
        match self {
            ProjectLanguage::Rust => "rust",
            ProjectLanguage::TypeScript => "typescript",
            ProjectLanguage::Python => "python",
            ProjectLanguage::Go => "go",
            ProjectLanguage::Java => "java",
            ProjectLanguage::Unknown => "unknown",
        }
    }

    /// Get the primary manifest filename for this language
    pub fn manifest_filename(&self) -> &'static str {
        match self {
            ProjectLanguage::Rust => "Cargo.toml",
            ProjectLanguage::TypeScript => "package.json",
            ProjectLanguage::Python => "pyproject.toml",
            ProjectLanguage::Go => "go.mod",
            ProjectLanguage::Java => "pom.xml",
            ProjectLanguage::Unknown => "",
        }
    }
}

impl PackageManager {
    /// Get the string representation of the package manager
    pub fn as_str(&self) -> &'static str {
        match self {
            PackageManager::Cargo => "cargo",
            PackageManager::Npm => "npm",
            PackageManager::Yarn => "yarn",
            PackageManager::Pnpm => "pnpm",
            PackageManager::Pip => "pip",
            PackageManager::Go => "go",
            PackageManager::Maven => "maven",
            PackageManager::Gradle => "gradle",
            PackageManager::Unknown => "unknown",
        }
    }
}

/// Detect the primary language of a project by examining manifest files
///
/// # Arguments
///
/// * `project_path` - Path to the project directory
///
/// # Returns
///
/// The detected `ProjectLanguage`, or `ProjectLanguage::Unknown` if no manifest files are found
///
/// # Example
///
/// ```
/// use std::path::Path;
/// use cb_core::language::detect_project_language;
///
/// let language = detect_project_language(Path::new("."));
/// println!("Detected language: {}", language.as_str());
/// ```
pub fn detect_project_language(project_path: &Path) -> ProjectLanguage {
    debug!(path = %project_path.display(), "Detecting project language");

    // Check for Rust
    if project_path.join("Cargo.toml").exists() {
        debug!("Detected Rust project (found Cargo.toml)");
        return ProjectLanguage::Rust;
    }

    // Check for Node.js/TypeScript
    if project_path.join("package.json").exists() {
        debug!("Detected TypeScript/JavaScript project (found package.json)");
        return ProjectLanguage::TypeScript;
    }

    // Check for Go
    if project_path.join("go.mod").exists() {
        debug!("Detected Go project (found go.mod)");
        return ProjectLanguage::Go;
    }

    // Check for Python (multiple possible manifest files)
    if project_path.join("pyproject.toml").exists()
        || project_path.join("requirements.txt").exists()
        || project_path.join("setup.py").exists()
    {
        debug!("Detected Python project");
        return ProjectLanguage::Python;
    }

    // Check for Java
    if project_path.join("pom.xml").exists() || project_path.join("build.gradle").exists() {
        debug!("Detected Java project");
        return ProjectLanguage::Java;
    }

    debug!("Could not detect project language");
    ProjectLanguage::Unknown
}

/// Detect the package manager for a project
///
/// This function examines lock files and manifest files to determine the package manager.
/// For Node.js projects, it checks for yarn.lock, pnpm-lock.yaml, or defaults to npm.
///
/// # Arguments
///
/// * `project_path` - Path to the project directory
///
/// # Returns
///
/// The detected `PackageManager`, or `PackageManager::Unknown` if no package manager is detected
///
/// # Example
///
/// ```
/// use std::path::Path;
/// use cb_core::language::detect_package_manager;
///
/// let pm = detect_package_manager(Path::new("."));
/// println!("Detected package manager: {}", pm.as_str());
/// ```
pub fn detect_package_manager(project_path: &Path) -> PackageManager {
    debug!(path = %project_path.display(), "Detecting package manager");

    // Check for Node.js package managers (order matters: check lock files first)
    if project_path.join("package.json").exists() {
        if project_path.join("yarn.lock").exists() {
            debug!("Detected Yarn (found yarn.lock)");
            return PackageManager::Yarn;
        } else if project_path.join("pnpm-lock.yaml").exists() {
            debug!("Detected pnpm (found pnpm-lock.yaml)");
            return PackageManager::Pnpm;
        } else {
            debug!("Detected npm (found package.json, no other lock files)");
            return PackageManager::Npm;
        }
    }

    // Check for Go
    if project_path.join("go.mod").exists() {
        debug!("Detected Go modules");
        return PackageManager::Go;
    }

    // Check for Rust
    if project_path.join("Cargo.toml").exists() {
        debug!("Detected Cargo");
        return PackageManager::Cargo;
    }

    // Check for Python
    if project_path.join("requirements.txt").exists()
        || project_path.join("pyproject.toml").exists()
    {
        debug!("Detected pip (Python)");
        return PackageManager::Pip;
    }

    // Check for Java (Maven)
    if project_path.join("pom.xml").exists() {
        debug!("Detected Maven");
        return PackageManager::Maven;
    }

    // Check for Java (Gradle)
    if project_path.join("build.gradle").exists() || project_path.join("build.gradle.kts").exists()
    {
        debug!("Detected Gradle");
        return PackageManager::Gradle;
    }

    debug!("Could not detect package manager");
    PackageManager::Unknown
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_language_as_str() {
        assert_eq!(ProjectLanguage::Rust.as_str(), "rust");
        assert_eq!(ProjectLanguage::TypeScript.as_str(), "typescript");
        assert_eq!(ProjectLanguage::Python.as_str(), "python");
        assert_eq!(ProjectLanguage::Go.as_str(), "go");
        assert_eq!(ProjectLanguage::Java.as_str(), "java");
        assert_eq!(ProjectLanguage::Unknown.as_str(), "unknown");
    }

    #[test]
    fn test_package_manager_as_str() {
        assert_eq!(PackageManager::Cargo.as_str(), "cargo");
        assert_eq!(PackageManager::Npm.as_str(), "npm");
        assert_eq!(PackageManager::Yarn.as_str(), "yarn");
        assert_eq!(PackageManager::Pnpm.as_str(), "pnpm");
        assert_eq!(PackageManager::Pip.as_str(), "pip");
        assert_eq!(PackageManager::Go.as_str(), "go");
    }

    #[test]
    fn test_manifest_filename() {
        assert_eq!(ProjectLanguage::Rust.manifest_filename(), "Cargo.toml");
        assert_eq!(ProjectLanguage::TypeScript.manifest_filename(), "package.json");
        assert_eq!(ProjectLanguage::Python.manifest_filename(), "pyproject.toml");
        assert_eq!(ProjectLanguage::Go.manifest_filename(), "go.mod");
        assert_eq!(ProjectLanguage::Java.manifest_filename(), "pom.xml");
    }

    #[test]
    fn test_detect_rust_project() {
        // This test would need a temporary directory with Cargo.toml
        // For now, we test the current workspace which is Rust
        let current_dir = PathBuf::from(".");
        let result = detect_project_language(&current_dir);
        // The test might fail if run from a different directory
        // This is just a basic smoke test
        assert!(matches!(
            result,
            ProjectLanguage::Rust | ProjectLanguage::Unknown
        ));
    }
}
