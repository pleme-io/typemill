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

/// Defines the scope of the import/reference scan
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanScope {
    /// Only find top-level `import`/`use` statements
    TopLevelOnly,
    /// Find all `use` or `import` statements, including those inside functions
    AllUseStatements,
    /// Find all `use` statements and qualified paths (e.g., `my_module::MyStruct`)
    QualifiedPaths,
    /// Find all references, including string literals (requires confirmation)
    All,
}

/// Represents a found reference to a module within a source file
#[derive(Debug, Clone)]
pub struct ModuleReference {
    /// Line number (1-indexed)
    pub line: usize,
    /// Column number (0-indexed)
    pub column: usize,
    /// Length of the reference in characters
    pub length: usize,
    /// The actual text that was found
    pub text: String,
    /// The type of reference
    pub kind: ReferenceKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReferenceKind {
    /// An `import` or `export` or `use` declaration
    Declaration,
    /// A qualified path (e.g., `my_module.MyStruct` or `my_module::function`)
    QualifiedPath,
    /// A reference inside a string literal
    StringLiteral,
}

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

    /// Find all references to a specific module within file content
    ///
    /// This is more powerful than `parse_imports` as it finds not just declarations,
    /// but also qualified paths and other usages within the code based on the scope.
    ///
    /// # Arguments
    ///
    /// * `content` - The file content to scan
    /// * `module_to_find` - The module name/path to search for
    /// * `scope` - The scope of the search (top-level only, all statements, qualified paths, etc.)
    ///
    /// # Returns
    ///
    /// Vector of all found references with their locations
    fn find_module_references(
        &self,
        content: &str,
        module_to_find: &str,
        scope: ScanScope,
    ) -> AstResult<Vec<ModuleReference>>;
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
            .split([':', '.'])
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
                if *first_segment != "crate"
                    && *first_segment != "self"
                    && *first_segment != "super"
                {
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
        let mut file: File = syn::parse_str(content)
            .map_err(|e| AstError::analysis(format!("Failed to parse Rust source: {}", e)))?;

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

        tracing::debug!(changes = changes_count, "Successfully rewrote Rust imports");

        Ok((new_content, changes_count))
    }

