//! Integration tests for edge case test harness
//!
//! These tests run against ALL discovered language plugins to ensure:
//! 1. All plugins handle Unicode identifiers without panicking
//! 2. All plugins handle extremely long lines (15K+ chars)
//! 3. All plugins handle files without newlines
//! 4. All plugins handle mixed line endings (\n, \r\n, \r)
//! 5. All plugins handle empty files
//! 6. All plugins handle whitespace-only files
//! 7. All plugins handle special regex characters
//! 8. All plugins handle null bytes
//!
//! This replaces ~48 duplicate tests across 6 individual language plugins
//! (Python, Java, Go, C#, Swift, C) with 8 centralized tests.

use mill_test_support::harness::edge_case_tests::*;

// Force linker to include plugin-bundle for inventory collection
#[cfg(test)]
extern crate mill_plugin_bundle;

#[tokio::test]
async fn test_all_plugins_parse_unicode_identifiers() {
    test_all_plugins_parse_unicode_identifiers().await;
}

#[tokio::test]
async fn test_all_plugins_parse_extremely_long_line() {
    test_all_plugins_parse_extremely_long_line().await;
}

#[tokio::test]
async fn test_all_plugins_parse_no_newlines() {
    test_all_plugins_parse_no_newlines().await;
}

#[tokio::test]
async fn test_all_plugins_scan_mixed_line_endings() {
    test_all_plugins_scan_mixed_line_endings().await;
}

#[tokio::test]
async fn test_all_plugins_parse_empty_file() {
    test_all_plugins_parse_empty_file().await;
}

#[tokio::test]
async fn test_all_plugins_parse_whitespace_only() {
    test_all_plugins_parse_whitespace_only().await;
}

#[tokio::test]
async fn test_all_plugins_scan_special_regex_chars() {
    test_all_plugins_scan_special_regex_chars().await;
}

#[tokio::test]
async fn test_all_plugins_handle_null_bytes() {
    test_all_plugins_handle_null_bytes().await;
}

/// Comprehensive test that runs all edge case scenarios
#[tokio::test]
async fn test_comprehensive_edge_cases() {
    test_all_plugins_handle_edge_cases().await;
}
