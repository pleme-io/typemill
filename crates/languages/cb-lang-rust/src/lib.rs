//! Rust Language Plugin for Codebuddy
//!
//! This crate provides complete Rust language support, implementing both:
//! - `LanguageIntelligencePlugin` - AST parsing and symbol extraction
//! - `LanguageAdapter` - Refactoring operations and import rewriting
//!
//! # Features
//!
//! - Full AST parsing using `syn`
//! - Symbol extraction (functions, structs, enums, etc.)
//! - Import analysis and rewriting
//! - Cargo.toml manifest parsing and manipulation
//! - Documentation extraction from doc comments
//! - Module file location and reference finding
//!
//! # Example
//!
//! ```rust,ignore
//! use cb_lang_rust::RustPlugin;
//! use cb_plugin_api::LanguagePlugin;
//!
//! let plugin = RustPlugin;
//! let source = "fn main() { println!(\"Hello\"); }";
//! let functions = plugin.list_functions(source).await.unwrap();
//! ```

mod adapter;
mod manifest;
mod parser;

use async_trait::async_trait;
use cb_plugin_api::{LanguageIntelligencePlugin, ManifestData, ParsedSource, PluginResult};
use std::path::Path;

/// Rust language plugin implementation
///
/// This plugin provides comprehensive Rust language support including:
/// - AST parsing and symbol extraction
/// - Import/use statement analysis
/// - Cargo.toml manifest handling
/// - Documentation extraction
pub struct RustPlugin;

impl RustPlugin {
    /// Create a new Rust plugin instance
    pub fn new() -> Self {
        Self
    }
}

impl Default for RustPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LanguageIntelligencePlugin for RustPlugin {
    fn name(&self) -> &'static str {
        "Rust"
    }

    fn file_extensions(&self) -> Vec<&'static str> {
        vec!["rs"]
    }

    async fn parse(&self, source: &str) -> PluginResult<ParsedSource> {
        // Extract all symbols from the source code
        let symbols = parser::extract_symbols(source)?;

        // Parse the source into a syn AST and serialize it as JSON
        let ast: syn::File = syn::parse_file(source).map_err(|e| {
            cb_plugin_api::PluginError::parse(format!("Failed to parse Rust code: {}", e))
        })?;

        // Serialize the AST to JSON using quote
        // For now, we'll store a simplified representation
        let ast_json = serde_json::json!({
            "type": "File",
            "items_count": ast.items.len(),
            "shebang": ast.shebang,
        });

        Ok(ParsedSource {
            data: ast_json,
            symbols,
        })
    }

    async fn analyze_manifest(&self, path: &Path) -> PluginResult<ManifestData> {
        // Verify this is a Cargo.toml file
        if path.file_name().and_then(|s| s.to_str()) != Some("Cargo.toml") {
            return Err(cb_plugin_api::PluginError::invalid_input(format!(
                "Expected Cargo.toml, got: {:?}",
                path.file_name()
            )));
        }

        manifest::load_cargo_toml(path).await
    }

    async fn list_functions(&self, source: &str) -> PluginResult<Vec<String>> {
        parser::list_functions(source)
    }

    async fn update_dependency(
        &self,
        manifest_path: &Path,
        old_name: &str,
        new_name: &str,
        new_path: Option<&str>,
    ) -> PluginResult<String> {
        // Read the current manifest content
        let content = tokio::fs::read_to_string(manifest_path)
            .await
            .map_err(|e| {
                cb_plugin_api::PluginError::manifest(format!("Failed to read manifest: {}", e))
            })?;

        // Use our manifest module to update the dependency
        manifest::rename_dependency(&content, old_name, new_name, new_path)
    }

    fn handles_manifest(&self, filename: &str) -> bool {
        filename == "Cargo.toml"
    }
}

// Re-export public API items
pub use manifest::{load_cargo_toml, parse_cargo_toml, rename_dependency};
pub use parser::{extract_symbols, list_functions, parse_imports, rewrite_use_tree};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rust_plugin_basic() {
        let plugin = RustPlugin::new();

        assert_eq!(plugin.name(), "Rust");
        assert_eq!(plugin.file_extensions(), vec!["rs"]);
        assert!(plugin.handles_extension("rs"));
        assert!(!plugin.handles_extension("py"));
    }

    #[tokio::test]
    async fn test_rust_plugin_parse() {
        let plugin = RustPlugin::new();
        let source = r#"
/// A test function
fn test_function() {
    println!("Hello, world!");
}

struct TestStruct {
    field: i32,
}
"#;

        let parsed = plugin.parse(source).await.unwrap();

        // Should extract both function and struct
        assert_eq!(parsed.symbols.len(), 2);

        // Check function
        let func = parsed
            .symbols
            .iter()
            .find(|s| s.name == "test_function")
            .unwrap();
        assert_eq!(func.kind, cb_plugin_api::SymbolKind::Function);
        assert!(func.documentation.is_some());

        // Check struct
        let struc = parsed
            .symbols
            .iter()
            .find(|s| s.name == "TestStruct")
            .unwrap();
        assert_eq!(struc.kind, cb_plugin_api::SymbolKind::Struct);
    }

    #[tokio::test]
    async fn test_rust_plugin_list_functions() {
        let plugin = RustPlugin::new();
        let source = r#"
fn first() {}
fn second() {}

impl MyStruct {
    fn method() {}
}
"#;

        let functions = plugin.list_functions(source).await.unwrap();
        assert_eq!(functions.len(), 3);
        assert!(functions.contains(&"first".to_string()));
        assert!(functions.contains(&"second".to_string()));
        assert!(functions.contains(&"method".to_string()));
    }

    #[tokio::test]
    async fn test_rust_plugin_parse_error() {
        let plugin = RustPlugin::new();
        let invalid_source = "fn incomplete_function {";

        let result = plugin.parse(invalid_source).await;
        assert!(result.is_err());
    }
}
