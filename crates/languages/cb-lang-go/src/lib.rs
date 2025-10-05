//! Go Language Plugin for Codebuddy
//!
//! This crate provides complete Go language support, implementing the
//! `LanguageIntelligencePlugin` trait from `cb-plugin-api`.
//!
//! # Features
//!
//! ## Import Analysis
//! - Full AST-based import parsing using Go's native parser
//! - Fallback regex-based parsing when `go` command is unavailable
//! - Support for all Go import styles (single, grouped, aliased, dot, blank)
//! - External dependency detection
//!
//! ## Symbol Extraction
//! - AST-based symbol extraction (functions, methods, structs, interfaces)
//! - Constant and variable declarations
//! - Documentation comment extraction
//! - Graceful fallback when Go toolchain is unavailable
//!
//! ## Manifest Support
//! - go.mod parsing and analysis
//! - Dependency extraction (direct and indirect)
//! - Replace directive support
//! - Exclude directive support
//! - Dependency version updates
//! - Manifest generation for new modules
//!
//! ## Refactoring Support
//! - Module file location for Go package layout
//! - Import rewriting for file renames
//! - Module reference finding with configurable scope
//! - Package-based module organization
//!
//! # Example
//!
//! ```rust
//! use cb_lang_go::GoPlugin;
//! use cb_plugin_api::LanguageIntelligencePlugin;
//! use std::path::Path;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let plugin = GoPlugin::new();
//!
//! // Parse Go source for symbols
//! let source = r#"
//! package main
//!
//! import "fmt"
//!
//! func main() {
//!     fmt.Println("Hello")
//! }
//! "#;
//!
//! let parsed = plugin.parse(source).await?;
//! assert!(!parsed.symbols.is_empty());
//!
//! // Analyze go.mod manifest
//! let manifest = plugin.analyze_manifest(Path::new("go.mod")).await?;
//! println!("Module: {}", manifest.name);
//!
//! # Ok(())
//! # }
//! ```
//!
//! # Architecture
//!
//! The plugin uses a dual-mode approach for parsing:
//!
//! 1. **AST Mode** (Primary): Embeds `resources/ast_tool.go` and spawns it as a subprocess
//!    to leverage Go's native `go/ast` and `go/parser` packages for accurate parsing.
//!
//! 2. **Regex Mode** (Fallback): When Go toolchain is unavailable, falls back to regex-based
//!    parsing for basic import detection. Symbol extraction returns empty list in fallback mode.
//!
//! This ensures the plugin works in environments without Go installed, while providing
//! full features when Go is available.

mod manifest;
mod parser;

use async_trait::async_trait;
use cb_plugin_api::{
    LanguageIntelligencePlugin, ManifestData, ModuleReference, ParsedSource, PluginError,
    PluginResult, ReferenceKind, ScanScope,
};
use std::path::{Path, PathBuf};

/// Go language plugin implementation.
pub struct GoPlugin;

impl GoPlugin {
    pub fn new() -> Self {
        Self
    }
}

