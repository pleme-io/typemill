//! Constants for Rust language plugin
//!
//! This module contains all hardcoded values used throughout the plugin,
//! including regex patterns, version numbers, and other configuration values.

use regex::Regex;
use std::sync::OnceLock;

/// Default Rust edition for new projects
pub const DEFAULT_EDITION: &str = "2021";

/// Parser version for import graph metadata
#[allow(dead_code)] // Future enhancement: Parser versioning
pub const PARSER_VERSION: &str = "0.1.0";

/// Regex pattern for extracting Rust test annotations
///
/// Matches: `#[test]`, `#[tokio::test]`, `#[async_std::test]`, `#[actix_rt::test]`
#[allow(dead_code)]
pub fn test_patterns() -> Vec<Regex> {
    vec![
        Regex::new(r"#\[test\]").expect("Valid test pattern regex"),
        Regex::new(r"#\[tokio::test\]").expect("Valid tokio::test pattern regex"),
        Regex::new(r"#\[async_std::test\]").expect("Valid async_std::test pattern regex"),
        Regex::new(r"#\[actix_rt::test\]").expect("Valid actix_rt::test pattern regex"),
    ]
}

/// Regex patterns for extracting Rust assertion macros
///
/// Matches: `assert!`, `assert_eq!`, `assert_ne!`, `debug_assert!`, etc.
#[allow(dead_code)]
pub fn assertion_patterns() -> Vec<Regex> {
    vec![
        Regex::new(r"\bassert!\(").expect("Valid assert! pattern regex"),
        Regex::new(r"\bassert_eq!\(").expect("Valid assert_eq! pattern regex"),
        Regex::new(r"\bassert_ne!\(").expect("Valid assert_ne! pattern regex"),
        Regex::new(r"\bdebug_assert!\(").expect("Valid debug_assert! pattern regex"),
        Regex::new(r"\bdebug_assert_eq!\(").expect("Valid debug_assert_eq! pattern regex"),
        Regex::new(r"\bdebug_assert_ne!\(").expect("Valid debug_assert_ne! pattern regex"),
    ]
}

/// Get cached string literal pattern
static STRING_LITERAL_PATTERN: OnceLock<Regex> = OnceLock::new();

/// Regex pattern for matching double-quoted string literals (handles escapes)
///
/// Matches: `"hello"`, `"hello \"world\""`
pub fn string_literal_pattern() -> &'static Regex {
    STRING_LITERAL_PATTERN.get_or_init(|| {
        Regex::new(r#""([^"\\]*(\\.[^"\\]*)*)""#).expect("Valid string literal regex")
    })
}

/// Get cached raw string patterns
static RAW_STRING_PATTERNS: OnceLock<Vec<(Regex, usize)>> = OnceLock::new();

/// Regex patterns for raw string literals with different hash counts
///
/// Matches: r"...", r#"..."#, r##"..."##, etc.
pub fn raw_string_patterns() -> &'static Vec<(Regex, usize)> {
    RAW_STRING_PATTERNS.get_or_init(|| {
        vec![
            (
                Regex::new(r#"r"([^"]*)""#).expect("Valid raw string regex"),
                0,
            ),
            (
                Regex::new(r##"r#"(.*?)"#"##).expect("Valid raw string r# regex"),
                1,
            ),
            (
                Regex::new(r###"r##"(.*?)"##"###).expect("Valid raw string r## regex"),
                2,
            ),
            (
                Regex::new(r####"r###"(.*?)"###"####).expect("Valid raw string r### regex"),
                3,
            ),
            (
                Regex::new(r#####"r####"(.*?)"####"#####).expect("Valid raw string r#### regex"),
                4,
            ),
            (
                Regex::new(r######"r#####"(.*?)"#####"######)
                    .expect("Valid raw string r##### regex"),
                5,
            ),
        ]
    })
}

/// Get cached variable declaration pattern
static VARIABLE_DECL_PATTERN: OnceLock<Regex> = OnceLock::new();

/// Regex pattern for variable declarations (let, const)
///
/// Matches: `let x = ...;`, `const FOO = ...;`, `let mut y: Type = ...;`
pub fn variable_decl_pattern() -> &'static Regex {
    VARIABLE_DECL_PATTERN.get_or_init(|| {
        Regex::new(r"(?:let\s+(?:mut\s+)?|const\s+)(\w+)(?::\s*[^=]+)?\s*=\s*(.+?)(?:;|$)")
            .expect("Valid variable declaration regex")
    })
}

/// Generate a regex pattern for matching qualified paths
///
/// # Arguments
/// * `module_name` - The module name to match (e.g., "old_crate", "utils")
///
/// # Returns
/// A regex pattern matching qualified paths like `module_name::symbol`
///
/// # Examples
/// ```ignore
/// let pattern = qualified_path_pattern("utils");
/// // Matches: "utils::helper", "utils::SomeType"
/// ```
pub fn qualified_path_pattern(module_name: &str) -> Result<Regex, regex::Error> {
    let pattern = format!(r"\b{}\s*::", regex::escape(module_name));
    Regex::new(&pattern)
}

/// Generate a regex pattern for matching word-bounded identifiers
///
/// # Arguments
/// * `identifier` - The identifier to match (e.g., variable name)
///
/// # Returns
/// A regex pattern matching the identifier with word boundaries
///
/// # Examples
/// ```ignore
/// let pattern = word_boundary_pattern("var_name");
/// // Matches: "var_name" but not "my_var_name" or "var_name_2"
/// ```
pub fn word_boundary_pattern(identifier: &str) -> Result<Regex, regex::Error> {
    let pattern = format!(r"\b{}\b", regex::escape(identifier));
    Regex::new(&pattern)
}

/// Generate a fancy_regex pattern for matching identifiers without alphanumeric neighbors
///
/// This is used for comment updates to avoid partial matches.
///
/// # Arguments
/// * `basename` - The basename to match (e.g., "mill-lsp")
///
/// # Returns
/// A fancy_regex pattern string
///
/// # Examples
/// ```ignore
/// let pattern = smart_boundary_pattern("mill-lsp");
/// // Matches: "mill-lsp", "// mill-lsp", "mill-lsp-style"
/// // Blocks: "mymill-lsp", "mill-lspsystem"
/// ```
pub fn smart_boundary_pattern(basename: &str) -> String {
    format!(
        r"(?<![a-zA-Z0-9]){}(?![a-zA-Z0-9])",
        fancy_regex::escape(basename)
    )
}
