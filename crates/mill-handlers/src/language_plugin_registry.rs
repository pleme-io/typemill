//! Language Plugin Registry for dynamic language support
//!
//! This module provides a registry for language plugins that implement the
//! `LanguagePlugin` trait. The registry allows for dynamic discovery and routing
//! of language-specific operations based on file extensions.
//!
//! **IMPORTANT**: This registry uses dependency injection. Language plugins must be
//! registered at the application layer (e.g., in apps/codebuddy/src/main.rs) and
//! injected via `from_registry()`. This eliminates compile-time coupling between
//! the handler layer and specific language implementations.

use mill_plugin_api::{ LanguagePlugin , PluginRegistry };
use std::sync::Arc;
use tracing::debug;

/// Language plugin registry for the handler layer
///
/// This registry wraps the core `PluginRegistry` from `mill-plugin-api` and
/// provides additional functionality for integration with the handler system.
///
/// **IMPORTANT**: This registry requires dependency injection. Create it using
/// `from_registry()` with a pre-built PluginRegistry. Never auto-build plugins
/// at this layer - that defeats the purpose of dependency injection.
#[derive(Clone)]
pub struct LanguagePluginRegistry {
    pub inner: Arc<PluginRegistry>,
}

impl LanguagePluginRegistry {
    /// Create a registry from an existing PluginRegistry (RECOMMENDED)
    ///
    /// This is the primary way to create a LanguagePluginRegistry. The PluginRegistry
    /// should be built at the application layer (e.g., in apps/codebuddy/src/main.rs)
    /// using `mill_services::services::registry_builder::build_language_plugin_registry()`,
    /// then injected here.
    ///
    /// # Example
    /// ```rust
    /// use mill_plugin_api::PluginRegistry;
    /// use mill_services::services::registry_builder::build_language_plugin_registry;
    /// use std::sync::Arc;
    ///
    /// // At application layer (apps/codebuddy/src/main.rs)
    /// let registry = build_language_plugin_registry();
    ///
    /// // Inject into handler layer
    /// let handler_registry = LanguagePluginRegistry::from_registry(registry);
    /// ```
    pub fn from_registry(registry: Arc<PluginRegistry>) -> Self {
        Self { inner: registry }
    }

    /// Get a plugin for a given file extension
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

    /// Get a plugin that can handle a specific manifest file
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

// NOTE: No Default impl - this would bypass dependency injection.
// Always use from_registry() to create LanguagePluginRegistry with an injected PluginRegistry.