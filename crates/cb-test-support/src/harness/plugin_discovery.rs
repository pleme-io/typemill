//! Plugin discovery helpers for integration testing
//!
//! This module provides utilities for discovering language plugins
//! and their test fixtures at runtime. It enables truly dynamic
//! plugin testing where adding a new language plugin automatically
//! includes it in the test suite.

use cb_plugin_api::{LanguagePlugin, LanguageTestFixtures, PluginRegistry};
use cb_services::services::registry_builder::build_language_plugin_registry;
use once_cell::sync::OnceCell;
use std::sync::Arc;

// Create a single, static instance of the plugin registry.
// This is initialized once and lives for the entire test run.
static REGISTRY: OnceCell<Arc<PluginRegistry>> = OnceCell::new();

fn get_or_init_registry() -> &'static Arc<PluginRegistry> {
    REGISTRY.get_or_init(build_language_plugin_registry)
}

/// Discover all installed language plugins that provide test fixtures.
///
/// This function queries the plugin registry and returns all plugins
/// that have implemented the `test_fixtures()` method.
///
/// # Returns
///
/// A vector of tuples containing:
/// - An `Arc` clone of the plugin.
/// - The test fixtures it provides.
pub fn discover_plugins_with_fixtures() -> Vec<(Arc<dyn LanguagePlugin>, LanguageTestFixtures)> {
    let registry = get_or_init_registry();
    registry
        .all()
        .iter()
        .filter_map(|plugin| plugin.test_fixtures().map(|fixtures| (plugin.clone(), fixtures)))
        .collect()
}

/// Get the display name of a language plugin.
///
/// Useful for logging and error messages.
pub fn plugin_language_name(plugin: &dyn LanguagePlugin) -> &str {
    plugin.metadata().name
}

/// Get the file extension for a language plugin.
///
/// Returns the first registered extension (e.g., "py", "ts", "rs").
pub fn plugin_file_extension(plugin: &dyn LanguagePlugin) -> &str {
    // It's safe to unwrap because the contract tests ensure extensions are not empty.
    plugin.metadata().extensions.first().unwrap()
}

/// Returns a reference to the global plugin registry for tests.
pub fn get_test_registry() -> &'static Arc<PluginRegistry> {
    get_or_init_registry()
}