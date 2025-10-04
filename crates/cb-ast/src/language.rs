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
                    if let Some(reference) =
                        self.span_to_reference(span, ReferenceKind::Declaration)
                    {
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
                    if let Some(reference) =
                        self.span_to_reference(span, ReferenceKind::QualifiedPath)
                    {
                        self.references.push(reference);
                    }
                }
            }
        }
    }
}

impl<'a> TsModuleVisitor<'a> {
    /// Convert a SWC span to a ModuleReference with line/column information
    fn span_to_reference(
        &self,
        span: swc_common::Span,
        kind: ReferenceKind,
    ) -> Option<ModuleReference> {
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
        content: &str,
        module_to_find: &str,
        scope: ScanScope,
    ) -> AstResult<Vec<ModuleReference>> {
        // Parse Python source code
        #[allow(deprecated)]
        let program = rustpython_parser::parse_program(content, "<string>")
            .map_err(|e| AstError::analysis(format!("Failed to parse Python source: {:?}", e)))?;

        // Create and run visitor
        let mut finder = PythonModuleFinder::new(module_to_find, scope);
        for stmt in &program {
            finder.visit_stmt(stmt);
        }

        Ok(finder.into_references())
    }
}

/// Visitor for finding module references in Python code
struct PythonModuleFinder<'a> {
    module_to_find: &'a str,
    scope: ScanScope,
    references: Vec<ModuleReference>,
}

impl<'a> PythonModuleFinder<'a> {
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

    fn visit_stmt(&mut self, stmt: &rustpython_parser::ast::Stmt) {
        use rustpython_parser::ast::Stmt;

        match stmt {
            Stmt::Import(import_stmt) => {
                // Handle: import module, import module as alias
                for alias in &import_stmt.names {
                    let module_name = alias.name.as_str();
                    if module_name == self.module_to_find
                        || module_name.starts_with(&format!("{}.", self.module_to_find))
                    {
                        self.references.push(ModuleReference {
                            line: 0,
                            column: 0,
                            length: self.module_to_find.len(),
                            text: module_name.to_string(),
                            kind: ReferenceKind::Declaration,
                        });
                    }
                }
            }
            Stmt::ImportFrom(import_from) => {
                // Handle: from module import ...
                if let Some(module) = &import_from.module {
                    let module_name = module.as_str();
                    if module_name == self.module_to_find
                        || module_name.starts_with(&format!("{}.", self.module_to_find))
                    {
                        self.references.push(ModuleReference {
                            line: 0,
                            column: 0,
                            length: self.module_to_find.len(),
                            text: module_name.to_string(),
                            kind: ReferenceKind::Declaration,
                        });
                    }
                }
            }
            Stmt::FunctionDef(func) => {
                // Recurse into function body for nested imports
                if self.scope != ScanScope::TopLevelOnly {
                    for stmt in &func.body {
                        self.visit_stmt(stmt);
                    }
                }
            }
            Stmt::ClassDef(class) => {
                // Recurse into class body
                if self.scope != ScanScope::TopLevelOnly {
                    for stmt in &class.body {
                        self.visit_stmt(stmt);
                    }
                }
            }
            _ => {}
        }

        // For QualifiedPaths and All scopes, check expressions
        if self.scope == ScanScope::QualifiedPaths || self.scope == ScanScope::All {
            if let Stmt::Expr(expr_stmt) = stmt {
                self.visit_expr(&expr_stmt.value);
            }
        }
    }

    fn visit_expr(&mut self, expr: &rustpython_parser::ast::Expr) {
        use rustpython_parser::ast::Expr;

        match expr {
            Expr::Attribute(attr) => {
                // Check for qualified paths like module.function()
                if let Expr::Name(name) = attr.value.as_ref() {
                    if name.id.as_str() == self.module_to_find {
                        self.references.push(ModuleReference {
                            line: 0,
                            column: 0,
                            length: self.module_to_find.len(),
                            text: format!("{}.{}", name.id, attr.attr),
                            kind: ReferenceKind::QualifiedPath,
                        });
                    }
                }
            }
            Expr::Constant(constant) if self.scope == ScanScope::All => {
                // Check string literals
                if let rustpython_parser::ast::Constant::Str(s) = &constant.value {
                    if s.contains(self.module_to_find) {
                        self.references.push(ModuleReference {
                            line: 0,
                            column: 0,
                            length: self.module_to_find.len(),
                            text: s.clone(),
                            kind: ReferenceKind::StringLiteral,
                        });
                    }
                }
            }
            _ => {}
        }
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
/// Visitor for finding module references in Go code using tree-sitter
struct GoModuleFinder<'a> {
    module_to_find: &'a str,
    scope: ScanScope,
    references: Vec<ModuleReference>,
    source: &'a str,
}

impl<'a> GoModuleFinder<'a> {
    fn new(module_to_find: &'a str, scope: ScanScope, source: &'a str) -> Self {
        Self {
            module_to_find,
            scope,
            references: Vec::new(),
            source,
        }
    }

