//! Shared regex patterns for TypeScript/JavaScript import parsing
//!
//! Consolidates regex compilation to avoid repeated heap allocation.
//! Uses lazy_static for one-time compilation.

use once_cell::sync::Lazy;
use regex::Regex;

/// ES6 import pattern: import ... from 'module'
/// Matches: import { foo } from "module"
pub static ES6_IMPORT_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"import\s+.*?from\s+['"]([^'"]+)['"]"#)
        .expect("ES6 import regex should be valid")
});

/// CommonJS require pattern: require('module')
/// Matches: const foo = require("module")
pub static REQUIRE_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"require\s*\(\s*['"]([^'"]+)['"]\s*\)"#)
        .expect("require regex should be valid")
});

/// Dynamic import pattern: import('module')
/// Matches: import("module")
pub static DYNAMIC_IMPORT_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"import\s*\(\s*['"]([^'"]+)['"]\s*\)"#)
        .expect("dynamic import regex should be valid")
});

/// ES6 import pattern with line start anchor (for line-by-line parsing)
/// Matches: import ... from 'module' at line start
pub static ES6_IMPORT_LINE_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"^import\s+.*?from\s+['"]([^'"]+)['"]"#)
        .expect("ES6 import line regex should be valid")
});

/// Create a regex pattern to check if a module is imported
///
/// # Arguments
///
/// * `module` - Module specifier (will be escaped)
///
/// # Returns
///
/// Tuple of (es6_pattern, require_pattern, dynamic_pattern)
pub fn module_import_patterns(module: &str) -> (String, String, String) {
    let escaped = regex::escape(module);
    (
        format!(r#"from\s+['"]{escaped}['"]"#),
        format!(r#"require\s*\(\s*['"]{escaped}['"]\s*\)"#),
        format!(r#"import\s*\(\s*['"]{escaped}['"]\s*\)"#),
    )
}
