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
/// `cb_services::services::build_language_plugin_registry()` to ensure all plugins are
/// registered in a single location.
#[derive(Clone)]
pub struct LanguagePluginRegistry {
    pub inner: Arc<PluginRegistry>,
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

impl Default for LanguagePluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}
