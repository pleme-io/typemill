//! Rename scope configuration for controlling what gets updated during rename operations

use serde::{Deserialize, Serialize};
use std::path::Path;

/// Configuration for what file types to update during rename operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenameScope {
    /// Update code files (Rust, TypeScript imports)
    #[serde(default = "default_true")]
    pub update_code: bool,

    /// Update string literals in code
    #[serde(default = "default_true")]
    pub update_string_literals: bool,

    /// Update documentation files (.md)
    #[serde(default = "default_true")]
    pub update_docs: bool,

    /// Update configuration files (.toml, .yaml, .yml)
    #[serde(default = "default_true")]
    pub update_configs: bool,

    /// Update .gitignore pattern files
    #[serde(default = "default_true")]
    pub update_gitignore: bool,

    /// Update code comments (experimental, opt-in)
    #[serde(default)]
    pub update_comments: bool,

    /// Update inline code and prose paths in markdown files (opt-in)
    /// When false (default), only updates markdown links: [text](path)
    /// When true, also updates:
    /// - Inline code: `integration-tests/src/`
    /// - Code blocks containing paths
    /// - Plain text paths in tables/directory trees
    ///
    /// WARNING: May update code examples. Review changes carefully.
    #[serde(default)]
    pub update_markdown_prose: bool,

    /// Update exact identifier matches in config files (opt-in)
    /// When false (default), only updates path-like strings (containing / or \)
    /// When true, also updates exact word matches:
    /// - Array items: ["old-name"]
    /// - Config values: key = "old-name"
    /// - Identifiers bounded by quotes, brackets, or separators
    ///
    /// Useful for updating crate names in non-Cargo.toml configs
    /// (deny.toml, dependabot.yml, etc.)
    ///
    /// WARNING: May cause false positives. Review changes carefully.
    #[serde(default)]
    pub update_exact_matches: bool,

    /// Custom exclude patterns (glob patterns)
    #[serde(default)]
    pub exclude_patterns: Vec<String>,

    /// Convenience flag: enable all update options at once
    /// When true, sets all update_* flags to true (can be overridden individually)
    #[serde(default)]
    pub update_all: bool,
}

fn default_true() -> bool {
    true
}

impl Default for RenameScope {
    fn default() -> Self {
        Self::standard() // CHANGED FROM: Self::all()
    }
}

impl RenameScope {
    /// Code-only preset: only update imports and string literals
    #[deprecated(since = "2.0.0", note = "Use `code()` instead")]
    pub fn code_only() -> Self {
        Self {
            update_code: true,
            update_string_literals: true,
            update_docs: false,
            update_configs: false,
            update_gitignore: false,
            update_comments: false,
            update_markdown_prose: false,
            update_exact_matches: false,
            exclude_patterns: vec![],
            update_all: false,
        }
    }

    /// All preset: update everything (default)
    #[deprecated(since = "2.0.0", note = "Use `standard()` instead")]
    pub fn all() -> Self {
        Self {
            update_code: true,
            update_string_literals: true,
            update_docs: true,
            update_configs: true,
            update_gitignore: true,
            update_comments: false,
            update_markdown_prose: false, // Still opt-in for safety
            update_exact_matches: true,   // Enable for Cargo crate names and config files
            exclude_patterns: vec![],
            update_all: false,
        }
    }

    /// Code preset: only update imports and string literals
    ///
    /// This is the new name for `code_only()`. Use this for minimal scope.
    pub fn code() -> Self {
        Self::code_only() // Delegate to existing implementation
    }

    /// Standard preset: update code + docs + configs (DEFAULT)
    ///
    /// This is the new name for `all()` and the recommended default scope.
    /// Updates all structural parts of the project without touching comments or prose.
    pub fn standard() -> Self {
        Self::all() // Delegate to existing implementation
    }

    /// Comments preset: standard scope + code comments
    ///
    /// Adds code comment updates to the standard scope.
    pub fn comments() -> Self {
        let mut scope = Self::standard();
        scope.update_comments = true;
        scope
    }

    /// Everything preset: comments scope + markdown prose
    ///
    /// The most comprehensive scope - updates everything including prose text.
    pub fn everything() -> Self {
        let mut scope = Self::comments();
        scope.update_markdown_prose = true;
        scope
    }

    /// Resolve the update_all flag by enabling all update options
    /// Individual flags take precedence if explicitly set after update_all
    pub fn resolve_update_all(mut self) -> Self {
        if self.update_all {
            self.update_code = true;
            self.update_string_literals = true;
            self.update_docs = true;
            self.update_configs = true;
            self.update_gitignore = true;
            self.update_comments = true;
            self.update_markdown_prose = true;
            self.update_exact_matches = true;
        }
        self
    }

    /// Check if this scope is comprehensive (all file types enabled)
    ///
    /// A comprehensive scope means we should scan ALL files matching the scope,
    /// not just files we detect as having references. This ensures 100% coverage
    /// for operations like `--update-all`.
    ///
    /// # Returns
    ///
    /// `true` if `update_all` is set OR all major file types are enabled
    pub fn is_comprehensive(&self) -> bool {
        self.update_all
            || (self.update_code
                && self.update_docs
                && self.update_configs
                && self.update_string_literals)
    }

