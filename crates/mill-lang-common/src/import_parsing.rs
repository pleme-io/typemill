//! Import statement parsing utilities
//!
//! Language-agnostic utilities for parsing and manipulating import statements.
//! Does NOT contain language-specific regex patterns - those should be in
//! individual language plugins.

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
/// use mill_lang_common::import_parsing::parse_import_alias;
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
/// use mill_lang_common::import_parsing::extract_package_name;
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

/// Split a qualified name and take the first N segments
///
/// This utility helps extract prefixes from qualified names using
/// language-specific delimiters:
/// - Java: `org.junit.jupiter.api.Test` with `.` delimiter
/// - Rust: `std::collections::HashMap` with `::` delimiter
/// - Go: `github.com/user/repo/pkg` with `/` delimiter
///
/// # Arguments
/// * `name` - The qualified name to split
/// * `delimiter` - The delimiter character(s) to split on
/// * `segments` - Number of segments to take from the beginning
///
/// # Returns
/// `Some(String)` with the prefix if enough segments exist, `None` otherwise
///
/// # Example
///
/// ```rust
/// use mill_lang_common::import_parsing::split_qualified_name_prefix;
///
/// // Java: Extract top-level package
/// assert_eq!(
///     split_qualified_name_prefix("org.junit.api.Test", ".", 2),
///     Some("org.junit".to_string())
/// );
///
/// // Rust: Extract root module
/// assert_eq!(
///     split_qualified_name_prefix("std::collections::HashMap", "::", 1),
///     Some("std".to_string())
/// );
///
/// // Go: Extract domain + org
/// assert_eq!(
///     split_qualified_name_prefix("github.com/user/repo", "/", 2),
///     Some("github.com/user".to_string())
/// );
///
/// // Not enough segments
/// assert_eq!(
///     split_qualified_name_prefix("foo", ".", 2),
///     None
/// );
/// ```
pub fn split_qualified_name_prefix(
    name: &str,
    delimiter: &str,
    segments: usize,
) -> Option<String> {
    let parts: Vec<&str> = name.split(delimiter).collect();
    if parts.len() >= segments {
        Some(parts[..segments].join(delimiter))
    } else {
        None
    }
}

/// Get the last segment of a qualified name
///
/// Extracts the final component from a qualified name, useful for
/// getting the simple name from a fully-qualified name.
///
/// # Arguments
/// * `name` - The qualified name to split
/// * `delimiter` - The delimiter character(s) to split on
///
/// # Returns
/// The last segment of the qualified name, or the entire name if no delimiter found
///
/// # Example
///
/// ```rust
/// use mill_lang_common::import_parsing::get_qualified_name_suffix;
///
/// assert_eq!(get_qualified_name_suffix("org.junit.Test", "."), "Test");
/// assert_eq!(get_qualified_name_suffix("std::HashMap", "::"), "HashMap");
/// assert_eq!(get_qualified_name_suffix("github.com/user/repo", "/"), "repo");
/// assert_eq!(get_qualified_name_suffix("SimpleType", "."), "SimpleType");
/// ```
pub fn get_qualified_name_suffix<'a>(name: &'a str, delimiter: &str) -> &'a str {
    name.rsplit(delimiter).next().unwrap_or(name)
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
    fn test_split_qualified_name_prefix() {
        // Java package names
        assert_eq!(
            split_qualified_name_prefix("org.junit.jupiter.api.Test", ".", 2),
            Some("org.junit".to_string())
        );
        assert_eq!(
            split_qualified_name_prefix("org.junit.Test", ".", 1),
            Some("org".to_string())
        );

        // Rust module paths
        assert_eq!(
            split_qualified_name_prefix("std::collections::HashMap", "::", 1),
            Some("std".to_string())
        );
        assert_eq!(
            split_qualified_name_prefix("std::collections::HashMap", "::", 2),
            Some("std::collections".to_string())
        );

        // Go import paths
        assert_eq!(
            split_qualified_name_prefix("github.com/user/repo/pkg", "/", 3),
            Some("github.com/user/repo".to_string())
        );

        // Not enough segments
        assert_eq!(split_qualified_name_prefix("foo", ".", 2), None);
        assert_eq!(split_qualified_name_prefix("foo.bar", ".", 3), None);

        // Edge cases
        assert_eq!(
            split_qualified_name_prefix("single", ".", 1),
            Some("single".to_string())
        );
    }

    #[test]
    fn test_get_qualified_name_suffix() {
        // Java class names
        assert_eq!(get_qualified_name_suffix("org.junit.Test", "."), "Test");
        assert_eq!(
            get_qualified_name_suffix("java.util.List", "."),
            "List"
        );

        // Rust types
        assert_eq!(
            get_qualified_name_suffix("std::collections::HashMap", "::"),
            "HashMap"
        );
        assert_eq!(get_qualified_name_suffix("std::Vec", "::"), "Vec");

        // Go packages
        assert_eq!(
            get_qualified_name_suffix("github.com/user/repo", "/"),
            "repo"
        );

        // Simple names (no delimiter)
        assert_eq!(get_qualified_name_suffix("SimpleType", "."), "SimpleType");
        assert_eq!(get_qualified_name_suffix("MyStruct", "::"), "MyStruct");

        // Single segment
        assert_eq!(get_qualified_name_suffix("foo", "."), "foo");
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
}
