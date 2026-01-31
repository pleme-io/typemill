//! TypeScript Configuration Parser
//!
//! Parses tsconfig.json files to extract compiler options, particularly
//! path mappings used for import resolution.

use anyhow::{Context, Result};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Represents a parsed tsconfig.json file
#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct TsConfig {
    /// Path to a base configuration file to inherit from
    pub extends: Option<String>,

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

/// Resolved TypeScript configuration with absolute paths
#[derive(Debug, Clone)]
pub(crate) struct ResolvedTsConfig {
    /// Effective base URL (absolute path)
    pub base_url: PathBuf,

    /// Path mappings with absolute replacement paths
    pub paths: IndexMap<String, Vec<PathBuf>>,

    /// Raw base URL string (used for inheritance)
    pub raw_base_url: Option<String>,
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

    /// Load and merge tsconfig.json with support for 'extends'
    pub fn load_and_merge(path: &Path) -> Result<ResolvedTsConfig> {
        let mut visited = HashSet::new();
        Self::load_and_merge_recursive(path, &mut visited)
    }

    fn load_and_merge_recursive(
        path: &Path,
        visited: &mut HashSet<PathBuf>,
    ) -> Result<ResolvedTsConfig> {
        let canonical_path = std::fs::canonicalize(path).unwrap_or(path.to_path_buf());
        if !visited.insert(canonical_path.clone()) {
            return Err(anyhow::anyhow!(
                "Circular extends dependency detected: {:?}",
                path
            ));
        }

        let config = Self::from_file(&canonical_path)?;
        let config_dir = canonical_path.parent().unwrap_or_else(|| Path::new("."));

        // 1. Load base config if 'extends' is present
        let mut resolved = if let Some(extends_path_str) = &config.extends {
            // Resolve extends path relative to current config file
            let extends_path = config_dir.join(extends_path_str);

            if extends_path.exists() {
                Self::load_and_merge_recursive(&extends_path, visited)?
            } else {
                // Fallback: start with empty config if extended file not found
                // (or maybe it's in node_modules, but we skip that for now)
                ResolvedTsConfig {
                    base_url: config_dir.to_path_buf(),
                    paths: IndexMap::new(),
                    raw_base_url: None,
                }
            }
        } else {
            // No extends: start with default empty config
            ResolvedTsConfig {
                base_url: config_dir.to_path_buf(),
                paths: IndexMap::new(),
                raw_base_url: None,
            }
        };

        // 2. Determine effective raw baseUrl (local overrides parent)
        let local_base_url_str = config
            .compiler_options
            .as_ref()
            .and_then(|opts| opts.base_url.clone());

        let effective_raw_base_url = local_base_url_str.or(resolved.raw_base_url);

        // 3. Determine absolute baseUrl for THIS config file
        let effective_base_url = if let Some(ref raw) = effective_raw_base_url {
            config_dir.join(raw)
        } else {
            config_dir.to_path_buf()
        };

        // Update resolved config with new base URL info
        resolved.base_url = effective_base_url.clone();
        resolved.raw_base_url = effective_raw_base_url;

        // 4. Resolve and merge paths
        if let Some(compiler_options) = config.compiler_options {
            if let Some(paths) = compiler_options.paths {
                for (pattern, replacements) in paths {
                    let abs_replacements: Vec<PathBuf> = replacements
                        .into_iter()
                        .map(|r| effective_base_url.join(r))
                        .collect();

                    // Child paths override parent paths (merge by key)
                    resolved.paths.insert(pattern, abs_replacements);
                }
            }
        }

        Ok(resolved)
    }

}

/// Strip JSON comments from content
///
/// TypeScript's tsconfig.json allows JavaScript-style comments (//, /* */),
/// but standard JSON parsers don't support them. This function removes
/// both line comments and block comments.
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
    use tempfile::{NamedTempFile, TempDir};

    fn create_file(path: &Path, content: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, content).unwrap();
    }

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
    fn test_extends_and_merge() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();

        // Base config: defines $lib
        let base_config = r#"{
            "compilerOptions": {
                "baseUrl": ".",
                "paths": {
                    "$lib/*": ["src/lib/*"],
                    "shared/*": ["shared/*"]
                }
            }
        }"#;
        create_file(&project_root.join("base.json"), base_config);

        // Child config: extends base, overrides shared, adds @
        let child_config = r#"{
            "extends": "./base.json",
            "compilerOptions": {
                "baseUrl": ".",
                "paths": {
                    "shared/*": ["new-shared/*"],
                    "@/*": ["src/*"]
                }
            }
        }"#;
        create_file(&project_root.join("tsconfig.json"), child_config);

        let resolved = TsConfig::load_and_merge(&project_root.join("tsconfig.json")).unwrap();

        // $lib should come from base
        assert!(resolved.paths.contains_key("$lib/*"));
        let lib_paths = resolved.paths.get("$lib/*").unwrap();
        // Base resolved absolute path
        assert!(lib_paths[0].ends_with("src/lib/*"));

        // shared should be overridden by child
        assert!(resolved.paths.contains_key("shared/*"));
        let shared_paths = resolved.paths.get("shared/*").unwrap();
        assert!(shared_paths[0].ends_with("new-shared/*"));

        // @ should come from child
        assert!(resolved.paths.contains_key("@/*"));
    }

    #[test]
    fn test_extends_nested_relative_resolution() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // structure:
        // /config/base.json (baseUrl: ".")
        // /app/tsconfig.json (extends: "../config/base.json")

        let config_dir = root.join("config");
        create_file(
            &config_dir.join("base.json"),
            r#"{
            "compilerOptions": {
                "baseUrl": ".",
                "paths": { "base/*": ["base-lib/*"] }
            }
        }"#,
        );

        let app_dir = root.join("app");
        create_file(
            &app_dir.join("tsconfig.json"),
            r#"{
            "extends": "../config/base.json",
            "compilerOptions": {
                "paths": { "app/*": ["app-lib/*"] }
            }
        }"#,
        );

        let resolved = TsConfig::load_and_merge(&app_dir.join("tsconfig.json")).unwrap();

        // Use canonical paths for assertion to avoid symlink/resolution issues
        let canonical_config_dir = std::fs::canonicalize(&config_dir).unwrap();
        let canonical_app_dir = std::fs::canonicalize(&app_dir).unwrap();

        // "base/*" should resolve relative to /config/base.json (because baseUrl="." in base)
        let base_paths = resolved.paths.get("base/*").unwrap();
        assert!(base_paths[0].starts_with(&canonical_config_dir));
        assert!(base_paths[0].ends_with("base-lib/*"));

        // "app/*" should resolve relative to /app/tsconfig.json (implicit baseUrl="." in child)
        let app_paths = resolved.paths.get("app/*").unwrap();
        assert!(app_paths[0].starts_with(&canonical_app_dir));
        assert!(app_paths[0].ends_with("app-lib/*"));
    }

    #[test]
    fn test_circular_extends() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        create_file(&root.join("a.json"), r#"{ "extends": "./b.json" }"#);
        create_file(&root.join("b.json"), r#"{ "extends": "./a.json" }"#);

        let result = TsConfig::load_and_merge(&root.join("a.json"));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Circular"));
    }
}
