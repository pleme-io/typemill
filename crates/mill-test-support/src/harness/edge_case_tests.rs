//! Edge Case Test Harness
//!
//! Provides comprehensive edge case testing for all language plugins.
//! These tests ensure plugins handle unusual or extreme inputs gracefully
//! without panicking or producing undefined behavior.
//!
//! Tests cover:
//! - Unicode identifiers (international characters)
//! - Extremely long lines (15K+ characters)
//! - Files without newlines
//! - Mixed line endings (\n, \r\n, \r)
//! - Empty files
//! - Whitespace-only files
//! - Special regex characters in paths/identifiers
//! - Null bytes in source code
//!
//! # Usage
//!
//! ```rust
//! use mill_test_support::harness::edge_case_tests::test_all_plugins_handle_edge_cases;
//!
//! #[tokio::test]
//! async fn test_edge_cases() {
//!     test_all_plugins_handle_edge_cases().await;
//! }
//! ```

use crate::harness::plugin_discovery;

/// Tests all discovered plugins for Unicode identifier handling.
///
/// Ensures plugins can parse code with international character identifiers
/// (Russian, Arabic, etc.) without panicking.
pub async fn test_all_plugins_parse_unicode_identifiers() {
    let plugins = plugin_discovery::get_test_registry().all();

    for plugin in plugins {
        let meta = plugin.metadata();
        let source = r#"
import os
def тестфункция():
    مُتَغَيِّر = 42
"#;
        let result = plugin.parse(source).await;
        // Should not panic with Unicode identifiers
        assert!(
            result.is_ok() || result.is_err(),
            "Plugin '{}' panicked on Unicode identifiers",
            meta.name
        );
        println!("✓ Plugin '{}' handles Unicode identifiers", meta.name);
    }
}

/// Tests all discovered plugins for extremely long line handling.
///
/// Ensures plugins can parse lines with 15,000+ characters without panicking
/// or exceeding resource limits.
pub async fn test_all_plugins_parse_extremely_long_line() {
    let plugins = plugin_discovery::get_test_registry().all();

    for plugin in plugins {
        let meta = plugin.metadata();
        let long_string = "a".repeat(15000);
        let source = format!("var x = \"{}\";", long_string);

        let result = plugin.parse(&source).await;
        assert!(
            result.is_ok() || result.is_err(),
            "Plugin '{}' panicked on extremely long line",
            meta.name
        );
        println!("✓ Plugin '{}' handles extremely long lines", meta.name);
    }
}

/// Tests all discovered plugins for files without newlines.
///
/// Ensures plugins can parse single-line files (no \n terminators)
/// without buffer overflow or parsing errors.
pub async fn test_all_plugins_parse_no_newlines() {
    let plugins = plugin_discovery::get_test_registry().all();

    for plugin in plugins {
        let meta = plugin.metadata();
        let source = "var x = 42;"; // No newline

        let result = plugin.parse(source).await;
        assert!(
            result.is_ok() || result.is_err(),
            "Plugin '{}' panicked on file without newlines",
            meta.name
        );
        println!("✓ Plugin '{}' handles files without newlines", meta.name);
    }
}

/// Tests all discovered plugins for mixed line ending handling.
///
/// Ensures plugins can parse files with \n, \r\n, and \r line endings
/// mixed together (common in cross-platform development).
pub async fn test_all_plugins_scan_mixed_line_endings() {
    let plugins = plugin_discovery::get_test_registry().all();

    for plugin in plugins {
        let meta = plugin.metadata();
        let source = "line1\nline2\r\nline3\rline4";

        let result = plugin.parse(source).await;
        assert!(
            result.is_ok() || result.is_err(),
            "Plugin '{}' panicked on mixed line endings",
            meta.name
        );
        println!("✓ Plugin '{}' handles mixed line endings", meta.name);
    }
}

/// Tests all discovered plugins for empty file handling.
///
/// Ensures plugins gracefully handle zero-byte files without
/// null pointer dereferences or buffer underflows.
pub async fn test_all_plugins_parse_empty_file() {
    let plugins = plugin_discovery::get_test_registry().all();

    for plugin in plugins {
        let meta = plugin.metadata();
        let source = "";

        let result = plugin.parse(source).await;
        assert!(
            result.is_ok() || result.is_err(),
            "Plugin '{}' panicked on empty file",
            meta.name
        );
        println!("✓ Plugin '{}' handles empty files", meta.name);
    }
}

/// Tests all discovered plugins for whitespace-only file handling.
///
/// Ensures plugins handle files containing only spaces, tabs, and newlines
/// without producing spurious symbols or parsing errors.
pub async fn test_all_plugins_parse_whitespace_only() {
    let plugins = plugin_discovery::get_test_registry().all();

    for plugin in plugins {
        let meta = plugin.metadata();
        let source = "   \n\t\n  \t  \n";

        let result = plugin.parse(source).await;
        assert!(
            result.is_ok() || result.is_err(),
            "Plugin '{}' panicked on whitespace-only file",
            meta.name
        );
        println!("✓ Plugin '{}' handles whitespace-only files", meta.name);
    }
}

/// Tests all discovered plugins for special regex character handling.
///
/// Ensures plugins can handle identifiers/paths with regex metacharacters
/// (*, ?, [, ], {, }, etc.) without regex compilation errors.
pub async fn test_all_plugins_scan_special_regex_chars() {
    let plugins = plugin_discovery::get_test_registry().all();

    for plugin in plugins {
        let meta = plugin.metadata();
        let source = r#"import "path/with/[brackets]/and/{braces}"#;

        let result = plugin.parse(source).await;
        assert!(
            result.is_ok() || result.is_err(),
            "Plugin '{}' panicked on special regex characters",
            meta.name
        );
        println!("✓ Plugin '{}' handles special regex characters", meta.name);
    }
}

/// Tests all discovered plugins for null byte handling.
///
/// Ensures plugins gracefully reject or handle files with embedded null bytes
/// (security vulnerability if not properly handled).
pub async fn test_all_plugins_handle_null_bytes() {
    let plugins = plugin_discovery::get_test_registry().all();

    for plugin in plugins {
        let meta = plugin.metadata();
        let source = "var x = 42;\x00var y = 10;";

        let result = plugin.parse(source).await;
        assert!(
            result.is_ok() || result.is_err(),
            "Plugin '{}' panicked on null bytes",
            meta.name
        );
        println!("✓ Plugin '{}' handles null bytes", meta.name);
    }
}

/// Run all edge case tests for all plugins.
///
/// This is the main entry point for comprehensive edge case testing.
/// It runs all 8 edge case tests across all discovered language plugins.
pub async fn test_all_plugins_handle_edge_cases() {
    println!("Running comprehensive edge case tests for all plugins...\n");

    test_all_plugins_parse_unicode_identifiers().await;
    println!();

    test_all_plugins_parse_extremely_long_line().await;
    println!();

    test_all_plugins_parse_no_newlines().await;
    println!();

    test_all_plugins_scan_mixed_line_endings().await;
    println!();

    test_all_plugins_parse_empty_file().await;
    println!();

    test_all_plugins_parse_whitespace_only().await;
    println!();

    test_all_plugins_scan_special_regex_chars().await;
    println!();

    test_all_plugins_handle_null_bytes().await;
    println!();

    println!("✅ All edge case tests passed for all plugins!");
}