    /// Check if a file path should be included based on scope
    pub fn should_include_file(&self, path: &Path) -> bool {
        // Check exclude patterns first
        let path_str = path.to_string_lossy();
        for pattern in &self.exclude_patterns {
            if let Ok(glob_pattern) = glob::Pattern::new(pattern) {
                if glob_pattern.matches(&path_str) {
                    return false;
                }
            }
        }

        // Check for special filenames without extensions
        if let Some(filename) = path.file_name().and_then(|f| f.to_str()) {
            if filename == ".gitignore" {
                return self.update_gitignore;
            }
        }

        // Check by extension
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            match ext {
                "md" | "markdown" => self.update_docs,
                "toml" => self.update_configs,
                "yaml" | "yml" => self.update_configs,
                "rs" | "ts" | "tsx" | "js" | "jsx" => self.update_code,
                _ => true, // Unknown extensions included by default
            }
        } else {
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_code_preset() {
        let scope = RenameScope::code();
        assert!(scope.update_code);
        assert!(scope.update_string_literals);
        assert!(!scope.update_docs);
        assert!(!scope.update_configs);
        assert!(!scope.update_all);
    }

    #[test]
    fn test_standard_preset() {
        let scope = RenameScope::standard();
        assert!(scope.update_code);
        assert!(scope.update_string_literals);
        assert!(scope.update_docs);
        assert!(scope.update_configs);
        assert!(!scope.update_comments); // Still opt-in
        assert!(!scope.update_all);
    }

    #[test]
    fn test_comments_preset() {
        let scope = RenameScope::comments();
        assert!(scope.update_code);
        assert!(scope.update_string_literals);
        assert!(scope.update_docs);
        assert!(scope.update_configs);
        assert!(scope.update_comments); // This is the key addition
        assert!(!scope.update_markdown_prose); // Still opt-in
    }

    #[test]
    fn test_everything_preset() {
        let scope = RenameScope::everything();
        assert!(scope.update_code);
        assert!(scope.update_string_literals);
        assert!(scope.update_docs);
        assert!(scope.update_configs);
        assert!(scope.update_comments);
        assert!(scope.update_markdown_prose); // This is the key addition
    }

    #[test]
    fn test_deprecated_aliases() {
        // Verify old names still work
        #[allow(deprecated)]
        let code_only = RenameScope::code_only();
        let code = RenameScope::code();
        assert_eq!(code_only.update_code, code.update_code);
        assert_eq!(code_only.update_docs, code.update_docs);

        #[allow(deprecated)]
        let all = RenameScope::all();
        let standard = RenameScope::standard();
        assert_eq!(all.update_code, standard.update_code);
        assert_eq!(all.update_docs, standard.update_docs);
    }

    #[test]
    fn test_update_all_flag() {
        let scope = RenameScope {
            update_all: true,
            ..RenameScope::default()
        }.resolve_update_all();

        assert!(scope.update_code);
        assert!(scope.update_string_literals);
        assert!(scope.update_docs);
        assert!(scope.update_configs);
        assert!(scope.update_comments);
        assert!(scope.update_markdown_prose);
        assert!(scope.update_exact_matches);
    }

    #[test]
    fn test_update_all_with_override() {
        let scope = RenameScope {
            update_all: true,
            update_comments: false, // Override: don't update comments
            ..RenameScope::default()
        }.resolve_update_all();

        assert!(scope.update_code);
        assert!(scope.update_docs);
        // update_comments is set BEFORE resolve, so it stays true from update_all
        // Individual overrides need to happen AFTER resolve, not in struct initialization
        assert!(scope.update_comments);
    }

    #[test]
    fn test_should_include_file() {
        let scope = RenameScope::code();

        assert!(scope.should_include_file(Path::new("src/main.rs")));
        assert!(!scope.should_include_file(Path::new("README.md")));
        assert!(!scope.should_include_file(Path::new("config.toml")));
    }

    #[test]
    fn test_exclude_patterns() {
        let scope = RenameScope {
            update_code: true,
            update_docs: true,
            update_configs: true,
            update_string_literals: true,
            update_comments: false,
            update_markdown_prose: false,
            update_exact_matches: false,
            update_all: false,
            exclude_patterns: vec!["**/test_*".to_string(), "**/fixtures/**".to_string()],
        };

        assert!(!scope.should_include_file(Path::new("src/test_utils.rs")));
        assert!(!scope.should_include_file(Path::new("fixtures/example.md")));
        assert!(scope.should_include_file(Path::new("src/main.rs")));
    }

    #[test]
    fn test_markdown_prose_opt_in() {
        let default_scope = RenameScope::standard();
        assert!(!default_scope.update_markdown_prose); // Opt-in by default

        let custom_scope = RenameScope {
            update_markdown_prose: true,
            ..RenameScope::standard()
        };
        assert!(custom_scope.update_markdown_prose);
    }
}
