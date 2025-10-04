//! Rust Language Adapter for Refactoring Operations
//!
//! This crate provides the `RustAdapter` which implements the `LanguageAdapter` trait
//! for Rust-specific refactoring operations. It composes the `cb-lang-rust` intelligence
//! plugin to leverage AST parsing capabilities.
//!
//! # Architecture
//!
//! The adapter pattern separates concerns:
//! - **Intelligence** (`cb-lang-rust`): Pure AST parsing, symbol extraction
//! - **Adapter** (this crate): Refactoring operations, import rewriting, file operations
//!
//! This separation allows the intelligence layer to remain pure and reusable across
//! different tools while the adapter handles project-specific refactoring logic.

use async_trait::async_trait;
use cb_ast::error::{AstError, AstResult};
use cb_ast::language::{LanguageAdapter, ModuleReference, ReferenceKind, ScanScope};
use cb_core::language::ProjectLanguage;
use cb_plugin_api::LanguageIntelligencePlugin;
use std::path::Path;
use std::sync::Arc;
use tracing::debug;

/// Rust language adapter for refactoring operations
///
/// Composes a `LanguageIntelligencePlugin` to provide refactoring capabilities
/// built on top of Rust AST parsing.
pub struct RustAdapter {
    /// The intelligence plugin used for AST parsing
    ///
    /// Currently, most operations use direct calls to `cb_lang_rust` functions,
    /// but this field enables future enhancements to use the trait-based API.
    #[allow(dead_code)]
    intelligence: Arc<dyn LanguageIntelligencePlugin>,
}

impl RustAdapter {
    /// Create a new Rust adapter with the given intelligence plugin
    ///
    /// # Arguments
    ///
    /// * `intelligence` - The intelligence plugin to use for AST parsing
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use cb_lang_rust::RustPlugin;
    /// use cb_lang_rust_adapter::RustAdapter;
    /// use std::sync::Arc;
    ///
    /// let intelligence = Arc::new(RustPlugin::new());
    /// let adapter = RustAdapter::new(intelligence);
    /// ```
    pub fn new(intelligence: Arc<dyn LanguageIntelligencePlugin>) -> Self {
        Self { intelligence }
    }

    /// Create a new Rust adapter with the default intelligence plugin
    ///
    /// This is a convenience method that creates a `RustPlugin` internally.
    pub fn default_intelligence() -> Self {
        Self::new(Arc::new(cb_lang_rust::RustPlugin::new()))
    }
}

impl Default for RustAdapter {
    fn default() -> Self {
        Self::default_intelligence()
    }
}

#[async_trait]
impl LanguageAdapter for RustAdapter {
    fn language(&self) -> ProjectLanguage {
        ProjectLanguage::Rust
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
    ) -> AstResult<Vec<std::path::PathBuf>> {
        debug!(
            package_path = %package_path.display(),
            module_path = %module_path,
            "Locating Rust module files"
        );

        // Start at the crate's source root (e.g., package_path/src)
        let src_root = package_path.join(self.source_dir());

        if !src_root.exists() {
            return Err(AstError::Analysis {
                message: format!("Source directory not found: {}", src_root.display()),
            });
        }

        // Split module path by either "::" or "." into segments
        let segments: Vec<&str> = module_path
            .split([':', '.'])
            .filter(|s| !s.is_empty())
            .collect();

        if segments.is_empty() {
            return Err(AstError::Analysis {
                message: "Module path cannot be empty".to_string(),
            });
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
            return Err(AstError::Analysis {
                message: format!(
                    "Module '{}' not found at {} (checked both {}.rs and {}/mod.rs)",
                    module_path,
                    current_path.display(),
                    final_segment,
                    final_segment
                ),
            });
        }

        debug!(
            files_count = found_files.len(),
            "Successfully located module files"
        );

        Ok(found_files)
    }

