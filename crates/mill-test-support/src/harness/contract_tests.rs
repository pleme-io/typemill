//! Plugin Contract Tests
//!
//! These tests are designed to be run against *all* discovered language plugins
//! to ensure they conform to the core `LanguagePlugin` trait contract. This
//! helps maintain quality and consistency as new language plugins are added.

#[allow(unused_imports)]
use crate::harness::plugin_discovery;
use cb_plugin_api::LanguagePlugin;
#[allow(unused_imports)]
use tokio::runtime::Runtime;

// Force linker to include plugin-bundle for inventory collection in tests
// The bundle includes all language plugins without direct coupling
#[cfg(test)]
extern crate mill_plugin_bundle;

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
#[allow(dead_code)]
fn test_metadata_contract(plugin: &dyn LanguagePlugin) {
    let meta = plugin.metadata();
    assert!(!meta.name.is_empty(), "Plugin name cannot be empty.");
    assert!(
        !meta.extensions.is_empty(),
        "Plugin must handle at least one file extension."
    );
    assert!(
        !meta.manifest_filename.is_empty(),
        "Plugin must specify a manifest filename."
    );
}

/// Ensures that the plugin's declared capabilities are consistent.
#[allow(dead_code)]
fn test_capabilities_contract(plugin: &dyn LanguagePlugin) {
    let caps = plugin.capabilities();

    if caps.imports {
        // Check if any import support trait is implemented
        let has_import_support = plugin.import_parser().is_some()
            || plugin.import_rename_support().is_some()
            || plugin.import_move_support().is_some()
            || plugin.import_mutation_support().is_some()
            || plugin.import_advanced_support().is_some();

        assert!(
            has_import_support,
            "Plugin claims import support but provides no implementation."
        );
    } else {
        // Check that no import support traits are implemented
        let has_import_support = plugin.import_parser().is_some()
            || plugin.import_rename_support().is_some()
            || plugin.import_move_support().is_some()
            || plugin.import_mutation_support().is_some()
            || plugin.import_advanced_support().is_some();

        assert!(
            !has_import_support,
            "Plugin does not claim import support but provides an implementation."
        );
    }

    if caps.workspace {
        assert!(
            plugin.workspace_support().is_some(),
            "Plugin claims workspace support but provides no implementation."
        );
    } else {
        assert!(
            plugin.workspace_support().is_none(),
            "Plugin does not claim workspace support but provides an implementation."
        );
    }
}

/// Ensures that the plugin's parsing logic can handle basic cases without panicking.
#[allow(dead_code)]
async fn test_parsing_contract(plugin: &dyn LanguagePlugin) {
    let meta = plugin.metadata();

    // Use language-appropriate minimal valid syntax
    let valid_minimal_code = match meta.name.as_ref() {
        "rust" => "fn main() {}",
        "typescript" => "const x = 1;",
        _ => "", // Default to empty for unknown languages
    };

    // Test: Plugin can parse valid minimal code without panicking
    let parse_result = plugin.parse(valid_minimal_code).await;
    assert!(
        parse_result.is_ok() || parse_result.is_err(),
        "Parser must return Ok or Err, not panic"
    );

    // Test: Plugin fails gracefully on empty input (may succeed or fail, but no panic)
    let empty_result = plugin.parse("").await;
    let _ = empty_result; // Don't assert - just ensure no panic

    // Test: analyze_manifest fails gracefully for non-existent file
    let manifest_result = plugin
        .analyze_manifest(std::path::Path::new("/__non_existent_file__"))
        .await;
    assert!(
        manifest_result.is_err(),
        "Analyzing a non-existent manifest should fail."
    );
}
