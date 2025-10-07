//! Rust Language Plugin for Codebuddy
//!
//! This crate provides complete Rust language support, implementing both:
//! - `LanguagePlugin` - AST parsing and symbol extraction
//! - Import and workspace support traits
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
pub use parser :: { extract_symbols , list_functions , parse_imports , rewrite_use_tree } ;
//! ```rust,ignore
//! use cb_lang_rust::RustPlugin;
//! use cb_plugin_api::LanguagePlugin;
//!
//! let plugin = RustPlugin::new();
//! let source = "fn main() { println!(\"Hello\"); }";
//! let functions = plugin.list_functions(source).await.unwrap();
//! ```

mod manifest;
pub mod parser;
pub mod refactoring;
mod workspace;

// Capability trait implementations
pub mod import_support;
pub mod workspace_support;

use async_trait::async_trait;
use cb_plugin_api::{LanguagePlugin, LanguageMetadata, LanguageCapabilities, ManifestData, ParsedSource, PluginResult};
use cb_lang_common::{read_manifest, manifest_templates::{ManifestTemplate, TomlManifestTemplate}};
use std::path::Path;

/// Rust language plugin implementation
///
/// This plugin provides comprehensive Rust language support including:
/// - AST parsing and symbol extraction
/// - Import/use statement analysis
/// - Cargo.toml manifest handling
/// - Documentation extraction
pub struct RustPlugin {
    metadata: LanguageMetadata,
    import_support: import_support::RustImportSupport,
    workspace_support: workspace_support::RustWorkspaceSupport,
}

impl RustPlugin {
    /// Create a new Rust plugin instance
    pub fn new() -> Self {
        Self {
            metadata: LanguageMetadata::RUST,
            import_support: import_support::RustImportSupport,
            workspace_support: workspace_support::RustWorkspaceSupport,
        }
    }
}

impl Default for RustPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LanguagePlugin for RustPlugin {
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

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn import_support(&self) -> Option<&dyn cb_plugin_api::ImportSupport> {
        Some(&self.import_support)
    }

    fn workspace_support(&self) -> Option<&dyn cb_plugin_api::WorkspaceSupport> {
        Some(&self.workspace_support)
    }
}

// ============================================================================
// Plugin-specific helper methods for consumers
// These are NOT part of capability traits - they're Rust-specific utilities
// ============================================================================

impl RustPlugin {
    /// Update a dependency in Cargo.toml manifest
    pub async fn update_dependency(
        &self,
        manifest_path: &Path,
        old_name: &str,
        new_name: &str,
        new_version: Option<&str>,
    ) -> PluginResult<String> {
        let content = read_manifest(manifest_path).await?;

        // For Rust, new_version might be a path or a version
        // If it looks like a path, use path dependency; otherwise use version
        if let Some(version_or_path) = new_version {
            if version_or_path.contains('/') || version_or_path.contains('\\') {
                // It's a path dependency - use rename with path
                manifest::rename_dependency(&content, old_name, new_name, Some(version_or_path))
            } else {
                // It's a version - use rename with version as path (Cargo.toml uses same field)
                manifest::rename_dependency(&content, old_name, new_name, Some(version_or_path))
            }
        } else {
            // No version provided, just update the name
            manifest::rename_dependency(&content, old_name, new_name, None)
        }
    }

    /// Locate module files for a given module path
    ///
    /// Navigates the Rust module system to find .rs files for a module path like "services::planner"
    pub async fn locate_module_files(
        &self,
        package_path: &Path,
        module_path: &str,
    ) -> PluginResult<Vec<std::path::PathBuf>> {
        // Handle empty module path
        if module_path.is_empty() {
            return Err(cb_plugin_api::PluginError::invalid_input(
                "Module path cannot be empty",
            ));
        }

        // Normalize module path (handle both :: and . separators)
        let normalized = module_path.replace('.', "::");
        let parts: Vec<&str> = normalized.split("::").collect();

        // Start from src/ directory
        let src_dir = package_path.join("src");
        if !src_dir.exists() {
            return Err(cb_plugin_api::PluginError::internal(format!(
                "Source directory not found: {}",
                src_dir.display()
            )));
        }

        let mut current_path = src_dir;
        let mut result_files = Vec::new();

        // Navigate through module path components
        for (i, part) in parts.iter().enumerate() {
            let is_last = i == parts.len() - 1;

            if is_last {
                // Check for module.rs or module/mod.rs
                let single_file = current_path.join(format!("{}.rs", part));
                let mod_dir = current_path.join(part).join("mod.rs");

                if single_file.exists() {
                    result_files.push(single_file);
                } else if mod_dir.exists() {
                    result_files.push(mod_dir);
                } else {
                    return Err(cb_plugin_api::PluginError::invalid_input(format!(
                        "Module not found: {}",
                        module_path
                    )));
                }
            } else {
                // Navigate to subdirectory
                current_path = current_path.join(part);
                if !current_path.exists() {
                    return Err(cb_plugin_api::PluginError::invalid_input(format!(
                        "Module path not found: {}",
                        current_path.display()
                    )));
                }
            }
        }

        Ok(result_files)
    }

