//! Generic import-based reference detection
//!
//! Fallback detection for languages without specialized detectors.
//! Uses import path resolution to find affected files.

use std::path::{Path, PathBuf};

/// Find affected files using generic import path resolution AND rewrite detection
///
/// This is the fallback detector for languages without specialized logic.
/// It uses TWO detection methods to ensure comprehensive coverage:
/// 1. Import path resolution (for module imports)
/// 2. Plugin rewrite detection (for string literals, config paths, etc.)
///
/// This ensures consistent behavior regardless of which code path is taken.
///
/// # Arguments
///
/// * `rename_info` - Optional JSON containing scope flags (update_exact_matches, etc.)
///   and cargo package info. Passed to plugins to control rewriting behavior.
pub(crate) fn find_generic_affected_files(
    old_path: &Path,
    new_path: &Path,
    project_root: &Path,
    project_files: &[PathBuf],
    plugins: &[std::sync::Arc<dyn mill_plugin_api::LanguagePlugin>],
    rename_info: Option<&serde_json::Value>,
) -> Vec<PathBuf> {
    use std::collections::HashSet;

    let mut affected = HashSet::new();

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
            // METHOD 1: Import-based detection (existing logic)
            let all_imports =
                get_all_imported_files(&content, file, plugins, project_files, project_root);

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
                    "File imports from old/new path - marking as affected (import detection)"
                );
                affected.insert(file.clone());
                continue; // Already marked, skip rewrite detection
            }

            // METHOD 2: Rewrite-based detection (NEW - calls all plugin detectors)
            // This catches string literals, TOML paths, YAML paths, etc.
            // IMPORTANT: Skip .rs files - they're fully handled by import updater
            // (which updates imports, mod declarations, use statements, AND qualified paths).
            // Including .rs here would create duplicate overlapping edits.
            if let Some(ext) = file.extension().and_then(|e| e.to_str()) {
                for plugin in plugins {
                    if plugin.handles_extension(ext) {
                        // Debug log for test investigation
                        #[cfg(test)]
                        println!(
                            "DEBUG: Checking plugin {} for file {}",
                            plugin.metadata().name,
                            file.display()
                        );

                        // Try rewriting to see if this file would be affected
                        // Pass rename_info so plugins receive scope flags (update_exact_matches, etc.)
                        let rewrite_result = plugin.rewrite_file_references(
                            &content,
                            old_path,
                            new_path,
                            file,
                            project_root,
                            rename_info,
                        );

                        #[cfg(test)]
                        if rewrite_result.is_none() {
                            println!(
                                "DEBUG: rewrite_file_references returned None for {}",
                                file.display()
                            );
                        }

                        if let Some((updated_content, change_count)) = rewrite_result {
                            #[cfg(test)]
                            println!(
                                "DEBUG: rewrite_file_references returned change_count={} for {}",
                                change_count,
                                file.display()
                            );

                            if change_count > 0 && updated_content != content {
                                tracing::info!(
                                    file = %file.display(),
                                    plugin = plugin.metadata().name,
                                    changes = change_count,
                                    "File affected by path reference - marking as affected (rewrite detection)"
                                );
                                affected.insert(file.clone());
                                break; // One plugin detected changes, no need to check others
                            }
                        }

                        break; // Only check the plugin that handles this extension
                    }
                }
            }
        }
    }

    let affected_vec: Vec<PathBuf> = affected.into_iter().collect();

    tracing::info!(
        affected_count = affected_vec.len(),
        "find_generic_affected_files completed"
    );

    affected_vec
}

