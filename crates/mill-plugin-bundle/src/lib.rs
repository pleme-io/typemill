//! Language Plugin Bundle
//!
//! This crate serves as the single collection point for all language plugins
//! in the mill system. It depends on all concrete language implementations
//! and provides a simple function to instantiate them.
//!
//! This separation ensures that core service layers (`mill-services`, `mill-ast`)
//! remain decoupled from specific language implementations, while the application
//! binary can easily access all available plugins.

use mill_plugin_api::{iter_plugins, LanguagePlugin};
use std::sync::Arc;

// Force linker to include language plugins by actively using them.
// This prevents linker dead code elimination from stripping the inventory submissions.
// We reference each plugin's public type to ensure the crate is linked.
#[cfg(feature = "lang-gitignore")]
use mill_lang_gitignore::GitignoreLanguagePlugin;
#[cfg(feature = "lang-markdown")]
use mill_lang_markdown::MarkdownPlugin;
#[cfg(feature = "lang-rust")]
use mill_lang_rust::RustPlugin;
#[cfg(feature = "lang-toml")]
use mill_lang_toml::TomlLanguagePlugin;
#[cfg(feature = "lang-typescript")]
use mill_lang_typescript::TypeScriptPlugin;
#[cfg(feature = "lang-yaml")]
use mill_lang_yaml::YamlLanguagePlugin;

// This function is never called but ensures the linker includes all plugin crates
#[allow(dead_code)]
fn _force_plugin_linkage() {
    // These type references ensure the plugin crates are linked
    // The actual plugin instances will be discovered via inventory
    #[cfg(feature = "lang-markdown")]
    let _: Option<MarkdownPlugin> = None;
    #[cfg(feature = "lang-rust")]
    let _: Option<RustPlugin> = None;
    #[cfg(feature = "lang-toml")]
    let _: Option<TomlLanguagePlugin> = None;
    #[cfg(feature = "lang-typescript")]
    let _: Option<TypeScriptPlugin> = None;
    #[cfg(feature = "lang-yaml")]
    let _: Option<YamlLanguagePlugin> = None;
    #[cfg(feature = "lang-gitignore")]
    let _: Option<GitignoreLanguagePlugin> = None;
}

/// Returns all language plugins available in this bundle.
///
/// This function uses the plugin registry's auto-discovery mechanism
/// to find all plugins that have self-registered using the `mill_plugin!` macro.
///
/// # Returns
///
/// A vector of all discovered language plugins, wrapped in `Arc` for shared ownership.
///
/// # Example
///
/// ```no_run
/// use mill_plugin_bundle::all_plugins;
/// use mill_plugin_api::PluginRegistry;
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
    #[cfg(all(test, feature = "lang-gitignore"))]
    extern crate mill_lang_gitignore;
    #[cfg(all(test, feature = "lang-markdown"))]
    extern crate mill_lang_markdown;
    #[cfg(all(test, feature = "lang-rust"))]
    extern crate mill_lang_rust;
    #[cfg(all(test, feature = "lang-toml"))]
    extern crate mill_lang_toml;
    #[cfg(all(test, feature = "lang-typescript"))]
    extern crate mill_lang_typescript;
    #[cfg(all(test, feature = "lang-yaml"))]
    extern crate mill_lang_yaml;

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
            assert!(names.insert(name), "Duplicate plugin name found: {}", name);
        }
    }
}