impl Default for GoPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LanguageIntelligencePlugin for GoPlugin {
    fn name(&self) -> &'static str {
        "Go"
    }

    fn file_extensions(&self) -> Vec<&'static str> {
        vec!["go"]
    }

    async fn parse(&self, source: &str) -> PluginResult<ParsedSource> {
        let symbols = parser::extract_symbols(source)?;

        Ok(ParsedSource {
            data: serde_json::json!({
                "language": "go",
                "symbols_count": symbols.len()
            }),
            symbols,
        })
    }

    async fn analyze_manifest(&self, path: &Path) -> PluginResult<ManifestData> {
        manifest::load_go_mod(path).await
    }

    fn handles_manifest(&self, filename: &str) -> bool {
        filename == "go.mod"
    }

    async fn update_dependency(
        &self,
        manifest_path: &Path,
        _old_name: &str,
        new_name: &str,
        new_version: Option<&str>,
    ) -> PluginResult<String> {
        // Read the manifest file
        let content = tokio::fs::read_to_string(manifest_path)
            .await
            .map_err(|e| PluginError::manifest(format!("Failed to read go.mod: {}", e)))?;

        // Update the dependency
        let version = new_version.ok_or_else(|| {
            PluginError::invalid_input("Version required for Go dependency updates")
        })?;

        manifest::update_dependency(&content, new_name, version)
    }

    fn manifest_filename(&self) -> &'static str {
        "go.mod"
    }

    fn source_dir(&self) -> &'static str {
        "" // Go has no standard source directory
    }

    fn entry_point(&self) -> &'static str {
        "" // Go has no single entry point file
    }

    fn module_separator(&self) -> &'static str {
        "/" // Go uses slashes in module paths (e.g., github.com/user/repo)
    }

    async fn locate_module_files(
        &self,
        package_path: &Path,
        module_path: &str,
    ) -> PluginResult<Vec<PathBuf>> {
        // In Go, packages are directory-based
        // A module path like "github.com/user/repo/pkg/util" would be at pkg/util/

        // Remove the base module path if it's included
        let relative_path = if let Some(base) = module_path.strip_prefix("github.com/") {
            // Extract just the path after the repo name
            let parts: Vec<&str> = base.split('/').collect();
            if parts.len() > 2 {
                parts[2..].join("/")
            } else {
                String::new()
            }
        } else {
            module_path.replace('.', "/")
        };

        // Go packages are all .go files in a directory
        let module_dir = if relative_path.is_empty() {
            package_path.to_path_buf()
        } else {
            package_path.join(relative_path)
        };

        if !module_dir.exists() {
            return Ok(Vec::new());
        }

        // Collect all .go files (except _test.go)
        let mut files = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&module_dir) {
            for entry in entries.flatten() {
                if let Ok(path) = entry.path().canonicalize() {
                    if let Some(file_name) = path.file_name() {
                        let name_str = file_name.to_string_lossy();
                        if name_str.ends_with(".go") && !name_str.ends_with("_test.go") {
                            files.push(path);
                        }
                    }
                }
            }
        }

        Ok(files)
    }

    async fn parse_imports(&self, file_path: &Path) -> PluginResult<Vec<String>> {
        // Read the file
        let content = tokio::fs::read_to_string(file_path)
            .await
            .map_err(|e| PluginError::internal(format!("Failed to read file: {}", e)))?;

        // Use the existing import parser
        let graph = parser::analyze_imports(&content, Some(file_path))?;

        // Extract just the module paths
        Ok(graph.imports.into_iter().map(|i| i.module_path).collect())
    }

    fn generate_manifest(&self, package_name: &str, dependencies: &[String]) -> String {
        let mut result = manifest::generate_manifest(package_name, "1.21");

        if !dependencies.is_empty() {
            result.push_str("\nrequire (\n");
            for dep in dependencies {
                // Default to latest version for new dependencies
                result.push_str(&format!("\t{} v0.0.0\n", dep));
            }
            result.push_str(")\n");
        }

        result
    }

    fn rewrite_import(&self, _old_import: &str, new_package_name: &str) -> String {
        // Go imports are module paths, so just return the new package name
        new_package_name.to_string()
    }

    fn rewrite_imports_for_rename(
        &self,
        content: &str,
        old_path: &Path,
        new_path: &Path,
        _importing_file: &Path,
        _project_root: &Path,
        _rename_info: Option<&serde_json::Value>,
    ) -> PluginResult<(String, usize)> {
        // For Go, we need to update import statements when files move
        // Extract the package paths from file paths
        let old_import = old_path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| PluginError::invalid_input("Invalid old path"))?;

        let new_import = new_path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| PluginError::invalid_input("Invalid new path"))?;

        if old_import == new_import {
            return Ok((content.to_string(), 0));
        }

        // Simple string replacement for now
        // In production, this should use AST-based rewriting
        let new_content = content.replace(
            &format!("\"{}\"", old_import),
            &format!("\"{}\"", new_import),
        );

        let changes = if new_content != content { 1 } else { 0 };
        Ok((new_content, changes))
    }

    fn find_module_references(
        &self,
        content: &str,
        module_to_find: &str,
        scope: ScanScope,
    ) -> PluginResult<Vec<ModuleReference>> {
        let mut references = Vec::new();

        for (line_idx, line) in content.lines().enumerate() {
            let line_num = line_idx + 1;

            match scope {
                ScanScope::TopLevelOnly | ScanScope::AllUseStatements => {
                    // Look for import statements
                    if line.trim().starts_with("import") || line.contains(&format!("\"{}\"", module_to_find)) {
                        if let Some(pos) = line.find(module_to_find) {
                            references.push(ModuleReference {
                                line: line_num,
                                column: pos,
                                length: module_to_find.len(),
                                text: module_to_find.to_string(),
                                kind: ReferenceKind::Declaration,
                            });
                        }
                    }
                }
                ScanScope::QualifiedPaths | ScanScope::All => {
                    // Look for any occurrence
                    if let Some(pos) = line.find(module_to_find) {
                        let kind = if line.trim().starts_with("import") {
                            ReferenceKind::Declaration
                        } else {
                            ReferenceKind::QualifiedPath
                        };

                        references.push(ModuleReference {
                            line: line_num,
                            column: pos,
                            length: module_to_find.len(),
                            text: module_to_find.to_string(),
                            kind,
                        });
                    }
                }
            }
        }

        Ok(references)
    }
}
