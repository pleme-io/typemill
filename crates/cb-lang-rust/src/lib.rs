//! Rust Language Plugin for Codebuddy
//!
//! This crate provides complete Rust language support, implementing both:
//! - `LanguagePlugin` - AST parsing and symbol extraction
//! - Import and workspace support traits

mod manifest;
pub mod parser;
pub mod refactoring;
mod workspace;

// Capability trait implementations
pub mod import_support;
pub mod workspace_support;

use async_trait::async_trait;
use cb_lang_common::{
    manifest_templates::{ManifestTemplate, TomlManifestTemplate},
    read_manifest,
};
use cb_plugin_api::{
    LanguageMetadata, LanguagePlugin, LspConfig, ManifestData, ParsedSource, PluginCapabilities,
    PluginResult,
};
use cb_plugin_registry::codebuddy_plugin;
use std::path::Path;

// Self-register the plugin with the Codebuddy system.
codebuddy_plugin! {
    name: "rust",
    extensions: ["rs"],
    manifest: "Cargo.toml",
    capabilities: RustPlugin::CAPABILITIES,
    factory: RustPlugin::new,
    lsp: Some(LspConfig::new("rust-analyzer", &["rust-analyzer"]))
}

/// Rust language plugin implementation.
#[derive(Default)]
pub struct RustPlugin {
    import_support: import_support::RustImportSupport,
    workspace_support: workspace_support::RustWorkspaceSupport,
}

impl RustPlugin {
    /// Static metadata for the Rust language.
    pub const METADATA: LanguageMetadata = LanguageMetadata {
        name: "rust",
        extensions: &["rs"],
        manifest_filename: "Cargo.toml",
        source_dir: "src",
        entry_point: "lib.rs",
        module_separator: "::",
    };

    /// The capabilities of this plugin.
    pub const CAPABILITIES: PluginCapabilities = PluginCapabilities {
        imports: true,
        workspace: true,
    };

    /// Creates a new, boxed instance of the plugin.
    pub fn new() -> Box<dyn LanguagePlugin> {
        Box::new(Self::default())
    }
}

#[async_trait]
impl LanguagePlugin for RustPlugin {
    fn metadata(&self) -> &LanguageMetadata {
        &Self::METADATA
    }

