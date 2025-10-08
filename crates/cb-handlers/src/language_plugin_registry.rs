//! Language Plugin Registry for dynamic language support
//!
//! This module provides a registry for language plugins that implement the
//! `LanguagePlugin` trait. The registry allows for dynamic discovery and routing
//! of language-specific operations based on file extensions.

use cb_plugin_api::{LanguagePlugin, PluginRegistry};
use cb_services::services::build_language_plugin_registry;
use std::sync::Arc;
use tracing::debug;

/// Language plugin registry for the handler layer
///
/// This registry wraps the core `PluginRegistry` from `cb-plugin-api` and
/// provides additional functionality for integration with the handler system.
///
/// **IMPORTANT**: This registry uses the centralized builder from
/// `cb_services::build_language_plugin_registry()` to ensure all plugins are
/// registered in a single location.
#[derive(Clone)]
pub struct LanguagePluginRegistry {
    inner: Arc<PluginRegistry>,
}

impl LanguagePluginRegistry {
    /// Create a new registry with all available language plugins
    ///
    /// This method uses the centralized plugin builder to ensure consistency
    /// across the application. All plugin registration happens in
    /// `crates/cb-services/src/services/registry_builder.rs`.
    pub fn new() -> Self {
        // Use the centralized builder - this is the ONLY correct way to create a registry
        let registry = build_language_plugin_registry();

        Self { inner: registry }
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
            debug!(extension = extension, "Found language plugin for extension");
        } else {
            debug!(
                extension = extension,
                "No language plugin found for extension"
            );
        }

        result
    }

    /// Get all registered language plugins
    pub fn all_plugins(&self) -> &[Arc<dyn LanguagePlugin>] {
        self.inner.all()
    }

    /// Get a list of all supported file extensions
    pub fn supported_extensions(&self) -> Vec<String> {
        let mut extensions = Vec::new();
        for plugin in self.inner.all() {
            for ext in plugin.metadata().extensions {
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

    /// Get all plugins that provide test fixtures
    ///
    /// This method filters the registered plugins to return only those
    /// that have implemented the `test_fixtures()` method and returned
    /// `Some(fixtures)`.
    ///
    /// Used by integration tests to discover available test scenarios.
    ///
    /// # Returns
    ///
    /// A vector of tuples containing:
    /// - Reference to the plugin
    /// - The test fixtures it provides
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let registry = LanguagePluginRegistry::new();
    /// for (plugin, fixtures) in registry.plugins_with_fixtures() {
    ///     println!("Testing {}", plugin.metadata().name);
    ///     for scenario in &fixtures.complexity_scenarios {
    ///         // Run test with scenario
    ///     }
    /// }
    /// ```
    pub fn plugins_with_fixtures(
        &self,
    ) -> Vec<(&dyn LanguagePlugin, cb_plugin_api::LanguageTestFixtures)> {
        self.inner
            .all()
            .iter()
            .filter_map(|plugin| {
                let fixtures = plugin.test_fixtures()?;
                Some((plugin.as_ref(), fixtures))
            })
            .collect()
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
    pub fn get_plugin_for_manifest(&self, filename: &str) -> Option<&dyn LanguagePlugin> {
        debug!(filename = filename, "Looking up plugin for manifest");

        for plugin in self.inner.all() {
            if plugin.handles_manifest(filename) {
                debug!(
                    filename = filename,
                    plugin = plugin.metadata().name,
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
