//! Constants and regex patterns for Python plugin
//!
//! This module centralizes all hardcoded values used throughout the plugin,
//! making them easier to maintain and update.

use once_cell::sync::Lazy;
use regex::Regex;

// === Version Constants ===

/// Default Python version for new projects
pub const DEFAULT_PYTHON_VERSION: &str = "3.11";

/// Minimum supported Python version
pub const MIN_PYTHON_VERSION: &str = "3.8";

/// Parser version metadata
pub const PARSER_VERSION: &str = "0.1.0";

// === Regex Patterns ===

/// Pattern for detecting import statements
///
/// Matches: `import module`, `import module as alias`
pub static IMPORT_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^import\s+([\w.]+)(?:\s+as\s+(\w+))?")
        .expect("Python import regex pattern should be valid")
});

/// Pattern for detecting from...import statements
///
/// Matches: `from module import name`, `from module import name as alias`
pub static FROM_IMPORT_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^from\s+([\w.]+)\s+import\s+(.+)")
        .expect("Python from-import regex pattern should be valid")
});

/// Pattern for detecting function definitions
///
/// Matches: `def function_name(args):`, `async def function_name(args):`
pub static FUNCTION_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?m)^[ \t]*(async\s+)?def\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*\(")
        .expect("Python function pattern should be valid")
});

/// Pattern for detecting class definitions
///
/// Matches: `class ClassName:`, `class ClassName(Base):`
pub static CLASS_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?m)^[ \t]*class\s+([a-zA-Z_][a-zA-Z0-9_]*)")
        .expect("Python class pattern should be valid")
});

/// Pattern for detecting decorators
///
/// Matches: `@decorator`, `@decorator(args)`
pub static DECORATOR_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?m)^[ \t]*@([a-zA-Z_][a-zA-Z0-9_.]*)")
        .expect("Python decorator pattern should be valid")
});

/// Pattern for detecting variable assignments
///
/// Matches: `variable = value`, `CONSTANT = value`
pub static VARIABLE_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?m)^[ \t]*([a-zA-Z_][a-zA-Z0-9_]*)\s*=\s*")
        .expect("Python variable pattern should be valid")
});

// === Helper Functions ===

/// Generate pattern for qualified path matching
///
/// # Arguments
/// * `module_name` - Module name to match
///
/// # Returns
/// Regex pattern matching qualified references to the module
pub(crate) fn qualified_path_pattern(module_name: &str) -> Regex {
    let pattern = format!(r"\b{}\.[\w.]+", regex::escape(module_name));
    Regex::new(&pattern).unwrap_or_else(|_| {
        // Fallback for invalid regex
        Regex::new(r"[\w.]+\.[\w.]+").expect("Fallback pattern should be valid")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_import_pattern() {
        assert!(IMPORT_PATTERN.is_match("import os"));
        assert!(IMPORT_PATTERN.is_match("import os as operating_system"));
        assert!(IMPORT_PATTERN.is_match("import json.decoder"));
    }

    #[test]
    fn test_from_import_pattern() {
        assert!(FROM_IMPORT_PATTERN.is_match("from os import path"));
        assert!(FROM_IMPORT_PATTERN.is_match("from typing import List, Dict"));
    }

    #[test]
    fn test_function_pattern() {
        assert!(FUNCTION_PATTERN.is_match("def foo():\n    pass"));
        assert!(FUNCTION_PATTERN.is_match("async def bar():\n    pass"));
        assert!(FUNCTION_PATTERN.is_match("    def indented():\n        pass"));
    }

    #[test]
    fn test_class_pattern() {
        assert!(CLASS_PATTERN.is_match("class MyClass:\n    pass"));
        assert!(CLASS_PATTERN.is_match("class Derived(Base):\n    pass"));
    }
}
