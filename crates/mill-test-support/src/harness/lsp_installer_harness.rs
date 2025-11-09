//! LSP Installer Test Harness
//!
//! Provides generic test utilities for testing LSP installer implementations
//! across all language plugins. This eliminates duplicated LSP installer tests
//! in individual language plugins.
//!
//! # Usage
//!
//! This harness is used via integration tests in `tests/lsp_installer_harness_integration.rs`.
//! Individual language plugins should NOT have their own LSP installer tests - the harness
//! automatically tests all discovered plugins.

#[allow(unused_imports)]
use crate::harness::plugin_discovery;

/// Tests that all LSP installers provide a non-empty LSP server name.
///
/// This is a basic sanity check that plugins implement the `lsp_name()` method correctly.
pub fn test_all_lsp_installers_have_names() {
    let plugins = plugin_discovery::get_test_registry().all().to_vec();

    assert!(
        !plugins.is_empty(),
        "No language plugins were discovered for LSP installer testing"
    );

    for plugin in plugins {
        let meta = plugin.metadata();

        if let Some(lsp_installer) = plugin.lsp_installer() {
            let lsp_name = lsp_installer.lsp_name();

            assert!(
                !lsp_name.is_empty(),
                "LSP installer for plugin '{}' must provide a non-empty LSP server name",
                meta.name
            );

            println!("✓ Plugin '{}' has LSP server: {}", meta.name, lsp_name);
        } else {
            println!("  Plugin '{}' has no LSP installer (config-only language)", meta.name);
        }
    }
}

/// Tests that all LSP installers can check installation status without panicking.
///
/// This doesn't assert the LSP is installed (may not be on CI/test systems),
/// but ensures the `check_installed()` method doesn't panic.
pub fn test_all_lsp_installers_can_check_availability() {
    let plugins = plugin_discovery::get_test_registry().all().to_vec();

    assert!(
        !plugins.is_empty(),
        "No language plugins were discovered for LSP installer testing"
    );

    for plugin in plugins {
        let meta = plugin.metadata();

        if let Some(lsp_installer) = plugin.lsp_installer() {
            // Just verify it doesn't panic - LSP may or may not be installed
            let check_result = lsp_installer.check_installed();

            // Log the result for debugging
            match check_result {
                Ok(Some(_path)) => println!("✓ Plugin '{}' LSP server is installed", meta.name),
                Ok(None) => println!("  Plugin '{}' LSP server is not installed (OK for tests)", meta.name),
                Err(e) => println!("  Plugin '{}' LSP check failed: {} (OK for tests)", meta.name, e),
            }
        } else {
            println!("  Plugin '{}' has no LSP installer (config-only language)", meta.name);
        }
    }
}
