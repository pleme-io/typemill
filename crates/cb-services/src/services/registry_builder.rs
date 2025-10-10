//! Centralized Language Plugin Registry Builder
//!
//! This module provides the single source of truth for building the default
//! language plugin registry. All services that need language intelligence
//! plugins should receive the registry via dependency injection.
//!
//! # Architecture
//!
//! The registry builder discovers all available language plugins at runtime
//! using the `cb-plugin-registry` crate. Plugins self-register using the
//! `codebuddy_plugin!` macro, and this builder collects them into the
//! `PluginRegistry`.

use cb_plugin_api::PluginRegistry;
use cb_plugin_registry::iter_plugins;
use std::sync::Arc;
use tracing::debug;

/// Build the language plugin registry.
///
/// This function iterates over all self-registered plugins discovered by
/// `cb-plugin-registry` and adds them to a new `PluginRegistry`.
pub fn build_language_plugin_registry() -> Arc<PluginRegistry> {
    let mut registry = PluginRegistry::new();
    let mut plugin_count = 0;

    for descriptor in iter_plugins() {
        let plugin = (descriptor.factory)();
        registry.register(plugin.into());
        plugin_count += 1;
    }

    debug!(
        plugin_count,
        "Built language plugin registry from discovered plugins"
    );

    Arc::new(registry)
}

#[cfg(test)]
mod tests {
    use super::*;

    // NOTE: These tests will only pass after the language plugins (`cb-lang-*`)
    // have been updated to use the `codebuddy_plugin!` macro for self-registration.
    // The `cb-services` crate depends on the language crates, so they will be
    // linked, allowing `inventory` to discover them.

    #[test]
    fn test_registry_builder_creates_non_empty_registry() {
        let registry = build_language_plugin_registry();
        // This test requires that plugins are linked and have registered themselves.
        assert!(!registry.all().is_empty(), "No plugins were discovered. Ensure language crates are linked and use codebuddy_plugin!.");
    }

    #[test]
    fn test_registry_builder_includes_rust_plugin() {
        let registry = build_language_plugin_registry();

        let plugin = registry.find_by_extension("rs");
        assert!(plugin.is_some(), "Rust plugin not found for extension 'rs'");
        // The name should match the one provided in the `codebuddy_plugin!` macro.
        assert_eq!(plugin.unwrap().metadata().name, "rust");
    }

    #[test]
    fn test_registry_builder_includes_typescript_plugin() {
        let registry = build_language_plugin_registry();

        let plugin = registry.find_by_extension("ts");
        assert!(plugin.is_some(), "TypeScript plugin not found for extension 'ts'");
        // The name should match the one provided in the `codebuddy_plugin!` macro.
        assert_eq!(plugin.unwrap().metadata().name, "typescript");
    }

    #[test]
    fn test_registry_builder_returns_arc() {
        let registry = build_language_plugin_registry();
        let registry2 = registry.clone();
        assert_eq!(registry.all().len(), registry2.all().len());
    }
}