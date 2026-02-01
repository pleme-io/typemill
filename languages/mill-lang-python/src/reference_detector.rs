//! Python-specific reference detection
//!
//! Handles detection of affected files for Python file moves and renames.
//! Detects import statements, from imports, and relative imports.

use async_trait::async_trait;
use mill_plugin_api::ReferenceDetector;
use regex::Regex;
use std::path::{Path, PathBuf};
use tokio::task::JoinSet;

/// Python reference detector implementation
#[derive(Default)]
pub struct PythonReferenceDetector;

impl PythonReferenceDetector {
    /// Creates a new Python reference detector instance.
    pub fn new() -> Self {
        Self
    }

    /// Convert a file path to a Python module path
    ///
    /// e.g., `src/utils/helpers.py` -> `utils.helpers`
    fn path_to_module(path: &Path, project_root: &Path) -> Option<String> {
        // Strip project root and src/ prefix if present
        let relative = path.strip_prefix(project_root).ok()?;

        // Remove .py extension
        let without_ext = relative.with_extension("");

        // Convert path separators to dots
        let module_path: String = without_ext
            .components()
            .filter_map(|c| c.as_os_str().to_str())
            .filter(|&s| s != "src") // Skip common src directory
            .collect::<Vec<_>>()
            .join(".");

        // Handle __init__.py - use parent module
        if module_path.ends_with(".__init__") {
            Some(module_path.trim_end_matches(".__init__").to_string())
        } else if module_path == "__init__" {
            None // Root __init__.py has no module name
        } else {
            Some(module_path)
        }
    }

    /// Check if an import statement references the target module
    fn import_matches(
        import_module: &str,
        target_module: &str,
        is_directory: bool,
    ) -> bool {
        if is_directory {
            // For directories, check if import starts with or equals the module
            import_module == target_module || import_module.starts_with(&format!("{}.", target_module))
        } else {
            // For files, check exact match or parent match
            import_module == target_module
        }
    }
}

/// Regex for Python import statement: `import module` or `import module as alias`
fn import_regex() -> Regex {
    Regex::new(r"^import\s+([a-zA-Z_][a-zA-Z0-9_.]*)(?:\s+as\s+\w+)?")
        .expect("import regex should be valid")
}

/// Regex for Python from import: `from module import ...`
fn from_import_regex() -> Regex {
    Regex::new(r"^from\s+([a-zA-Z_][a-zA-Z0-9_.]*|\.+[a-zA-Z_][a-zA-Z0-9_.]*|\.+)\s+import")
        .expect("from import regex should be valid")
}

/// Regex for relative imports: `from . import ...` or `from .. import ...`
fn relative_import_regex() -> Regex {
    Regex::new(r"^from\s+(\.+)([a-zA-Z_][a-zA-Z0-9_.]*)?\s+import")
        .expect("relative import regex should be valid")
}

