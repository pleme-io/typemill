//! Language Plugin Bundle
//!
//! This crate serves as the single collection point for all language plugins
//! in the codebuddy system. It depends on all concrete language implementations
//! and provides a simple function to instantiate them.
//!
//! This separation ensures that core service layers (`cb-services`, `cb-ast`)
//! remain decoupled from specific language implementations, while the application
//! binary can easily access all available plugins.

use cb_plugin_api::{iter_plugins, LanguagePlugin};
use std::sync::Arc;

// Force linker to include language plugins by actively using them.
// This prevents linker dead code elimination from stripping the inventory submissions.
// We reference each plugin's public type to ensure the crate is linked.
use cb_lang_markdown::MarkdownPlugin;
use cb_lang_rust::RustPlugin;
use cb_lang_toml::TomlLanguagePlugin;
use cb_lang_typescript::TypeScriptPlugin;
use cb_lang_yaml::YamlLanguagePlugin;

// This function is never called but ensures the linker includes all plugin crates
#[allow(dead_code)]
fn _force_plugin_linkage() {
    // These type references ensure the plugin crates are linked
    // The actual plugin instances will be discovered via inventory
    let _: Option<MarkdownPlugin> = None;
    let _: Option<RustPlugin> = None;
    let _: Option<TomlLanguagePlugin> = None;
    let _: Option<TypeScriptPlugin> = None;
    let _: Option<YamlLanguagePlugin> = None;
}

/// Returns all language plugins available in this bundle.
///
/// This function uses the plugin registry's auto-discovery mechanism
/// to find all plugins that have self-registered using the `codebuddy_plugin!` macro.
///
/// # Returns
///
/// A vector of all discovered language plugins, wrapped in `Arc` for shared ownership.
///
/// # Example
///
/// ```no_run
/// use codebuddy_plugin_bundle::all_plugins;
/// use cb_plugin_api::PluginRegistry;
///
/// let plugins = all_plugins();
/// let mut registry = PluginRegistry::new();
/// for plugin in plugins {
///     registry.register(plugin);
/// }
/// ```
pub fn all_plugins() -> Vec<Arc<dyn LanguagePlugin>> {
    let plugins: Vec<_> = iter_plugins()
        .map(|descriptor| {
            tracing::debug!(
                plugin_name = descriptor.name,
                extensions = ?descriptor.extensions,
                "Discovered language plugin via inventory"
            );
            let plugin = (descriptor.factory)();
            Arc::from(plugin) as Arc<dyn LanguagePlugin>
        })
        .collect();

    tracing::info!(
        plugin_count = plugins.len(),
        "Language plugin bundle discovery complete"
    );

    if plugins.is_empty() {
        tracing::warn!("No language plugins discovered - inventory system may be broken");
    }

    plugins
}

#[cfg(test)]
mod tests {
    use super::*;

    // Force linker to include language plugins for inventory collection in tests
    #[cfg(test)]
    extern crate cb_lang_markdown;
    #[cfg(test)]
    extern crate cb_lang_rust;
    #[cfg(test)]
    extern crate cb_lang_typescript;
    #[cfg(test)]
    extern crate cb_lang_toml;
    #[cfg(test)]
    extern crate cb_lang_yaml;

    #[test]
    fn test_all_plugins_returns_plugins() {
        let plugins = all_plugins();

        // Should have at least the core plugins (Rust, TypeScript, etc.)
        assert!(
            plugins.len() >= 3,
            "Expected at least 3 plugins (Rust, TypeScript, Markdown), found {}",
            plugins.len()
        );
    }

    #[test]
    fn test_plugins_have_unique_names() {
        let plugins = all_plugins();
        let mut names = std::collections::HashSet::new();

        for plugin in plugins {
            let name = plugin.metadata().name;
            assert!(
                names.insert(name),
                "Duplicate plugin name found: {}",
                name
            );
        }
    }
}
