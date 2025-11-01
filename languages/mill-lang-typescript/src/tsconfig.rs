//! TypeScript Configuration Parser
//!
//! Parses tsconfig.json files to extract compiler options, particularly
//! path mappings used for import resolution.

use anyhow::{Context, Result};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Represents a parsed tsconfig.json file
#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct TsConfig {
    /// Compiler options including path mappings
    #[serde(rename = "compilerOptions")]
    pub compiler_options: Option<CompilerOptions>,
}

/// TypeScript compiler options
#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct CompilerOptions {
    /// Base URL for resolving non-relative module names
    #[serde(rename = "baseUrl")]
    pub base_url: Option<String>,

    /// Path mappings for module resolution
    /// Example: { "$lib/*": ["src/lib/*"], "@/*": ["src/*"] }
    ///
    /// Uses IndexMap to preserve insertion order, which matches TypeScript's
    /// pattern matching behavior (first matching pattern wins).
    pub paths: Option<IndexMap<String, Vec<String>>>,
}

impl TsConfig {
    /// Parse tsconfig.json from a file
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the tsconfig.json file
    ///
    /// # Returns
    ///
    /// Parsed TsConfig or error if file cannot be read or parsed
    pub fn from_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read tsconfig.json at {:?}", path))?;

        // Strip JSON comments (tsconfig.json allows // comments)
        let content = strip_json_comments(&content);

        serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse tsconfig.json at {:?}", path))
    }

    /// Find the nearest tsconfig.json by walking up from a starting path
    ///
    /// # Arguments
    ///
    /// * `start_path` - Path to start searching from (typically a source file)
    ///
    /// # Returns
    ///
    /// Path to nearest tsconfig.json, or None if not found
    ///
    /// # Note
    ///
    /// This is a public API method. Internally, TypeScriptPathAliasResolver uses
    /// a cached version (find_nearest_tsconfig) for better performance.
    #[allow(dead_code)]
    pub fn find_nearest(start_path: &Path) -> Option<PathBuf> {
        let mut current = start_path.parent()?;

        loop {
            let candidate = current.join("tsconfig.json");
            if candidate.exists() {
                return Some(candidate);
            }

            // Move up one directory
            current = current.parent()?;
        }
    }

    /// Get the base URL as an absolute path
    ///
    /// # Arguments
    ///
    /// * `tsconfig_dir` - Directory containing the tsconfig.json file
    ///
    /// # Returns
    ///
    /// Absolute path to the base URL directory
    pub fn get_base_url(&self, tsconfig_dir: &Path) -> PathBuf {
        if let Some(ref compiler_options) = self.compiler_options {
            if let Some(ref base_url) = compiler_options.base_url {
                return tsconfig_dir.join(base_url);
            }
        }
        // Default to tsconfig directory if no baseUrl specified
        tsconfig_dir.to_path_buf()
    }
}

