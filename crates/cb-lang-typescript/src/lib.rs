//! TypeScript/JavaScript Language Plugin for Codebuddy
//!
//! This crate provides complete TypeScript and JavaScript language support,
//! implementing the `LanguageIntelligencePlugin` trait from `cb-plugin-api`.
//!
//! # Features
//!
//! ## Import Analysis
//! - Full AST-based import parsing using Node.js with Babel parser
//! - Fallback regex-based parsing when Node.js is unavailable
//! - Support for ES6 imports (`import ... from '...'`)
//! - Support for CommonJS (`require('...')`)
//! - Support for dynamic imports (`import('...')`)
//! - Support for type-only imports (`import type`)
//! - External dependency detection
//!
//! ## Symbol Extraction
//! - AST-based symbol extraction (functions, classes, interfaces, types, enums)
//! - Regular and async functions
//! - Arrow functions
//! - TypeScript interfaces and type aliases
//! - Enums
//! - Documentation comment extraction
//! - Graceful fallback when Node.js is unavailable
//!
//! ## Manifest Support
//! - package.json parsing and analysis
//! - Dependency extraction (dependencies, devDependencies, peerDependencies, optionalDependencies)
//! - Git, path, workspace, and registry dependencies
//! - Version range support (^, ~, >=, etc.)
//! - Dependency version updates
//! - Manifest generation for new packages
//!
//! ## Refactoring Support
//! - Module file location for TypeScript/JavaScript layout
//! - Import rewriting for file renames (ES6 + CommonJS + dynamic)
//! - Module reference finding with configurable scope
//! - Relative path calculation for imports
//!
//! # Example
//!
//! ```rust
//! use cb_lang_typescript::TypeScriptPlugin;
//! use cb_plugin_api::LanguagePlugin;
//! use std::path::Path;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let plugin = TypeScriptPlugin::new();
//!
//! // Parse TypeScript source for symbols
//! let source = r#"
//! import React from 'react';
//!
//! interface User {
//!     name: string;
//!     age: number;
//! }
//!
//! function greet(user: User) {
//!     console.log(`Hello, ${user.name}!`);
//! }
//! "#;
//!
//! let parsed = plugin.parse(source).await?;
//! assert!(!parsed.symbols.is_empty());
//!
//! // Analyze package.json manifest
//! let manifest = plugin.analyze_manifest(Path::new("package.json")).await?;
//! println!("Package: {}", manifest.name);
//!
//! # Ok(())
//! # }
//! ```
//!
//! # Architecture
//!
//! The plugin uses a dual-mode approach for parsing:
//!
//! 1. **AST Mode** (Primary): Embeds `resources/ast_tool.js` and spawns it as a subprocess
//!    to leverage Node.js with Babel parser (`@babel/parser`) for accurate parsing of both
//!    TypeScript and JavaScript. Supports JSX/TSX through Babel plugins.
//!
//! 2. **Regex Mode** (Fallback): When Node.js is unavailable, falls back to regex-based
//!    parsing for basic import detection. Symbol extraction returns empty list in fallback mode.
//!
//! This ensures the plugin works in environments without Node.js installed, while providing
//! full features when Node.js is available.
//!
//! # Supported File Extensions
//!
//! - `.ts` - TypeScript
//! - `.tsx` - TypeScript with JSX
//! - `.js` - JavaScript
//! - `.jsx` - JavaScript with JSX
//! - `.mjs` - ES Module JavaScript
//! - `.cjs` - CommonJS JavaScript
mod manifest;
pub mod parser;
pub mod refactoring;
pub mod import_support;
pub mod workspace_support;
use async_trait::async_trait;
use cb_plugin_api::{
    ImportSupport, LanguageCapabilities, LanguageMetadata, LanguagePlugin, ManifestData,
    ParsedSource, PluginError, PluginResult, WorkspaceSupport,
};
use cb_lang_common::read_manifest;
use std::path::Path;
/// TypeScript/JavaScript language plugin implementation.
pub struct TypeScriptPlugin {
    metadata: LanguageMetadata,
    import_support: import_support::TypeScriptImportSupport,
    workspace_support: workspace_support::TypeScriptWorkspaceSupport,
}
impl TypeScriptPlugin {
    pub fn new() -> Self {
        Self {
            metadata: LanguageMetadata::TYPESCRIPT,
            import_support: import_support::TypeScriptImportSupport::new(),
            workspace_support: workspace_support::TypeScriptWorkspaceSupport::new(),
        }
    }
}
impl Default for TypeScriptPlugin {
    fn default() -> Self {
        Self::new()
    }
}
#[async_trait]
impl LanguagePlugin for TypeScriptPlugin {
    fn metadata(&self) -> &LanguageMetadata {
        &self.metadata
    }
    fn capabilities(&self) -> LanguageCapabilities {
        LanguageCapabilities {
            imports: true,
            workspace: true,
        }
    }
    async fn parse(&self, source: &str) -> PluginResult<ParsedSource> {
        let symbols = parser::extract_symbols(source)?;
        Ok(ParsedSource {
            data: serde_json::json!(
                { "language" : "typescript", "symbols_count" : symbols.len() }
            ),
            symbols,
        })
    }
    async fn analyze_manifest(&self, path: &Path) -> PluginResult<ManifestData> {
        manifest::load_package_json(path).await
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
impl TypeScriptPlugin {
    pub async fn update_dependency(
        &self,
        manifest_path: &Path,
        _old_name: &str,
        new_name: &str,
        new_version: Option<&str>,
    ) -> PluginResult<String> {
        let content = read_manifest(manifest_path).await?;
        let version = new_version
            .ok_or_else(|| {
                PluginError::invalid_input(
                    "Version required for package.json dependency updates",
                )
            })?;
        manifest::update_dependency(&content, new_name, version)
    }
    pub fn generate_manifest(
        &self,
        package_name: &str,
        dependencies: &[String],
    ) -> String {
        manifest::generate_manifest(package_name, dependencies)
    }
    /// Find module references (minimal implementation for compatibility)
    pub fn find_module_references(
        &self,
        content: &str,
        module_to_find: &str,
        _scope: cb_plugin_api::ScanScope,
    ) -> Vec<cb_plugin_api::ModuleReference> {
        use cb_plugin_api::{ModuleReference, ReferenceKind};
        let mut references = Vec::new();
        for (line_num, line) in content.lines().enumerate() {
            if (line.contains("import") || line.contains("from"))
                && line.contains(module_to_find)
            {
                references
                    .push(ModuleReference {
                        line: line_num + 1,
                        column: 0,
                        length: line.len(),
                        text: line.to_string(),
                        kind: ReferenceKind::Declaration,
                    });
            }
        }
        references
    }
    /// Rewrite imports for rename (minimal implementation for compatibility)
    pub fn rewrite_imports_for_rename(
        &self,
        content: &str,
        old_path: &Path,
        new_path: &Path,
        importing_file: &Path,
        _project_root: &Path,
        _rename_info: Option<&serde_json::Value>,
    ) -> PluginResult<(String, usize)> {
        // Use the standalone function with full context
        Ok(import_support::rewrite_imports_for_move_with_context(
            content,
            old_path,
            new_path,
            importing_file,
        ))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_typescript_capabilities() {
        let plugin = TypeScriptPlugin::new();
        let caps = plugin.capabilities();
        assert!(caps.imports, "TypeScript plugin should support imports");
        assert!(caps.workspace, "TypeScript plugin should support workspace");
    }
    #[test]
    fn test_typescript_workspace_support() {
        let plugin = TypeScriptPlugin::new();
        assert!(
            plugin.workspace_support().is_some(),
            "TypeScript should have workspace support"
        );
    }
}