#[async_trait]
impl ReferenceDetector for PythonReferenceDetector {
    /// Find affected Python files for a file move or rename
    ///
    /// Scans all Python files for:
    /// - Import statements: `import module`
    /// - From imports: `from module import ...`
    /// - Relative imports: `from . import ...`, `from ..module import ...`
    ///
    /// Returns a list of files that import from the old path.
    async fn find_affected_files(
        &self,
        old_path: &Path,
        _new_path: &Path,
        project_root: &Path,
        project_files: &[PathBuf],
    ) -> Vec<PathBuf> {
        let mut affected = Vec::new();

        tracing::info!(
            project_root = %project_root.display(),
            old_path = %old_path.display(),
            old_is_dir = old_path.is_dir(),
            "Starting Python reference detection"
        );

        let is_directory = old_path.is_dir();

        // Convert old path to module name
        let target_module = if is_directory {
            // For directories, use the directory name as the module
            old_path
                .file_name()
                .and_then(|n| n.to_str())
                .map(|s| s.to_string())
        } else {
            Self::path_to_module(old_path, project_root)
        };

        let target_module = match target_module {
            Some(m) if !m.is_empty() => m,
            _ => {
                tracing::warn!(
                    old_path = %old_path.display(),
                    "Could not determine module name for path"
                );
                return affected;
            }
        };

        tracing::info!(
            target_module = %target_module,
            "Computed target module name"
        );

        // Compile regexes once
        let import_re = import_regex();
        let from_import_re = from_import_regex();
        let relative_re = relative_import_regex();

        // Parallelize file scanning
        let mut set = JoinSet::new();

        for file in project_files {
            // Skip the file being moved
            if file == old_path {
                continue;
            }

            // Skip files inside the directory being moved
            if is_directory && file.starts_with(old_path) {
                continue;
            }

            // Only check Python files
            let ext = file.extension().and_then(|e| e.to_str()).unwrap_or("");
            if ext != "py" {
                continue;
            }

            // Skip common virtual env and cache directories
            let path_str = file.to_string_lossy();
            if path_str.contains("__pycache__")
                || path_str.contains(".venv")
                || path_str.contains("venv")
                || path_str.contains(".tox")
                || path_str.contains("site-packages")
            {
                continue;
            }

            let file_path = file.clone();
            let target = target_module.clone();
            let import_re = import_re.clone();
            let from_import_re = from_import_re.clone();
            let relative_re = relative_re.clone();
            let is_dir = is_directory;
            let old_path_owned = old_path.to_path_buf();
            let project_root_owned = project_root.to_path_buf();

            set.spawn(async move {
                if let Ok(content) = tokio::fs::read_to_string(&file_path).await {
                    let mut has_reference = false;

                    for line in content.lines() {
                        let trimmed = line.trim();

                        // Skip comments and empty lines
                        if trimmed.starts_with('#') || trimmed.is_empty() {
                            continue;
                        }

                        // Check `import module` statements
                        if let Some(cap) = import_re.captures(trimmed) {
                            if let Some(module) = cap.get(1) {
                                if Self::import_matches(module.as_str(), &target, is_dir) {
                                    has_reference = true;
                                    break;
                                }
                            }
                        }

                        // Check `from module import ...` statements
                        if let Some(cap) = from_import_re.captures(trimmed) {
                            if let Some(module) = cap.get(1) {
                                let module_str = module.as_str();

                                // Handle absolute imports
                                if !module_str.starts_with('.')
                                    && Self::import_matches(module_str, &target, is_dir)
                                {
                                    has_reference = true;
                                    break;
                                }
                            }
                        }

                        // Check relative imports
                        if let Some(cap) = relative_re.captures(trimmed) {
                            let dots = cap.get(1).map(|m| m.as_str()).unwrap_or("");
                            let module_suffix = cap.get(2).map(|m| m.as_str()).unwrap_or("");

                            // Resolve relative import to absolute module
                            if let Some(resolved) = resolve_relative_import(
                                &file_path,
                                &project_root_owned,
                                dots,
                                module_suffix,
                            ) {
                                if Self::import_matches(&resolved, &target, is_dir) {
                                    has_reference = true;
                                    break;
                                }

                                // Also check if the resolved path matches the old path
                                if is_dir {
                                    let old_module = old_path_owned
                                        .file_name()
                                        .and_then(|n| n.to_str())
                                        .unwrap_or("");
                                    if resolved == old_module || resolved.starts_with(&format!("{}.", old_module)) {
                                        has_reference = true;
                                        break;
                                    }
                                }
                            }
                        }
                    }

                    if has_reference {
                        tracing::debug!(
                            file = %file_path.display(),
                            target = %target,
                            "Found Python file importing from target module"
                        );
                        return Some(file_path);
                    }
                }
                None
            });
        }

        while let Some(res) = set.join_next().await {
            if let Ok(Some(file)) = res {
                if !affected.contains(&file) {
                    affected.push(file);
                }
            }
        }

        tracing::info!(
            affected_count = affected.len(),
            "Found Python files affected by file move/rename"
        );

        affected
    }
}

