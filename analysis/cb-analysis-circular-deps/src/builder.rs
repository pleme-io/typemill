//! Builds a `DependencyGraph` from a project's source files.

use cb_analysis_graph::dependency::{Dependency, DependencyGraph, DependencyKind};
use cb_plugin_api::{
    import_support::ImportParser,
    PluginRegistry,
};
use ignore::WalkBuilder;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{debug, warn};

pub struct DependencyGraphBuilder<'a> {
    plugin_registry: &'a PluginRegistry,
}

impl<'a> DependencyGraphBuilder<'a> {
    pub fn new(plugin_registry: &'a PluginRegistry) -> Self {
        Self { plugin_registry }
    }

    pub fn build(&self, project_root: &Path) -> Result<DependencyGraph, String> {
        let mut graph = DependencyGraph::new();
        let source_files = self.collect_source_files(project_root);

        debug!(
            project_root = %project_root.display(),
            files_count = source_files.len(),
            "Building dependency graph"
        );

        for file_path in source_files {
            let content = fs::read_to_string(&file_path)
                .map_err(|e| format!("Failed to read {:?}: {}", file_path, e))?;

            let extension = file_path.extension().and_then(|s| s.to_str()).unwrap_or("");
            if let Some(plugin) = self.plugin_registry.find_by_extension(extension) {
                if let Some(import_parser) = plugin.import_parser() {
                    let imports = ImportParser::parse_imports(import_parser, &content);
                    for import_path in imports {
                        if let Some(resolved_path) = self.resolve_path(
                            &file_path,
                            &import_path,
                            project_root,
                            import_parser,
                        ) {
                            // Try to extract symbols from the import
                            let symbols =
                                self.extract_symbols(&content, &import_path, import_parser);

                            let dependency = Dependency {
                                kind: DependencyKind::Import,
                                symbols,
                            };
                            graph.add_dependency(
                                &file_path,
                                &resolved_path,
                                dependency,
                                plugin.metadata().name,
                            );
                        }
                    }
                }
            }
        }

        debug!(
            nodes = graph.graph.node_count(),
            edges = graph.graph.edge_count(),
            "Dependency graph built"
        );

        Ok(graph)
    }

    /// Collect source files from project, respecting .gitignore
    fn collect_source_files(&self, project_root: &Path) -> Vec<PathBuf> {
        let mut files = Vec::new();

        for result in WalkBuilder::new(project_root)
            .hidden(false) // Don't automatically skip hidden files
            .git_ignore(true) // Respect .gitignore
            .git_global(true) // Use global gitignore
            .git_exclude(true) // Use .git/info/exclude
            .build()
        {
            match result {
                Ok(entry) => {
                    if entry.file_type().is_some_and(|ft| ft.is_file()) {
                        if let Some(ext) = entry.path().extension().and_then(|s| s.to_str()) {
                            if self.plugin_registry.find_by_extension(ext).is_some() {
                                files.push(entry.path().to_path_buf());
                            }
                        }
                    }
                }
                Err(err) => {
                    warn!(error = %err, "Failed to read directory entry");
                }
            }
        }

        files
    }

    /// Enhanced path resolver supporting TypeScript and Rust module resolution
    fn resolve_path(
        &self,
        current_file: &Path,
        import_path: &str,
        project_root: &Path,
        _import_parser: &dyn ImportParser,
    ) -> Option<PathBuf> {
        use path_clean::PathClean;

        let parent = current_file.parent()?;

        // Determine language from file extension
        let extension = current_file.extension()?.to_str()?;

        match extension {
            "ts" | "tsx" | "js" | "jsx" => {
                self.resolve_typescript_path(parent, import_path, project_root)
            }
            "rs" => self.resolve_rust_path(parent, import_path, project_root),
            _ => {
                // Generic fallback
                let resolved = parent.join(import_path);
                if resolved.exists() {
                    Some(resolved.clean())
                } else {
                    None
                }
            }
        }
    }

