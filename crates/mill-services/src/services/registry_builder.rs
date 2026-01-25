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
//! `mill_plugin!` macro, and this builder collects them into the
//! `PluginRegistry`.

use mill_plugin_api::LanguagePlugin;
use mill_plugin_api::PluginDiscovery;
use std::sync::Arc;
use tracing::{debug, warn};

/// Build the language plugin registry from a list of plugins.
///
/// This function takes an already instantiated list of language plugins and
/// registers them into a `PluginDiscovery` instance. This allows the caller
/// (e.g., the binary entry point) to control which plugins are included,
/// decoupling the service layer from specific plugin crates.
///
/// # Arguments
/// * `plugins` - A list of instantiated language plugins
///
/// # Returns
/// An `Arc<PluginDiscovery>` containing the registered plugins.
pub fn build_language_plugin_registry(
    plugins: Vec<Arc<dyn LanguagePlugin>>,
) -> Arc<PluginDiscovery> {
    let mut registry = PluginDiscovery::new();
    let mut plugin_count = 0;

    debug!("Plugin registration started");

    for plugin in plugins {
        let name = plugin.metadata().name;
        debug!(plugin_name = %name, "Registering plugin");
        registry.register(plugin);
        plugin_count += 1;
    }

    debug!(plugin_count, "Plugin registration complete");

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

    #[test]
    fn test_registry_builder_creates_non_empty_registry() {
        // In the new architecture, we test that the registry builder correctly registers
        // the plugins we pass to it.
        use mill_plugin_api::{
            LanguagePlugin, ManifestData, ParsedSource, PluginCapabilities, PluginResult,
        };
        use mill_plugin_system::PluginMetadata;
        use std::any::Any;

        struct MockPlugin;

        #[async_trait::async_trait]
        impl LanguagePlugin for MockPlugin {
            fn metadata(&self) -> &mill_plugin_api::LanguageMetadata {
                static METADATA: mill_plugin_api::LanguageMetadata =
                    mill_plugin_api::LanguageMetadata {
                        name: "mock",
                        extensions: &["mock"],
                        manifest_filename: "Mockfile",
                        source_dir: "src",
                        entry_point: "lib.rs",
                        module_separator: "::",
                    };
                &METADATA
            }
            fn as_any(&self) -> &dyn Any {
                self
            }

            async fn parse(&self, _: &str) -> PluginResult<ParsedSource> {
                // Manually construct ParsedSource since it doesn't impl Default
                Ok(ParsedSource {
                    data: serde_json::Value::Null,
                    symbols: vec![],
                })
            }

            async fn analyze_manifest(&self, _: &std::path::Path) -> PluginResult<ManifestData> {
                // Manually construct ManifestData
                Ok(ManifestData {
                    name: "mock".to_string(),
                    version: "0.0.1".to_string(),
                    dependencies: vec![],
                    dev_dependencies: vec![],
                    raw_data: serde_json::Value::Null,
                })
            }

            fn capabilities(&self) -> PluginCapabilities {
                PluginCapabilities::default()
            }
        }

        let plugins: Vec<Arc<dyn LanguagePlugin>> = vec![Arc::new(MockPlugin)];
        let registry = build_language_plugin_registry(plugins);

        assert!(!registry.all().is_empty());
        assert!(registry.find_by_extension("mock").is_some());
    }
}
