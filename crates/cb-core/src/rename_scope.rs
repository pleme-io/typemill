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

    /// Update examples directory
    #[serde(default = "default_true")]
    pub update_examples: bool,

    /// Update code comments (experimental, opt-in)
    #[serde(default)]
    pub update_comments: bool,

    /// Custom exclude patterns (glob patterns)
    #[serde(default)]
    pub exclude_patterns: Vec<String>,
}

fn default_true() -> bool {
    true
}

impl Default for RenameScope {
    fn default() -> Self {
        Self::all()
    }
}

impl RenameScope {
    /// Code-only preset: only update imports and string literals
    pub fn code_only() -> Self {
        Self {
            update_code: true,
            update_string_literals: true,
            update_docs: false,
            update_configs: false,
            update_examples: true,
            update_comments: false,
            exclude_patterns: vec![],
        }
    }

    /// All preset: update everything (default)
    pub fn all() -> Self {
        Self {
            update_code: true,
            update_string_literals: true,
            update_docs: true,
            update_configs: true,
            update_examples: true,
            update_comments: false,
            exclude_patterns: vec![],
        }
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
    fn test_code_only_preset() {
        let scope = RenameScope::code_only();
        assert!(scope.update_code);
        assert!(scope.update_string_literals);
        assert!(!scope.update_docs);
        assert!(!scope.update_configs);
    }

    #[test]
    fn test_all_preset() {
        let scope = RenameScope::all();
        assert!(scope.update_code);
        assert!(scope.update_string_literals);
        assert!(scope.update_docs);
        assert!(scope.update_configs);
        assert!(scope.update_examples);
        assert!(!scope.update_comments); // Still opt-in
    }

    #[test]
    fn test_should_include_file() {
        let scope = RenameScope::code_only();

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
            update_examples: true,
            update_comments: false,
            exclude_patterns: vec!["**/test_*".to_string(), "**/fixtures/**".to_string()],
        };

        assert!(!scope.should_include_file(Path::new("src/test_utils.rs")));
        assert!(!scope.should_include_file(Path::new("fixtures/example.md")));
        assert!(scope.should_include_file(Path::new("src/main.rs")));
    }
}