    fn capabilities(&self) -> PluginCapabilities {
        Self::CAPABILITIES
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

    fn rewrite_file_references(
        &self,
        content: &str,
        old_path: &Path,
        new_path: &Path,
        current_file: &Path,
        project_root: &Path,
        rename_info: Option<&serde_json::Value>,
    ) -> Option<(String, usize)> {
        tracing::info!(
            old_path = %old_path.display(),
            new_path = %new_path.display(),
            current_file = %current_file.display(),
            "RustPlugin::rewrite_file_references ENTRY"
        );

        let result = self.rewrite_imports_for_rename(
            content,
            old_path,
            new_path,
            current_file,
            project_root,
            rename_info,
        );

        match &result {
            Ok((content, count)) => {
                tracing::info!(
                    content_len = content.len(),
                    changes_count = count,
                    "RustPlugin::rewrite_file_references OK"
                );
            }
            Err(e) => {
                tracing::error!(
                    error = ?e,
                    "RustPlugin::rewrite_file_references ERR"
                );
            }
        }

        result.ok()
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
            let insert_pos = manifest[version_pos..]
                .find('\n')
                .map(|p| version_pos + p + 1);
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
        use cb_plugin_api::{ModuleReference, ReferenceKind};
        use syn::{File, Item};

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

                    // We need to replace the entire use statement line, not just the module name
                    // The text field contains the full formatted import from quote::quote!
                    // So column should be 0 and length should be the entire line length
                    let column = 0;
                    let length = actual_line.len();

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
        tracing::info!(
            old_path = %_old_path.display(),
            new_path = %_new_path.display(),
            "RustPlugin::rewrite_imports_for_rename ENTRY"
        );

        // Delegate to import capability with simpler signature
        if let Some(import_support) = self.import_support() {
            if let Some(info) = rename_info {
                let old_name = info["old_crate_name"].as_str().unwrap_or("");
                let new_name = info["new_crate_name"].as_str().unwrap_or("");
                Ok(import_support.rewrite_imports_for_rename(content, old_name, new_name))
            } else {
                // This is a file move, not a crate rename. Infer crate from path.
                // Strip project_root to get relative path, then extract crate name (first component)

                // Canonicalize project_root to handle symlinks (e.g., /var vs /private/var on macOS)
                let canonical_project = _project_root
                    .canonicalize()
                    .unwrap_or_else(|_| _project_root.to_path_buf());

                // Try to canonicalize paths, fallback to original if they don't exist yet
                let canonical_old = _old_path
                    .canonicalize()
                    .unwrap_or_else(|_| _old_path.to_path_buf());
                let canonical_new = _new_path
                    .canonicalize()
                    .unwrap_or_else(|_| _new_path.to_path_buf());

                // Extract crate name from relative path (first component after project root)
                let old_crate_name = canonical_old
                    .strip_prefix(&canonical_project)
                    .ok()
                    .and_then(|rel| {
                        tracing::debug!(
                            relative_old = %rel.display(),
                            "Stripped old path to get relative"
                        );
                        rel.components().next()
                    })
                    .and_then(|c| c.as_os_str().to_str())
                    .map(String::from);

                let new_crate_name = canonical_new
                    .strip_prefix(&canonical_project)
                    .ok()
                    .and_then(|rel| {
                        tracing::debug!(
                            relative_new = %rel.display(),
                            "Stripped new path to get relative"
                        );
                        rel.components().next()
                    })
                    .and_then(|c| c.as_os_str().to_str())
                    .map(String::from);

                tracing::debug!(
                    old_crate = ?old_crate_name,
                    new_crate = ?new_crate_name,
                    "Extracted crate names from paths"
                );

                // Fallback: If crate name extraction failed, try finding Cargo.toml
                let old_crate_name = old_crate_name.or_else(|| {
                    tracing::info!(
                        old_path = %_old_path.display(),
                        "Path extraction failed for old_path, trying Cargo.toml fallback"
                    );
                    find_crate_name_from_cargo_toml(_old_path)
                });

                let new_crate_name = new_crate_name.or_else(|| {
                    tracing::info!(
                        new_path = %_new_path.display(),
                        "Path extraction failed for new_path, trying Cargo.toml fallback"
                    );
                    find_crate_name_from_cargo_toml(_new_path)
                });

                tracing::info!(
                    old_crate = ?old_crate_name,
                    new_crate = ?new_crate_name,
                    old_path = %_old_path.display(),
                    new_path = %_new_path.display(),
                    project_root = %_project_root.display(),
                    "After fallback crate name extraction"
                );

                if let (Some(ref old_name), Some(ref new_name)) = (&old_crate_name, &new_crate_name)
                {
                    tracing::info!(
                        old_name = %old_name,
                        new_name = %new_name,
                        "Both crate names extracted successfully"
                    );

                    // Always compute full module paths including file structure
                    // This allows us to detect moves within the same crate
                    let old_module_path =
                        compute_module_path_from_file(_old_path, old_name, &canonical_project);
                    let new_module_path =
                        compute_module_path_from_file(_new_path, new_name, &canonical_project);

                    tracing::info!(
                        old_module_path = %old_module_path,
                        new_module_path = %new_module_path,
                        "Computed full module paths for comparison"
                    );

                    // Rewrite if module paths differ (handles both cross-crate and same-crate moves)
                    if old_module_path != new_module_path {
                        let result = import_support.rewrite_imports_for_rename(
                            content,
                            &old_module_path,
                            &new_module_path,
                        );
                        tracing::info!(
                            result_len = result.0.len(),
                            changes_count = result.1,
                            "Rewrite completed"
                        );
                        return Ok(result);
                    } else {
                        tracing::info!("Module paths are identical - no rewrite needed");
                    }
                } else {
                    tracing::error!(
                        old_crate = ?old_crate_name,
                        new_crate = ?new_crate_name,
                        "Failed to extract crate names - no rewrite possible"
                    );
                }

                tracing::info!("Returning no changes: (content, 0)");
                Ok((content.to_string(), 0))
            }
        } else {
            Ok((content.to_string(), 0))
        }
    }
}

/// Compute the full module path from a file path
///
/// # Examples
/// - `common/src/utils.rs` → `common::utils`
/// - `common/src/utils/mod.rs` → `common::utils` (mod.rs represents the parent directory)
/// - `common/src/foo/bar/mod.rs` → `common::foo::bar`
/// - `new_utils/src/lib.rs` → `new_utils` (lib.rs is the crate root)
/// - `common/src/main.rs` → `common` (main.rs is the crate root)
/// - `common/src/foo/bar.rs` → `common::foo::bar`
fn compute_module_path_from_file(
    file_path: &Path,
    crate_name: &str,
    project_root: &Path,
) -> String {
    // Get the file path relative to project root
    let rel_path = file_path.strip_prefix(project_root).unwrap_or(file_path);

    // Get components after the crate name
    let mut components: Vec<&str> = rel_path
        .components()
        .filter_map(|c| c.as_os_str().to_str())
        .collect();

    // Remove the crate name (first component)
    if !components.is_empty() {
        components.remove(0);
    }

    // Remove "src" if present
    if components.first().map(|s| *s) == Some("src") {
        components.remove(0);
    }

    // Special handling for mod.rs files
    // mod.rs represents the parent directory's module, not a module named "mod"
    // Example: common/src/utils/mod.rs → common::utils (not common::utils::mod)
    if components.last().map(|s| *s) == Some("mod.rs") {
        components.pop(); // Remove "mod.rs"
        // The parent directory name is now the last component (the module name)
    }

    // If the file is lib.rs or main.rs, it's the crate root
    if components.last().map(|s| *s) == Some("lib.rs")
        || components.last().map(|s| *s) == Some("main.rs")
    {
        return crate_name.to_string();
    }

    // Remove the .rs extension from the last component
    if let Some(last) = components.last_mut() {
        if let Some(stripped) = last.strip_suffix(".rs") {
            *last = stripped;
        }
    }

    // Build the module path: crate_name::module1::module2...
    let mut module_path = crate_name.to_string();
    for component in components {
        if !component.is_empty() {
            module_path.push_str("::");
            module_path.push_str(component);
        }
    }

    module_path
}

/// Helper function to extract crate name from Cargo.toml
/// Used as fallback when path-based extraction fails (e.g., file doesn't exist yet)
fn find_crate_name_from_cargo_toml(file_path: &Path) -> Option<String> {
    let mut current = file_path.parent()?;
    while current.components().count() > 0 {
        let cargo_toml = current.join("Cargo.toml");
        if cargo_toml.exists() {
            if let Ok(content) = std::fs::read_to_string(&cargo_toml) {
                for line in content.lines() {
                    let trimmed = line.trim();
                    if trimmed.starts_with("name") && trimmed.contains('=') {
                        if let Some(name_part) = trimmed.split('=').nth(1) {
                            let name = name_part.trim().trim_matches('"').trim_matches('\'');
                            tracing::info!(
                                crate_name = %name,
                                cargo_toml = %cargo_toml.display(),
                                "Found crate name in Cargo.toml"
                            );
                            return Some(name.to_string());
                        }
                    }
                }
            }
            break;
        }
        current = current.parent()?;
    }
    tracing::warn!(
        file_path = %file_path.display(),
        "Could not find Cargo.toml walking up from file path"
    );
    None
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
        let plugin_trait: &dyn LanguagePlugin = plugin.as_ref();

        assert_eq!(plugin_trait.metadata().name, "rust");
        assert_eq!(plugin_trait.metadata().extensions, &["rs"]);
        assert!(plugin_trait.handles_extension("rs"));
        assert!(!plugin_trait.handles_extension("py"));
    }

