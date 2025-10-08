//! Plugin discovery helpers for integration testing
//!
//! This module provides utilities for discovering language plugins
//! and their test fixtures at runtime. It enables truly dynamic
//! plugin testing where adding a new language plugin automatically
//! includes it in the test suite.

use cb_handlers::LanguagePluginRegistry;
use cb_plugin_api::{LanguagePlugin, LanguageTestFixtures};
use once_cell::sync::Lazy;

// Create a single, static instance of the plugin registry.
// This ensures that the registry and its plugins live for the entire
// duration of the test run, solving the lifetime issue.
static REGISTRY: Lazy<LanguagePluginRegistry> = Lazy::new(LanguagePluginRegistry::new);


/// Discover all installed language plugins that provide test fixtures
///
/// This function queries the plugin registry and returns all plugins
/// that have implemented the `test_fixtures()` method.
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
/// let plugins = discover_plugins_with_fixtures();
/// for (plugin, fixtures) in plugins {
///     println!("Found plugin: {}", plugin_language_name(plugin));
///     println!("  - {} complexity scenarios", fixtures.complexity_scenarios.len());
///     println!("  - {} refactoring scenarios", fixtures.refactoring_scenarios.len());
/// }
/// ```
pub fn discover_plugins_with_fixtures() -> Vec<(&'static dyn LanguagePlugin, LanguageTestFixtures)> {
    REGISTRY.plugins_with_fixtures()
}

/// Get the display name of a language plugin
///
/// Useful for logging and error messages.
pub fn plugin_language_name(plugin: &dyn LanguagePlugin) -> &str {
    plugin.metadata().name
}

/// Get the file extension for a language plugin
///
/// Returns the first registered extension (e.g., "py", "ts", "rs").
pub fn plugin_file_extension(plugin: &dyn LanguagePlugin) -> &str {
    plugin.metadata().extensions[0]
}