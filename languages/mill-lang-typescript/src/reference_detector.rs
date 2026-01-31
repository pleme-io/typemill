//! TypeScript/JavaScript-specific reference detection
//!
//! Handles detection of affected files for TypeScript/JavaScript package renames.
//! Detects cross-package references in:
//! - ES6 imports (`import X from 'package-name'`)
//! - CommonJS requires (`const X = require('package-name')`)
//! - package.json dependencies
//!
//! This is used during rename operations to detect when a renamed package
//! is referenced by other code in the workspace.

use async_trait::async_trait;
use mill_plugin_api::ReferenceDetector;
use once_cell::sync::Lazy;
use regex::Regex;
use std::path::{Path, PathBuf};
use tokio::task::JoinSet;

/// TypeScript/JavaScript reference detector implementation
///
/// Detects files that reference a package being renamed, including:
/// - Source files with import/require statements referencing the package
/// - package.json files with dependencies on the package
#[derive(Default)]
pub struct TypeScriptReferenceDetector;

impl TypeScriptReferenceDetector {
    /// Creates a new TypeScript reference detector instance.
    pub fn new() -> Self {
        Self
    }

    /// Extract the package name from a path
    ///
    /// For directories: Uses directory name (e.g., "packages/my-lib" -> "my-lib")
    /// For files: Uses parent directory name or file stem
    fn extract_package_name(path: &Path) -> Option<String> {
        if path.is_dir() {
            // Check for package.json to get the actual package name
            let package_json = path.join("package.json");
            if package_json.exists() {
                if let Ok(content) = std::fs::read_to_string(&package_json) {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                        if let Some(name) = json.get("name").and_then(|v| v.as_str()) {
                            return Some(name.to_string());
                        }
                    }
                }
            }
            // Fallback to directory name
            path.file_name()
                .and_then(|n| n.to_str())
                .map(String::from)
        } else {
            // For files, try to find package.json in parent directories
            let mut current = path.parent();
            while let Some(dir) = current {
                let package_json = dir.join("package.json");
                if package_json.exists() {
                    if let Ok(content) = std::fs::read_to_string(&package_json) {
                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                            if let Some(name) = json.get("name").and_then(|v| v.as_str()) {
                                return Some(name.to_string());
                            }
                        }
                    }
                    break;
                }
                current = dir.parent();
            }
            None
        }
    }

    /// Check if a file contains imports/requires of the given package
    fn content_references_package(content: &str, package_name: &str) -> bool {
        // ES6 import patterns
        let es6_patterns = [
            format!(r#"from\s+['"]{package_name}['"]"#),
            format!(r#"from\s+['"]{package_name}/[^'"]*['"]"#),
        ];

        // CommonJS require patterns
        let require_patterns = [
            format!(r#"require\s*\(\s*['"]{package_name}['"]\s*\)"#),
            format!(r#"require\s*\(\s*['"]{package_name}/[^'"]*['"]\s*\)"#),
        ];

        // Dynamic import patterns
        let dynamic_patterns = [
            format!(r#"import\s*\(\s*['"]{package_name}['"]\s*\)"#),
            format!(r#"import\s*\(\s*['"]{package_name}/[^'"]*['"]\s*\)"#),
        ];

        // Check all patterns
        for pattern in es6_patterns
            .iter()
            .chain(require_patterns.iter())
            .chain(dynamic_patterns.iter())
        {
            if let Ok(re) = Regex::new(pattern) {
                if re.is_match(content) {
                    return true;
                }
            }
        }

        false
    }

    /// Check if a package.json file has a dependency on the given package
    fn package_json_references_package(content: &str, package_name: &str) -> bool {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(content) {
            let dependency_fields = [
                "dependencies",
                "devDependencies",
                "peerDependencies",
                "optionalDependencies",
            ];

            for field in dependency_fields {
                if let Some(deps) = json.get(field).and_then(|v| v.as_object()) {
                    if deps.contains_key(package_name) {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Check if a file is a TypeScript/JavaScript source file
    fn is_ts_js_file(path: &Path) -> bool {
        matches!(
            path.extension().and_then(|e| e.to_str()),
            Some("ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs" | "mts" | "cts")
        )
    }

    /// Check if a file is a package.json file
    fn is_package_json(path: &Path) -> bool {
        path.file_name()
            .and_then(|n| n.to_str())
            .map(|n| n == "package.json")
            .unwrap_or(false)
    }
}

#[async_trait]
impl ReferenceDetector for TypeScriptReferenceDetector {
    /// Find affected TypeScript/JavaScript files for a package rename
    ///
    /// This handles:
    /// 1. ES6 imports: `import X from 'package-name'`
    /// 2. CommonJS requires: `const X = require('package-name')`
    /// 3. Dynamic imports: `import('package-name')`
    /// 4. package.json dependencies
    ///
    /// # Arguments
    ///
    /// * `old_path` - The current path being renamed (package directory)
    /// * `new_path` - The target path after rename
    /// * `project_root` - Root directory of the project
    /// * `project_files` - List of all files in the project
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
            "Starting TypeScript/JavaScript cross-package reference detection"
        );

        // Extract package names
        let old_package_name = Self::extract_package_name(old_path);
        let new_package_name = Self::extract_package_name(new_path);

        tracing::info!(
            old_package = ?old_package_name,
            new_package = ?new_package_name,
            "Extracted package names"
        );

        // If we can't determine package names, skip detection
        let old_package_name = match old_package_name {
            Some(name) => name,
            None => {
                tracing::warn!(
                    old_path = %old_path.display(),
                    "Could not determine old package name, skipping reference detection"
                );
                return affected;
            }
        };

        let new_package_name = match new_package_name {
            Some(name) => name,
            None => {
                // For new path, fall back to directory/file name
                new_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(String::from)
                    .unwrap_or_else(|| old_package_name.clone())
            }
        };

        // Skip if package names are the same (no rename occurring)
        if old_package_name == new_package_name {
            tracing::info!(
                package_name = %old_package_name,
                "Package names are identical, no reference updates needed"
            );
            return affected;
        }

        tracing::info!(
            old_package = %old_package_name,
            new_package = %new_package_name,
            "Detected package rename, scanning for references"
        );

        // Parallelize file scanning using JoinSet
        let mut set = JoinSet::new();

        for file in project_files {
            // Skip files inside the renamed package itself
            if file.starts_with(old_path) {
                continue;
            }

            // Only check TypeScript/JavaScript files and package.json files
            let is_source = Self::is_ts_js_file(file);
            let is_pkg_json = Self::is_package_json(file);

            if !is_source && !is_pkg_json {
                continue;
            }

            let file_path = file.clone();
            let package_name = old_package_name.clone();
            let check_source = is_source;
            let check_pkg_json = is_pkg_json;

            set.spawn(async move {
                match tokio::fs::read_to_string(&file_path).await {
                    Ok(content) => {
                        let has_reference = if check_source {
                            Self::content_references_package(&content, &package_name)
                        } else if check_pkg_json {
                            Self::package_json_references_package(&content, &package_name)
                        } else {
                            false
                        };

                        if has_reference {
                            tracing::debug!(
                                file = %file_path.display(),
                                package = %package_name,
                                is_source = check_source,
                                is_package_json = check_pkg_json,
                                "Found file referencing package"
                            );
                            Some(file_path)
                        } else {
                            None
                        }
                    }
                    Err(e) => {
                        tracing::warn!(
                            file = %file_path.display(),
                            error = %e,
                            "Failed to read file for reference detection"
                        );
                        None
                    }
                }
            });
        }

        // Collect results from all spawned tasks
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
            "Found files affected by package rename"
        );

        affected
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_content_references_package_es6_import() {
        let content = r#"
import React from 'react';
import { useState } from 'react';
import { helper } from 'my-utils';
import * as utils from 'my-utils/helpers';
"#;

        assert!(TypeScriptReferenceDetector::content_references_package(
            content, "react"
        ));
        assert!(TypeScriptReferenceDetector::content_references_package(
            content, "my-utils"
        ));
        assert!(!TypeScriptReferenceDetector::content_references_package(
            content, "lodash"
        ));
    }

    #[test]
    fn test_content_references_package_require() {
        let content = r#"
const fs = require('fs');
const path = require('path');
const myLib = require('my-library');
const helper = require('my-library/utils');
"#;

        assert!(TypeScriptReferenceDetector::content_references_package(
            content, "fs"
        ));
        assert!(TypeScriptReferenceDetector::content_references_package(
            content, "path"
        ));
        assert!(TypeScriptReferenceDetector::content_references_package(
            content,
            "my-library"
        ));
        assert!(!TypeScriptReferenceDetector::content_references_package(
            content, "express"
        ));
    }

    #[test]
    fn test_content_references_package_dynamic_import() {
        let content = r#"
async function loadModule() {
    const mod = await import('my-dynamic-module');
    const subMod = await import('my-dynamic-module/sub');
}
"#;

        assert!(TypeScriptReferenceDetector::content_references_package(
            content,
            "my-dynamic-module"
        ));
        assert!(!TypeScriptReferenceDetector::content_references_package(
            content, "other-module"
        ));
    }

    #[test]
    fn test_package_json_references_package() {
        let content = r#"{
  "name": "my-app",
  "version": "1.0.0",
  "dependencies": {
    "react": "^18.0.0",
    "my-utils": "workspace:*"
  },
  "devDependencies": {
    "typescript": "^5.0.0"
  }
}"#;

        assert!(TypeScriptReferenceDetector::package_json_references_package(
            content, "react"
        ));
        assert!(TypeScriptReferenceDetector::package_json_references_package(
            content, "my-utils"
        ));
        assert!(TypeScriptReferenceDetector::package_json_references_package(
            content,
            "typescript"
        ));
        assert!(!TypeScriptReferenceDetector::package_json_references_package(
            content, "lodash"
        ));
    }

    #[test]
    fn test_is_ts_js_file() {
        assert!(TypeScriptReferenceDetector::is_ts_js_file(Path::new(
            "app.ts"
        )));
        assert!(TypeScriptReferenceDetector::is_ts_js_file(Path::new(
            "component.tsx"
        )));
        assert!(TypeScriptReferenceDetector::is_ts_js_file(Path::new(
            "script.js"
        )));
        assert!(TypeScriptReferenceDetector::is_ts_js_file(Path::new(
            "app.jsx"
        )));
        assert!(TypeScriptReferenceDetector::is_ts_js_file(Path::new(
            "module.mjs"
        )));
        assert!(TypeScriptReferenceDetector::is_ts_js_file(Path::new(
            "module.cjs"
        )));
        assert!(TypeScriptReferenceDetector::is_ts_js_file(Path::new(
            "types.mts"
        )));
        assert!(TypeScriptReferenceDetector::is_ts_js_file(Path::new(
            "types.cts"
        )));
        assert!(!TypeScriptReferenceDetector::is_ts_js_file(Path::new(
            "styles.css"
        )));
        assert!(!TypeScriptReferenceDetector::is_ts_js_file(Path::new(
            "config.json"
        )));
    }

    #[test]
    fn test_is_package_json() {
        assert!(TypeScriptReferenceDetector::is_package_json(Path::new(
            "package.json"
        )));
        assert!(TypeScriptReferenceDetector::is_package_json(Path::new(
            "packages/lib/package.json"
        )));
        assert!(!TypeScriptReferenceDetector::is_package_json(Path::new(
            "tsconfig.json"
        )));
        assert!(!TypeScriptReferenceDetector::is_package_json(Path::new(
            "package-lock.json"
        )));
    }

    #[tokio::test]
    async fn test_find_affected_files_package_rename() {
        // Setup: Create a workspace with two packages
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();

        // Create old_package with package.json
        tokio::fs::create_dir_all(project_root.join("packages/old-utils/src"))
            .await
            .unwrap();
        tokio::fs::write(
            project_root.join("packages/old-utils/package.json"),
            r#"{"name": "old-utils", "version": "1.0.0"}"#,
        )
        .await
        .unwrap();
        tokio::fs::write(
            project_root.join("packages/old-utils/src/index.ts"),
            "export function helper() {}\n",
        )
        .await
        .unwrap();

        // Create app package that imports from old-utils
        tokio::fs::create_dir_all(project_root.join("packages/app/src"))
            .await
            .unwrap();
        tokio::fs::write(
            project_root.join("packages/app/package.json"),
            r#"{"name": "app", "version": "1.0.0", "dependencies": {"old-utils": "workspace:*"}}"#,
        )
        .await
        .unwrap();
        tokio::fs::write(
            project_root.join("packages/app/src/main.ts"),
            "import { helper } from 'old-utils';\n\nhelper();\n",
        )
        .await
        .unwrap();

        // Define paths
        let old_path = project_root.join("packages/old-utils");
        let new_path = project_root.join("packages/new-utils");

        // Project files list
        let project_files = vec![
            project_root.join("packages/old-utils/src/index.ts"),
            project_root.join("packages/old-utils/package.json"),
            project_root.join("packages/app/src/main.ts"),
            project_root.join("packages/app/package.json"),
        ];

        // Test: Run the detector
        let detector = TypeScriptReferenceDetector::new();
        let affected = detector
            .find_affected_files(&old_path, &new_path, project_root, &project_files)
            .await;

        // Verify: app/src/main.ts should be affected (imports from old-utils)
        assert!(
            affected
                .iter()
                .any(|p| p.ends_with("app/src/main.ts")),
            "app/src/main.ts should be detected as affected (imports from old-utils). Affected files: {:?}",
            affected
        );

        // Verify: app/package.json should be affected (has dependency on old-utils)
        assert!(
            affected
                .iter()
                .any(|p| p.ends_with("app/package.json")),
            "app/package.json should be detected as affected (has dependency on old-utils). Affected files: {:?}",
            affected
        );

        // Verify: old-utils files should NOT be affected (they're inside the renamed package)
        assert!(
            !affected
                .iter()
                .any(|p| p.to_string_lossy().contains("old-utils")),
            "Files inside old-utils should not be in affected list. Affected files: {:?}",
            affected
        );
    }

    #[tokio::test]
    async fn test_find_affected_files_require_syntax() {
        // Setup: Create a workspace with CommonJS requires
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();

        // Create old_lib package
        tokio::fs::create_dir_all(project_root.join("lib"))
            .await
            .unwrap();
        tokio::fs::write(
            project_root.join("lib/package.json"),
            r#"{"name": "my-lib", "version": "1.0.0"}"#,
        )
        .await
        .unwrap();
        tokio::fs::write(
            project_root.join("lib/index.js"),
            "module.exports = { util: function() {} };\n",
        )
        .await
        .unwrap();

        // Create app that uses require
        tokio::fs::create_dir_all(project_root.join("app"))
            .await
            .unwrap();
        tokio::fs::write(
            project_root.join("app/package.json"),
            r#"{"name": "app", "version": "1.0.0"}"#,
        )
        .await
        .unwrap();
        tokio::fs::write(
            project_root.join("app/main.js"),
            "const lib = require('my-lib');\nlib.util();\n",
        )
        .await
        .unwrap();

        // Define paths
        let old_path = project_root.join("lib");
        let new_path = project_root.join("new-lib");

        let project_files = vec![
            project_root.join("lib/index.js"),
            project_root.join("lib/package.json"),
            project_root.join("app/main.js"),
            project_root.join("app/package.json"),
        ];

        let detector = TypeScriptReferenceDetector::new();
        let affected = detector
            .find_affected_files(&old_path, &new_path, project_root, &project_files)
            .await;

        // Verify: app/main.js should be affected (requires my-lib)
        assert!(
            affected.iter().any(|p| p.ends_with("app/main.js")),
            "app/main.js should be detected as affected (requires my-lib). Affected files: {:?}",
            affected
        );
    }

    #[tokio::test]
    async fn test_find_affected_files_scoped_package() {
        // Setup: Test with scoped package names (@org/package)
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();

        // Create scoped package
        tokio::fs::create_dir_all(project_root.join("packages/utils"))
            .await
            .unwrap();
        tokio::fs::write(
            project_root.join("packages/utils/package.json"),
            r#"{"name": "@myorg/utils", "version": "1.0.0"}"#,
        )
        .await
        .unwrap();

        // Create app that imports scoped package
        tokio::fs::create_dir_all(project_root.join("packages/app"))
            .await
            .unwrap();
        tokio::fs::write(
            project_root.join("packages/app/package.json"),
            r#"{"name": "@myorg/app", "version": "1.0.0", "dependencies": {"@myorg/utils": "^1.0.0"}}"#,
        )
        .await
        .unwrap();
        tokio::fs::write(
            project_root.join("packages/app/index.ts"),
            "import { helper } from '@myorg/utils';\n",
        )
        .await
        .unwrap();

        let old_path = project_root.join("packages/utils");
        let new_path = project_root.join("packages/helpers");

        let project_files = vec![
            project_root.join("packages/utils/package.json"),
            project_root.join("packages/app/package.json"),
            project_root.join("packages/app/index.ts"),
        ];

        let detector = TypeScriptReferenceDetector::new();
        let affected = detector
            .find_affected_files(&old_path, &new_path, project_root, &project_files)
            .await;

        // Verify scoped package detection
        assert!(
            affected.iter().any(|p| p.ends_with("app/index.ts")),
            "app/index.ts should be detected as affected (imports @myorg/utils). Affected files: {:?}",
            affected
        );
        assert!(
            affected.iter().any(|p| p.ends_with("app/package.json")),
            "app/package.json should be detected as affected (depends on @myorg/utils). Affected files: {:?}",
            affected
        );
    }

    #[tokio::test]
    async fn test_find_affected_files_no_change_when_same_name() {
        // Setup: Test that no files are affected when package name doesn't change
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();

        // Create package
        tokio::fs::create_dir_all(project_root.join("lib"))
            .await
            .unwrap();
        tokio::fs::write(
            project_root.join("lib/package.json"),
            r#"{"name": "my-lib", "version": "1.0.0"}"#,
        )
        .await
        .unwrap();

        // Create another package that imports
        tokio::fs::create_dir_all(project_root.join("app"))
            .await
            .unwrap();
        tokio::fs::write(
            project_root.join("app/index.ts"),
            "import { x } from 'my-lib';\n",
        )
        .await
        .unwrap();

        // Same path (no actual rename)
        let old_path = project_root.join("lib");
        let new_path = project_root.join("lib");

        let project_files = vec![
            project_root.join("lib/package.json"),
            project_root.join("app/index.ts"),
        ];

        let detector = TypeScriptReferenceDetector::new();
        let affected = detector
            .find_affected_files(&old_path, &new_path, project_root, &project_files)
            .await;

        // Should be empty - no rename happening
        assert!(
            affected.is_empty(),
            "No files should be affected when package name doesn't change. Affected files: {:?}",
            affected
        );
    }

    #[test]
    fn test_content_references_subpath_import() {
        // Test imports with subpaths like 'package/utils' or 'package/components'
        let content = r#"
import { helper } from 'my-package/utils';
import Component from 'my-package/components/Button';
const data = require('my-package/data');
"#;

        assert!(TypeScriptReferenceDetector::content_references_package(
            content,
            "my-package"
        ));
    }

    #[test]
    fn test_content_no_false_positives() {
        // Ensure we don't match partial package names
        let content = r#"
import { x } from 'my-package-extended';
import { y } from 'another-my-package';
"#;

        // Should NOT match 'my-package' when the actual import is 'my-package-extended'
        // This is a known limitation - full word boundary matching would require more complex regex
        // For now, we accept this as a trade-off for simplicity
        assert!(!TypeScriptReferenceDetector::content_references_package(
            content, "my-package"
        ));
    }
}
