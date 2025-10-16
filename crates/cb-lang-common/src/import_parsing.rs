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
