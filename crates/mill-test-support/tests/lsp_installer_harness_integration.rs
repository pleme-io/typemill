//! Integration tests for LSP installer harness
//!
//! These tests run against ALL discovered language plugins to ensure:
//! 1. All plugins provide LSP server names
//! 2. All plugins can check LSP installation status without panicking
//!
//! This replaces ~20 duplicate tests across 9 individual language plugins.

use mill_test_support::harness::lsp_installer_harness::{
    test_all_lsp_installers_can_check_availability, test_all_lsp_installers_have_names,
};

// Force linker to include plugin-bundle for inventory collection
#[cfg(test)]
extern crate mill_plugin_bundle;

#[test]
fn test_all_lsp_installers_have_names() {
    test_all_lsp_installers_have_names();
}

#[test]
fn test_all_lsp_installers_can_check_availability() {
    test_all_lsp_installers_can_check_availability();
}