    fn into_references(self) -> Vec<ModuleReference> {
        self.references
    }

    fn visit_node(&mut self, node: tree_sitter::Node, cursor: &mut tree_sitter::TreeCursor) {
        // Check import declarations
        if node.kind() == "import_spec" {
            // import_spec contains the import path as a string literal
            if let Some(path_node) = node.child_by_field_name("path") {
                let import_path = self.node_text(path_node);
                // Remove quotes from import path
                let import_path = import_path.trim_matches('"');

                // Check if this import references our module
                if import_path == self.module_to_find
                    || import_path.ends_with(&format!("/{}", self.module_to_find))
                {
                    self.references.push(ModuleReference {
                        line: path_node.start_position().row,
                        column: path_node.start_position().column,
                        length: self.module_to_find.len(),
                        text: import_path.to_string(),
                        kind: ReferenceKind::Declaration,
                    });
                }
            }
        }

        // Check qualified identifiers (module.Function calls) if in appropriate scope
        if matches!(self.scope, ScanScope::QualifiedPaths | ScanScope::All)
            && node.kind() == "selector_expression"
        {
            // selector_expression: operand.field
            // Check if operand is our module
            if let Some(operand) = node.child_by_field_name("operand") {
                if operand.kind() == "identifier" {
                    let ident = self.node_text(operand);
                    if ident == self.module_to_find {
                        let full_text = self.node_text(node);
                        self.references.push(ModuleReference {
                            line: operand.start_position().row,
                            column: operand.start_position().column,
                            length: self.module_to_find.len(),
                            text: full_text,
                            kind: ReferenceKind::QualifiedPath,
                        });
                    }
                }
            }
        }

        // Check string literals if scanning all
        if self.scope == ScanScope::All
            && (node.kind() == "interpreted_string_literal" || node.kind() == "raw_string_literal")
        {
            let text = self.node_text(node);
            if text.contains(self.module_to_find) {
                self.references.push(ModuleReference {
                    line: node.start_position().row,
                    column: node.start_position().column,
                    length: self.module_to_find.len(),
                    text,
                    kind: ReferenceKind::StringLiteral,
                });
            }
        }

        // Recursively visit children
        if cursor.goto_first_child() {
            loop {
                let child = cursor.node();
                self.visit_node(child, cursor);

                if !cursor.goto_next_sibling() {
                    break;
                }
            }
            cursor.goto_parent();
        }
    }

    fn node_text(&self, node: tree_sitter::Node) -> String {
        node.utf8_text(self.source.as_bytes())
            .unwrap_or("")
            .to_string()
    }
}

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
        content: &str,
        module_to_find: &str,
        scope: ScanScope,
    ) -> AstResult<Vec<ModuleReference>> {
        // Create tree-sitter parser with Go grammar
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_go::LANGUAGE.into())
            .map_err(|e| AstError::analysis(format!("Failed to set Go language: {:?}", e)))?;

        // Parse the Go source code
        let tree = parser
            .parse(content, None)
            .ok_or_else(|| AstError::analysis("Failed to parse Go source code"))?;

        // Create and run visitor
        let mut finder = GoModuleFinder::new(module_to_find, scope, content);
        let root = tree.root_node();
        let mut cursor = root.walk();
        finder.visit_node(root, &mut cursor);

        Ok(finder.into_references())
    }
}

/// Visitor for finding module references in Java code using tree-sitter
struct JavaModuleFinder<'a> {
    module_to_find: &'a str,
    scope: ScanScope,
    references: Vec<ModuleReference>,
    source: &'a str,
}

impl<'a> JavaModuleFinder<'a> {
    fn new(module_to_find: &'a str, scope: ScanScope, source: &'a str) -> Self {
        Self {
            module_to_find,
            scope,
            references: Vec::new(),
            source,
        }
    }

    fn into_references(self) -> Vec<ModuleReference> {
        self.references
    }