    fn find_module_references(
        &self,
        content: &str,
        module_to_find: &str,
        scope: ScanScope,
    ) -> AstResult<Vec<ModuleReference>> {
        use syn::{File, Item, UseTree};

        // Parse the Rust source file
        let file: File = syn::parse_str(content)
            .map_err(|e| AstError::analysis(format!("Failed to parse Rust source: {}", e)))?;

        let mut references = Vec::new();

        // Helper to find module name in a use tree
        fn find_in_use_tree(
            tree: &UseTree,
            module_to_find: &str,
            references: &mut Vec<ModuleReference>,
        ) {
            match tree {
                UseTree::Path(path) => {
                    if path.ident == module_to_find {
                        references.push(ModuleReference {
                            line: 0,
                            column: 0,
                            length: module_to_find.len(),
                            text: module_to_find.to_string(),
                            kind: ReferenceKind::Declaration,
                        });
                    }
                    find_in_use_tree(&path.tree, module_to_find, references);
                }
                UseTree::Name(name) => {
                    if name.ident == module_to_find {
                        references.push(ModuleReference {
                            line: 0,
                            column: 0,
                            length: module_to_find.len(),
                            text: module_to_find.to_string(),
                            kind: ReferenceKind::Declaration,
                        });
                    }
                }
                UseTree::Group(group) => {
                    for item in &group.items {
                        find_in_use_tree(item, module_to_find, references);
                    }
                }
                _ => {}
            }
        }

        // Helper to recursively search items
        fn search_items(
            items: &[Item],
            module_to_find: &str,
            scope: ScanScope,
            references: &mut Vec<ModuleReference>,
            depth: usize,
        ) {
            for item in items {
                match item {
                    Item::Use(use_item) => {
                        // For TopLevelOnly, only process if depth == 0
                        if scope == ScanScope::TopLevelOnly && depth > 0 {
                            continue;
                        }
                        find_in_use_tree(&use_item.tree, module_to_find, references);
                    }
                    Item::Fn(func) if scope != ScanScope::TopLevelOnly => {
                        // Search for use statements inside functions
                        for stmt in &func.block.stmts {
                            if let syn::Stmt::Item(Item::Use(use_item)) = stmt {
                                find_in_use_tree(&use_item.tree, module_to_find, references);
                            }
                        }
                    }
                    Item::Mod(module) if scope != ScanScope::TopLevelOnly => {
                        if let Some((_, items)) = &module.content {
                            search_items(items, module_to_find, scope, references, depth + 1);
                        }
                    }
                    _ => {}
                }
            }
        }

        // Search top-level and nested items
        search_items(&file.items, module_to_find, scope, &mut references, 0);

        // For QualifiedPaths and All scopes, use simple string matching as fallback
        // TODO: Implement proper AST-based path finding without syn::visit to avoid compiler issues
        if scope == ScanScope::QualifiedPaths || scope == ScanScope::All {
            // Simple regex-based search for qualified paths like "module::function"
            let pattern = format!(r"\b{}\s*::", regex::escape(module_to_find));
            if let Ok(re) = regex::Regex::new(&pattern) {
                for (line_num, line) in content.lines().enumerate() {
                    if re.is_match(line) {
                        references.push(ModuleReference {
                            line: line_num,
                            column: 0,
                            length: module_to_find.len(),
                            text: line.trim().to_string(),
                            kind: ReferenceKind::QualifiedPath,
                        });
                    }
                }
            }
        }

        // For All scope, search string literals using regex
        if scope == ScanScope::All {
            let pattern = format!(r#""[^"]*{}[^"]*""#, regex::escape(module_to_find));
            if let Ok(re) = regex::Regex::new(&pattern) {
                for (line_num, line) in content.lines().enumerate() {
                    if re.is_match(line) {
                        references.push(ModuleReference {
                            line: line_num,
                            column: 0,
                            length: module_to_find.len(),
                            text: line.trim().to_string(),
                            kind: ReferenceKind::StringLiteral,
                        });
                    }
                }
            }
        }

        Ok(references)
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

        let old_target_stem = old_path.file_stem().and_then(|s| s.to_str()).unwrap_or("");

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

    fn find_module_references(
        &self,
        content: &str,
        module_to_find: &str,
        scope: ScanScope,
    ) -> AstResult<Vec<ModuleReference>> {
        use swc_common::sync::Lrc;
        use swc_common::{FileName, FilePathMapping, SourceMap};
        use swc_ecma_ast::EsVersion;
        use swc_ecma_parser::{lexer::Lexer, Parser, StringInput, Syntax, TsSyntax};
        use swc_ecma_visit::VisitWith;

        // 1. Setup SWC parser
        let cm = Lrc::new(SourceMap::new(FilePathMapping::empty()));
        let fm = cm.new_source_file(Lrc::new(FileName::Anon), content.to_string());
        let lexer = Lexer::new(
            Syntax::Typescript(TsSyntax {
                tsx: false,
                decorators: false,
                ..Default::default()
            }),
            EsVersion::latest(),
            StringInput::from(&*fm),
            None,
        );
        let mut parser = Parser::new_from(lexer);

        let module = match parser.parse_module() {
            Ok(m) => m,
            Err(_e) => {
                // If parsing fails, return empty vec
                return Ok(Vec::new());
            }
        };

        // 2. Create and run the visitor
        let mut visitor = TsModuleVisitor {
            module_to_find,
            references: Vec::new(),
            scope,
            source_map: &cm,
            source_file: &fm,
        };
        module.visit_with(&mut visitor);

        Ok(visitor.references)
    }
}

/// Visitor for finding module references in TypeScript/JavaScript AST
struct TsModuleVisitor<'a> {
    module_to_find: &'a str,
    references: Vec<ModuleReference>,
    scope: ScanScope,
    source_map: &'a swc_common::SourceMap,
    source_file: &'a swc_common::SourceFile,
}

impl<'a> swc_ecma_visit::Visit for TsModuleVisitor<'a> {
    fn visit_import_decl(&mut self, import: &swc_ecma_ast::ImportDecl) {
        // Only process imports if scope allows
        if self.scope == ScanScope::TopLevelOnly
            || self.scope == ScanScope::AllUseStatements
            || self.scope == ScanScope::QualifiedPaths
            || self.scope == ScanScope::All
        {
            if let Some(src_str) = import.src.raw.as_ref() {
                if src_str.contains(self.module_to_find) {
                    let span = import.src.span;
                    if let Some(reference) = self.span_to_reference(span, ReferenceKind::Declaration) {
                        self.references.push(reference);
                    }
                }
            }
        }
    }

