//! Generic import-based reference detection
//!
//! Fallback detection for languages without specialized detectors.
//! Uses import path resolution to find affected files.

use std::path::{Path, PathBuf};

/// Find affected files using generic import path resolution
///
/// This is the fallback detector for languages without specialized logic.
/// It resolves import specifiers to file paths and checks if any import
/// references the old or new path.
pub fn find_generic_affected_files(
    old_path: &Path,
    new_path: &Path,
    project_root: &Path,
    project_files: &[PathBuf],
    plugins: &[std::sync::Arc<dyn cb_plugin_api::LanguagePlugin>],
) -> Vec<PathBuf> {
    let mut affected = Vec::new();

    tracing::info!(
        old_path = %old_path.display(),
        new_path = %new_path.display(),
        project_files_count = project_files.len(),
        "find_generic_affected_files called"
    );

    for file in project_files {
        if file == old_path || file == new_path {
            continue;
        }
        if let Ok(content) = std::fs::read_to_string(file) {
            let all_imports = get_all_imported_files(&content, file, plugins, project_files, project_root);

            tracing::debug!(
                file = %file.display(),
                imports_count = all_imports.len(),
                imports = ?all_imports.iter().map(|p| p.display().to_string()).collect::<Vec<_>>(),
                "Parsed imports from file"
            );

            // Check if imports reference either the old path (pre-move) or new path (post-move)
            if all_imports.contains(&old_path.to_path_buf())
                || all_imports.contains(&new_path.to_path_buf())
            {
                tracing::info!(
                    file = %file.display(),
                    "File imports from old/new path - marking as affected"
                );
                affected.push(file.clone());
            }
        }
    }

    tracing::info!(
        affected_count = affected.len(),
        "find_generic_affected_files completed"
    );

    affected
}

/// Get all files imported by the given file content
///
/// Uses plugin import support if available, falls back to regex-based extraction.
pub fn get_all_imported_files(
    content: &str,
    current_file: &Path,
    plugins: &[std::sync::Arc<dyn cb_plugin_api::LanguagePlugin>],
    project_files: &[PathBuf],
    project_root: &Path,
) -> Vec<PathBuf> {
    let mut imported_files = Vec::new();

    if let Some(ext) = current_file.extension().and_then(|e| e.to_str()) {
        for plugin in plugins {
            if plugin.handles_extension(ext) {
                if let Some(import_support) = plugin.import_support() {
                    let import_specifiers = import_support.parse_imports(content);
                    for specifier in import_specifiers {
                        if let Some(resolved) =
                            resolve_import_to_file(&specifier, current_file, project_files, project_root)
                        {
                            imported_files.push(resolved);
                        }
                    }
                    return imported_files;
                }
            }
        }
    }

    // Fallback: use regex-based extraction
    for line in content.lines() {
        if let Some(specifier) = extract_import_path(line) {
            if let Some(resolved) =
                resolve_import_to_file(&specifier, current_file, project_files, project_root)
            {
                imported_files.push(resolved);
            }
        }
    }

    imported_files
}

/// Resolve an import specifier to a file path
///
/// Delegates to ImportPathResolver for consistent resolution logic.
fn resolve_import_to_file(
    specifier: &str,
    importing_file: &Path,
    project_files: &[PathBuf],
    project_root: &Path,
) -> Option<PathBuf> {
    let resolver = cb_ast::ImportPathResolver::new(project_root);
    resolver.resolve_import_to_file(specifier, importing_file, project_files)
}

/// Extract import path from a line of code using regex patterns
///
/// Handles common import patterns:
/// - `from "path"` or `from 'path'`
/// - `require("path")` or `require('path')`
///
/// Returns the extracted path if found.
pub fn extract_import_path(line: &str) -> Option<String> {
    if line.contains("from") {
        if let Some(start) = line.find(['\'', '"']) {
            let quote_char = &line[start..=start];
            let path_start = start + 1;
            if let Some(end) = line[path_start..].find(quote_char) {
                return Some(line[path_start..path_start + end].to_string());
            }
        }
    }
    if line.contains("require") {
        if let Some(start) = line.find(['\'', '"']) {
            let quote_char = &line[start..=start];
            let path_start = start + 1;
            if let Some(end) = line[path_start..].find(quote_char) {
                return Some(line[path_start..path_start + end].to_string());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_extract_import_path() {
        assert_eq!(
            extract_import_path("import { foo } from './bar';"),
            Some("./bar".to_string())
        );
        assert_eq!(
            extract_import_path("import { foo } from \"./bar\";"),
            Some("./bar".to_string())
        );
        assert_eq!(
            extract_import_path("const bar = require('./bar');"),
            Some("./bar".to_string())
        );
        assert_eq!(
            extract_import_path("const bar = require(\"./bar\");"),
            Some("./bar".to_string())
        );
        assert_eq!(extract_import_path("let x = 1;"), None);
        assert_eq!(extract_import_path("this is from a file"), None);
    }

    #[test]
    fn test_generic_detector_with_typescript() {
        // Create temp workspace
        let workspace = TempDir::new().unwrap();
        let root = workspace.path();

        // Create TypeScript files
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(
            root.join("src/utils.ts"),
            r#"export const myUtil = () => {
    return "utility function";
};

export function helperFunc(data: string): string {
    return data.toUpperCase();
}
"#,
        ).unwrap();

        fs::write(
            root.join("src/main.ts"),
            r#"import { myUtil, helperFunc } from './utils';

export function main() {
    const result = myUtil();
    const processed = helperFunc(result);
    console.log(processed);
}
"#,
        ).unwrap();

        let old_path = root.join("src/utils.ts");
        let new_path = root.join("src/renamed_utils.ts");
        let project_files = vec![
            root.join("src/utils.ts"),
            root.join("src/main.ts"),
        ];

        // Create TypeScript plugin
        let ts_plugin = cb_lang_typescript::TypeScriptPlugin::new();
        let plugins: Vec<std::sync::Arc<dyn cb_plugin_api::LanguagePlugin>> =
            vec![std::sync::Arc::from(ts_plugin)];

        // Test generic detector
        let affected = find_generic_affected_files(
            &old_path,
            &new_path,
            root,
            &project_files,
            &plugins,
        );

        println!("DEBUG: Old path: {}", old_path.display());
        println!("DEBUG: New path: {}", new_path.display());
        println!("DEBUG: Project root: {}", root.display());
        println!("DEBUG: Project files: {:?}", project_files);
        println!("DEBUG: Affected files: {:?}", affected);
        println!("DEBUG: Affected count: {}", affected.len());

        assert!(
            affected.contains(&root.join("src/main.ts")),
            "main.ts should be detected as affected. Affected files: {:?}",
            affected
        );
    }
}
