//! TypeScript-specific reference detection
//!
//! Handles detection of affected files for TypeScript/JavaScript file moves and renames.
//! Detects ES6 imports, CommonJS requires, dynamic imports, and re-exports.

use async_trait::async_trait;
use mill_plugin_api::ReferenceDetector;
use regex::Regex;
use std::path::{Path, PathBuf};
use tokio::task::JoinSet;

use crate::constants::{DYNAMIC_IMPORT_RE, ES6_IMPORT_RE, REQUIRE_RE};

/// TypeScript/JavaScript reference detector implementation
#[derive(Default)]
pub struct TypeScriptReferenceDetector;

impl TypeScriptReferenceDetector {
    /// Creates a new TypeScript reference detector instance.
    pub fn new() -> Self {
        Self
    }

    /// Check if a module path references the target file
    ///
    /// Handles both relative paths (./foo, ../bar) and compares against
    /// the expected relative path from the importing file to the target.
    fn module_path_matches(
        module_path: &str,
        old_path: &Path,
        importing_file: &Path,
        project_root: &Path,
    ) -> bool {
        // Skip node_modules and external packages
        if !module_path.starts_with('.') && !module_path.starts_with('/') {
            return false;
        }

        // Get the directory containing the importing file
        let importing_dir = importing_file.parent().unwrap_or(project_root);

        // Resolve the module path relative to the importing file
        let resolved_path = if module_path.starts_with("./") || module_path.starts_with("../") {
            importing_dir.join(module_path)
        } else if module_path.starts_with('/') {
            // Absolute path from project root
            project_root.join(module_path.trim_start_matches('/'))
        } else {
            return false;
        };

        // Normalize the resolved path
        let normalized = normalize_path(&resolved_path);

        // Check if it matches the old path (with or without extension)
        let old_normalized = normalize_path(old_path);
        let old_stem = old_path.with_extension("");
        let old_stem_normalized = normalize_path(&old_stem);

        // Direct match
        if normalized == old_normalized {
            return true;
        }

        // Match without extension (TypeScript allows omitting .ts/.js)
        if normalized == old_stem_normalized {
            return true;
        }

        // Match with index file (import './foo' could mean './foo/index')
        let with_index = normalized.join("index");
        if with_index == old_stem_normalized {
            return true;
        }

        // Check various extensions
        for ext in &["ts", "tsx", "js", "jsx", "mjs", "cjs"] {
            let with_ext = normalized.with_extension(ext);
            if with_ext == old_normalized {
                return true;
            }
        }

        false
    }
}

/// Normalize a path by resolving . and .. components
fn normalize_path(path: &Path) -> PathBuf {
    let mut components = Vec::new();
    for component in path.components() {
        match component {
            std::path::Component::ParentDir => {
                components.pop();
            }
            std::path::Component::CurDir => {}
            c => components.push(c),
        }
    }
    components.iter().collect()
}

