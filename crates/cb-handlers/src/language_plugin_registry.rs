//! Language Plugin Registry for dynamic language support
//!
//! This module provides a registry for language plugins that implement the
//! `LanguagePlugin` trait. The registry allows for dynamic discovery and routing
//! of language-specific operations based on file extensions.

use cb_plugin_api::{LanguagePlugin, PluginRegistry as ApiPluginRegistry};
use std::sync::Arc;
use tracing::{debug, info};

/// Language plugin registry for the handler layer
///
/// This registry wraps the core `PluginRegistry` from `cb-plugin-api` and
/// provides additional functionality for integration with the handler system.
#[derive(Clone)]
pub struct LanguagePluginRegistry {
    inner: Arc<ApiPluginRegistry>,
}

impl LanguagePluginRegistry {
    /// Create a new registry with all available language plugins
    ///
    /// This method statically loads all language plugins that are compiled into
    /// the application. In the future, this could be extended to support dynamic
    /// plugin loading from external libraries.
    pub fn new() -> Self {
        let mut registry = ApiPluginRegistry::new();

        // Register Rust plugin
        #[cfg(feature = "lang-rust")]
        {
            info!(plugin = "rust", "Registering Rust language plugin");
            registry.register(Box::new(cb_lang_rust::RustPlugin::new()));
        }

        // Future language plugins will be registered here
        // #[cfg(feature = "lang-python")]
        // registry.register(Box::new(cb_lang_python::PythonPlugin::new()));
        //
        // #[cfg(feature = "lang-typescript")]
        // registry.register(Box::new(cb_lang_typescript::TypeScriptPlugin::new()));

        let plugin_count = registry.all().len();
        info!(
            plugin_count = plugin_count,
            "Language plugin registry initialized"
        );

        Self {
            inner: Arc::new(registry),
        }
    }

    /// Get a plugin for a given file extension
    ///
    /// # Arguments
    ///
    /// * `extension` - File extension without the dot (e.g., "rs", "py")
    ///
    /// # Returns
    ///
    /// An `Arc` to the language plugin if found, `None` otherwise
    pub fn get_plugin(&self, extension: &str) -> Option<&dyn LanguagePlugin> {
        debug!(extension = extension, "Looking up language plugin");
        let result = self.inner.find_by_extension(extension);

        if result.is_some() {
            debug!(
                extension = extension,
                "Found language plugin for extension"
            );
        } else {
            debug!(
                extension = extension,
                "No language plugin found for extension"
            );
        }

        result
    }

    /// Get all registered language plugins
    pub fn all_plugins(&self) -> &[Box<dyn LanguagePlugin>] {
        self.inner.all()
    }

    /// Get a list of all supported file extensions
    pub fn supported_extensions(&self) -> Vec<String> {
        let mut extensions = Vec::new();
        for plugin in self.inner.all() {
            for ext in plugin.file_extensions() {
                extensions.push(ext.to_string());
            }
        }
        extensions.sort();
        extensions.dedup();
        extensions
    }

    /// Check if a file extension is supported
    pub fn supports_extension(&self, extension: &str) -> bool {
        self.inner.find_by_extension(extension).is_some()
    }

    /// Get a plugin that can handle a specific manifest file
    ///
    /// # Arguments
    ///
    /// * `filename` - Manifest filename (e.g., "Cargo.toml", "package.json")
    ///
    /// # Returns
    ///
    /// A reference to the language plugin if found, `None` otherwise
    pub fn get_plugin_for_manifest(&self, filename: &str) -> Option<&dyn cb_plugin_api::LanguagePlugin> {
        debug!(filename = filename, "Looking up plugin for manifest");

        for plugin in self.inner.all() {
            if plugin.handles_manifest(filename) {
                debug!(
                    filename = filename,
                    plugin = plugin.name(),
                    "Found plugin for manifest"
                );
                return Some(plugin.as_ref());
            }
        }

        debug!(filename = filename, "No plugin found for manifest");
        None
    }
}

impl Default for LanguagePluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_initialization() {
        let registry = LanguagePluginRegistry::new();
        assert!(!registry.all_plugins().is_empty());
    }

    #[cfg(feature = "lang-rust")]
    #[test]
    fn test_rust_plugin_registered() {
        let registry = LanguagePluginRegistry::new();

        // Should find plugin for .rs files
        assert!(registry.get_plugin("rs").is_some());
        assert!(registry.supports_extension("rs"));

        // Should be in supported extensions list
        let extensions = registry.supported_extensions();
        assert!(extensions.contains(&"rs".to_string()));
    }

    #[test]
    fn test_unsupported_extension() {
        let registry = LanguagePluginRegistry::new();

        // Should not find plugin for unsupported extension
        assert!(registry.get_plugin("xyz").is_none());
        assert!(!registry.supports_extension("xyz"));
    }

    #[test]
    fn test_supported_extensions_list() {
        let registry = LanguagePluginRegistry::new();
        let extensions = registry.supported_extensions();

        // Should have at least one extension (Rust)
        assert!(!extensions.is_empty());

        // Should be sorted and deduplicated
        let mut sorted = extensions.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(extensions, sorted);
    }
}