/// Get all files imported by the given file content
///
/// Uses plugin import support if available, falls back to regex-based extraction.
pub(crate) fn get_all_imported_files(
    content: &str,
    current_file: &Path,
    plugins: &[std::sync::Arc<dyn mill_plugin_api::LanguagePlugin>],
    project_files: &[PathBuf],
    project_root: &Path,
) -> Vec<PathBuf> {
    let mut imported_files = Vec::new();

    if let Some(ext) = current_file.extension().and_then(|e| e.to_str()) {
        for plugin in plugins {
            if plugin.handles_extension(ext) {
                if let Some(import_parser) = plugin.import_parser() {
                    let import_specifiers = import_parser.parse_imports(content);
                    for specifier in import_specifiers {
                        if let Some(resolved) = resolve_import_to_file(
                            &specifier,
                            current_file,
                            project_files,
                            project_root,
                            plugins,
                        ) {
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
            if let Some(resolved) = resolve_import_to_file(
                &specifier,
                current_file,
                project_files,
                project_root,
                plugins,
            ) {
                imported_files.push(resolved);
            }
        }
    }

    imported_files
}

/// Resolve an import specifier to a file path
///
/// Delegates to ImportPathResolver for consistent resolution logic.
/// Now includes path alias resolution via language plugins.
fn resolve_import_to_file(
    specifier: &str,
    importing_file: &Path,
    project_files: &[PathBuf],
    project_root: &Path,
    plugins: &[std::sync::Arc<dyn mill_plugin_api::LanguagePlugin>],
) -> Option<PathBuf> {
    let resolver = mill_ast::ImportPathResolver::with_plugins(project_root, plugins.to_vec());
    resolver.resolve_import_to_file(specifier, importing_file, project_files)
}

/// Extract import path from a line of code using regex patterns
///
/// Handles common import patterns:
/// - `from "path"` or `from 'path'`
/// - `require("path")` or `require('path')`
///
/// Returns the extracted path if found.
pub(crate) fn extract_import_path(line: &str) -> Option<String> {
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
        )
        .unwrap();

        fs::write(
            root.join("src/main.ts"),
            r#"import { myUtil, helperFunc } from './utils';

export function main() {
    const result = myUtil();
    const processed = helperFunc(result);
    console.log(processed);
}
"#,
        )
        .unwrap();

        let old_path = root.join("src/utils.ts");
        let new_path = root.join("src/renamed_utils.ts");
        let project_files = vec![root.join("src/utils.ts"), root.join("src/main.ts")];

        // Get plugins from registry
        let bundle_plugins = mill_plugin_bundle::all_plugins();
        let plugin_registry =
            crate::services::registry_builder::build_language_plugin_registry(bundle_plugins);
        let plugins = plugin_registry.all();

        // Test generic detector
        let affected =
            find_generic_affected_files(&old_path, &new_path, root, &project_files, plugins, None);

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

    /// Test that generic detector finds YAML files with path references
    /// This verifies the rewrite-based detection works in the fallback path
    #[test]
    fn test_generic_detector_finds_yaml_files() {
        let workspace = TempDir::new().unwrap();
        let root = workspace.path();

        // Create a directory structure
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("src/old_file.rs"), "// Rust file").unwrap();

        // Create a YAML file with a RELATIVE path reference (matching production usage)
        let yaml_content = "jobs:\n  test:\n    path: src/old_file.rs\n";
        fs::write(root.join("config.yml"), yaml_content).unwrap();

        // Test with FILE rename (not directory rename)
        // The YAML content has "src/old_file.rs", so we rename that file
        let old_path = Path::new("src/old_file.rs"); // Relative path
        let new_path = Path::new("src/new_file.rs"); // Relative path

        let project_files = vec![root.join("src/old_file.rs"), root.join("config.yml")];

        // Get plugins from registry
        let bundle_plugins = mill_plugin_bundle::all_plugins();
        let plugin_registry =
            crate::services::registry_builder::build_language_plugin_registry(bundle_plugins);
        let plugins = plugin_registry.all();

        let affected =
            find_generic_affected_files(old_path, new_path, root, &project_files, plugins, None);

        assert!(
            affected.iter().any(|p| p.ends_with("config.yml")),
            "YAML file should be detected by generic finder for file rename. Found: {:?}",
            affected
        );
    }

    /// Test that generic detector finds TOML files with path references
    /// This verifies the rewrite-based detection works in the fallback path
    #[test]
    fn test_generic_detector_finds_toml_files() {
        let workspace = TempDir::new().unwrap();
        let root = workspace.path();

        // Create a directory structure
        fs::create_dir_all(root.join("scripts")).unwrap();
        fs::write(
            root.join("scripts/old_build.sh"),
            "#!/bin/bash\necho 'build'",
        )
        .unwrap();

        // Create a TOML file with a RELATIVE path reference
        fs::write(
            root.join("config.toml"),
            "[scripts]\nbuild = \"scripts/old_build.sh\"\n",
        )
        .unwrap();

        // Test with FILE rename
        let old_path = Path::new("scripts/old_build.sh");
        let new_path = Path::new("scripts/new_build.sh");
        let project_files = vec![root.join("scripts/old_build.sh"), root.join("config.toml")];

        // Get plugins from registry
        let bundle_plugins = mill_plugin_bundle::all_plugins();
        let plugin_registry =
            crate::services::registry_builder::build_language_plugin_registry(bundle_plugins);
        let plugins = plugin_registry.all();

        let affected =
            find_generic_affected_files(old_path, new_path, root, &project_files, plugins, None);

        assert!(
            affected.iter().any(|p| p.ends_with("config.toml")),
            "TOML file should be detected by generic finder for file rename. Found: {:?}",
            affected
        );
    }

    /// Test that generic detector finds Markdown files with path references
    /// This verifies the rewrite-based detection works in the fallback path
    #[test]
    fn test_generic_detector_finds_markdown_files() {
        let workspace = TempDir::new().unwrap();
        let root = workspace.path();

        // Create a directory structure
        fs::create_dir_all(root.join("docs")).unwrap();
        fs::write(root.join("docs/guide.md"), "# Guide").unwrap();

        // Create a Markdown file with a link reference
        fs::write(
            root.join("README.md"),
            "# Project\n\nSee [Guide](docs/guide.md) for details.\n",
        )
        .unwrap();

        let old_path = root.join("docs/guide.md");
        let new_path = root.join("docs/tutorial.md");
        let project_files = vec![root.join("docs/guide.md"), root.join("README.md")];

        // Create Markdown plugin using the bundle
        // Get plugins from registry
        let bundle_plugins = mill_plugin_bundle::all_plugins();
        let plugin_registry =
            crate::services::registry_builder::build_language_plugin_registry(bundle_plugins);
        let plugins = plugin_registry.all();

        let affected =
            find_generic_affected_files(&old_path, &new_path, root, &project_files, plugins, None);

        assert!(
            affected.iter().any(|p| p.ends_with("README.md")),
            "Markdown file should be detected by generic finder. Found: {:?}",
            affected
        );
    }

    // NOTE: TypeScript string literal detection test omitted
    // TypeScript plugin does not yet implement string literal rewriting
    // This will be added in a future enhancement

    /// Test that generic detector finds string literals in Rust files
    /// This verifies the rewrite-based detection works for Rust string literals
    #[test]
    fn test_generic_detector_finds_rust_string_literals() {
        let workspace = TempDir::new().unwrap();
        let root = workspace.path();

        // Create a directory structure
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(
            root.join("src/old_config.rs"),
            "pub const CONFIG: &str = \"config\";",
        )
        .unwrap();

        // Create a Rust file with a string literal path reference
        fs::write(
            root.join("src/main.rs"),
            r#"fn main() {
    let path = "src/old_config.rs";
    println!("Config at: {}", path);
}
"#,
        )
        .unwrap();

        // Test with FILE rename using relative paths
        let old_path = Path::new("src/old_config.rs");
        let new_path = Path::new("src/new_config.rs");
        let project_files = vec![root.join("src/old_config.rs"), root.join("src/main.rs")];

        // Get plugins from registry
        let bundle_plugins = mill_plugin_bundle::all_plugins();
        let plugin_registry =
            crate::services::registry_builder::build_language_plugin_registry(bundle_plugins);
        let plugins = plugin_registry.all();

        let affected =
            find_generic_affected_files(old_path, new_path, root, &project_files, plugins, None);

        assert!(
            affected.iter().any(|p| p.ends_with("main.rs")),
            "Rust file with string literal should be detected by generic finder. Found: {:?}",
            affected
        );
    }
}
