//! Import statement parsing utilities
//!
//! Language-agnostic utilities for parsing and manipulating import statements.
//! Does NOT contain language-specific regex patterns - those should be in
//! individual language plugins.

use regex::Regex;

/// Parse "name as alias" pattern common across many languages
///
/// This pattern appears in:
/// - Python: `import foo as bar`, `from x import y as z`
/// - TypeScript: `import { foo as bar }`, `import foo as bar`
/// - Go: `import alias "package"`
///
/// # Example
///
/// ```rust
/// use cb_lang_common::import_parsing::parse_import_alias;
///
/// let (name, alias) = parse_import_alias("foo as bar");
/// assert_eq!(name, "foo");
/// assert_eq!(alias, Some("bar".to_string()));
///
/// let (name, alias) = parse_import_alias("baz");
/// assert_eq!(name, "baz");
/// assert_eq!(alias, None);
/// ```
pub fn parse_import_alias(text: &str) -> (String, Option<String>) {
    let text = text.trim();

    if let Some((name, alias)) = text.split_once(" as ") {
        (name.trim().to_string(), Some(alias.trim().to_string()))
    } else {
        (text.to_string(), None)
    }
}

/// Split comma-separated import list into individual items with aliases
///
/// Handles patterns like:
/// - `foo, bar, baz`
/// - `foo as f, bar, baz as b`
/// - `{ foo, bar as b }` (strips braces)
///
/// # Example
///
/// ```rust
/// use cb_lang_common::import_parsing::split_import_list;
///
/// let items = split_import_list("foo, bar as b, baz");
/// assert_eq!(items.len(), 3);
/// assert_eq!(items[0], ("foo".to_string(), None));
/// assert_eq!(items[1], ("bar".to_string(), Some("b".to_string())));
/// assert_eq!(items[2], ("baz".to_string(), None));
/// ```
pub fn split_import_list(text: &str) -> Vec<(String, Option<String>)> {
    let text = text.trim();

    // Remove surrounding braces if present
    let text = text
        .strip_prefix('{')
        .and_then(|s| s.strip_suffix('}'))
        .unwrap_or(text)
        .trim();

    // Split by comma and parse each item
    text.split(',')
        .map(|item| parse_import_alias(item.trim()))
        .filter(|(name, _)| !name.is_empty())
        .collect()
}

/// Detector for external vs internal dependencies
///
/// Configurable patterns for detecting whether an import path refers to
/// an external dependency (from a package manager) or an internal module
/// (relative/workspace import).
///
/// # Example
///
/// ```rust
/// use cb_lang_common::import_parsing::ExternalDependencyDetector;
///
/// let detector = ExternalDependencyDetector::new()
///     .with_relative_prefix("./")
///     .with_relative_prefix("../")
///     .with_internal_pattern(r"^@/");
///
/// assert!(!detector.is_external("./utils/helpers"));
/// assert!(!detector.is_external("../shared/config"));
/// assert!(!detector.is_external("@/components/Button"));
/// assert!(detector.is_external("react"));
/// assert!(detector.is_external("@types/node"));
/// ```
pub struct ExternalDependencyDetector {
    relative_prefixes: Vec<String>,
    internal_patterns: Vec<Regex>,
}

impl ExternalDependencyDetector {
    /// Create a new detector with no patterns
    pub fn new() -> Self {
        Self {
            relative_prefixes: Vec::new(),
            internal_patterns: Vec::new(),
        }
    }

    /// Add a relative path prefix (e.g., "./", "../")
    pub fn with_relative_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.relative_prefixes.push(prefix.into());
        self
    }

    /// Add an internal module pattern (regex)
    ///
    /// Use this for workspace-specific patterns like `@/` for path aliases.
    pub fn with_internal_pattern(mut self, pattern: &str) -> Self {
        if let Ok(regex) = Regex::new(pattern) {
            self.internal_patterns.push(regex);
        }
        self
    }

    /// Check if a module path is an external dependency
    ///
    /// Returns `false` if the path matches any relative prefix or internal pattern,
    /// `true` otherwise.
    pub fn is_external(&self, path: &str) -> bool {
        // Check for relative imports
        for prefix in &self.relative_prefixes {
            if path.starts_with(prefix) {
                return false;
            }
        }

        // Check for internal patterns
        for pattern in &self.internal_patterns {
            if pattern.is_match(path) {
                return false;
            }
        }

        // If none of the internal patterns matched, it's external
        true
    }

    /// Check if a module path is an internal/relative import
    pub fn is_internal(&self, path: &str) -> bool {
        !self.is_external(path)
    }
}