    fn visit_member_expr(&mut self, member_expr: &swc_ecma_ast::MemberExpr) {
        // Only process qualified paths if scope allows
        if self.scope == ScanScope::QualifiedPaths || self.scope == ScanScope::All {
            if let Some(ident) = member_expr.obj.as_ident() {
                if ident.sym.as_ref() == self.module_to_find {
                    let span = member_expr.span;
                    if let Some(reference) = self.span_to_reference(span, ReferenceKind::QualifiedPath) {
                        self.references.push(reference);
                    }
                }
            }
        }
    }
}

impl<'a> TsModuleVisitor<'a> {
    /// Convert a SWC span to a ModuleReference with line/column information
    fn span_to_reference(&self, span: swc_common::Span, kind: ReferenceKind) -> Option<ModuleReference> {
        let lo = self.source_map.lookup_char_pos(span.lo);

        // Extract the actual text from the span
        let start = (span.lo.0 - self.source_file.start_pos.0) as usize;
        let end = (span.hi.0 - self.source_file.start_pos.0) as usize;
        let text = self.source_file.src.get(start..end)?.to_string();

        Some(ModuleReference {
            line: lo.line,
            column: lo.col.0,
            length: (span.hi.0 - span.lo.0) as usize,
            text,
            kind,
        })
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

    fn find_module_references(
        &self,
        _content: &str,
        _module_to_find: &str,
        _scope: ScanScope,
    ) -> AstResult<Vec<ModuleReference>> {
        // TODO: Implement Python AST-based reference finding
        Ok(Vec::new())
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

/// Go language adapter
pub struct GoAdapter;

#[async_trait]
impl LanguageAdapter for GoAdapter {
    fn language(&self) -> ProjectLanguage {
        ProjectLanguage::Go
    }

    fn manifest_filename(&self) -> &'static str {
        "go.mod"
    }

    fn source_dir(&self) -> &'static str {
        "" // Go projects don't have a standard source directory
    }

    fn entry_point(&self) -> &'static str {
        "" // Go doesn't have a single entry point file
    }

    fn module_separator(&self) -> &'static str {
        "/"
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
            "Locating Go module files"
        );

        // Go modules are directories containing .go files
        // module_path format: "internal/utils" or "pkg/service"

        let module_dir = package_path.join(module_path);

        if !module_dir.exists() || !module_dir.is_dir() {
            return Err(AstError::Analysis {
                message: format!("Module directory not found: {}", module_dir.display()),
            });
        }

        // Find all .go files in the directory (non-recursive)
        let mut go_files = Vec::new();

        let mut entries =
            tokio::fs::read_dir(&module_dir)
                .await
                .map_err(|e| AstError::Analysis {
                    message: format!("Failed to read directory {}: {}", module_dir.display(), e),
                })?;