    async fn parse_imports(&self, file_path: &Path) -> AstResult<Vec<String>> {
        debug!(
            file_path = %file_path.display(),
            "Parsing Rust imports using intelligence plugin"
        );

        // Read file content
        let content = tokio::fs::read_to_string(file_path)
            .await
            .map_err(|e| AstError::Analysis {
                message: format!("Failed to read file {}: {}", file_path.display(), e),
            })?;

        // Use the intelligence plugin to parse
        let imports = cb_lang_rust::parse_imports(&content)
            .map_err(|e| AstError::Analysis {
                message: format!("Failed to parse imports: {}", e),
            })?;

        // Extract module paths from import info
        let module_paths: Vec<String> = imports
            .iter()
            .map(|imp| imp.module_path.clone())
            .collect();

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

    fn handles_extension(&self, ext: &str) -> bool {
        matches!(ext, "rs")
    }

    fn rewrite_imports_for_rename(
        &self,
        content: &str,
        _old_path: &Path,
        _new_path: &Path,
        _importing_file: &Path,
        _project_root: &Path,
        rename_info: Option<&serde_json::Value>,
    ) -> AstResult<(String, usize)> {
        // If no rename_info provided, no rewriting needed
        let rename_info = match rename_info {
            Some(info) => info,
            None => return Ok((content.to_string(), 0)),
        };

        // Extract old and new crate names from rename_info
        let old_crate_name = rename_info["old_crate_name"]
            .as_str()
            .ok_or_else(|| AstError::analysis("Missing old_crate_name in rename_info"))?;

        let new_crate_name = rename_info["new_crate_name"]
            .as_str()
            .ok_or_else(|| AstError::analysis("Missing new_crate_name in rename_info"))?;

        debug!(
            old_crate = %old_crate_name,
            new_crate = %new_crate_name,
            "Rewriting Rust imports for crate rename"
        );

        // Use cb-lang-rust to parse and rewrite
        let _imports = cb_lang_rust::parse_imports(content)
            .map_err(|e| AstError::analysis(format!("Failed to parse imports: {}", e)))?;

        // For now, use simple string replacement
        // TODO: Use cb-lang-rust::rewrite_use_tree for proper AST-based rewriting
        let mut updated_content = content.to_string();
        let mut changes_count = 0;

        for line in content.lines() {
            if line.contains("use ") && line.contains(old_crate_name) {
                changes_count += 1;
            }
        }

        updated_content = updated_content.replace(old_crate_name, new_crate_name);

        debug!(changes = changes_count, "Rewrote Rust imports");

        Ok((updated_content, changes_count))
    }

    fn find_module_references(
        &self,
        content: &str,
        module_to_find: &str,
        scope: ScanScope,
    ) -> AstResult<Vec<ModuleReference>> {
        use syn::visit::Visit;
        use syn::File;

        // Parse the Rust source file
        let file: File = syn::parse_str(content)
            .map_err(|e| AstError::analysis(format!("Failed to parse Rust source: {}", e)))?;

        // Create and run visitor
        let mut finder = RustModuleFinder::new(module_to_find, scope);
        finder.visit_file(&file);

        Ok(finder.into_references())
    }
}

/// Visitor for finding module references in Rust code using syn::visit
struct RustModuleFinder<'a> {
    module_to_find: &'a str,
    scope: ScanScope,
    references: Vec<ModuleReference>,
}

impl<'a> RustModuleFinder<'a> {
    fn new(module_to_find: &'a str, scope: ScanScope) -> Self {
        Self {
            module_to_find,
            scope,
            references: Vec::new(),
        }
    }

    fn into_references(self) -> Vec<ModuleReference> {
        self.references
    }

    fn check_use_tree(&mut self, tree: &syn::UseTree) {
        match tree {
            syn::UseTree::Path(path) => {
                if path.ident == self.module_to_find {
                    self.references.push(ModuleReference {
                        line: 0,
                        column: 0,
                        length: self.module_to_find.len(),
                        text: self.module_to_find.to_string(),
                        kind: ReferenceKind::Declaration,
                    });
                }
                self.check_use_tree(&path.tree);
            }
            syn::UseTree::Name(name) => {
                if name.ident == self.module_to_find {
                    self.references.push(ModuleReference {
                        line: 0,
                        column: 0,
                        length: self.module_to_find.len(),
                        text: self.module_to_find.to_string(),
                        kind: ReferenceKind::Declaration,
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
        if self.scope == ScanScope::QualifiedPaths || self.scope == ScanScope::All {
            if let Some(segment) = node.path.segments.first() {
                if segment.ident == self.module_to_find {
                    self.references.push(ModuleReference {
                        line: 0,
                        column: 0,
                        length: self.module_to_find.len(),
                        text: quote::quote!(#node).to_string(),
                        kind: ReferenceKind::QualifiedPath,
                    });
                }
            }
        }

        // Continue visiting
        syn::visit::visit_expr_path(self, node);
    }

    fn visit_expr_lit(&mut self, node: &'ast syn::ExprLit) {
        // Only check string literals if scope is All
        if self.scope == ScanScope::All {
            if let syn::Lit::Str(lit_str) = &node.lit {
                let value = lit_str.value();
                if value.contains(self.module_to_find) {
                    self.references.push(ModuleReference {
                        line: 0,
                        column: 0,
                        length: self.module_to_find.len(),
                        text: value,
                        kind: ReferenceKind::StringLiteral,
                    });
                }
            }
        }

        // Continue visiting
        syn::visit::visit_expr_lit(self, node);
    }
}
