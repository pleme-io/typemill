//! ImportGraph builder utilities
//!
//! Provides a builder pattern for constructing ImportGraph instances,
//! reducing boilerplate across language plugins.

use codebuddy_foundation::protocol::{ImportGraph, ImportGraphMetadata, ImportInfo};
use std::collections::HashSet;
use std::path::Path;

/// Builder for constructing ImportGraph instances
///
/// This builder provides a convenient way to construct ImportGraph objects
/// with consistent defaults and automatic timestamp generation.
///
/// # Example
///
/// ```rust,ignore
/// use cb_lang_common::import_graph::ImportGraphBuilder;
///
/// let graph = ImportGraphBuilder::new("typescript")
///     .with_source_file(Some(&path))
///     .with_imports(imports)
///     .with_external_dependencies(external_deps)
///     .build();
/// ```
pub struct ImportGraphBuilder {
    source_file: String,
    imports: Vec<ImportInfo>,
    importers: Vec<String>,
    language: String,
    parser_version: String,
    circular_dependencies: Vec<Vec<String>>,
    external_dependencies: Vec<String>,
}

impl ImportGraphBuilder {
    /// Create a new ImportGraph builder for the specified language
    ///
    /// # Arguments
    ///
    /// * `language` - Language identifier (e.g., "rust", "python", "typescript")
    pub fn new(language: impl Into<String>) -> Self {
        Self {
            source_file: "in-memory".to_string(),
            imports: Vec::new(),
            importers: Vec::new(),
            language: language.into(),
            parser_version: "0.1.0-plugin".to_string(),
            circular_dependencies: Vec::new(),
            external_dependencies: Vec::new(),
        }
    }

    /// Set the source file path
    ///
    /// If `path` is `None`, defaults to "in-memory.{ext}" where ext is inferred
    /// from the language.
    pub fn with_source_file(mut self, path: Option<&Path>) -> Self {
        self.source_file = if let Some(p) = path {
            p.to_string_lossy().to_string()
        } else {
            // Infer default file extension from language
            let ext = match self.language.as_str() {
                "rust" => "rs",
                "python" => "py",
                "typescript" => "ts",
                "javascript" => "js",
                "go" => "go",
                "java" => "java",
                _ => "txt",
            };
            format!("in-memory.{}", ext)
        };
        self
    }

    /// Set the imports list
    pub fn with_imports(mut self, imports: Vec<ImportInfo>) -> Self {
        self.imports = imports;
        self
    }

    /// Set the importers list
    pub fn with_importers(mut self, importers: Vec<String>) -> Self {
        self.importers = importers;
        self
    }

    /// Set the parser version string
    pub fn with_parser_version(mut self, version: impl Into<String>) -> Self {
        self.parser_version = version.into();
        self
    }

    /// Set circular dependencies
    ///
    /// Each inner Vec represents a circular dependency chain
    pub fn with_circular_dependencies(mut self, circular: Vec<Vec<String>>) -> Self {
        self.circular_dependencies = circular;
        self
    }

    /// Set external dependencies
    pub fn with_external_dependencies(mut self, external: Vec<String>) -> Self {
        self.external_dependencies = external;
        self
    }

    /// Extract external dependencies from imports using a detector function
    ///
    /// This is a convenience method that filters imports and extracts
    /// external dependencies automatically.
    ///
    /// # Arguments
    ///
    /// * `detector` - Function that returns `true` if a module path is external
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let graph = ImportGraphBuilder::new("typescript")
    ///     .with_imports(imports.clone())
    ///     .extract_external_dependencies(|path| {
    ///         !path.starts_with("./") && !path.starts_with("../")
    ///     })
    ///     .build();
    /// ```
    pub fn extract_external_dependencies<F>(mut self, detector: F) -> Self
    where
        F: Fn(&str) -> bool,
    {
        let external: Vec<String> = self
            .imports
            .iter()
            .filter_map(|imp| {
                if detector(&imp.module_path) {
                    Some(imp.module_path.clone())
                } else {
                    None
                }
            })
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();

        self.external_dependencies = external;
        self
    }

    /// Build the final ImportGraph
    pub fn build(self) -> ImportGraph {
        ImportGraph {
            source_file: self.source_file,
            imports: self.imports,
            importers: self.importers,
            metadata: ImportGraphMetadata {
                language: self.language,
                parsed_at: chrono::Utc::now(),
                parser_version: self.parser_version,
                circular_dependencies: self.circular_dependencies,
                external_dependencies: self.external_dependencies,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_basic() {
        let graph = ImportGraphBuilder::new("typescript")
            .with_source_file(None)
            .build();

        assert_eq!(graph.source_file, "in-memory.ts");
        assert_eq!(graph.metadata.language, "typescript");
        assert_eq!(graph.metadata.parser_version, "0.1.0-plugin");
        assert!(graph.imports.is_empty());
        assert!(graph.metadata.external_dependencies.is_empty());
    }

    #[test]
    fn test_builder_with_path() {
        let path = Path::new("/project/src/main.rs");
        let graph = ImportGraphBuilder::new("rust")
            .with_source_file(Some(&path))
            .build();

        assert_eq!(graph.source_file, "/project/src/main.rs");
    }

    #[test]
    fn test_builder_with_parser_version() {
        let graph = ImportGraphBuilder::new("python")
            .with_parser_version("1.2.3")
            .build();

        assert_eq!(graph.metadata.parser_version, "1.2.3");
    }

    #[test]
    fn test_default_extensions() {
        assert_eq!(
            ImportGraphBuilder::new("rust")
                .with_source_file(None)
                .build()
                .source_file,
            "in-memory.rs"
        );
        assert_eq!(
            ImportGraphBuilder::new("python")
                .with_source_file(None)
                .build()
                .source_file,
            "in-memory.py"
        );
        assert_eq!(
            ImportGraphBuilder::new("go")
                .with_source_file(None)
                .build()
                .source_file,
            "in-memory.go"
        );
        assert_eq!(
            ImportGraphBuilder::new("java")
                .with_source_file(None)
                .build()
                .source_file,
            "in-memory.java"
        );
    }
}