    fn visit_node(&mut self, node: tree_sitter::Node, cursor: &mut tree_sitter::TreeCursor) {
        // Check import declarations
        if node.kind() == "import_declaration" {
            // import_declaration contains either a scoped_identifier or identifier
            if let Some(import_node) = node
                .child_by_field_name("import")
                .or_else(|| node.named_child(0))
            {
                let import_path = self.node_text(import_node);

                // Java imports are fully qualified: com.example.utils.Helper
                // Check if this import references our module (package or class)
                if import_path.starts_with(&format!("{}.", self.module_to_find))
                    || import_path == self.module_to_find
                    || import_path.contains(&format!(".{}.", self.module_to_find))
                    || import_path.ends_with(&format!(".{}", self.module_to_find))
                {
                    self.references.push(ModuleReference {
                        line: import_node.start_position().row,
                        column: import_node.start_position().column,
                        length: self.module_to_find.len(),
                        text: import_path,
                        kind: ReferenceKind::Declaration,
                    });
                }
            }
        }

        // Check qualified method calls (module.ClassName.method) if in appropriate scope
        if matches!(self.scope, ScanScope::QualifiedPaths | ScanScope::All)
            && node.kind() == "method_invocation"
        {
            // method_invocation: object.method()
            if let Some(object) = node.child_by_field_name("object") {
                if object.kind() == "field_access" {
                    // Could be: MyClass.staticMethod() or instance.method()
                    if let Some(field_object) = object.child_by_field_name("object") {
                        let ident = self.node_text(field_object);
                        if ident == self.module_to_find {
                            let full_text = self.node_text(node);
                            self.references.push(ModuleReference {
                                line: field_object.start_position().row,
                                column: field_object.start_position().column,
                                length: self.module_to_find.len(),
                                text: full_text,
                                kind: ReferenceKind::QualifiedPath,
                            });
                        }
                    }
                } else if object.kind() == "identifier" {
                    let ident = self.node_text(object);
                    if ident == self.module_to_find {
                        let full_text = self.node_text(node);
                        self.references.push(ModuleReference {
                            line: object.start_position().row,
                            column: object.start_position().column,
                            length: self.module_to_find.len(),
                            text: full_text,
                            kind: ReferenceKind::QualifiedPath,
                        });
                    }
                }
            }
        }

        // Check string literals if scanning all
        if self.scope == ScanScope::All && node.kind() == "string_literal" {
            let text = self.node_text(node);
            if text.contains(self.module_to_find) {
                self.references.push(ModuleReference {
                    line: node.start_position().row,
                    column: node.start_position().column,
                    length: self.module_to_find.len(),
                    text,
                    kind: ReferenceKind::StringLiteral,
                });
            }
        }

        // Recursively visit children
        if cursor.goto_first_child() {
            loop {
                let child = cursor.node();
                self.visit_node(child, cursor);

                if !cursor.goto_next_sibling() {
                    break;
                }
            }
            cursor.goto_parent();
        }
    }

    fn node_text(&self, node: tree_sitter::Node) -> String {
        node.utf8_text(self.source.as_bytes())
            .unwrap_or("")
            .to_string()
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
        content: &str,
        module_to_find: &str,
        scope: ScanScope,
    ) -> AstResult<Vec<ModuleReference>> {
        // Create tree-sitter parser with Java grammar
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_java::LANGUAGE.into())
            .map_err(|e| AstError::analysis(format!("Failed to set Java language: {:?}", e)))?;

        // Parse the Java source code
        let tree = parser
            .parse(content, None)
            .ok_or_else(|| AstError::analysis("Failed to parse Java source code"))?;

        // Create and run visitor
        let mut finder = JavaModuleFinder::new(module_to_find, scope, content);
        let root = tree.root_node();
        let mut cursor = root.walk();
        finder.visit_node(root, &mut cursor);

        Ok(finder.into_references())
    }
}

// Test-only mock for Rust adapter (for package_extractor tests)
// The real Rust adapter is now in cb-lang-rust crate
#[cfg(test)]
pub struct RustAdapter;

#[cfg(test)]
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
        if module_path.is_empty() {
            return Err(AstError::analysis("Module path cannot be empty"));
        }

        let src_root = package_path.join("src");
        if !src_root.exists() {
            return Err(AstError::analysis(format!(
                "Source directory not found: {}",
                src_root.display()
            )));
        }

        // Convert module path to file path (handle :: and . separators)
        let normalized_path = module_path.replace("::", "/").replace(".", "/");

        // Try multiple possibilities
        let candidates = vec![
            src_root.join(format!("{}.rs", normalized_path)),
            src_root.join(&normalized_path).join("mod.rs"),
            src_root.join(&normalized_path).join("lib.rs"),
        ];

        for candidate in candidates {
            if candidate.exists() {
                return Ok(vec![candidate]);
            }
        }

        Err(AstError::analysis(format!(
            "Module file not found for: {}",
            module_path
        )))
    }
    async fn parse_imports(&self, _file_path: &Path) -> AstResult<Vec<String>> {
        Ok(vec![])
    }
    fn generate_manifest(&self, package_name: &str, dependencies: &[String]) -> String {
        let mut manifest = format!(
            "[package]\nname = \"{}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
            package_name
        );
        if !dependencies.is_empty() {
            manifest.push_str("\n[dependencies]\n");
            for dep in dependencies {
                manifest.push_str(&format!("{} = \"*\"\n", dep));
            }
        }
        manifest
    }
    fn rewrite_import(&self, _old_import: &str, new_package_name: &str) -> String {
        format!("use {};", new_package_name)
    }
    fn handles_extension(&self, ext: &str) -> bool {
        ext == "rs"
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
        Ok((content.to_string(), 0))
    }
    fn find_module_references(
        &self,
        _content: &str,
        _module_to_find: &str,
        _scope: ScanScope,
    ) -> AstResult<Vec<ModuleReference>> {
        Ok(vec![])
    }
}