    #[tokio::test]
    async fn test_rust_plugin_parse() {
        let plugin = RustPlugin::new();
        let plugin_trait: &dyn LanguagePlugin = plugin.as_ref();
        let source = r#"
/// A test function
fn test_function() {
    println!("Hello, world!");
}

struct TestStruct {
    field: i32,
}
"#;

        let parsed = plugin_trait.parse(source).await.unwrap();

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
        let plugin_trait: &dyn LanguagePlugin = plugin.as_ref();
        let source = r#"
fn first() {}
fn second() {}

impl MyStruct {
    fn method() {}
}
"#;

        let functions = plugin_trait.list_functions(source).await.unwrap();
        assert_eq!(functions.len(), 3);
        assert!(functions.contains(&"first".to_string()));
        assert!(functions.contains(&"second".to_string()));
        assert!(functions.contains(&"method".to_string()));
    }

    #[tokio::test]
    async fn test_rust_plugin_parse_error() {
        let plugin = RustPlugin::new();
        let plugin_trait: &dyn LanguagePlugin = plugin.as_ref();
        let invalid_source = "fn incomplete_function {";

        let result = plugin_trait.parse(invalid_source).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_rewrite_imports_preserves_non_use_content() {
        let plugin = RustPlugin::new();
        let plugin_trait: &dyn LanguagePlugin = plugin.as_ref();

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
        let import_support = plugin_trait.import_support().unwrap();
        let (result, count) =
            import_support.rewrite_imports_for_rename(source, "old_crate", "new_crate");

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

    #[test]
    fn test_compute_module_path_from_file_simple() {
        let project_root = Path::new("/workspace");

        // Test simple file: common/src/utils.rs → common::utils
        let file_path = Path::new("/workspace/common/src/utils.rs");
        let result = compute_module_path_from_file(file_path, "common", project_root);
        assert_eq!(result, "common::utils");
    }

    #[test]
    fn test_compute_module_path_from_file_mod_rs() {
        let project_root = Path::new("/workspace");

        // Test mod.rs: common/src/utils/mod.rs → common::utils (NOT common::utils::mod)
        let file_path = Path::new("/workspace/common/src/utils/mod.rs");
        let result = compute_module_path_from_file(file_path, "common", project_root);
        assert_eq!(result, "common::utils");
    }

    #[test]
    fn test_compute_module_path_from_file_nested_mod_rs() {
        let project_root = Path::new("/workspace");

        // Test nested mod.rs: common/src/foo/bar/mod.rs → common::foo::bar
        let file_path = Path::new("/workspace/common/src/foo/bar/mod.rs");
        let result = compute_module_path_from_file(file_path, "common", project_root);
        assert_eq!(result, "common::foo::bar");
    }

    #[test]
    fn test_compute_module_path_from_file_lib_rs() {
        let project_root = Path::new("/workspace");

        // Test lib.rs (crate root): common/src/lib.rs → common
        let file_path = Path::new("/workspace/common/src/lib.rs");
        let result = compute_module_path_from_file(file_path, "common", project_root);
        assert_eq!(result, "common");
    }

    #[test]
    fn test_compute_module_path_from_file_main_rs() {
        let project_root = Path::new("/workspace");

        // Test main.rs (crate root): common/src/main.rs → common
        let file_path = Path::new("/workspace/common/src/main.rs");
        let result = compute_module_path_from_file(file_path, "common", project_root);
        assert_eq!(result, "common");
    }

    #[test]
    fn test_compute_module_path_from_file_nested() {
        let project_root = Path::new("/workspace");

        // Test nested file: common/src/foo/bar.rs → common::foo::bar
        let file_path = Path::new("/workspace/common/src/foo/bar.rs");
        let result = compute_module_path_from_file(file_path, "common", project_root);
        assert_eq!(result, "common::foo::bar");
    }

    #[tokio::test]
    async fn test_rewrite_imports_same_crate_file_move() {
        let plugin = RustPlugin::default();
        let project_root = Path::new("/workspace");

        // Source file with import from common::utils
        let source = r#"use common::utils::calculate;

pub fn process(x: i32) -> i32 {
    calculate(x)
}"#;

        // Simulate moving common/src/utils.rs → common/src/helpers.rs
        let old_path = Path::new("/workspace/common/src/utils.rs");
        let new_path = Path::new("/workspace/common/src/helpers.rs");
        let current_file = Path::new("/workspace/common/src/processor.rs");

        let result = plugin
            .rewrite_imports_for_rename(source, old_path, new_path, current_file, project_root, None)
            .unwrap();

        let (new_content, count) = result;

        // Should have changed 1 import
        assert_eq!(count, 1, "Expected 1 import to be updated");

        // Verify import was rewritten from common::utils to common::helpers
        assert!(
            new_content.contains("use common::helpers::calculate"),
            "Import should be updated to common::helpers::calculate"
        );
        assert!(
            !new_content.contains("use common::utils"),
            "Old import with common::utils should be gone"
        );

        // Verify function body unchanged
        assert!(new_content.contains("calculate(x)"));
    }
}
