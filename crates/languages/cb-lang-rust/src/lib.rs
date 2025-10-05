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

mod manifest;
pub mod parser;
mod workspace;

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

    // LanguageAdapter methods
    fn language(&self) -> cb_core::language::ProjectLanguage {
        cb_core::language::ProjectLanguage::Rust
    }

    fn manifest_filename(&self) -> &'static str {
        "Cargo.toml"
    }

    fn source_dir(&self) -> &'static str {
        "src"
    }

    fn entry_point(&self) -> &'static str {
        "lib.rs"
    }

    fn module_separator(&self) -> &'static str {
        "::"
    }

    async fn locate_module_files(
        &self,
        package_path: &Path,
        module_path: &str,
    ) -> PluginResult<Vec<std::path::PathBuf>> {
        use tracing::debug;

        debug!(
            package_path = %package_path.display(),
            module_path = %module_path,
            "Locating Rust module files"
        );

        // Start at the crate's source root (e.g., package_path/src)
        let src_root = package_path.join(self.source_dir());

        if !src_root.exists() {
            return Err(cb_plugin_api::PluginError::internal(format!(
                "Source directory not found: {}",
                src_root.display()
            )));
        }

        // Split module path by either "::" or "." into segments
        let segments: Vec<&str> = module_path
            .split([':', '.'])
            .filter(|s| !s.is_empty())
            .collect();

        if segments.is_empty() {
            return Err(cb_plugin_api::PluginError::invalid_input(
                "Module path cannot be empty".to_string(),
            ));
        }

        // Build path by joining segments
        let mut current_path = src_root.clone();

        // Navigate through all segments except the last
        for segment in &segments[..segments.len() - 1] {
            current_path = current_path.join(segment);
        }

        // For the final segment, check both naming conventions
        let final_segment = segments[segments.len() - 1];
        let mut found_files = Vec::new();

        // Check for module_name.rs
        let file_path = current_path.join(format!("{}.rs", final_segment));
        if file_path.exists() && file_path.is_file() {
            debug!(file_path = %file_path.display(), "Found module file");
            found_files.push(file_path);
        }

        // Check for module_name/mod.rs
        let mod_path = current_path.join(final_segment).join("mod.rs");
        if mod_path.exists() && mod_path.is_file() {
            debug!(file_path = %mod_path.display(), "Found mod.rs file");
            found_files.push(mod_path);
        }

        if found_files.is_empty() {
            return Err(cb_plugin_api::PluginError::internal(format!(
                "Module '{}' not found at {} (checked both {}.rs and {}/mod.rs)",
                module_path,
                current_path.display(),
                final_segment,
                final_segment
            )));
        }

        debug!(
            files_count = found_files.len(),
            "Successfully located module files"
        );

        Ok(found_files)
    }

    async fn parse_imports(&self, file_path: &Path) -> PluginResult<Vec<String>> {
        use tracing::debug;

        debug!(
            file_path = %file_path.display(),
            "Parsing Rust imports"
        );

        // Read file content
        let content = tokio::fs::read_to_string(file_path)
            .await
            .map_err(|e| {
                cb_plugin_api::PluginError::internal(format!(
                    "Failed to read file {}: {}",
                    file_path.display(),
                    e
                ))
            })?;

        // Use our own parse_imports function
        let imports = parser::parse_imports(&content).map_err(|e| {
            cb_plugin_api::PluginError::parse(format!("Failed to parse imports: {}", e))
        })?;

        // Extract module paths from import info
        let module_paths: Vec<String> = imports.iter().map(|imp| imp.module_path.clone()).collect();

        debug!(imports_count = module_paths.len(), "Parsed imports");

        Ok(module_paths)
    }

    fn generate_manifest(&self, package_name: &str, dependencies: &[String]) -> String {
        use std::fmt::Write;

        let mut manifest = String::new();

        // [package] section
        writeln!(manifest, "[package]").unwrap();
        writeln!(manifest, "name = \"{}\"", package_name).unwrap();
        writeln!(manifest, "version = \"0.1.0\"").unwrap();
        writeln!(manifest, "edition = \"2021\"").unwrap();

        // Add blank line before dependencies section if there are any
        if !dependencies.is_empty() {
            writeln!(manifest).unwrap();
            writeln!(manifest, "[dependencies]").unwrap();

            // Add each dependency with wildcard version
            for dep in dependencies {
                writeln!(manifest, "{} = \"*\"", dep).unwrap();
            }
        }

        manifest
    }

    fn rewrite_import(&self, old_import: &str, new_package_name: &str) -> String {
        // Transform internal import path to external crate import
        // e.g., "crate::services::planner" -> "use cb_planner;"
        // e.g., "crate::services::planner::Config" -> "use cb_planner::Config;"

        // Remove common prefixes like "crate::", "self::", "super::"
        let trimmed = old_import
            .trim_start_matches("crate::")
            .trim_start_matches("self::")
            .trim_start_matches("super::");

        // Find the extracted module name and what comes after
        // The path segments after the module name become the new import path
        let segments: Vec<&str> = trimmed.split("::").collect();

        if segments.is_empty() {
            // Just use the new package name
            format!("use {};", new_package_name)
        } else if segments.len() == 1 {
            // Only the module name itself
            format!("use {};", new_package_name)
        } else {
            // Module name plus additional path
            // Skip the first segment (the old module name) and use the rest
            let remaining_path = segments[1..].join("::");
            format!("use {}::{};", new_package_name, remaining_path)
        }
    }

    fn rewrite_imports_for_rename(
        &self,
        content: &str,
        _old_path: &Path,
        _new_path: &Path,
        _importing_file: &Path,
        _project_root: &Path,
        rename_info: Option<&serde_json::Value>,
    ) -> PluginResult<(String, usize)> {
        use tracing::debug;

        // If no rename_info provided, no rewriting needed
        let rename_info = match rename_info {
            Some(info) => info,
            None => return Ok((content.to_string(), 0)),
        };

        // Extract old and new crate names from rename_info
        let old_crate_name = rename_info["old_crate_name"]
            .as_str()
            .ok_or_else(|| {
                cb_plugin_api::PluginError::invalid_input("Missing old_crate_name in rename_info")
            })?;

        let new_crate_name = rename_info["new_crate_name"]
            .as_str()
            .ok_or_else(|| {
                cb_plugin_api::PluginError::invalid_input("Missing new_crate_name in rename_info")
            })?;

        debug!(
            old_crate = %old_crate_name,
            new_crate = %new_crate_name,
            "Rewriting Rust imports for crate rename"
        );

        let mut result = String::new();
        let mut changes_count = 0;
        let lines: Vec<&str> = content.lines().collect();

        // Process line by line, using AST-based rewriting for use statements only
        for (idx, line) in lines.iter().enumerate() {
            let trimmed = line.trim();

            // Check if this line is a use statement containing our crate name
            if trimmed.starts_with("use ") && trimmed.contains(old_crate_name) {
                // Try to parse this line as a use statement
                match syn::parse_str::<syn::ItemUse>(trimmed) {
                    Ok(item_use) => {
                        // Try to rewrite using AST-based transformation
                        if let Some(new_tree) =
                            parser::rewrite_use_tree(&item_use.tree, old_crate_name, new_crate_name)
                        {
                            // Preserve original indentation
                            let indent = line.len() - trimmed.len();
                            let indent_str = &line[..indent];

                            // Write rewritten use statement with original indentation
                            result.push_str(indent_str);
                            result.push_str(&format!("use {};", quote::quote!(#new_tree)));

                            // Add newline if not last line
                            if idx < lines.len() - 1 {
                                result.push('\n');
                            }
                            changes_count += 1;
                            continue;
                        }
                    }
                    Err(_) => {
                        // If parsing fails (e.g., multi-line use statement), keep original
                        // This is safe - we won't break anything, just won't rewrite it
                        debug!(
                            line = %line,
                            "Could not parse use statement, keeping original"
                        );
                    }
                }
            }

            // Keep original line (either not a use statement, or parsing failed)
            result.push_str(line);

            // Add newline if not last line
            if idx < lines.len() - 1 {
                result.push('\n');
            }
        }

        debug!(changes = changes_count, "Rewrote Rust imports using AST");

        Ok((result, changes_count))
    }

    fn find_module_references(
        &self,
        content: &str,
        module_to_find: &str,
        scope: cb_plugin_api::ScanScope,
    ) -> PluginResult<Vec<cb_plugin_api::ModuleReference>> {
        use syn::visit::Visit;
        use syn::File;

        // Parse the Rust source file
        let file: File = syn::parse_str(content).map_err(|e| {
            cb_plugin_api::PluginError::parse(format!("Failed to parse Rust source: {}", e))
        })?;

        // Create and run visitor
        let mut finder = RustModuleFinder::new(module_to_find, scope);
        finder.visit_file(&file);

        Ok(finder.into_references())
    }

    async fn add_manifest_path_dependency(
        &self,
        manifest_content: &str,
        dep_name: &str,
        dep_path: &str,
        source_path: &Path,
    ) -> PluginResult<String> {
        workspace::add_path_dependency(manifest_content, dep_name, dep_path, source_path)
    }

    async fn add_workspace_member(
        &self,
        workspace_content: &str,
        new_member_path: &str,
        workspace_root: &Path,
    ) -> PluginResult<String> {
        workspace::add_workspace_member(workspace_content, new_member_path, workspace_root)
    }

    async fn generate_workspace_manifest(
        &self,
        member_paths: &[&str],
        workspace_root: &Path,
    ) -> PluginResult<String> {
        workspace::generate_workspace_manifest(member_paths, workspace_root)
    }

    async fn is_workspace_manifest(&self, manifest_content: &str) -> PluginResult<bool> {
        Ok(workspace::is_workspace_manifest(manifest_content))
    }

    async fn remove_module_declaration(
        &self,
        source: &str,
        module_name: &str,
    ) -> PluginResult<String> {
        let mut file = syn::parse_file(source).map_err(|e| {
            cb_plugin_api::PluginError::parse(format!("Failed to parse Rust source: {}", e))
        })?;

        // Remove module declarations matching the module name
        file.items.retain(|item| {
            if let syn::Item::Mod(item_mod) = item {
                item_mod.ident != module_name
            } else {
                true
            }
        });

        // Convert back to source code
        Ok(quote::quote!(#file).to_string())
    }

    async fn find_source_files(&self, dir: &Path) -> PluginResult<Vec<std::path::PathBuf>> {
        use std::fs;
        use cb_plugin_api::PluginError;

        let mut source_files = Vec::new();
        let entries = fs::read_dir(dir)
            .map_err(|e| PluginError::internal(format!("Failed to read directory: {}", e)))?;

        for entry in entries {
            let entry = entry.map_err(|e| {
                PluginError::internal(format!("Failed to read directory entry: {}", e))
            })?;
            let path = entry.path();

            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == "rs" {
                        source_files.push(path);
                    }
                }
            }
        }

        Ok(source_files)
    }
}

/// Visitor for finding module references in Rust code using syn::visit
struct RustModuleFinder<'a> {
    module_to_find: &'a str,
    scope: cb_plugin_api::ScanScope,
    references: Vec<cb_plugin_api::ModuleReference>,
}

impl<'a> RustModuleFinder<'a> {
    fn new(module_to_find: &'a str, scope: cb_plugin_api::ScanScope) -> Self {
        Self {
            module_to_find,
            scope,
            references: Vec::new(),
        }
    }

    fn into_references(self) -> Vec<cb_plugin_api::ModuleReference> {
        self.references
    }

    fn check_use_tree(&mut self, tree: &syn::UseTree) {
        match tree {
            syn::UseTree::Path(path) => {
                if path.ident == self.module_to_find {
                    self.references.push(cb_plugin_api::ModuleReference {
                        line: 0,
                        column: 0,
                        length: self.module_to_find.len(),
                        text: self.module_to_find.to_string(),
                        kind: cb_plugin_api::ReferenceKind::Declaration,
                    });
                }
                self.check_use_tree(&path.tree);
            }
            syn::UseTree::Name(name) => {
                if name.ident == self.module_to_find {
                    self.references.push(cb_plugin_api::ModuleReference {
                        line: 0,
                        column: 0,
                        length: self.module_to_find.len(),
                        text: self.module_to_find.to_string(),
                        kind: cb_plugin_api::ReferenceKind::Declaration,
                    });
                }
            }
            syn::UseTree::Group(group) => {
                for item in &group.items {
                    self.check_use_tree(item);
                }
            }
            _ => {}
        }
    }
}

impl<'ast, 'a> syn::visit::Visit<'ast> for RustModuleFinder<'a> {
    fn visit_item_use(&mut self, node: &'ast syn::ItemUse) {
        // Check the use tree for our module
        self.check_use_tree(&node.tree);

        // Continue visiting child nodes
        syn::visit::visit_item_use(self, node);
    }

    fn visit_expr_path(&mut self, node: &'ast syn::ExprPath) {
        // Only check qualified paths if scope allows
        if self.scope == cb_plugin_api::ScanScope::QualifiedPaths
            || self.scope == cb_plugin_api::ScanScope::All
        {
            if let Some(segment) = node.path.segments.first() {
                if segment.ident == self.module_to_find {
                    self.references.push(cb_plugin_api::ModuleReference {
                        line: 0,
                        column: 0,
                        length: self.module_to_find.len(),
                        text: quote::quote!(#node).to_string(),
                        kind: cb_plugin_api::ReferenceKind::QualifiedPath,
                    });
                }
            }
        }

        // Continue visiting
        syn::visit::visit_expr_path(self, node);
    }

    fn visit_expr_lit(&mut self, node: &'ast syn::ExprLit) {
        // Only check string literals if scope is All
        if self.scope == cb_plugin_api::ScanScope::All {
            if let syn::Lit::Str(lit_str) = &node.lit {
                let value = lit_str.value();
                if value.contains(self.module_to_find) {
                    self.references.push(cb_plugin_api::ModuleReference {
                        line: 0,
                        column: 0,
                        length: self.module_to_find.len(),
                        text: value,
                        kind: cb_plugin_api::ReferenceKind::StringLiteral,
                    });
                }
            }
        }

        // Continue visiting
        syn::visit::visit_expr_lit(self, node);
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

        let rename_info = serde_json::json!({
            "old_crate_name": "old_crate",
            "new_crate_name": "new_crate",
        });

        let (result, count) = plugin
            .rewrite_imports_for_rename(
                source,
                Path::new(""),
                Path::new(""),
                Path::new(""),
                Path::new(""),
                Some(&rename_info),
            )
            .unwrap();

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
