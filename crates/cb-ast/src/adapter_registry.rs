//! Language Adapter Registry
//!
//! Provides a central registry for language adapters, enabling dynamic
//! language support without hardcoded adapter lists.
//!
//! # Architecture
//!
//! The registry pattern allows:
//! - Dynamic registration of language adapters at startup
//! - Extension-based lookup for file operations
//! - Clean separation between core logic and language-specific implementations
//!
//! # Example
//!
//! ```rust,ignore
//! use cb_ast::adapter_registry::LanguageAdapterRegistry;
//! use cb_lang_rust_adapter::RustAdapter;
//! use std::sync::Arc;
//!
//! let mut registry = LanguageAdapterRegistry::new();
//! registry.register(Arc::new(RustAdapter::default_intelligence()));
//!
//! // Later, lookup by file extension
//! if let Some(adapter) = registry.find_by_extension("rs") {
//!     // Use the adapter for refactoring operations
//! }
//! ```

use crate::language::LanguageAdapter;
use std::sync::Arc;
use tracing::debug;

/// Registry for managing language adapters
///
/// Provides centralized management of language-specific refactoring adapters.
/// Adapters are registered at startup and looked up dynamically based on
/// file extensions.
pub struct LanguageAdapterRegistry {
    adapters: Vec<Arc<dyn LanguageAdapter>>,
}

impl LanguageAdapterRegistry {
    /// Create a new empty adapter registry
    pub fn new() -> Self {
        debug!("Creating new LanguageAdapterRegistry");
        Self {
            adapters: Vec::new(),
        }
    }

    /// Register a new language adapter
    ///
    /// # Arguments
    ///
    /// * `adapter` - The adapter to register
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let mut registry = LanguageAdapterRegistry::new();
    /// registry.register(Arc::new(RustAdapter::default_intelligence()));
    /// ```
    pub fn register(&mut self, adapter: Arc<dyn LanguageAdapter>) {
        let language = adapter.language();
        debug!(language = %language.as_str(), "Registering language adapter");
        self.adapters.push(adapter);
    }

    /// Find an adapter that handles the given file extension
    ///
    /// # Arguments
    ///
    /// * `extension` - File extension without the dot (e.g., "rs", "ts", "py")
    ///
    /// # Returns
    ///
    /// The first adapter that handles this extension, or None if no adapter found
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let adapter = registry.find_by_extension("rs")?;
    /// let imports = adapter.parse_imports(&file_path).await?;
    /// ```
    pub fn find_by_extension(&self, extension: &str) -> Option<&Arc<dyn LanguageAdapter>> {
        self.adapters
            .iter()
            .find(|adapter| adapter.handles_extension(extension))
    }

    /// Get all registered adapters
    ///
    /// Useful for operations that need to iterate over all language adapters,
    /// such as workspace-wide analysis.
    pub fn all(&self) -> &[Arc<dyn LanguageAdapter>] {
        &self.adapters
    }

    /// Get the number of registered adapters
    pub fn len(&self) -> usize {
        self.adapters.len()
    }

    /// Check if the registry is empty
    pub fn is_empty(&self) -> bool {
        self.adapters.is_empty()
    }

    /// Create a registry with commonly used adapters
    ///
    /// This is a convenience method that registers adapters for:
    /// - Rust (using cb-lang-rust-adapter if available)
    ///
    /// Note: Other language adapters from the old system are deprecated.
    /// Only use this for backward compatibility during migration.
    #[cfg(feature = "default-adapters")]
    pub fn with_default_adapters() -> Self {
        let mut registry = Self::new();

        #[cfg(feature = "rust-adapter")]
        {
            use cb_lang_rust_adapter::RustAdapter;
            registry.register(Arc::new(RustAdapter::default_intelligence()));
        }

        registry
    }
}

impl Default for LanguageAdapterRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::language::{LanguageAdapter, ModuleReference, ScanScope};
    use crate::AstResult;
    use async_trait::async_trait;
    use cb_core::language::ProjectLanguage;
    use std::path::Path;

    /// Mock adapter for testing
    struct MockRustAdapter;

    #[async_trait]
    impl LanguageAdapter for MockRustAdapter {
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
            _package_path: &Path,
            _module_path: &str,
        ) -> AstResult<Vec<std::path::PathBuf>> {
            Ok(vec![])
        }

        async fn parse_imports(&self, _file_path: &Path) -> AstResult<Vec<String>> {
            Ok(vec![])
        }

        fn generate_manifest(&self, _package_name: &str, _dependencies: &[String]) -> String {
            String::new()
        }

        fn rewrite_import(&self, _old_import: &str, _new_package_name: &str) -> String {
            String::new()
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

    #[test]
    fn test_registry_basic_operations() {
        let mut registry = LanguageAdapterRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);

        registry.register(Arc::new(MockRustAdapter));
        assert!(!registry.is_empty());
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn test_find_by_extension() {
        let mut registry = LanguageAdapterRegistry::new();
        registry.register(Arc::new(MockRustAdapter));

        let adapter = registry.find_by_extension("rs");
        assert!(adapter.is_some());

        let adapter = registry.find_by_extension("py");
        assert!(adapter.is_none());
    }

    #[test]
    fn test_all_adapters() {
        let mut registry = LanguageAdapterRegistry::new();
        registry.register(Arc::new(MockRustAdapter));

        let all = registry.all();
        assert_eq!(all.len(), 1);
    }
}
