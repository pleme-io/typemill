//! Integration tests for list_functions_harness
//!
//! These tests validate that the list_functions_harness correctly tests all discovered
//! language plugins for function listing capabilities.

use mill_test_support::harness::list_functions_harness;

/// Test that all plugins can list multiple functions
#[tokio::test]
async fn test_all_plugins_list_functions_multiple() {
    list_functions_harness::test_all_plugins_list_functions_multiple().await;
}

/// Test that all plugins correctly handle sources with no functions
#[tokio::test]
async fn test_all_plugins_list_functions_empty() {
    list_functions_harness::test_all_plugins_list_functions_empty().await;
}

/// Comprehensive test runner for all list_functions tests
#[tokio::test]
async fn test_all_plugins_list_functions() {
    list_functions_harness::test_all_plugins_list_functions().await;
}