impl Default for ExternalDependencyDetector {
    fn default() -> Self {
        Self::new()
    }
}

/// Extract the package name from a scoped or namespaced import
///
/// Handles patterns like:
/// - `@scope/package` -> `@scope/package`
/// - `@scope/package/subpath` -> `@scope/package`
/// - `package/subpath` -> `package`
/// - `domain.com/org/package` -> `domain.com/org/package`
///
/// # Example
///
/// ```rust
/// use cb_lang_common::import_parsing::extract_package_name;
///
/// assert_eq!(extract_package_name("@types/node"), "@types/node");
/// assert_eq!(extract_package_name("@types/node/fs"), "@types/node");
/// assert_eq!(extract_package_name("lodash/fp"), "lodash");
/// assert_eq!(extract_package_name("github.com/user/repo"), "github.com/user/repo");
/// ```
pub fn extract_package_name(path: &str) -> String {
    // Handle scoped packages (@scope/package)
    if path.starts_with('@') {
        let parts: Vec<&str> = path.splitn(3, '/').collect();
        if parts.len() >= 2 {
            return format!("{}/{}", parts[0], parts[1]);
        }
        return path.to_string();
    }

    // Handle domain-based packages (github.com/org/repo)
    if path.contains('.') && path.contains('/') {
        let parts: Vec<&str> = path.splitn(4, '/').collect();
        if parts.len() >= 3 {
            return format!("{}/{}/{}", parts[0], parts[1], parts[2]);
        }
    }

    // Simple package (package or package/subpath)
    path.split('/').next().unwrap_or(path).to_string()
}

/// Normalize import path by removing quotes and whitespace
///
/// Handles various quote styles and trims whitespace.
pub fn normalize_import_path(path: &str) -> String {
    path.trim()
        .trim_matches('"')
        .trim_matches('\'')
        .trim_matches('`')
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_import_alias() {
        let (name, alias) = parse_import_alias("foo as bar");
        assert_eq!(name, "foo");
        assert_eq!(alias, Some("bar".to_string()));

        let (name, alias) = parse_import_alias("baz");
        assert_eq!(name, "baz");
        assert_eq!(alias, None);

        let (name, alias) = parse_import_alias("  foo  as  bar  ");
        assert_eq!(name, "foo");
        assert_eq!(alias, Some("bar".to_string()));
    }

    #[test]
    fn test_split_import_list() {
        let items = split_import_list("foo, bar as b, baz");
        assert_eq!(items.len(), 3);
        assert_eq!(items[0], ("foo".to_string(), None));
        assert_eq!(items[1], ("bar".to_string(), Some("b".to_string())));
        assert_eq!(items[2], ("baz".to_string(), None));

        let items = split_import_list("{ foo, bar }");
        assert_eq!(items.len(), 2);
        assert_eq!(items[0], ("foo".to_string(), None));
        assert_eq!(items[1], ("bar".to_string(), None));
    }

    #[test]
    fn test_external_dependency_detector() {
        let detector = ExternalDependencyDetector::new()
            .with_relative_prefix("./")
            .with_relative_prefix("../")
            .with_internal_pattern(r"^@/");

        assert!(!detector.is_external("./utils/helpers"));
        assert!(!detector.is_external("../shared/config"));
        assert!(!detector.is_external("@/components/Button"));
        assert!(detector.is_external("react"));
        assert!(detector.is_external("@types/node"));
    }

    #[test]
    fn test_extract_package_name() {
        assert_eq!(extract_package_name("@types/node"), "@types/node");
        assert_eq!(extract_package_name("@types/node/fs"), "@types/node");
        assert_eq!(extract_package_name("lodash/fp"), "lodash");
        assert_eq!(extract_package_name("lodash"), "lodash");
        assert_eq!(
            extract_package_name("github.com/user/repo"),
            "github.com/user/repo"
        );
        assert_eq!(
            extract_package_name("github.com/user/repo/subpkg"),
            "github.com/user/repo"
        );
    }

    #[test]
    fn test_normalize_import_path() {
        assert_eq!(normalize_import_path("\"./foo\""), "./foo");
        assert_eq!(normalize_import_path("'./bar'"), "./bar");
        assert_eq!(normalize_import_path("`./baz`"), "./baz");
        assert_eq!(normalize_import_path("  ./qux  "), "./qux");
    }
}
