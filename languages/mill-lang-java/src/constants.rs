//! Constants and regex patterns for Java language plugin
//!
//! This module centralizes all hardcoded values used throughout the plugin,
//! including regex patterns, version numbers, and other configuration values.

use once_cell::sync::Lazy;
use regex::Regex;

// === Version Constants ===

/// Default Java version for new projects
pub const DEFAULT_JAVA_VERSION: &str = "17";

/// Minimum supported Java version
pub const MIN_JAVA_VERSION: &str = "11";

/// Parser version for import graph metadata
pub const PARSER_VERSION: &str = "0.1.0";

// === Regex Patterns ===

/// Pattern for detecting test annotations
///
/// Matches:
/// - `@Test`
/// - Standard JUnit 5 test marker
pub static TEST_ANNOTATION: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"@Test").expect("Valid @Test annotation regex"));

/// Pattern for detecting parameterized test annotations
///
/// Matches:
/// - `@ParameterizedTest`
/// - JUnit 5 parameterized test marker
pub static PARAMETERIZED_TEST_ANNOTATION: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"@ParameterizedTest").expect("Valid @ParameterizedTest annotation regex")
});

/// Pattern for detecting repeated test annotations
///
/// Matches:
/// - `@RepeatedTest`
/// - JUnit 5 repeated test marker
pub static REPEATED_TEST_ANNOTATION: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"@RepeatedTest").expect("Valid @RepeatedTest annotation regex"));

/// Pattern for detecting assert statements
///
/// Matches:
/// - `assert condition;`
/// - Java built-in assertion keyword
pub static ASSERT_KEYWORD: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\bassert\b").expect("Valid assert keyword regex"));

/// Pattern for detecting assertEquals calls
///
/// Matches:
/// - `assertEquals(expected, actual)`
/// - JUnit assertion method
pub static ASSERT_EQUALS: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"assertEquals").expect("Valid assertEquals regex"));

/// Pattern for detecting assertTrue calls
///
/// Matches:
/// - `assertTrue(condition)`
/// - JUnit assertion method
pub static ASSERT_TRUE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"assertTrue").expect("Valid assertTrue regex"));

/// Pattern for detecting assertThat calls
///
/// Matches:
/// - `assertThat(actual, matcher)`
/// - AssertJ/Hamcrest assertion method
pub static ASSERT_THAT: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"assertThat").expect("Valid assertThat regex"));

/// Pattern for detecting method definitions
///
/// Matches:
/// - `public void methodName()`
/// - `private static int calculate(int x)`
/// - Method declarations with visibility, modifiers, return type, and name
pub static METHOD_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?m)^\s*(?:public|private|protected)?\s*(?:static)?\s*(?:final)?\s*\w+\s+(\w+)\s*\(",
    )
    .expect("Valid method pattern regex")
});

/// Pattern for detecting class definitions
///
/// Matches:
/// - `class MyClass`
/// - `public class Example`
/// - Class declarations with optional visibility modifier
pub static CLASS_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?m)^\s*(?:public|private|protected)?\s*(?:abstract|final)?\s*class\s+(\w+)")
        .expect("Valid class pattern regex")
});

/// Pattern for detecting interface definitions
///
/// Matches:
/// - `interface MyInterface`
/// - `public interface Example`
/// - Interface declarations with optional visibility modifier
pub static INTERFACE_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?m)^\s*(?:public|private|protected)?\s*interface\s+(\w+)")
        .expect("Valid interface pattern regex")
});

/// Pattern for detecting enum definitions
///
/// Matches:
/// - `enum Color`
/// - `public enum Status`
/// - Enum declarations with optional visibility modifier
pub static ENUM_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?m)^\s*(?:public|private|protected)?\s*enum\s+(\w+)")
        .expect("Valid enum pattern regex")
});

/// Pattern for detecting package declarations
///
/// Matches:
/// - `package com.example;`
/// - `package org.test.util;`
/// - Package declaration statements
pub static PACKAGE_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^\s*package\s+([\w.]+)\s*;").expect("Valid package pattern regex"));

