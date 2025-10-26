//! Version parsing and dependency source detection
//!
//! Language-agnostic utilities for parsing version specifiers and
//! detecting dependency sources (registry, git, path, workspace).

use mill_plugin_api::DependencySource;

/// Detect the type of dependency from a version/path specification string
///
/// Supports common patterns across package managers:
/// - Git URLs: `git+https://...`, `https://...git`, `git@github.com:...`
/// - Local paths: `./...`, `../...`, `file:...`
/// - Version ranges: everything else
///
/// # Example
///
/// ```rust
/// use mill_lang_common::versioning::detect_dependency_source;
/// use mill_plugin_api::DependencySource;
///
/// let git = detect_dependency_source("git+https://github.com/user/repo.git");
/// assert!(matches!(git, DependencySource::Git { .. }));
///
/// let path = detect_dependency_source("./local/package");
/// assert!(matches!(path, DependencySource::Path(_)));
///
/// let version = detect_dependency_source("^1.2.3");
/// assert!(matches!(version, DependencySource::Version(_)));
/// ```
pub fn detect_dependency_source(spec: &str) -> DependencySource {
    let spec = spec.trim();

    // Git sources
    if spec.starts_with("git+")
        || spec.starts_with("git@")
        || spec.contains(".git")
        || spec.starts_with("github:")
        || spec.starts_with("gitlab:")
        || spec.starts_with("bitbucket:")
    {
        let (url, rev) = parse_git_url(spec);
        return DependencySource::Git { url, rev };
    }

    // Path sources
    if spec.starts_with("./")
        || spec.starts_with("../")
        || spec.starts_with("file:")
        || spec.starts_with('/')
        || (cfg!(windows)
            && (
                // C:\path style
                (spec.len() >= 3 && spec.chars().nth(1) == Some(':'))
            // \\server\share UNC style
            || spec.starts_with(r"\\")
            ))
    {
        return DependencySource::Path(spec.to_string());
    }

    // HTTP/HTTPS tarballs
    if (spec.starts_with("http://") || spec.starts_with("https://")) && !spec.contains(".git") {
        return DependencySource::Path(spec.to_string());
    }

    // Default: version specifier
    DependencySource::Version(spec.to_string())
}

/// Parse a version specifier and extract the base version number
///
/// Strips common prefixes like `^`, `~`, `>=`, etc.
///
/// # Example
///
/// ```rust
/// use mill_lang_common::versioning::extract_version_number;
///
/// assert_eq!(extract_version_number("^1.2.3"), "1.2.3");
/// assert_eq!(extract_version_number("~2.0.0"), "2.0.0");
/// assert_eq!(extract_version_number(">=3.1.4"), "3.1.4");
/// assert_eq!(extract_version_number("1.0.0"), "1.0.0");
/// ```
pub fn extract_version_number(spec: &str) -> String {
    let spec = spec.trim();

    // Strip common version prefixes
    spec.trim_start_matches('^')
        .trim_start_matches('~')
        .trim_start_matches(">=")
        .trim_start_matches("<=")
        .trim_start_matches('>')
        .trim_start_matches('<')
        .trim_start_matches('=')
        .trim()
        .to_string()
}

/// Check if a version specifier allows any version
///
/// Returns true for patterns like `*`, `latest`, `x`, etc.
pub fn is_any_version(spec: &str) -> bool {
    matches!(spec.trim(), "*" | "latest" | "x" | "X" | "")
}

/// Check if a version spec is a range (contains operators)
pub fn is_version_range(spec: &str) -> bool {
    spec.contains("||")
        || spec.contains(" - ")
        || spec.starts_with('^')
        || spec.starts_with('~')
        || spec.starts_with('>')
        || spec.starts_with('<')
}

/// Parse a git URL and extract repository information
///
/// Returns (url, branch/tag/commit) tuple
pub fn parse_git_url(spec: &str) -> (String, Option<String>) {
    let spec = spec.trim().trim_start_matches("git+");

    // Check for #branch or #commit
    if let Some((url, ref_spec)) = spec.split_once('#') {
        (url.to_string(), Some(ref_spec.to_string()))
    } else {
        (spec.to_string(), None)
    }
}