/// Regex for ES6 re-exports: export ... from 'module'
fn re_export_regex() -> Regex {
    Regex::new(r#"export\s+.*?from\s+['"]([^'"]+)['"]"#).expect("re-export regex should be valid")
}

#[async_trait]
impl ReferenceDetector for TypeScriptReferenceDetector {
    /// Find affected TypeScript/JavaScript files for a file move or rename
    ///
    /// Scans all TS/JS files for:
    /// - ES6 imports: `import ... from 'module'`
    /// - CommonJS requires: `require('module')`
    /// - Dynamic imports: `import('module')`
    /// - Re-exports: `export ... from 'module'`
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
            "Starting TypeScript/JavaScript reference detection"
        );

        // For directories, we need to find files that import from any file in that directory
        let is_directory = old_path.is_dir();

        // TypeScript/JavaScript extensions to check
        let ts_extensions = ["ts", "tsx", "js", "jsx", "mjs", "cjs"];

        // Compile regexes once
        let es6_re = ES6_IMPORT_RE.clone();
        let require_re = REQUIRE_RE.clone();
        let dynamic_re = DYNAMIC_IMPORT_RE.clone();
        let reexport_re = re_export_regex();

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

            // Only check TypeScript/JavaScript files
            let ext = file.extension().and_then(|e| e.to_str()).unwrap_or("");
            if !ts_extensions.contains(&ext) {
                continue;
            }

            // Skip node_modules
            if file
                .components()
                .any(|c| c.as_os_str() == "node_modules")
            {
                continue;
            }

            let file_path = file.clone();
            let old_path_owned = old_path.to_path_buf();
            let project_root_owned = project_root.to_path_buf();
            let es6_re = es6_re.clone();
            let require_re = require_re.clone();
            let dynamic_re = dynamic_re.clone();
            let reexport_re = reexport_re.clone();
            let is_dir = is_directory;

            set.spawn(async move {
                if let Ok(content) = tokio::fs::read_to_string(&file_path).await {
                    // Check all import patterns
                    let mut has_reference = false;

                    // ES6 imports
                    for cap in es6_re.captures_iter(&content) {
                        if let Some(module_path) = cap.get(1) {
                            if Self::module_path_matches(
                                module_path.as_str(),
                                &old_path_owned,
                                &file_path,
                                &project_root_owned,
                            ) {
                                has_reference = true;
                                break;
                            }

                            // For directories, check if import starts with the directory path
                            if is_dir {
                                let module = module_path.as_str();
                                if module.starts_with("./") || module.starts_with("../") {
                                    let importing_dir =
                                        file_path.parent().unwrap_or(&project_root_owned);
                                    let resolved = importing_dir.join(module);
                                    let normalized = normalize_path(&resolved);
                                    let old_normalized = normalize_path(&old_path_owned);
                                    if normalized.starts_with(&old_normalized) {
                                        has_reference = true;
                                        break;
                                    }
                                }
                            }
                        }
                    }

                    // CommonJS requires
                    if !has_reference {
                        for cap in require_re.captures_iter(&content) {
                            if let Some(module_path) = cap.get(1) {
                                if Self::module_path_matches(
                                    module_path.as_str(),
                                    &old_path_owned,
                                    &file_path,
                                    &project_root_owned,
                                ) {
                                    has_reference = true;
                                    break;
                                }
                            }
                        }
                    }

                    // Dynamic imports
                    if !has_reference {
                        for cap in dynamic_re.captures_iter(&content) {
                            if let Some(module_path) = cap.get(1) {
                                if Self::module_path_matches(
                                    module_path.as_str(),
                                    &old_path_owned,
                                    &file_path,
                                    &project_root_owned,
                                ) {
                                    has_reference = true;
                                    break;
                                }
                            }
                        }
                    }

                    // Re-exports
                    if !has_reference {
                        for cap in reexport_re.captures_iter(&content) {
                            if let Some(module_path) = cap.get(1) {
                                if Self::module_path_matches(
                                    module_path.as_str(),
                                    &old_path_owned,
                                    &file_path,
                                    &project_root_owned,
                                ) {
                                    has_reference = true;
                                    break;
                                }
                            }
                        }
                    }

                    if has_reference {
                        tracing::debug!(
                            file = %file_path.display(),
                            old_path = %old_path_owned.display(),
                            "Found TypeScript file importing from old path"
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
            "Found TypeScript files affected by file move/rename"
        );

        affected
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_es6_import_detection() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();

        // Create utils.ts (the file being renamed)
        tokio::fs::write(project_root.join("utils.ts"), "export function helper() {}")
            .await
            .unwrap();

        // Create app.ts that imports from utils.ts
        tokio::fs::write(
            project_root.join("app.ts"),
            "import { helper } from './utils';\n\nhelper();",
        )
        .await
        .unwrap();

        let old_path = project_root.join("utils.ts");
        let new_path = project_root.join("helpers.ts");

        let project_files = vec![project_root.join("utils.ts"), project_root.join("app.ts")];

        let detector = TypeScriptReferenceDetector::new();
        let affected = detector
            .find_affected_files(&old_path, &new_path, project_root, &project_files)
            .await;

        assert!(
            affected.contains(&project_root.join("app.ts")),
            "app.ts should be detected as affected. Affected: {:?}",
            affected
        );
    }

    #[tokio::test]
    async fn test_require_detection() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();

        // Create utils.js
        tokio::fs::write(
            project_root.join("utils.js"),
            "module.exports = { helper: () => {} };",
        )
        .await
        .unwrap();

        // Create app.js with require
        tokio::fs::write(
            project_root.join("app.js"),
            "const { helper } = require('./utils');\n\nhelper();",
        )
        .await
        .unwrap();

        let old_path = project_root.join("utils.js");
        let new_path = project_root.join("helpers.js");

        let project_files = vec![project_root.join("utils.js"), project_root.join("app.js")];

        let detector = TypeScriptReferenceDetector::new();
        let affected = detector
            .find_affected_files(&old_path, &new_path, project_root, &project_files)
            .await;

        assert!(
            affected.contains(&project_root.join("app.js")),
            "app.js should be detected as affected. Affected: {:?}",
            affected
        );
    }

    #[tokio::test]
    async fn test_reexport_detection() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();

        // Create utils.ts
        tokio::fs::write(project_root.join("utils.ts"), "export function helper() {}")
            .await
            .unwrap();

        // Create index.ts with re-export
        tokio::fs::write(
            project_root.join("index.ts"),
            "export { helper } from './utils';",
        )
        .await
        .unwrap();

        let old_path = project_root.join("utils.ts");
        let new_path = project_root.join("helpers.ts");

        let project_files = vec![project_root.join("utils.ts"), project_root.join("index.ts")];

        let detector = TypeScriptReferenceDetector::new();
        let affected = detector
            .find_affected_files(&old_path, &new_path, project_root, &project_files)
            .await;

        assert!(
            affected.contains(&project_root.join("index.ts")),
            "index.ts should be detected as affected (re-export). Affected: {:?}",
            affected
        );
    }

    #[tokio::test]
    async fn test_directory_move_detection() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();

        // Create utils/index.ts
        tokio::fs::create_dir_all(project_root.join("utils"))
            .await
            .unwrap();
        tokio::fs::write(
            project_root.join("utils/index.ts"),
            "export function helper() {}",
        )
        .await
        .unwrap();

        // Create app.ts that imports from utils directory
        tokio::fs::write(
            project_root.join("app.ts"),
            "import { helper } from './utils';\n\nhelper();",
        )
        .await
        .unwrap();

        let old_path = project_root.join("utils");
        let new_path = project_root.join("helpers");

        let project_files = vec![
            project_root.join("utils/index.ts"),
            project_root.join("app.ts"),
        ];

        let detector = TypeScriptReferenceDetector::new();
        let affected = detector
            .find_affected_files(&old_path, &new_path, project_root, &project_files)
            .await;

        assert!(
            affected.contains(&project_root.join("app.ts")),
            "app.ts should be detected as affected (directory import). Affected: {:?}",
            affected
        );
    }

    #[tokio::test]
    async fn test_nested_import_detection() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();

        // Create src/utils/helpers.ts
        tokio::fs::create_dir_all(project_root.join("src/utils"))
            .await
            .unwrap();
        tokio::fs::write(
            project_root.join("src/utils/helpers.ts"),
            "export function process() {}",
        )
        .await
        .unwrap();

        // Create src/app.ts that imports from ../utils/helpers
        tokio::fs::write(
            project_root.join("src/app.ts"),
            "import { process } from './utils/helpers';\n\nprocess();",
        )
        .await
        .unwrap();

        let old_path = project_root.join("src/utils/helpers.ts");
        let new_path = project_root.join("src/utils/processors.ts");

        let project_files = vec![
            project_root.join("src/utils/helpers.ts"),
            project_root.join("src/app.ts"),
        ];

        let detector = TypeScriptReferenceDetector::new();
        let affected = detector
            .find_affected_files(&old_path, &new_path, project_root, &project_files)
            .await;

        assert!(
            affected.contains(&project_root.join("src/app.ts")),
            "src/app.ts should be detected as affected. Affected: {:?}",
            affected
        );
    }

    #[test]
    fn test_module_path_matches() {
        let project_root = Path::new("/project");
        let old_path = Path::new("/project/src/utils.ts");
        let importing_file = Path::new("/project/src/app.ts");

        // Should match relative import
        assert!(TypeScriptReferenceDetector::module_path_matches(
            "./utils",
            old_path,
            importing_file,
            project_root
        ));

        // Should match with extension
        assert!(TypeScriptReferenceDetector::module_path_matches(
            "./utils.ts",
            old_path,
            importing_file,
            project_root
        ));

        // Should not match external package
        assert!(!TypeScriptReferenceDetector::module_path_matches(
            "lodash",
            old_path,
            importing_file,
            project_root
        ));

        // Should not match different file
        assert!(!TypeScriptReferenceDetector::module_path_matches(
            "./other",
            old_path,
            importing_file,
            project_root
        ));
    }
}