/// Pattern for detecting import statements
///
/// Matches:
/// - `import java.util.List;`
/// - `import static org.junit.Assert.*;`
/// - Import declarations (both regular and static)
pub static IMPORT_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^\s*import\s+(?:static\s+)?([\w.]+(?:\.\*)?)\s*;")
        .expect("Valid import pattern regex")
});

/// Pattern for detecting wildcard import statements
///
/// Matches:
/// - `import java.util.*;`
/// - Import statements with wildcard (*)
pub static WILDCARD_IMPORT_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^\s*import\s+([\w.]+)\.\*\s*;").expect("Valid wildcard import pattern regex")
});

/// Pattern for detecting static import statements
///
/// Matches:
/// - `import static org.junit.Assert.assertEquals;`
/// - `import static com.example.Constants.*;`
/// - Static import declarations
pub static STATIC_IMPORT_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^\s*import\s+static\s+([\w.]+(?:\.\*)?)\s*;")
        .expect("Valid static import pattern regex")
});

/// Pattern for detecting qualified class names
///
/// Matches:
/// - `com.example.MyClass`
/// - `org.junit.Test`
/// - Fully qualified class references
pub static QUALIFIED_NAME_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\b([a-z][a-z0-9_]*\.)+[A-Z][a-zA-Z0-9_]*\b")
        .expect("Valid qualified name pattern regex")
});

// === Helper Functions ===

/// Get all test annotation patterns
///
/// Returns a vector of regex patterns for identifying test methods
#[allow(dead_code)]
pub(crate) fn test_patterns() -> Vec<Regex> {
    vec![
        TEST_ANNOTATION.clone(),
        PARAMETERIZED_TEST_ANNOTATION.clone(),
        REPEATED_TEST_ANNOTATION.clone(),
    ]
}

/// Get all assertion patterns
///
/// Returns a vector of regex patterns for identifying assertions
#[allow(dead_code)]
pub(crate) fn assertion_patterns() -> Vec<Regex> {
    vec![
        ASSERT_KEYWORD.clone(),
        ASSERT_EQUALS.clone(),
        ASSERT_TRUE.clone(),
        ASSERT_THAT.clone(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_test_annotation() {
        assert!(TEST_ANNOTATION.is_match("@Test"));
        assert!(TEST_ANNOTATION.is_match("    @Test"));
    }

    #[test]
    fn test_class_pattern() {
        assert!(CLASS_PATTERN.is_match("public class MyClass {"));
        assert!(CLASS_PATTERN.is_match("class Example {"));
        assert!(CLASS_PATTERN.is_match("  private class Inner {"));
    }

    #[test]
    fn test_interface_pattern() {
        assert!(INTERFACE_PATTERN.is_match("public interface MyInterface {"));
        assert!(INTERFACE_PATTERN.is_match("interface Example {"));
    }

    #[test]
    fn test_package_pattern() {
        assert!(PACKAGE_PATTERN.is_match("package com.example;"));
        assert!(PACKAGE_PATTERN.is_match("package org.test.util;"));
    }

    #[test]
    fn test_import_pattern() {
        assert!(IMPORT_PATTERN.is_match("import java.util.List;"));
        assert!(IMPORT_PATTERN.is_match("import static org.junit.Assert.*;"));
    }

    #[test]
    fn test_wildcard_import() {
        assert!(WILDCARD_IMPORT_PATTERN.is_match("import java.util.*;"));
        assert!(!WILDCARD_IMPORT_PATTERN.is_match("import java.util.List;"));
    }

    #[test]
    fn test_static_import() {
        assert!(STATIC_IMPORT_PATTERN.is_match("import static org.junit.Assert.assertEquals;"));
        assert!(STATIC_IMPORT_PATTERN.is_match("import static com.example.Constants.*;"));
    }

    #[test]
    fn test_assertion_patterns() {
        let patterns = assertion_patterns();
        assert_eq!(patterns.len(), 4);
    }

    #[test]
    fn test_test_patterns() {
        let patterns = test_patterns();
        assert_eq!(patterns.len(), 3);
    }
}