/// Normalize a version string to semver format
///
/// Attempts to convert various version formats to x.y.z format
pub fn normalize_version(version: &str) -> String {
    let version = extract_version_number(version);

    // Split by . and take up to 3 parts
    let parts: Vec<&str> = version.split('.').take(3).collect();

    match parts.len() {
        0 => "0.0.0".to_string(),
        1 => format!("{}.0.0", parts[0]),
        2 => format!("{}.{}.0", parts[0], parts[1]),
        _ => parts.join("."),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_dependency_source_git() {
        assert!(matches!(
            detect_dependency_source("git+https://github.com/user/repo.git"),
            DependencySource::Git { .. }
        ));

        assert!(matches!(
            detect_dependency_source("git@github.com:user/repo.git"),
            DependencySource::Git { .. }
        ));

        assert!(matches!(
            detect_dependency_source("github:user/repo"),
            DependencySource::Git { .. }
        ));
    }

    #[test]
    fn test_detect_dependency_source_path() {
        assert!(matches!(
            detect_dependency_source("./local/package"),
            DependencySource::Path(_)
        ));

        assert!(matches!(
            detect_dependency_source("../shared/lib"),
            DependencySource::Path(_)
        ));

        assert!(matches!(
            detect_dependency_source("file:///absolute/path"),
            DependencySource::Path(_)
        ));

        // Windows paths (only tested on Windows)
        #[cfg(windows)]
        {
            assert!(matches!(
                detect_dependency_source(r"C:\Users\project"),
                DependencySource::Path(_)
            ));

            assert!(matches!(
                detect_dependency_source(r"\\server\share\path"),
                DependencySource::Path(_)
            ));
        }
    }

    #[test]
    fn test_detect_dependency_source_version() {
        assert!(matches!(
            detect_dependency_source("^1.2.3"),
            DependencySource::Version(_)
        ));

        assert!(matches!(
            detect_dependency_source("~2.0.0"),
            DependencySource::Version(_)
        ));

        assert!(matches!(
            detect_dependency_source("1.0.0"),
            DependencySource::Version(_)
        ));
    }

    #[test]
    fn test_extract_version_number() {
        assert_eq!(extract_version_number("^1.2.3"), "1.2.3");
        assert_eq!(extract_version_number("~2.0.0"), "2.0.0");
        assert_eq!(extract_version_number(">=3.1.4"), "3.1.4");
        assert_eq!(extract_version_number("1.0.0"), "1.0.0");
    }

    #[test]
    fn test_is_any_version() {
        assert!(is_any_version("*"));
        assert!(is_any_version("latest"));
        assert!(is_any_version("x"));
        assert!(is_any_version("X"));
        assert!(!is_any_version("1.0.0"));
    }

    #[test]
    fn test_is_version_range() {
        assert!(is_version_range("^1.0.0"));
        assert!(is_version_range("~2.0.0"));
        assert!(is_version_range(">=1.0.0"));
        assert!(is_version_range("1.0.0 - 2.0.0"));
        assert!(is_version_range("1.0.0 || 2.0.0"));
        assert!(!is_version_range("1.0.0"));
    }

    #[test]
    fn test_parse_git_url() {
        let (url, ref_spec) = parse_git_url("git+https://github.com/user/repo.git#main");
        assert_eq!(url, "https://github.com/user/repo.git");
        assert_eq!(ref_spec, Some("main".to_string()));

        let (url, ref_spec) = parse_git_url("https://github.com/user/repo.git");
        assert_eq!(url, "https://github.com/user/repo.git");
        assert_eq!(ref_spec, None);
    }

    #[test]
    fn test_normalize_version() {
        assert_eq!(normalize_version("1"), "1.0.0");
        assert_eq!(normalize_version("1.2"), "1.2.0");
        assert_eq!(normalize_version("1.2.3"), "1.2.3");
        assert_eq!(normalize_version("^1.2.3"), "1.2.3");
    }
}
