//! Centralized Language Plugin Registry Builder
//!
//! This module provides the single source of truth for building the default
//! language plugin registry. All services that need language intelligence
//! plugins should receive the registry via dependency injection rather than
//! constructing it directly.
//!
//! # Architecture
//!
//! The registry builder uses conditional compilation (`cfg` attributes) to
//! register only the language plugins that are enabled via cargo features.
//! This allows for flexible deployment configurations while maintaining a
//! single point of control.
//!
//! # Usage
//!
//! ```rust,ignore
//! use cb_services::build_language_plugin_registry;
//!
//! // In service initialization:
//! let registry = build_language_plugin_registry();
//! let file_service = FileService::new(
//!     project_root,
//!     ast_cache,
//!     lock_manager,
//!     operation_queue,
//!     config,
//!     registry.clone(),
//! );
//! ```

use cb_plugin_api::PluginRegistry;
use std::sync::Arc;

/// Build the default language plugin registry with all compiled-in plugins
///
/// This is the **SINGLE SOURCE OF TRUTH** for plugin registration. All services
/// should receive this registry via dependency injection.
///
/// # Features
///
/// The following cargo features control which plugins are included:
/// - `lang-rust` - Rust language support (default)
/// - `lang-go` - Go language support (default)
/// - `lang-typescript` - TypeScript/JavaScript language support (default)
///
/// # Returns
///
/// An `Arc<PluginRegistry>` containing all enabled language plugins.
///
/// # Example
///
/// ```rust,ignore
/// let registry = build_language_plugin_registry();
/// let plugin = registry.find_by_extension("rs");
/// assert!(plugin.is_some());
/// ```
pub fn build_language_plugin_registry() -> Arc<PluginRegistry> {
    let mut registry = PluginRegistry::new();
    let mut plugin_count = 0;

    // Register Rust plugin
    #[cfg(feature = "lang-rust")]
    {
        registry.register(Arc::new(cb_lang_rust::RustPlugin::new()));
        plugin_count += 1;
    }

    // Register Go plugin
    #[cfg(feature = "lang-go")]
    {
        registry.register(Arc::new(cb_lang_go::GoPlugin::new()));
        plugin_count += 1;
    }

    // Register TypeScript plugin
    #[cfg(feature = "lang-typescript")]
    {
        registry.register(Arc::new(cb_lang_typescript::TypeScriptPlugin::new()));
        plugin_count += 1;
    }

    // Register Python plugin
    #[cfg(feature = "lang-python")]
    {
        registry.register(Arc::new(cb_lang_python::PythonPlugin::new()));
        plugin_count += 1;
    }

    // Register Java plugin
    #[cfg(feature = "lang-java")]
    {
        registry.register(Arc::new(cb_lang_java::JavaPlugin::new()));
        plugin_count += 1;
    }

    let _ = plugin_count; // Suppress unused variable warning when no features enabled

    Arc::new(registry)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_builder_creates_non_empty_registry() {
        let registry = build_language_plugin_registry();

        // Should have at least one plugin (assuming default features are enabled)
        assert!(!registry.all().is_empty());
    }

    #[cfg(feature = "lang-rust")]
    #[test]
    fn test_registry_builder_includes_rust_plugin() {
        let registry = build_language_plugin_registry();

        // Should be able to find plugin for .rs files
        let plugin = registry.find_by_extension("rs");
        assert!(plugin.is_some());
        assert_eq!(plugin.unwrap().name(), "Rust");
    }

    #[cfg(feature = "lang-go")]
    #[test]
    fn test_registry_builder_includes_go_plugin() {
        let registry = build_language_plugin_registry();

        // Should be able to find plugin for .go files
        let plugin = registry.find_by_extension("go");
        assert!(plugin.is_some());
        assert_eq!(plugin.unwrap().name(), "Go");
    }

    #[cfg(feature = "lang-typescript")]
    #[test]
    fn test_registry_builder_includes_typescript_plugin() {
        let registry = build_language_plugin_registry();

        // Should be able to find plugin for .ts files
        let plugin = registry.find_by_extension("ts");
        assert!(plugin.is_some());
        assert_eq!(plugin.unwrap().name(), "TypeScript");
    }

    #[cfg(feature = "lang-python")]
    #[test]
    fn test_registry_builder_includes_python_plugin() {
        let registry = build_language_plugin_registry();

        // Should be able to find plugin for .py files
        let plugin = registry.find_by_extension("py");
        assert!(plugin.is_some());
        assert_eq!(plugin.unwrap().name(), "Python");
    }

    #[test]
    fn test_registry_builder_returns_arc() {
        let registry = build_language_plugin_registry();

        // Can clone the Arc for sharing across services
        let registry2 = registry.clone();
        assert_eq!(registry.all().len(), registry2.all().len());
    }
}
