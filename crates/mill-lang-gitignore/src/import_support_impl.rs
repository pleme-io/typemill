//! Import/rename support implementation for .gitignore files

use mill_plugin_api::PluginResult;
use std::path::Path;

pub struct GitignoreImportSupport;

impl GitignoreImportSupport {
    pub fn new() -> Self {
        Self
    }

    /// Rewrite path patterns in .gitignore file
    ///
    /// Updates patterns that match the old path while preserving:
    /// - Comments (lines starting with #)
    /// - Blank lines
    /// - Generic glob patterns (*, **, etc.)
    pub fn rewrite_gitignore_patterns(
        &self,
        content: &str,
        old_path: &Path,
        new_path: &Path,
    ) -> PluginResult<(String, usize)> {
        let mut changes = 0;
        let old_path_str = old_path.to_string_lossy();
        let new_path_str = new_path.to_string_lossy();

        let updated_lines: Vec<String> = content
            .lines()
            .map(|line| {
                // Preserve comments and blank lines
                if line.trim().is_empty() || line.trim().starts_with('#') {
                    return line.to_string();
                }

                // Check if this line contains the old path
                if Self::should_update_pattern(line, &old_path_str) {
                    let updated = line.replace(&*old_path_str, &*new_path_str);
                    if updated != line {
                        changes += 1;
                        return updated;
                    }
                }

                line.to_string()
            })
            .collect();

        // Preserve original line ending style
        let result = if content.ends_with('\n') {
            format!("{}\n", updated_lines.join("\n"))
        } else {
            updated_lines.join("\n")
        };

        Ok((result, changes))
    }

    /// Determine if a pattern should be updated
    ///
    /// Returns true if the pattern contains the old path and is not a pure glob pattern
    fn should_update_pattern(pattern: &str, old_path: &str) -> bool {
        let trimmed = pattern.trim();

        // Skip pure glob patterns (no directory separators)
        if !trimmed.contains('/') && !trimmed.contains('\\') {
            return false;
        }

        // Check if pattern contains the old path
        trimmed.contains(old_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_update_directory_pattern() {
        let support = GitignoreImportSupport::new();
        let content = "tests/e2e/fixtures/\ntests/e2e/*.tmp\n";
        let old_path = Path::new("tests/e2e");
        let new_path = Path::new("tests/integration");

        let (result, changes) = support
            .rewrite_gitignore_patterns(content, old_path, new_path)
            .unwrap();

        assert_eq!(changes, 2);
        assert!(result.contains("tests/integration/fixtures/"));
        assert!(result.contains("tests/integration/*.tmp"));
        assert!(!result.contains("tests/e2e"));
    }

    #[test]
    fn test_preserve_comments() {
        let support = GitignoreImportSupport::new();
        let content = "# Build output\ntests/e2e/\n# End\n";
        let old_path = Path::new("tests/e2e");
        let new_path = Path::new("tests/integration");

        let (result, changes) = support
            .rewrite_gitignore_patterns(content, old_path, new_path)
            .unwrap();

        assert_eq!(changes, 1);
        assert!(result.contains("# Build output"));
        assert!(result.contains("# End"));
        assert!(result.contains("tests/integration/"));
    }

    #[test]
    fn test_preserve_generic_globs() {
        let support = GitignoreImportSupport::new();
        let content = "*.log\n**/*.tmp\ntarget/\ntests/e2e/\n";
        let old_path = Path::new("tests/e2e");
        let new_path = Path::new("tests/integration");

        let (result, changes) = support
            .rewrite_gitignore_patterns(content, old_path, new_path)
            .unwrap();

        assert_eq!(changes, 1);
        assert!(result.contains("*.log")); // Unchanged
        assert!(result.contains("**/*.tmp")); // Unchanged
        assert!(result.contains("target/")); // Unchanged
        assert!(result.contains("tests/integration/"));
    }

    #[test]
    fn test_preserve_blank_lines() {
        let support = GitignoreImportSupport::new();
        let content = "tests/e2e/\n\ntarget/\n";
        let old_path = Path::new("tests/e2e");
        let new_path = Path::new("tests/integration");

        let (result, changes) = support
            .rewrite_gitignore_patterns(content, old_path, new_path)
            .unwrap();

        assert_eq!(changes, 1);
        let lines: Vec<&str> = result.lines().collect();
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[1], ""); // Blank line preserved
    }

    #[test]
    fn test_no_changes_when_no_match() {
        let support = GitignoreImportSupport::new();
        let content = "*.log\ntarget/\n";
        let old_path = Path::new("tests/e2e");
        let new_path = Path::new("tests/integration");

        let (result, changes) = support
            .rewrite_gitignore_patterns(content, old_path, new_path)
            .unwrap();

        assert_eq!(changes, 0);
        assert_eq!(result, content);
    }
}
