//! Go Language Plugin for Codebuddy
//!
//! This crate provides complete Go language support, implementing the
//! `LanguagePlugin` trait from `cb-plugin-api`.
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
//! use cb_plugin_api::LanguagePlugin;
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

pub mod import_support;
mod manifest;
pub mod parser;
pub mod refactoring;
pub mod workspace_support;

use async_trait::async_trait;
use cb_plugin_api::{
    ImportSupport, LanguageCapabilities, LanguageMetadata, LanguagePlugin, ManifestData,
    ParsedSource, PluginResult, WorkspaceSupport,
};
use std::path::Path;

/// Go language plugin implementation.
pub struct GoPlugin {
    metadata: LanguageMetadata,
    import_support: import_support::GoImportSupport,
    workspace_support: workspace_support::GoWorkspaceSupport,
}

impl GoPlugin {
    pub fn new() -> Self {
        Self {
            metadata: LanguageMetadata::GO,
            import_support: import_support::GoImportSupport,
            workspace_support: workspace_support::GoWorkspaceSupport::new(),
        }
    }
}

impl Default for GoPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LanguagePlugin for GoPlugin {
    fn metadata(&self) -> &LanguageMetadata {
        &self.metadata
    }

    fn capabilities(&self) -> LanguageCapabilities {
        LanguageCapabilities {
            imports: true,
            workspace: true, // ✅ Go workspace support via go.work
        }
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

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn import_support(&self) -> Option<&dyn ImportSupport> {
        Some(&self.import_support)
    }

    fn workspace_support(&self) -> Option<&dyn WorkspaceSupport> {
        Some(&self.workspace_support)
    }
}

// ============================================================================
// Additional Methods (utility methods for compatibility)
// ============================================================================

impl GoPlugin {
    /// Update a dependency in go.mod manifest
    pub async fn update_dependency(
        &self,
        manifest_path: &Path,
        _old_name: &str,
        new_name: &str,
        new_version: Option<&str>,
    ) -> PluginResult<String> {
        let content = tokio::fs::read_to_string(manifest_path)
            .await
            .map_err(|e| {
                cb_plugin_api::PluginError::manifest(format!("Failed to read go.mod: {}", e))
            })?;

        // Use the manifest update_dependency function
        let version = new_version.ok_or_else(|| {
            cb_plugin_api::PluginError::invalid_input(
                "Version required for go.mod dependency updates",
            )
        })?;

        manifest::update_dependency(&content, new_name, version)
    }

    /// Find module references (minimal implementation for compatibility)
    pub fn find_module_references(
        &self,
        content: &str,
        module_to_find: &str,
        _scope: cb_plugin_api::ScanScope,
    ) -> PluginResult<Vec<cb_plugin_api::ModuleReference>> {
        use cb_plugin_api::{ModuleReference, ReferenceKind};

        let mut references = Vec::new();

        // Simple line-based search for import statements containing the module
        for (line_num, line) in content.lines().enumerate() {
            if line.trim().starts_with("import") && line.contains(module_to_find) {
                references.push(ModuleReference {
                    line: line_num + 1,
                    column: 0,
                    length: line.len(),
                    text: line.to_string(),
                    kind: ReferenceKind::Declaration,
                });
            }
        }

        Ok(references)
    }

    /// Rewrite imports for rename (minimal implementation for compatibility)
    pub fn rewrite_imports_for_rename(
        &self,
        content: &str,
        old_path: &Path,
        new_path: &Path,
        _importing_file: &Path,
        _project_root: &Path,
        _rename_info: Option<&serde_json::Value>,
    ) -> PluginResult<(String, usize)> {
        if let Some(import_support) = self.import_support() {
            Ok(import_support.rewrite_imports_for_move(content, old_path, new_path))
        } else {
            Ok((content.to_string(), 0))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_go_capabilities() {
        let plugin = GoPlugin::new();
        let caps = plugin.capabilities();

        assert!(caps.imports, "Go plugin should support imports");
        assert!(caps.workspace, "Go plugin should support workspace");
    }

    #[test]
    fn test_go_workspace_support() {
        let plugin = GoPlugin::new();
        assert!(
            plugin.workspace_support().is_some(),
            "Go should have workspace support"
        );
    }
}