        while let Some(entry) = entries.next_entry().await.map_err(|e| AstError::Analysis {
            message: format!("Error reading directory entry: {}", e),
        })? {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == "go" {
                        // Exclude test files
                        if let Some(file_stem) = path.file_stem() {
                            if !file_stem.to_string_lossy().ends_with("_test") {
                                debug!(file_path = %path.display(), "Found Go file");
                                go_files.push(path);
                            }
                        }
                    }
                }
            }
        }

        if go_files.is_empty() {
            return Err(AstError::Analysis {
                message: format!("No .go files found in module: {}", module_dir.display()),
            });
        }

        debug!(
            files_count = go_files.len(),
            "Successfully located Go module files"
        );

        Ok(go_files)
    }

    async fn parse_imports(&self, file_path: &Path) -> AstResult<Vec<String>> {
        use std::collections::HashSet;
        use tracing::debug;

        debug!(
            file_path = %file_path.display(),
            "Parsing Go imports"
        );

        // Read file content
        let content =
            tokio::fs::read_to_string(file_path)
                .await
                .map_err(|e| AstError::Analysis {
                    message: format!("Failed to read file {}: {}", file_path.display(), e),
                })?;

        // Use existing build_import_graph which calls parse_go_imports_ast
        let import_graph = crate::parser::build_import_graph(&content, file_path)?;

        // Extract unique import paths
        let mut dependencies = HashSet::new();

        for import_info in import_graph.imports {
            // Go imports are full package paths
            // Filter out standard library (no dots in path typically means stdlib)
            // External packages usually have domain names: "github.com/user/repo"
            if import_info.module_path.contains('.') {
                dependencies.insert(import_info.module_path);
            }
        }

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

        // module declaration
        writeln!(manifest, "module {}", package_name).unwrap();
        writeln!(manifest).unwrap();
        writeln!(manifest, "go 1.21").unwrap();

        // Add dependencies if any
        if !dependencies.is_empty() {
            writeln!(manifest).unwrap();
            writeln!(manifest, "require (").unwrap();
            for dep in dependencies {
                writeln!(manifest, "\t{} v0.0.0", dep).unwrap();
            }
            writeln!(manifest, ")").unwrap();
        }

        manifest
    }

    fn rewrite_import(&self, _old_import: &str, new_package_name: &str) -> String {
        // Transform internal import to external module import
        // e.g., "github.com/user/project/internal/utils" -> "github.com/user/new-package"
        new_package_name.to_string()
    }

    fn handles_extension(&self, ext: &str) -> bool {
        matches!(ext, "go")
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

        // Extract old and new module paths from rename_info
        let old_module = rename_info["old_module_path"]
            .as_str()
            .ok_or_else(|| AstError::analysis("Missing old_module_path in rename_info"))?;

        let new_module = rename_info["new_module_path"]
            .as_str()
            .ok_or_else(|| AstError::analysis("Missing new_module_path in rename_info"))?;

        tracing::debug!(
            old_module = %old_module,
            new_module = %new_module,
            "Rewriting Go imports for module rename"
        );

        // Simple line-by-line replacement for Go imports
        // Go import format: import "module/path" or import ( ... )
        let mut updated_content = String::new();
        let mut changes_count = 0;

        for line in content.lines() {
            if line.contains("import") && line.contains(old_module) {
                let updated_line = line.replace(old_module, new_module);
                updated_content.push_str(&updated_line);
                changes_count += 1;
            } else {
                updated_content.push_str(line);
            }
            updated_content.push('\n');
        }

        tracing::debug!(changes = changes_count, "Successfully rewrote Go imports");

        Ok((updated_content.trim_end().to_string(), changes_count))
    }

    fn find_module_references(
        &self,
        _content: &str,
        _module_to_find: &str,
        _scope: ScanScope,
    ) -> AstResult<Vec<ModuleReference>> {
        // TODO: Implement Go AST-based reference finding
        Ok(Vec::new())
    }
}

/// Java language adapter
pub struct JavaAdapter;

#[async_trait]
impl LanguageAdapter for JavaAdapter {
    fn language(&self) -> ProjectLanguage {
        ProjectLanguage::Java
    }