/// Resolve a relative import to an absolute module path
fn resolve_relative_import(
    importing_file: &Path,
    project_root: &Path,
    dots: &str,
    module_suffix: &str,
) -> Option<String> {
    let levels = dots.len();

    // Get the directory containing the importing file
    let mut current_dir = importing_file.parent()?;

    // Go up `levels` directories (each dot represents one level up)
    // The first dot means current package, not parent
    for _ in 1..levels {
        current_dir = current_dir.parent()?;
    }

    // Build the module path from the resolved directory
    let relative = current_dir.strip_prefix(project_root).ok()?;

    let base_parts: Vec<&str> = relative
        .components()
        .filter_map(|c| c.as_os_str().to_str())
        .filter(|&s| s != "src")
        .collect();

    let mut module_path = base_parts.join(".");

    if !module_suffix.is_empty() {
        if !module_path.is_empty() {
            module_path.push('.');
        }
        module_path.push_str(module_suffix);
    }

    if module_path.is_empty() {
        None
    } else {
        Some(module_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_import_detection() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();

        // Create utils.py (the file being renamed)
        tokio::fs::write(project_root.join("utils.py"), "def helper(): pass")
            .await
            .unwrap();

        // Create app.py that imports from utils
        tokio::fs::write(
            project_root.join("app.py"),
            "import utils\n\nutils.helper()",
        )
        .await
        .unwrap();

        let old_path = project_root.join("utils.py");
        let new_path = project_root.join("helpers.py");

        let project_files = vec![project_root.join("utils.py"), project_root.join("app.py")];

        let detector = PythonReferenceDetector::new();
        let affected = detector
            .find_affected_files(&old_path, &new_path, project_root, &project_files)
            .await;

        assert!(
            affected.contains(&project_root.join("app.py")),
            "app.py should be detected as affected. Affected: {:?}",
            affected
        );
    }

    #[tokio::test]
    async fn test_from_import_detection() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();

        // Create utils.py
        tokio::fs::write(project_root.join("utils.py"), "def helper(): pass")
            .await
            .unwrap();

        // Create app.py with from import
        tokio::fs::write(
            project_root.join("app.py"),
            "from utils import helper\n\nhelper()",
        )
        .await
        .unwrap();

        let old_path = project_root.join("utils.py");
        let new_path = project_root.join("helpers.py");

        let project_files = vec![project_root.join("utils.py"), project_root.join("app.py")];

        let detector = PythonReferenceDetector::new();
        let affected = detector
            .find_affected_files(&old_path, &new_path, project_root, &project_files)
            .await;

        assert!(
            affected.contains(&project_root.join("app.py")),
            "app.py should be detected as affected. Affected: {:?}",
            affected
        );
    }

    #[tokio::test]
    async fn test_directory_move_detection() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();

        // Create utils/__init__.py
        tokio::fs::create_dir_all(project_root.join("utils"))
            .await
            .unwrap();
        tokio::fs::write(
            project_root.join("utils/__init__.py"),
            "from .helpers import process",
        )
        .await
        .unwrap();
        tokio::fs::write(
            project_root.join("utils/helpers.py"),
            "def process(): pass",
        )
        .await
        .unwrap();

        // Create app.py that imports from utils package
        tokio::fs::write(
            project_root.join("app.py"),
            "from utils import process\n\nprocess()",
        )
        .await
        .unwrap();

        let old_path = project_root.join("utils");
        let new_path = project_root.join("helpers");

        let project_files = vec![
            project_root.join("utils/__init__.py"),
            project_root.join("utils/helpers.py"),
            project_root.join("app.py"),
        ];

        let detector = PythonReferenceDetector::new();
        let affected = detector
            .find_affected_files(&old_path, &new_path, project_root, &project_files)
            .await;

        assert!(
            affected.contains(&project_root.join("app.py")),
            "app.py should be detected as affected (package import). Affected: {:?}",
            affected
        );
    }

    #[tokio::test]
    async fn test_dotted_import_detection() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();

        // Create package/submodule.py
        tokio::fs::create_dir_all(project_root.join("package"))
            .await
            .unwrap();
        tokio::fs::write(project_root.join("package/__init__.py"), "")
            .await
            .unwrap();
        tokio::fs::write(
            project_root.join("package/submodule.py"),
            "def func(): pass",
        )
        .await
        .unwrap();

        // Create app.py with dotted import
        tokio::fs::write(
            project_root.join("app.py"),
            "import package.submodule\n\npackage.submodule.func()",
        )
        .await
        .unwrap();

        let old_path = project_root.join("package/submodule.py");
        let new_path = project_root.join("package/renamed.py");

        let project_files = vec![
            project_root.join("package/__init__.py"),
            project_root.join("package/submodule.py"),
            project_root.join("app.py"),
        ];

        let detector = PythonReferenceDetector::new();
        let affected = detector
            .find_affected_files(&old_path, &new_path, project_root, &project_files)
            .await;

        assert!(
            affected.contains(&project_root.join("app.py")),
            "app.py should be detected as affected (dotted import). Affected: {:?}",
            affected
        );
    }

    #[test]
    fn test_path_to_module() {
        let project_root = Path::new("/project");

        // Regular file
        assert_eq!(
            PythonReferenceDetector::path_to_module(
                Path::new("/project/utils.py"),
                project_root
            ),
            Some("utils".to_string())
        );

        // Nested file
        assert_eq!(
            PythonReferenceDetector::path_to_module(
                Path::new("/project/package/module.py"),
                project_root
            ),
            Some("package.module".to_string())
        );

        // __init__.py
        assert_eq!(
            PythonReferenceDetector::path_to_module(
                Path::new("/project/package/__init__.py"),
                project_root
            ),
            Some("package".to_string())
        );

        // With src directory
        assert_eq!(
            PythonReferenceDetector::path_to_module(
                Path::new("/project/src/package/module.py"),
                project_root
            ),
            Some("package.module".to_string())
        );
    }

    #[test]
    fn test_import_matches() {
        // Exact match for file
        assert!(PythonReferenceDetector::import_matches("utils", "utils", false));

        // No match for different module
        assert!(!PythonReferenceDetector::import_matches("helpers", "utils", false));

        // Directory match
        assert!(PythonReferenceDetector::import_matches("utils", "utils", true));
        assert!(PythonReferenceDetector::import_matches("utils.helpers", "utils", true));

        // No partial match for files
        assert!(!PythonReferenceDetector::import_matches("utils.helpers", "utils", false));
    }
}
