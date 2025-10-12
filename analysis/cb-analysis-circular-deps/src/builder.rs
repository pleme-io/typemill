//! Builds a `DependencyGraph` from a project's source files.

use cb_analysis_graph::dependency::{Dependency, DependencyGraph, DependencyKind};
use cb_plugin_api::{LanguagePlugin, PluginRegistry};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

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

        for file_path in source_files {
            let content = fs::read_to_string(&file_path)
                .map_err(|e| format!("Failed to read {:?}: {}", file_path, e))?;

            let extension = file_path.extension().and_then(|s| s.to_str()).unwrap_or("");
            if let Some(plugin) = self.plugin_registry.find_by_extension(extension) {
                if let Some(import_support) = plugin.import_support() {
                    let imports = import_support.parse_imports(&content);
                    for import_path in imports {
                        // Here we would resolve the import path to an absolute path.
                        // For simplicity, we'll assume it's a relative path for now.
                        if let Some(resolved_path) = self.resolve_path(&file_path, &import_path) {
                            let dependency = Dependency {
                                kind: DependencyKind::Import,
                                symbols: vec![], // Parsing symbols is a future enhancement.
                            };
                            graph.add_dependency(&file_path, &resolved_path, dependency, plugin.metadata().name);
                        }
                    }
                }
            }
        }
        Ok(graph)
    }

    fn collect_source_files(&self, project_root: &Path) -> Vec<PathBuf> {
        let mut files = Vec::new();
        for entry in WalkDir::new(project_root).into_iter().filter_map(|e| e.ok()) {
            if entry.file_type().is_file() {
                if let Some(ext) = entry.path().extension().and_then(|s| s.to_str()) {
                    if self.plugin_registry.find_by_extension(ext).is_some() {
                        files.push(entry.path().to_path_buf());
                    }
                }
            }
        }
        files
    }

    // A simplified path resolver. A real implementation would be more robust.
    fn resolve_path(&self, current_file: &Path, import_path: &str) -> Option<PathBuf> {
        use path_clean::PathClean;
        if let Some(parent) = current_file.parent() {
            let mut resolved = parent.join(import_path);
            if !import_path.ends_with(".rs") && !import_path.ends_with(".ts") {
                // This is a very basic attempt to resolve module extensions.
                if resolved.with_extension("rs").exists() {
                    resolved.set_extension("rs");
                } else if resolved.with_extension("ts").exists() {
                     resolved.set_extension("ts");
                }
            }
            let resolved = resolved.clean();
            if resolved.exists() {
                return Some(resolved);
            }
        }
        None
    }
}
