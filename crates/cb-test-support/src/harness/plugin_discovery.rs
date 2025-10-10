//! Plugin discovery helpers for integration testing
//!
//! This module provides utilities for discovering language plugins
//! and their test fixtures at runtime. It enables truly dynamic
//! plugin testing where adding a new language plugin automatically
//! includes it in the test suite.

use cb_handlers::LanguagePluginRegistry;
use cb_plugin_api::{LanguagePlugin, LanguageTestFixtures};
use once_cell::sync::OnceCell;

// Create a single, static instance of the plugin registry.
// This ensures that the registry and its plugins live for the entire
// duration of the test run, solving the lifetime issue.
//
// IMPORTANT: This uses Handle::try_current() to detect if we're already
// in a tokio runtime (e.g., inside #[tokio::test]). If so, it uses that
// runtime to avoid the "Cannot start a runtime from within a runtime" panic.
// If no runtime exists, it creates one in a separate thread.
static REGISTRY: OnceCell<LanguagePluginRegistry> = OnceCell::new();

fn get_or_init_registry() -> &'static LanguagePluginRegistry {
    REGISTRY.get_or_init(|| {
        // ALWAYS spawn in a separate thread to avoid nested runtime issues
        // Even if we're in a tokio runtime, we can't block_on() from within it
        std::thread::spawn(|| {
            tokio::runtime::Runtime::new()
                .expect("Failed to create runtime for plugin registry")
                .block_on(LanguagePluginRegistry::new())
        })
        .join()
        .expect("Failed to join registry initialization thread")
    })
}

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
pub fn discover_plugins_with_fixtures() -> Vec<(&'static dyn LanguagePlugin, LanguageTestFixtures)>
{
    let registry = get_or_init_registry();
    registry.plugins_with_fixtures()
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
