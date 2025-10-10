//! Plugin Contract Tests
//!
//! These tests are designed to be run against *all* discovered language plugins
//! to ensure they conform to the core `LanguagePlugin` trait contract. This
//! helps maintain quality and consistency as new language plugins are added.

use crate::harness::plugin_discovery;
use cb_plugin_api::{LanguagePlugin, PluginCapabilities};
use tokio::runtime::Runtime;

/// A test harness that runs a set of contract tests for each discovered plugin.
#[test]
fn test_all_plugins_conform_to_contract() {
    let plugins = plugin_discovery::get_test_registry().all().to_vec();
    assert!(!plugins.is_empty(), "No language plugins were discovered. Ensure plugins are linked and use codebuddy_plugin! macro.");

    let rt = Runtime::new().unwrap();

    for plugin in plugins {
        println!("Testing contract for plugin: {}", plugin.metadata().name);

        // Test 1: Metadata must be valid.
        test_metadata_contract(plugin.as_ref());

        // Test 2: Capabilities must be coherent.
        test_capabilities_contract(plugin.as_ref());

        // Test 3: Parsing must not panic on empty or simple input.
        rt.block_on(test_parsing_contract(plugin.as_ref()));
    }
}

/// Ensures that the plugin's metadata is well-formed.
fn test_metadata_contract(plugin: &dyn LanguagePlugin) {
    let meta = plugin.metadata();
    assert!(!meta.name.is_empty(), "Plugin name cannot be empty.");
    assert!(!meta.extensions.is_empty(), "Plugin must handle at least one file extension.");
    assert!(!meta.manifest_filename.is_empty(), "Plugin must specify a manifest filename.");
}

/// Ensures that the plugin's declared capabilities are consistent.
fn test_capabilities_contract(plugin: &dyn LanguagePlugin) {
    let caps = plugin.capabilities();

    if caps.imports {
        assert!(plugin.import_support().is_some(), "Plugin claims import support but provides no implementation.");
    } else {
        assert!(plugin.import_support().is_none(), "Plugin does not claim import support but provides an implementation.");
    }

    if caps.workspace {
        assert!(plugin.workspace_support().is_some(), "Plugin claims workspace support but provides no implementation.");
    } else {
        assert!(plugin.workspace_support().is_none(), "Plugin does not claim workspace support but provides an implementation.");
    }
}

/// Ensures that the plugin's parsing logic can handle basic cases without panicking.
async fn test_parsing_contract(plugin: &dyn LanguagePlugin) {
    // Test 3a: Must not panic on empty string.
    let empty_result = plugin.parse("").await;
    assert!(empty_result.is_ok(), "Parsing an empty string should not fail.");

    // Test 3b: Must not panic on simple, common text.
    let simple_result = plugin.parse("hello world").await;
    assert!(simple_result.is_ok(), "Parsing a simple string should not fail.");

    // Test 3c: analyze_manifest should fail gracefully for non-existent file.
    let manifest_result = plugin.analyze_manifest(std::path::Path::new("/__non_existent_file__")).await;
    assert!(manifest_result.is_err(), "Analyzing a non-existent manifest should fail.");
}