//! Language and package manager detection utilities
//!
//! This module provides shared functionality for detecting project languages
//! and package managers based on manifest files and project structure.

use crate::iter_plugins;
use std::path::Path;
use tracing::debug;

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

/// Detect the primary language of a project by examining manifest files.
///
/// This function iterates through all registered language plugins and checks for
/// the existence of the manifest file they declare. The first one found determines
/// the project language.
///
/// # Arguments
///
/// * `project_path` - Path to the project directory
///
/// # Returns
///
/// The name of the detected language, or `None` if no known manifest files are found.
pub fn detect_project_language(project_path: &Path) -> Option<&'static str> {
    debug!(path = %project_path.display(), "Detecting project language");

    for descriptor in iter_plugins() {
        if project_path.join(descriptor.manifest_filename).exists() {
            debug!(
                "Detected '{}' project (found {})",
                descriptor.name, descriptor.manifest_filename
            );
            return Some(descriptor.name);
        }
    }

    debug!("Could not detect any known project language");
    None
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
    fn test_detect_unknown_project() {
        use tempfile::tempdir;

        let dir = tempdir().unwrap();
        // Empty directory with no manifest files.
        // Since no plugins are registered in this test context, this will be None.
        let result = detect_project_language(dir.path());
        assert_eq!(result, None);
    }

    #[test]
    fn test_detect_npm_package_manager() {
        use std::fs::File;
        use tempfile::tempdir;

        let dir = tempdir().unwrap();
        File::create(dir.path().join("package.json")).unwrap();
        // No lock file means npm

        let result = detect_package_manager(dir.path());
        assert_eq!(result, PackageManager::Npm);
    }

    #[test]
    fn test_detect_yarn_package_manager() {
        use std::fs::File;
        use tempfile::tempdir;

        let dir = tempdir().unwrap();
        File::create(dir.path().join("package.json")).unwrap();
        File::create(dir.path().join("yarn.lock")).unwrap();

        let result = detect_package_manager(dir.path());
        assert_eq!(result, PackageManager::Yarn);
    }

    #[test]
    fn test_detect_pnpm_package_manager() {
        use std::fs::File;
        use tempfile::tempdir;

        let dir = tempdir().unwrap();
        File::create(dir.path().join("package.json")).unwrap();
        File::create(dir.path().join("pnpm-lock.yaml")).unwrap();

        let result = detect_package_manager(dir.path());
        assert_eq!(result, PackageManager::Pnpm);
    }

    #[test]
    fn test_detect_cargo_package_manager() {
        use std::fs::File;
        use tempfile::tempdir;

        let dir = tempdir().unwrap();
        File::create(dir.path().join("Cargo.toml")).unwrap();

        let result = detect_package_manager(dir.path());
        assert_eq!(result, PackageManager::Cargo);
    }

    #[test]
    fn test_detect_go_package_manager() {
        use std::fs::File;
        use tempfile::tempdir;

        let dir = tempdir().unwrap();
        File::create(dir.path().join("go.mod")).unwrap();

        let result = detect_package_manager(dir.path());
        assert_eq!(result, PackageManager::Go);
    }

    #[test]
    fn test_detect_pip_package_manager() {
        use std::fs::File;
        use tempfile::tempdir;

        let dir = tempdir().unwrap();
        File::create(dir.path().join("requirements.txt")).unwrap();

        let result = detect_package_manager(dir.path());
        assert_eq!(result, PackageManager::Pip);
    }

    #[test]
    fn test_detect_maven_package_manager() {
        use std::fs::File;
        use tempfile::tempdir;

        let dir = tempdir().unwrap();
        File::create(dir.path().join("pom.xml")).unwrap();

        let result = detect_package_manager(dir.path());
        assert_eq!(result, PackageManager::Maven);
    }

    #[test]
    fn test_detect_gradle_package_manager() {
        use std::fs::File;
        use tempfile::tempdir;

        let dir = tempdir().unwrap();
        File::create(dir.path().join("build.gradle")).unwrap();

        let result = detect_package_manager(dir.path());
        assert_eq!(result, PackageManager::Gradle);
    }

    #[test]
    fn test_detect_unknown_package_manager() {
        use tempfile::tempdir;

        let dir = tempdir().unwrap();
        // Empty directory

        let result = detect_package_manager(dir.path());
        assert_eq!(result, PackageManager::Unknown);
    }

    #[test]
    fn test_priority_yarn_over_npm() {
        use std::fs::File;
        use tempfile::tempdir;

        // Yarn lock file should take priority
        let dir = tempdir().unwrap();
        File::create(dir.path().join("package.json")).unwrap();
        File::create(dir.path().join("yarn.lock")).unwrap();

        let result = detect_package_manager(dir.path());
        assert_eq!(result, PackageManager::Yarn);
    }

    #[test]
    fn test_priority_pnpm_over_npm() {
        use std::fs::File;
        use tempfile::tempdir;

        // pnpm lock file should take priority
        let dir = tempdir().unwrap();
        File::create(dir.path().join("package.json")).unwrap();
        File::create(dir.path().join("pnpm-lock.yaml")).unwrap();

        let result = detect_package_manager(dir.path());
        assert_eq!(result, PackageManager::Pnpm);
    }
}
