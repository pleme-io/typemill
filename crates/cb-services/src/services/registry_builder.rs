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
use cb_plugin_api::iter_plugins;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tracing::{debug, warn};

/// Build the language plugin registry.
///
/// This function iterates over all self-registered plugins discovered by
/// `cb-plugin-registry` and adds them to a new `PluginRegistry`.
///
/// # Validation
///
/// - Validates that plugin names are unique
/// - Validates that file extensions don't conflict between plugins
/// - Logs warnings for any conflicts detected
pub fn build_language_plugin_registry() -> Arc<PluginRegistry> {
    let mut registry = PluginRegistry::new();
    let mut plugin_count = 0;

    debug!("Plugin discovery started");

    // Validation sets
    let mut seen_names = HashSet::new();
    let mut extension_to_plugin: HashMap<&str, &str> = HashMap::new();

    for descriptor in iter_plugins() {
        debug!(
            plugin_name = descriptor.name,
            extensions = ?descriptor.extensions,
            "Discovered plugin for registration"
        );

        // Validate unique plugin name
        if !seen_names.insert(descriptor.name) {
            warn!(
                plugin_name = descriptor.name,
                "Duplicate plugin name detected - only the first registration will be used"
            );
            continue;
        }

        // Validate unique extensions
        let mut has_conflict = false;
        for ext in descriptor.extensions {
            if let Some(existing_plugin) = extension_to_plugin.get(ext) {
                warn!(
                    extension = ext,
                    existing_plugin = existing_plugin,
                    new_plugin = descriptor.name,
                    "File extension conflict - extension already claimed by another plugin"
                );
                has_conflict = true;
            } else {
                extension_to_plugin.insert(ext, descriptor.name);
            }
        }

        if has_conflict {
            warn!(
                plugin_name = descriptor.name,
                "Skipping plugin due to extension conflicts"
            );
            continue;
        }

        let plugin = (descriptor.factory)();
        registry.register(plugin.into());
        plugin_count += 1;
    }

    debug!(
        plugin_count,
        "Plugin discovery complete"
    );

    if plugin_count == 0 {
        warn!("No plugins discovered - plugin system may be broken");
    }

    // Validate required plugins
    if registry.find_by_extension("rs").is_none() {
        warn!("RustPlugin not found in registry");
    } else {
        debug!("RustPlugin found in registry");
    }

    if registry.find_by_extension("ts").is_none() {
        warn!("TypeScriptPlugin not found in registry");
    } else {
        debug!("TypeScriptPlugin found in registry");
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
        assert!(
            plugin.is_some(),
            "TypeScript plugin not found for extension 'ts'"
        );
        // The name should match the one provided in the `codebuddy_plugin!` macro.
        assert_eq!(plugin.unwrap().metadata().name, "typescript");
    }

    #[test]
    fn test_registry_builder_includes_markdown_plugin() {
        let registry = build_language_plugin_registry();

        let plugin = registry.find_by_extension("md");
        assert!(
            plugin.is_some(),
            "Markdown plugin not found for extension 'md'"
        );
        assert_eq!(plugin.unwrap().metadata().name, "Markdown");
    }

    #[test]
    fn test_registry_builder_returns_arc() {
        let registry = build_language_plugin_registry();
        let registry2 = registry.clone();
        assert_eq!(registry.all().len(), registry2.all().len());
    }
}