/// Strip JSON comments from content
///
/// TypeScript's tsconfig.json allows JavaScript-style comments (//, /* */),
/// but standard JSON parsers don't support them. This function removes
/// both line comments and block comments.
///
/// # Arguments
///
/// * `content` - JSON content with potential comments
///
/// # Returns
///
/// JSON content with all comments removed
///
/// # Supported Comment Styles
///
/// - Line comments: `// comment` (entire line or inline)
/// - Block comments: `/* comment */` (single-line or multi-line)
///
/// # Implementation
///
/// This is a state-machine-based parser that correctly handles:
/// - Inline comments: `"key": "value" // comment`
/// - Block comments: `/* comment */`
/// - Multi-line block comments
/// - Comments inside strings (preserved)
pub(crate) fn strip_json_comments(content: &str) -> String {
    let mut result = String::with_capacity(content.len());
    let mut chars = content.chars().peekable();
    let mut in_string = false;
    let mut in_line_comment = false;
    let mut in_block_comment = false;
    let mut escape_next = false;

    while let Some(ch) = chars.next() {
        // Handle string boundaries (don't process comments inside strings)
        if !in_line_comment && !in_block_comment {
            if ch == '"' && !escape_next {
                in_string = !in_string;
                result.push(ch);
                escape_next = false;
                continue;
            }

            if in_string {
                result.push(ch);
                escape_next = ch == '\\' && !escape_next;
                continue;
            }
        }

        // Handle escape sequences
        if in_string && escape_next {
            escape_next = false;
        }

        // Check for comment start
        if !in_string && !in_line_comment && !in_block_comment && ch == '/' {
            if let Some(&next_ch) = chars.peek() {
                if next_ch == '/' {
                    // Start of line comment
                    in_line_comment = true;
                    chars.next(); // consume second /
                    continue;
                } else if next_ch == '*' {
                    // Start of block comment
                    in_block_comment = true;
                    chars.next(); // consume *
                    continue;
                }
            }
        }

        // Handle line comment end (newline)
        if in_line_comment && ch == '\n' {
            in_line_comment = false;
            result.push(ch); // preserve newline
            continue;
        }

        // Handle block comment end (*/)
        if in_block_comment && ch == '*' {
            if let Some(&next_ch) = chars.peek() {
                if next_ch == '/' {
                    // End of block comment
                    in_block_comment = false;
                    chars.next(); // consume /
                    continue;
                }
            }
        }

        // Add character to result if not in a comment
        if !in_line_comment && !in_block_comment {
            result.push(ch);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_parse_tsconfig_with_path_mappings() {
        let config_json = r#"{
            "compilerOptions": {
                "baseUrl": ".",
                "paths": {
                    "$lib/*": ["src/lib/*"],
                    "@/*": ["src/*"]
                }
            }
        }"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(config_json.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let config = TsConfig::from_file(temp_file.path()).unwrap();

        assert!(config.compiler_options.is_some());
        let compiler_options = config.compiler_options.as_ref().unwrap();

        assert_eq!(compiler_options.base_url.as_deref(), Some("."));
        assert!(compiler_options.paths.is_some());

        let paths = compiler_options.paths.as_ref().unwrap();
        assert_eq!(paths.len(), 2);
        assert_eq!(paths.get("$lib/*").unwrap(), &vec!["src/lib/*"]);
        assert_eq!(paths.get("@/*").unwrap(), &vec!["src/*"]);
    }

    #[test]
    fn test_parse_tsconfig_with_comments() {
        let config_json = r#"{
            // This is a comment
            "compilerOptions": {
                "baseUrl": ".",
                // Path mappings for SvelteKit
                "paths": {
                    "$lib/*": ["src/lib/*"]
                }
            }
        }"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(config_json.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let config = TsConfig::from_file(temp_file.path()).unwrap();
        assert!(config.compiler_options.is_some());
    }

    #[test]
    fn test_get_base_url_with_explicit_path() {
        let config = TsConfig {
            compiler_options: Some(CompilerOptions {
                base_url: Some("src".to_string()),
                paths: None,
            }),
        };

        let tsconfig_dir = Path::new("/workspace/web");
        let base_url = config.get_base_url(tsconfig_dir);

        assert_eq!(base_url, Path::new("/workspace/web/src"));
    }

    #[test]
    fn test_get_base_url_defaults_to_tsconfig_dir() {
        let config = TsConfig {
            compiler_options: Some(CompilerOptions {
                base_url: None,
                paths: None,
            }),
        };

        let tsconfig_dir = Path::new("/workspace/web");
        let base_url = config.get_base_url(tsconfig_dir);

        assert_eq!(base_url, Path::new("/workspace/web"));
    }

    #[test]
    fn test_get_base_url_no_compiler_options() {
        let config = TsConfig {
            compiler_options: None,
        };

        let tsconfig_dir = Path::new("/workspace/web");
        let base_url = config.get_base_url(tsconfig_dir);

        assert_eq!(base_url, Path::new("/workspace/web"));
    }

    #[test]
    fn test_find_nearest_tsconfig() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();

        // Create directory structure: project/src/lib/
        let src_dir = project_root.join("src");
        let lib_dir = src_dir.join("lib");
        std::fs::create_dir_all(&lib_dir).unwrap();

        // Create tsconfig.json at project root
        let tsconfig_path = project_root.join("tsconfig.json");
        std::fs::write(&tsconfig_path, "{}").unwrap();

        // Create a test file in lib/
        let test_file = lib_dir.join("test.ts");
        std::fs::write(&test_file, "").unwrap();

        // Find tsconfig from test file
        let found = TsConfig::find_nearest(&test_file);
        assert!(found.is_some());
        assert_eq!(found.unwrap(), tsconfig_path);
    }

    #[test]
    fn test_find_nearest_tsconfig_not_found() {
        // Use a path that definitely has no tsconfig.json
        let temp_dir = tempfile::TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.ts");
        std::fs::write(&test_file, "").unwrap();

        // Should not find tsconfig.json (returns None when reaching filesystem root)
        let found = TsConfig::find_nearest(&test_file);
        assert!(found.is_none() || !found.unwrap().exists());
    }

    #[test]
    fn test_strip_json_comments() {
        let input = r#"{
            // Line comment
            "key": "value",
            // Another comment
            "key2": "value2"
        }"#;

        let output = strip_json_comments(input);

        // Should not contain comment lines
        assert!(!output.contains("// Line comment"));
        assert!(!output.contains("// Another comment"));

        // Should still contain JSON content
        assert!(output.contains(r#""key": "value""#));
        assert!(output.contains(r#""key2": "value2""#));
    }

    #[test]
    fn test_strip_inline_comments() {
        let input = r#"{
            "compilerOptions": { // TypeScript options
                "baseUrl": ".", // Base path
                "paths": { // Path mappings
                    "$lib/*": ["src/lib/*"] // SvelteKit alias
                }
            }
        }"#;

        let output = strip_json_comments(input);

        // Should not contain inline comments
        assert!(!output.contains("// TypeScript options"));
        assert!(!output.contains("// Base path"));
        assert!(!output.contains("// Path mappings"));
        assert!(!output.contains("// SvelteKit alias"));

        // Should still contain JSON content
        assert!(output.contains(r#""compilerOptions""#));
        assert!(output.contains(r#""baseUrl": ".""#));
        assert!(output.contains(r#""$lib/*""#));

        // Should parse as valid JSON
        let parsed: serde_json::Value = serde_json::from_str(&output)
            .expect("Output should be valid JSON");
        assert!(parsed.is_object());
    }

    #[test]
    fn test_strip_block_comments() {
        let input = r#"{
            /* This is a block comment */
            "compilerOptions": {
                "baseUrl": ".", /* inline block */
                "paths": {
                    "$lib/*": ["src/lib/*"]
                }
            }
        }"#;

        let output = strip_json_comments(input);

        // Should not contain block comments
        assert!(!output.contains("/* This is a block comment */"));
        assert!(!output.contains("/* inline block */"));

        // Should still contain JSON content
        assert!(output.contains(r#""compilerOptions""#));
        assert!(output.contains(r#""baseUrl": ".""#));

        // Should parse as valid JSON
        let parsed: serde_json::Value = serde_json::from_str(&output)
            .expect("Output should be valid JSON");
        assert!(parsed.is_object());
    }

    #[test]
    fn test_strip_multiline_block_comments() {
        let input = r#"{
            /*
             * Multi-line block comment
             * with multiple lines
             */
            "compilerOptions": {
                "baseUrl": ".",
                "paths": {
                    "$lib/*": ["src/lib/*"]
                }
            }
        }"#;

        let output = strip_json_comments(input);

        // Should not contain any part of block comment
        assert!(!output.contains("Multi-line block comment"));
        assert!(!output.contains("with multiple lines"));

        // Should parse as valid JSON
        let parsed: serde_json::Value = serde_json::from_str(&output)
            .expect("Output should be valid JSON");
        assert!(parsed.is_object());
    }

    #[test]
    fn test_preserve_comments_in_strings() {
        let input = r#"{
            "description": "This // is not a comment",
            "note": "Neither /* is */ this"
        }"#;

        let output = strip_json_comments(input);

        // Should preserve "comments" that are inside strings
        assert!(output.contains("This // is not a comment"));
        assert!(output.contains("Neither /* is */ this"));

        // Should parse as valid JSON
        let parsed: serde_json::Value = serde_json::from_str(&output)
            .expect("Output should be valid JSON");
        assert_eq!(parsed["description"], "This // is not a comment");
        assert_eq!(parsed["note"], "Neither /* is */ this");
    }

    #[test]
    fn test_parse_tsconfig_with_inline_and_block_comments() {
        // Real-world tsconfig.json with various comment styles
        let config_json = r#"{
            /* TypeScript Configuration */
            "compilerOptions": {
                "baseUrl": ".", // Project root
                "paths": { // Path mappings for module resolution
                    "$lib/*": ["src/lib/*"], // SvelteKit alias
                    "@/*": ["src/*"] /* Common Next.js pattern */
                }
            }
        }"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(config_json.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        // Should parse successfully
        let config = TsConfig::from_file(temp_file.path())
            .expect("Should parse tsconfig with comments");

        assert!(config.compiler_options.is_some());
        let compiler_options = config.compiler_options.as_ref().unwrap();

        assert_eq!(compiler_options.base_url.as_deref(), Some("."));
        assert!(compiler_options.paths.is_some());

        let paths = compiler_options.paths.as_ref().unwrap();
        assert_eq!(paths.len(), 2);
        assert_eq!(paths.get("$lib/*").unwrap(), &vec!["src/lib/*"]);
        assert_eq!(paths.get("@/*").unwrap(), &vec!["src/*"]);
    }
}
