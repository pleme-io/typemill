//! Language-specific adapters for multi-language code operations
//!
//! Provides a trait-based abstraction for language-specific operations including:
//! - File extension handling
//! - Import statement rewriting
//! - Package manifest generation
//! - Module file location
//! - Import dependency parsing

use crate::error::{AstError, AstResult};
use crate::import_updater::ImportPathResolver;
use async_trait::async_trait;
use cb_core::language::ProjectLanguage;
use std::path::Path;

/// Language-specific adapter for package extraction operations
///
/// This trait abstracts language-specific operations needed for extracting
/// modules to packages, enabling support for multiple programming languages.
#[async_trait]
pub trait LanguageAdapter: Send + Sync {
    /// Get the language this adapter supports
    fn language(&self) -> ProjectLanguage;

    /// Get the package manifest filename (e.g., "Cargo.toml", "package.json")
    fn manifest_filename(&self) -> &'static str;

    /// Get the source directory name (e.g., "src" for Rust/TS, "" for Python)
    fn source_dir(&self) -> &'static str;

    /// Get the entry point filename (e.g., "lib.rs", "index.ts", "__init__.py")
    fn entry_point(&self) -> &'static str;

    /// Get the module path separator (e.g., "::" for Rust, "." for Python/TS)
    fn module_separator(&self) -> &'static str;

    /// Locate module files given a module path within a package
    ///
    /// # Arguments
    ///
    /// * `package_path` - Path to the source package
    /// * `module_path` - Dotted module path (e.g., "services.planner")
    ///
    /// # Returns
    ///
    /// Vector of file paths that comprise the module
    async fn locate_module_files(
        &self,
        package_path: &Path,
        module_path: &str,
    ) -> AstResult<Vec<std::path::PathBuf>>;

    /// Parse imports/dependencies from a file
    ///
    /// # Arguments
    ///
    /// * `file_path` - Path to the file to analyze
    ///
    /// # Returns
    ///
    /// Vector of import statements/paths found in the file
    async fn parse_imports(&self, file_path: &Path) -> AstResult<Vec<String>>;

    /// Generate a package manifest for a new package
    ///
    /// # Arguments
    ///
    /// * `package_name` - Name of the new package
    /// * `dependencies` - List of dependencies the package needs
    ///
    /// # Returns
    ///
    /// String containing the manifest file content
    fn generate_manifest(&self, package_name: &str, dependencies: &[String]) -> String;

    /// Update an import statement from internal to external
    ///
    /// # Arguments
    ///
    /// * `old_import` - Original import path (e.g., "crate::services::planner")
    /// * `new_package_name` - New package name (e.g., "cb_planner")
    ///
    /// # Returns
    ///
    /// Updated import statement
    fn rewrite_import(&self, old_import: &str, new_package_name: &str) -> String;

    /// Check if this adapter handles the given file extension
    ///
    /// # Arguments
    ///
    /// * `ext` - File extension without the dot (e.g., "rs", "ts", "py")
    ///
    /// # Returns
    ///
    /// true if this adapter handles files with this extension
    fn handles_extension(&self, ext: &str) -> bool;

    /// Rewrite import statements in file content for a rename operation
    ///
    /// # Arguments
    ///
    /// * `content` - The file content to process
    /// * `old_path` - Original path before rename
    /// * `new_path` - New path after rename
    /// * `importing_file` - Path of the file being processed
    /// * `project_root` - Root directory of the project
    /// * `rename_info` - Optional language-specific rename context (JSON)
    ///
    /// # Returns
    ///
    /// Tuple of (updated_content, number_of_changes)
    fn rewrite_imports_for_rename(
        &self,
        content: &str,
        old_path: &Path,
        new_path: &Path,
        importing_file: &Path,
        project_root: &Path,
        rename_info: Option<&serde_json::Value>,
    ) -> AstResult<(String, usize)>;
}