    /// Parse imports from a file path (async wrapper)
    pub async fn parse_imports(&self, file_path: &Path) -> PluginResult<Vec<String>> {
        let content = read_manifest(file_path).await?;

        // Use the parser module to extract imports
        let import_infos = parser::parse_imports(&content)?;

        // Extract just the module paths
        Ok(import_infos.iter().map(|i| i.module_path.clone()).collect())
    }

    /// Generate a Cargo.toml manifest
    pub fn generate_manifest(&self, package_name: &str, dependencies: &[String]) -> String {
        let template = TomlManifestTemplate::new("package");
        let mut manifest = template.generate(package_name, "0.1.0", dependencies);

        // Add Rust-specific edition field
        if let Some(version_pos) = manifest.find("version = \"0.1.0\"") {
            let insert_pos = manifest[version_pos..].find('\n').map(|p| version_pos + p + 1);
            if let Some(pos) = insert_pos {
                manifest.insert_str(pos, "edition = \"2021\"\n");
            }
        }

        manifest
    }

    /// Remove module declaration from source
    pub async fn remove_module_declaration(
        &self,
        source: &str,
        module_name: &str,
    ) -> PluginResult<String> {
        use syn::{File, Item};

        // Parse the source file
        let ast: File = syn::parse_file(source).map_err(|e| {
            cb_plugin_api::PluginError::parse(format!("Failed to parse Rust code: {}", e))
        })?;

        // Filter out module declarations matching the name
        let filtered_items: Vec<Item> = ast
            .items
            .into_iter()
            .filter(|item| {
                if let Item::Mod(item_mod) = item {
                    item_mod.ident != module_name
                } else {
                    true
                }
            })
            .collect();

        // Reconstruct the file
        let new_ast = File {
            shebang: ast.shebang,
            attrs: ast.attrs,
            items: filtered_items,
        };

        // Convert back to source code
        Ok(quote::quote!(#new_ast).to_string())
    }

    /// Add path dependency to manifest
    pub async fn add_manifest_path_dependency(
        &self,
        manifest_content: &str,
        dep_name: &str,
        dep_path: &str,
        source_path: &Path,
    ) -> PluginResult<String> {
        workspace::add_path_dependency(manifest_content, dep_name, dep_path, source_path)
    }

    /// Generate workspace manifest
    pub async fn generate_workspace_manifest(
        &self,
        member_paths: &[&str],
        workspace_root: &Path,
    ) -> PluginResult<String> {
        workspace::generate_workspace_manifest(member_paths, workspace_root)
    }

    /// Find source files in directory
    pub async fn find_source_files(&self, dir: &Path) -> PluginResult<Vec<std::path::PathBuf>> {
        use tokio::fs;

        let mut result = Vec::new();
        let mut queue = vec![dir.to_path_buf()];

        while let Some(current_dir) = queue.pop() {
            let mut entries = fs::read_dir(&current_dir).await.map_err(|e| {
                cb_plugin_api::PluginError::internal(format!(
                    "Failed to read directory {}: {}",
                    current_dir.display(),
                    e
                ))
            })?;

            while let Some(entry) = entries.next_entry().await.map_err(|e| {
                cb_plugin_api::PluginError::internal(format!("Failed to read entry: {}", e))
            })? {
                let path = entry.path();
                let metadata = entry.metadata().await.map_err(|e| {
                    cb_plugin_api::PluginError::internal(format!("Failed to get metadata: {}", e))
                })?;

                if metadata.is_dir() {
                    queue.push(path);
                } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
                    result.push(path);
                }
            }
        }

        Ok(result)
    }

    /// Rewrite import statement
    pub fn rewrite_import(&self, old_import: &str, new_package_name: &str) -> String {
        // Transform "crate::module" or "crate" to "new_package::module"
        if old_import == "crate" {
            format!("use {};", new_package_name)
        } else if let Some(rest) = old_import.strip_prefix("crate::") {
            format!("use {}::{};", new_package_name, rest)
        } else {
            // If it doesn't start with crate::, return as-is
            format!("use {};", old_import)
        }
    }

    /// Find module references with full signature
    pub fn find_module_references(
        &self,
        content: &str,
        module_to_find: &str,
        _scope: cb_plugin_api::ScanScope,
    ) -> PluginResult<Vec<cb_plugin_api::ModuleReference>> {
        use syn::{File, Item};
        use cb_plugin_api::{ModuleReference, ReferenceKind};

        let ast: File = syn::parse_file(content).map_err(|e| {
            cb_plugin_api::PluginError::parse(format!("Failed to parse Rust code: {}", e))
        })?;

        let mut references = Vec::new();
        let lines: Vec<&str> = content.lines().collect();

        for (line_num, item) in ast.items.iter().enumerate() {
            if let Item::Use(item_use) = item {
                let use_str = quote::quote!(#item_use).to_string();
                if use_str.contains(module_to_find) {
                    // Get the actual line length from source content, not from quote-generated string
                    // The quote-generated string may have different formatting/whitespace
                    let actual_line = if line_num < lines.len() {
                        lines[line_num]
                    } else {
                        ""
                    };

                    // Find the actual position of the module name in the source line
                    let (column, length) = if let Some(start_pos) = actual_line.find(module_to_find) {
                        (start_pos, module_to_find.len())
                    } else {
                        // Fallback: replace entire line
                        (0, actual_line.len())
                    };

                    references.push(ModuleReference {
                        line: line_num + 1, // 1-based
                        column,
                        length,
                        text: use_str,
                        kind: ReferenceKind::Declaration,
                    });
                }
            }
        }

        Ok(references)
    }

    /// Rewrite imports for rename with full signature
    pub fn rewrite_imports_for_rename(
        &self,
        content: &str,
        _old_path: &Path,
        _new_path: &Path,
        _importing_file: &Path,
        _project_root: &Path,
        rename_info: Option<&serde_json::Value>,
    ) -> PluginResult<(String, usize)> {
        // Delegate to import capability with simpler signature
        if let Some(import_support) = self.import_support() {
            if let Some(info) = rename_info {
                let old_name = info["old_crate_name"].as_str().unwrap_or("");
                let new_name = info["new_crate_name"].as_str().unwrap_or("");
                Ok(import_support.rewrite_imports_for_rename(content, old_name, new_name))
            } else {
                Ok((content.to_string(), 0))
            }
        } else {
            Ok((content.to_string(), 0))
        }
    }
}

// Re-export public API items
pub use manifest::{load_cargo_toml, parse_cargo_toml, rename_dependency};
pub use parser::{extract_symbols, list_functions, parse_imports, rewrite_use_tree};
pub use workspace::{
    add_path_dependency, add_workspace_member, generate_workspace_manifest, is_workspace_manifest,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rust_plugin_basic() {
        let plugin = RustPlugin::new();

        assert_eq!(plugin.metadata().name, "Rust");
        assert_eq!(plugin.metadata().extensions, &["rs"]);
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

    #[tokio::test]
    async fn test_rewrite_imports_preserves_non_use_content() {
        let plugin = RustPlugin::new();

        // Source with use statements AND other content that contains the crate name
        let source = r#"use old_crate::Foo;
use old_crate::bar::Baz;

/// Documentation mentioning old_crate
pub struct Wrapper {
    old_crate_field: String,  // Should NOT be changed
}

impl Wrapper {
    fn init_old_crate() {  // Should NOT be changed
        log::info!("Using old_crate");  // Should NOT be changed
    }
}"#;

        // Use the ImportSupport trait method instead
        let import_support = plugin.import_support().unwrap();
        let (result, count) = import_support.rewrite_imports_for_rename(
            source,
            "old_crate",
            "new_crate",
        );

        // Should have changed exactly 2 use statements
        assert_eq!(count, 2);

        // Verify use statements were rewritten (note: quote! adds spaces around ::)
        assert!(result.contains("use new_crate"));
        assert!(result.contains("Foo"));
        assert!(result.contains("bar"));
        assert!(result.contains("Baz"));

        // Verify other content was NOT changed
        assert!(result.contains("old_crate_field"));
        assert!(result.contains("fn init_old_crate()"));
        assert!(result.contains(r#"log::info!("Using old_crate");"#));
        assert!(result.contains("/// Documentation mentioning old_crate"));

        // Verify old use statements with old_crate are gone
        assert!(!result.contains("use old_crate"));
    }
}