    /// TypeScript/JavaScript path resolution
    /// Handles: .ts/.tsx/.js/.jsx extensions, index files, directory imports
    fn resolve_typescript_path(
        &self,
        parent: &Path,
        import_path: &str,
        _project_root: &Path,
    ) -> Option<PathBuf> {
        use path_clean::PathClean;

        // Skip external packages (node_modules)
        if !import_path.starts_with('.') && !import_path.starts_with('/') {
            return None;
        }

        let mut resolved = parent.join(import_path);

        // If the import already has an extension, try it directly
        if import_path.ends_with(".ts")
            || import_path.ends_with(".tsx")
            || import_path.ends_with(".js")
            || import_path.ends_with(".jsx")
        {
            let cleaned = resolved.clean();
            if cleaned.exists() {
                return Some(cleaned);
            }
        }

        // Try adding TypeScript extensions
        for ext in &["ts", "tsx", "js", "jsx"] {
            resolved.set_extension(ext);
            if resolved.exists() {
                return Some(resolved.clean());
            }
        }

        // Try index files in directory
        let as_dir = parent.join(import_path);
        if as_dir.is_dir() {
            for index_file in &["index.ts", "index.tsx", "index.js", "index.jsx"] {
                let index_path = as_dir.join(index_file);
                if index_path.exists() {
                    return Some(index_path.clean());
                }
            }
        }

        None
    }

    /// Rust path resolution
    /// Handles: .rs extension, mod.rs for modules, crate:: and super:: paths
    fn resolve_rust_path(
        &self,
        parent: &Path,
        import_path: &str,
        _project_root: &Path,
    ) -> Option<PathBuf> {
        use path_clean::PathClean;

        // Skip external crates (will be in Cargo.toml dependencies)
        // Only handle relative paths (crate::, super::, self::)
        if !import_path.starts_with("crate::")
            && !import_path.starts_with("super::")
            && !import_path.starts_with("self::")
        {
            return None;
        }

        // Convert Rust path notation to file path
        // e.g., crate::foo::bar -> foo/bar.rs or foo/bar/mod.rs
        let path_str = import_path
            .replace("crate::", "")
            .replace("super::", "../")
            .replace("self::", "./")
            .replace("::", "/");

        let mut resolved = parent.join(&path_str);

        // Try .rs file
        resolved.set_extension("rs");
        if resolved.exists() {
            return Some(resolved.clean());
        }

        // Try mod.rs in directory
        let mod_path = parent.join(&path_str).join("mod.rs");
        if mod_path.exists() {
            return Some(mod_path.clean());
        }

        None
    }

    /// Extract imported symbols from the import statement
    /// Uses language plugin's import parsing to get actual symbol names
    fn extract_symbols(
        &self,
        content: &str,
        import_path: &str,
        import_parser: &dyn ImportParser,
    ) -> Vec<String> {
        // Parse all imports from the content
        let _all_imports = ImportParser::parse_imports(import_parser, content);

        // Find the import that matches our import_path
        // Note: This is a simplified approach - in production we'd want
        // to use the full ImportInfo structure from parse_imports_detailed
        // For now, we return empty vec as placeholder since parse_imports
        // only returns module paths, not symbols
        //
        // TODO: Extend ImportSupport trait with parse_imports_detailed
        // that returns Vec<ImportInfo> with symbol information

        // Heuristic: try to find the import line and extract symbols
        for line in content.lines() {
            if line.contains(import_path) {
                // Simple extraction for common patterns
                if let Some(start) = line.find('{') {
                    if let Some(end) = line.find('}') {
                        let symbols_str = &line[start + 1..end];
                        return symbols_str
                            .split(',')
                            .map(|s| s.trim().split_whitespace().next().unwrap_or("").to_string())
                            .filter(|s| !s.is_empty())
                            .collect();
                    }
                }
            }
        }

        vec![]
    }
}