/// Rust language adapter
pub struct RustAdapter;

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
        use tracing::debug;

        debug!(
            package_path = %package_path.display(),
            module_path = %module_path,
            "Locating Rust module files"
        );

        // Start at the crate's source root (e.g., package_path/src)
        let src_root = package_path.join(self.source_dir());

        if !src_root.exists() {
            return Err(crate::error::AstError::Analysis {
                message: format!("Source directory not found: {}", src_root.display()),
            });
        }

        // Split module path by either "::" or "." into segments
        let segments: Vec<&str> = module_path
            .split(|c| c == ':' || c == '.')
            .filter(|s| !s.is_empty())
            .collect();

        if segments.is_empty() {
            return Err(crate::error::AstError::Analysis {
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
            return Err(crate::error::AstError::Analysis {
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
        use std::collections::HashSet;
        use tracing::debug;

        debug!(
            file_path = %file_path.display(),
            "Parsing Rust imports"
        );

        // Read the file content
        let content = tokio::fs::read_to_string(file_path).await.map_err(|e| {
            crate::error::AstError::Analysis {
                message: format!("Failed to read file {}: {}", file_path.display(), e),
            }
        })?;

        // Parse imports using the refactored AST parser
        let import_infos = crate::rust_parser::parse_rust_imports_ast(&content)?;

        // Extract unique external crate names
        let mut dependencies = HashSet::new();

        for import_info in import_infos {
            // Split the module path by "::" to get segments
            let segments: Vec<&str> = import_info.module_path.split("::").collect();

            if let Some(first_segment) = segments.first() {
                // Filter out internal imports (crate, self, super)
                if *first_segment != "crate" && *first_segment != "self" && *first_segment != "super" {
                    // This is an external crate dependency
                    dependencies.insert(first_segment.to_string());
                }
            }
        }

        // Convert HashSet to sorted Vec for consistent output
        let mut result: Vec<String> = dependencies.into_iter().collect();
        result.sort();

        debug!(
            dependencies_count = result.len(),
            "Extracted external dependencies"
        );

        Ok(result)
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
        use syn::{File, Item};

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

        tracing::debug!(
            old_crate = %old_crate_name,
            new_crate = %new_crate_name,
            "Rewriting Rust imports for crate rename"
        );

        // Parse the Rust source file
        let mut file: File = syn::parse_str(content).map_err(|e| {
            AstError::analysis(format!("Failed to parse Rust source: {}", e))
        })?;

        let mut changes_count = 0;

        // Iterate through all items and rewrite use statements
        for item in &mut file.items {
            if let Item::Use(use_item) = item {
                // Try to rewrite the use tree
                if let Some(new_tree) = crate::rust_parser::rewrite_use_tree(
                    &use_item.tree,
                    old_crate_name,
                    new_crate_name,
                ) {
                    use_item.tree = new_tree;
                    changes_count += 1;
                }
            }
        }

        // If no changes were made, return original content
        if changes_count == 0 {
            return Ok((content.to_string(), 0));
        }

        // Use prettyplease to format the modified AST
        let new_content = prettyplease::unparse(&file);

        tracing::debug!(
            changes = changes_count,
            "Successfully rewrote Rust imports"
        );

        Ok((new_content, changes_count))
    }
}

/// TypeScript/JavaScript language adapter
pub struct TypeScriptAdapter;

#[async_trait]
impl LanguageAdapter for TypeScriptAdapter {
    fn language(&self) -> ProjectLanguage {
        ProjectLanguage::TypeScript
    }

    fn manifest_filename(&self) -> &'static str {
        "package.json"
    }

    fn source_dir(&self) -> &'static str {
        "src"
    }

    fn entry_point(&self) -> &'static str {
        "index.ts"
    }

    fn module_separator(&self) -> &'static str {
        "."
    }

    async fn locate_module_files(
        &self,
        _package_path: &Path,
        _module_path: &str,
    ) -> AstResult<Vec<std::path::PathBuf>> {
        unimplemented!("TypeScriptAdapter::locate_module_files not yet implemented")
    }

    async fn parse_imports(&self, _file_path: &Path) -> AstResult<Vec<String>> {
        unimplemented!("TypeScriptAdapter::parse_imports not yet implemented")
    }

    fn generate_manifest(&self, _package_name: &str, _dependencies: &[String]) -> String {
        unimplemented!("TypeScriptAdapter::generate_manifest not yet implemented")
    }

    fn rewrite_import(&self, _old_import: &str, _new_package_name: &str) -> String {
        unimplemented!("TypeScriptAdapter::rewrite_import not yet implemented")
    }

    fn handles_extension(&self, ext: &str) -> bool {
        matches!(ext, "ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs")
    }

    fn rewrite_imports_for_rename(
        &self,
        content: &str,
        old_path: &Path,
        new_path: &Path,
        importing_file: &Path,
        project_root: &Path,
        _rename_info: Option<&serde_json::Value>,
    ) -> AstResult<(String, usize)> {
        let resolver = ImportPathResolver::new(project_root);
        let mut updated_content = String::new();
        let mut updates_count = 0;

        let old_target_stem = old_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("");

        for line in content.lines() {
            if line.contains("import") || line.contains("require") {
                if line.contains(old_target_stem) {
                    // This line likely contains an import that needs updating
                    if let Some(updated_line) =
                        update_import_line_ts(line, importing_file, old_path, new_path, &resolver)
                    {
                        updated_content.push_str(&updated_line);
                        updates_count += 1;
                    } else {
                        updated_content.push_str(line);
                    }
                } else {
                    updated_content.push_str(line);
                }
            } else {
                updated_content.push_str(line);
            }
            updated_content.push('\n');
        }

        Ok((updated_content.trim_end().to_string(), updates_count))
    }
}