    fn manifest_filename(&self) -> &'static str {
        "pom.xml" // Default to Maven, could also be build.gradle
    }

    fn source_dir(&self) -> &'static str {
        "src/main/java"
    }

    fn entry_point(&self) -> &'static str {
        "" // Java doesn't have a single entry point like Rust's lib.rs
    }

    fn module_separator(&self) -> &'static str {
        "."
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
            "Locating Java package files"
        );

        // Java packages map to directory structure
        // module_path format: "com.example.utils" -> src/main/java/com/example/utils/

        let src_root = package_path.join(self.source_dir());

        if !src_root.exists() {
            return Err(AstError::Analysis {
                message: format!("Source directory not found: {}", src_root.display()),
            });
        }

        // Convert dotted package name to path
        // "com.example.utils" -> "com/example/utils"
        let package_path_str = module_path.replace('.', "/");
        let package_dir = src_root.join(&package_path_str);

        if !package_dir.exists() || !package_dir.is_dir() {
            return Err(AstError::Analysis {
                message: format!("Package directory not found: {}", package_dir.display()),
            });
        }

        // Find all .java files in the package directory (non-recursive)
        let mut java_files = Vec::new();

        let mut entries =
            tokio::fs::read_dir(&package_dir)
                .await
                .map_err(|e| AstError::Analysis {
                    message: format!("Failed to read directory {}: {}", package_dir.display(), e),
                })?;

        while let Some(entry) = entries.next_entry().await.map_err(|e| AstError::Analysis {
            message: format!("Error reading directory entry: {}", e),
        })? {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == "java" {
                        debug!(file_path = %path.display(), "Found Java file");
                        java_files.push(path);
                    }
                }
            }
        }

        if java_files.is_empty() {
            return Err(AstError::Analysis {
                message: format!("No .java files found in package: {}", package_dir.display()),
            });
        }

        debug!(
            files_count = java_files.len(),
            "Successfully located Java package files"
        );

        Ok(java_files)
    }

    async fn parse_imports(&self, file_path: &Path) -> AstResult<Vec<String>> {
        use std::collections::HashSet;
        use tracing::debug;

        debug!(
            file_path = %file_path.display(),
            "Parsing Java imports"
        );

        // Read file content
        let content =
            tokio::fs::read_to_string(file_path)
                .await
                .map_err(|e| AstError::Analysis {
                    message: format!("Failed to read file {}: {}", file_path.display(), e),
                })?;

        // Parse Java imports using regex (simple but effective)
        // import com.example.utils.Helper;
        // import static com.example.Constants.*;
        let import_regex = regex::Regex::new(r#"^\s*import\s+(?:static\s+)?([a-zA-Z0-9_.]+)"#)
            .map_err(|e| AstError::analysis(format!("Regex compilation failed: {}", e)))?;

        let mut dependencies = HashSet::new();

        for line in content.lines() {
            if let Some(captures) = import_regex.captures(line) {
                if let Some(import_path) = captures.get(1) {
                    let full_import = import_path.as_str();

                    // Extract package name (everything except last segment)
                    // "com.example.utils.Helper" -> "com.example.utils"
                    if let Some(last_dot) = full_import.rfind('.') {
                        let package_name = &full_import[..last_dot];

                        // Filter out java.* and javax.* (standard library)
                        if !package_name.starts_with("java.") && !package_name.starts_with("javax.")
                        {
                            dependencies.insert(package_name.to_string());
                        }
                    }
                }
            }
        }

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

        // Generate basic pom.xml structure
        writeln!(manifest, "<?xml version=\"1.0\" encoding=\"UTF-8\"?>").unwrap();
        writeln!(
            manifest,
            "<project xmlns=\"http://maven.apache.org/POM/4.0.0\""
        )
        .unwrap();
        writeln!(
            manifest,
            "         xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\""
        )
        .unwrap();
        writeln!(manifest, "         xsi:schemaLocation=\"http://maven.apache.org/POM/4.0.0 http://maven.apache.org/xsd/maven-4.0.0.xsd\">").unwrap();
        writeln!(manifest, "    <modelVersion>4.0.0</modelVersion>").unwrap();
        writeln!(manifest).unwrap();

        // Extract group and artifact IDs from package_name
        // Assume format: "com.example.artifactid"
        let parts: Vec<&str> = package_name.rsplitn(2, '.').collect();
        let (artifact_id, group_id) = if parts.len() == 2 {
            (parts[0], parts[1])
        } else {
            (package_name, "com.example")
        };

        writeln!(manifest, "    <groupId>{}</groupId>", group_id).unwrap();
        writeln!(manifest, "    <artifactId>{}</artifactId>", artifact_id).unwrap();
        writeln!(manifest, "    <version>1.0.0</version>").unwrap();
        writeln!(manifest).unwrap();

        // Add dependencies if any
        if !dependencies.is_empty() {
            writeln!(manifest, "    <dependencies>").unwrap();
            for dep in dependencies {
                let dep_parts: Vec<&str> = dep.rsplitn(2, '.').collect();
                let (dep_artifact, dep_group) = if dep_parts.len() == 2 {
                    (dep_parts[0], dep_parts[1])
                } else {
                    (dep.as_str(), "com.example")
                };

                writeln!(manifest, "        <dependency>").unwrap();
                writeln!(manifest, "            <groupId>{}</groupId>", dep_group).unwrap();
                writeln!(
                    manifest,
                    "            <artifactId>{}</artifactId>",
                    dep_artifact
                )
                .unwrap();
                writeln!(manifest, "            <version>1.0.0</version>").unwrap();
                writeln!(manifest, "        </dependency>").unwrap();
            }
            writeln!(manifest, "    </dependencies>").unwrap();
        }

        writeln!(manifest, "</project>").unwrap();

        manifest
    }

    fn rewrite_import(&self, old_import: &str, new_package_name: &str) -> String {
        // Transform internal import to external package import
        // e.g., "com.example.project.internal.Utils" -> "com.example.newpackage.Utils"

        // Extract the class name (last segment)
        if let Some(last_dot) = old_import.rfind('.') {
            let class_name = &old_import[last_dot + 1..];
            format!("{}.{}", new_package_name, class_name)
        } else {
            // No dot, just use new package name
            new_package_name.to_string()
        }
    }

    fn handles_extension(&self, ext: &str) -> bool {
        matches!(ext, "java")
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

        // Extract old and new package names from rename_info
        let old_package = rename_info["old_package_name"]
            .as_str()
            .ok_or_else(|| AstError::analysis("Missing old_package_name in rename_info"))?;

        let new_package = rename_info["new_package_name"]
            .as_str()
            .ok_or_else(|| AstError::analysis("Missing new_package_name in rename_info"))?;

        tracing::debug!(
            old_package = %old_package,
            new_package = %new_package,
            "Rewriting Java imports for package rename"
        );

        // Simple line-by-line replacement for Java imports
        // Java import format: import com.example.package.ClassName;
        let mut updated_content = String::new();
        let mut changes_count = 0;

        for line in content.lines() {
            if line.trim().starts_with("import") && line.contains(old_package) {
                let updated_line = line.replace(old_package, new_package);
                updated_content.push_str(&updated_line);
                changes_count += 1;
            } else {
                updated_content.push_str(line);
            }
            updated_content.push('\n');
        }

        tracing::debug!(changes = changes_count, "Successfully rewrote Java imports");

        Ok((updated_content.trim_end().to_string(), changes_count))
    }

    fn find_module_references(
        &self,
        _content: &str,
        _module_to_find: &str,
        _scope: ScanScope,
    ) -> AstResult<Vec<ModuleReference>> {
        // TODO: Implement Java AST-based reference finding
        Ok(Vec::new())
    }
}