/// Python language adapter
pub struct PythonAdapter;

#[async_trait]
impl LanguageAdapter for PythonAdapter {
    fn language(&self) -> ProjectLanguage {
        ProjectLanguage::Python
    }

    fn manifest_filename(&self) -> &'static str {
        "pyproject.toml"
    }

    fn source_dir(&self) -> &'static str {
        ""
    }

    fn entry_point(&self) -> &'static str {
        "__init__.py"
    }

    fn module_separator(&self) -> &'static str {
        "."
    }

    async fn locate_module_files(
        &self,
        _package_path: &Path,
        _module_path: &str,
    ) -> AstResult<Vec<std::path::PathBuf>> {
        unimplemented!("PythonAdapter::locate_module_files not yet implemented")
    }

    async fn parse_imports(&self, _file_path: &Path) -> AstResult<Vec<String>> {
        unimplemented!("PythonAdapter::parse_imports not yet implemented")
    }

    fn generate_manifest(&self, _package_name: &str, _dependencies: &[String]) -> String {
        unimplemented!("PythonAdapter::generate_manifest not yet implemented")
    }

    fn rewrite_import(&self, _old_import: &str, _new_package_name: &str) -> String {
        unimplemented!("PythonAdapter::rewrite_import not yet implemented")
    }

    fn handles_extension(&self, ext: &str) -> bool {
        matches!(ext, "py")
    }

    fn rewrite_imports_for_rename(
        &self,
        content: &str,
        _old_path: &Path,
        _new_path: &Path,
        _importing_file: &Path,
        _project_root: &Path,
        _rename_info: Option<&serde_json::Value>,
    ) -> AstResult<(String, usize)> {
        // Python import rewriting not yet implemented
        Ok((content.to_string(), 0))
    }
}

/// Helper function to update a single import line for TypeScript/JavaScript
fn update_import_line_ts(
    line: &str,
    importing_file: &Path,
    old_target: &Path,
    new_target: &Path,
    resolver: &ImportPathResolver,
) -> Option<String> {
    use crate::import_updater::extract_import_path;

    // Extract the import path from the line
    let import_path = extract_import_path(line)?;

    // Calculate the new import path
    if let Ok(new_import_path) =
        resolver.calculate_new_import_path(importing_file, old_target, new_target, &import_path)
    {
        // Replace the old import path with the new one
        Some(line.replace(&import_path, &new_import_path))
    } else {
        None
    }
}
