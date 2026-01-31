//! TypeScript Path Alias Resolver
//!
//! Implements path alias resolution for TypeScript projects using tsconfig.json
//! path mappings. Supports common patterns like:
//! - SvelteKit: `$lib/*` → `src/lib/*`
//! - Next.js: `@/*` → `src/*`
//! - Vite: `~/*` → `./*`

use crate::tsconfig::{ResolvedTsConfig, TsConfig};
use indexmap::IndexMap;
use mill_plugin_api::path_alias_resolver::PathAliasResolver;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// TypeScript-specific path alias resolver
///
/// Resolves import specifiers using tsconfig.json path mappings.
/// Caches parsed tsconfig.json files for performance.
pub struct TypeScriptPathAliasResolver {
    /// Cache of parsed tsconfig.json files (keyed by tsconfig.json path)
    /// Uses Arc to avoid cloning large configs on every cache hit
    tsconfig_cache: Arc<Mutex<HashMap<PathBuf, Arc<ResolvedTsConfig>>>>,

    /// Cache of tsconfig.json path lookups (keyed by directory)
    /// Maps directory → nearest tsconfig.json path
    /// Avoids repeated filesystem walks for find_nearest()
    tsconfig_path_cache: Arc<Mutex<HashMap<PathBuf, Option<PathBuf>>>>,
}

impl TypeScriptPathAliasResolver {
    /// Create a new TypeScript path alias resolver
    pub fn new() -> Self {
        Self {
            tsconfig_cache: Arc::new(Mutex::new(HashMap::new())),
            tsconfig_path_cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Find nearest tsconfig.json or jsconfig.json with caching
    ///
    /// # Arguments
    ///
    /// * `importing_file` - File to start searching from
    ///
    /// # Returns
    ///
    /// Path to nearest config file if found
    fn find_nearest_tsconfig(&self, importing_file: &Path) -> Option<PathBuf> {
        let mut current = importing_file.parent()?;
        let mut ancestors_to_cache: Vec<PathBuf> = Vec::new();

        // Walk up the directory tree
        loop {
            // Check cache first for this level
            {
                let cache = self.tsconfig_path_cache.lock().ok()?;
                if let Some(cached_result) = cache.get(current) {
                    // Cache hit! Clone the result before dropping lock
                    let result = cached_result.clone();
                    drop(cache); // Release read lock

                    // Cache all ancestors we collected with this result
                    if !ancestors_to_cache.is_empty() {
                        if let Ok(mut cache) = self.tsconfig_path_cache.lock() {
                            for ancestor in &ancestors_to_cache {
                                cache.insert(ancestor.clone(), result.clone());
                            }
                        }
                    }
                    return result;
                }
            }

            // Not in cache - check if config exists at this level
            // Priority: tsconfig.json > jsconfig.json
            let tsconfig = current.join("tsconfig.json");
            let jsconfig = current.join("jsconfig.json");

            let candidate = if tsconfig.exists() {
                Some(tsconfig)
            } else if jsconfig.exists() {
                Some(jsconfig)
            } else {
                None
            };

            if let Some(found) = candidate {
                // Found it! Cache this result for current directory AND all ancestors
                let result = Some(found);
                if let Ok(mut cache) = self.tsconfig_path_cache.lock() {
                    cache.insert(current.to_path_buf(), result.clone());
                    for ancestor in &ancestors_to_cache {
                        cache.insert(ancestor.clone(), result.clone());
                    }
                }
                return result;
            }

            // Remember this directory for caching later
            ancestors_to_cache.push(current.to_path_buf());

            // Move up one directory
            current = current.parent()?;
        }
    }

    /// Load tsconfig.json with caching and merging
    fn load_tsconfig(&self, tsconfig_path: &Path) -> Option<Arc<ResolvedTsConfig>> {
        // Check cache first
        {
            let cache = self.tsconfig_cache.lock().ok()?;
            if let Some(config) = cache.get(tsconfig_path) {
                // Cheap Arc clone - just increments reference count
                return Some(Arc::clone(config));
            }
        }

        // Parse, merge and cache
        // We use load_and_merge instead of from_file
        let config = Arc::new(TsConfig::load_and_merge(tsconfig_path).ok()?);
        {
            let mut cache = self.tsconfig_cache.lock().ok()?;
            cache.insert(tsconfig_path.to_path_buf(), Arc::clone(&config));
        }

        Some(config)
    }

    /// Try to match specifier against path mapping patterns
    fn match_path_alias(
        &self,
        specifier: &str,
        paths: &IndexMap<String, Vec<PathBuf>>,
    ) -> Option<String> {
        // IndexMap iteration preserves insertion order, so patterns are tried
        // in the order they appear in tsconfig.json
        for (pattern, replacements) in paths {
            if let Some(resolved) = self.try_match_pattern(specifier, pattern, replacements) {
                return Some(resolved);
            }
        }
        None
    }

    /// Try to match a single pattern
    fn try_match_pattern(
        &self,
        specifier: &str,
        pattern: &str,
        replacements: &[PathBuf],
    ) -> Option<String> {
        // Check if pattern contains a wildcard
        if let Some(star_idx) = pattern.find('*') {
            // Split pattern into prefix and suffix around the wildcard
            let prefix = &pattern[..star_idx];
            let suffix = &pattern[star_idx + 1..];

            // Check if specifier matches the pattern
            if specifier.starts_with(prefix) && specifier.ends_with(suffix) {
                // Extract the matched portion (what the wildcard captures)
                let captured = &specifier[prefix.len()..specifier.len() - suffix.len()];

                // Try each replacement path in order
                for replacement in replacements {
                    let replacement_str = replacement.to_string_lossy();

                    // Substitute captured string into replacement
                    // Replacement is an absolute PathBuf, so we convert to string and replace *
                    let resolved_path_str = if let Some(star_idx) = replacement_str.find('*') {
                        let mut result =
                            String::with_capacity(replacement_str.len() + captured.len());
                        result.push_str(&replacement_str[..star_idx]);
                        result.push_str(captured);
                        result.push_str(&replacement_str[star_idx + 1..]);
                        result
                    } else {
                        replacement_str.to_string()
                    };

                    let resolved = PathBuf::from(resolved_path_str);

                    // Return first replacement that exists on disk
                    if self.path_exists_with_extensions(&resolved) {
                        return Some(resolved.to_string_lossy().to_string());
                    }
                }

                // Fallback to first replacement if none exist
                if let Some(replacement) = replacements.first() {
                    let replacement_str = replacement.to_string_lossy();
                    let resolved_path_str = if let Some(star_idx) = replacement_str.find('*') {
                        let mut result =
                            String::with_capacity(replacement_str.len() + captured.len());
                        result.push_str(&replacement_str[..star_idx]);
                        result.push_str(captured);
                        result.push_str(&replacement_str[star_idx + 1..]);
                        result
                    } else {
                        replacement_str.to_string()
                    };
                    return Some(resolved_path_str);
                }
            }
        } else if pattern == specifier {
            // Exact match (no wildcard)
            for replacement in replacements {
                // Replacement is already absolute
                if self.path_exists_with_extensions(replacement) {
                    return Some(replacement.to_string_lossy().to_string());
                }
            }

            if let Some(replacement) = replacements.first() {
                return Some(replacement.to_string_lossy().to_string());
            }
        }

        None
    }

    /// Check if a path exists, trying various TypeScript/JavaScript extensions
    fn path_exists_with_extensions(&self, base_path: &Path) -> bool {
        // Try the path as-is first
        if base_path.exists() {
            return true;
        }

        // Try with common TypeScript/JavaScript extensions
        let extensions = ["", ".ts", ".tsx", ".js", ".jsx", ".d.ts"];
        for ext in &extensions {
            let path_with_ext = if ext.is_empty() {
                base_path.to_path_buf()
            } else {
                base_path.with_extension(ext.trim_start_matches('.'))
            };

            if path_with_ext.exists() {
                return true;
            }
        }

        // Try as a directory with index files
        let index_files = [
            "index.ts",
            "index.tsx",
            "index.js",
            "index.jsx",
            "index.d.ts",
        ];
        for index in &index_files {
            let index_path = base_path.join(index);
            if index_path.exists() {
                return true;
            }
        }

        false
    }
}

impl Default for TypeScriptPathAliasResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl PathAliasResolver for TypeScriptPathAliasResolver {
    fn resolve_alias(
        &self,
        specifier: &str,
        importing_file: &Path,
        _project_root: &Path,
    ) -> Option<String> {
        // 1. Find nearest tsconfig.json (with caching)
        let tsconfig_path = self.find_nearest_tsconfig(importing_file)?;

        // 2. Load and parse tsconfig
        let config = self.load_tsconfig(&tsconfig_path)?;

        // 3. Use paths from resolved config
        // base_url is implicit in resolved paths now
        self.match_path_alias(specifier, &config.paths)
    }

    fn is_potential_alias(&self, specifier: &str) -> bool {
        // Common TypeScript alias patterns
        specifier.starts_with('$')       // SvelteKit
            || specifier.starts_with('@')    // Next.js, common
            || specifier.starts_with('~')    // Vite, Nuxt
            || (!specifier.starts_with('.') && !specifier.starts_with('/')) // Could be bare specifier
    }
}

// Re-add helper for reverse resolution (path_to_alias)
impl TypeScriptPathAliasResolver {
    /// Convert an absolute file path back to its alias form
    pub fn path_to_alias(
        &self,
        absolute_path: &Path,
        importing_file: &Path,
        project_root: &Path,
    ) -> Option<String> {
        // 1. Find nearest tsconfig.json
        let tsconfig_path = self.find_nearest_tsconfig(importing_file)?;

        // 2. Load config
        let config = self.load_tsconfig(&tsconfig_path)?;

        // 3. Try each alias pattern in order
        for (pattern, replacements) in &config.paths {
            // Try each replacement path for this pattern
            for replacement in replacements {
                if let Some(alias) = self.try_convert_path_to_alias(
                    absolute_path,
                    pattern,
                    replacement,
                    &config.base_url,
                    project_root,
                ) {
                    return Some(alias);
                }
            }
        }

        None
    }

    fn try_convert_path_to_alias(
        &self,
        absolute_path: &Path,
        pattern: &str,
        replacement: &Path, // Changed to Path
        _base_url: &Path,
        _project_root: &Path,
    ) -> Option<String> {
        // Normalize the absolute path for comparison
        let mut path_to_check = absolute_path.to_path_buf();

        if path_to_check.extension().is_some() {
            let ext = path_to_check.extension().unwrap().to_string_lossy();
            if ext == "ts" || ext == "tsx" || ext == "js" || ext == "jsx" {
                path_to_check = path_to_check.with_extension("");
            }
        }

        // replacement is already absolute in ResolvedTsConfig
        let replacement_str = replacement.to_string_lossy();

        if let Some(star_idx) = pattern.find('*') {
            let pattern_prefix = &pattern[..star_idx];
            let pattern_suffix = &pattern[star_idx + 1..];

            if let Some(replacement_star_idx) = replacement_str.find('*') {
                // Replacement has wildcard
                // But replacement is absolute path.
                // E.g. /abs/path/src/*

                let replacement_prefix = &replacement_str[..replacement_star_idx];
                let replacement_suffix = &replacement_str[replacement_star_idx + 1..];

                // Check if path_to_check starts with replacement_prefix
                let replacement_prefix_path = Path::new(replacement_prefix);

                // We need to match string prefix for absolute paths because Path::strip_prefix works component-wise
                // but '*' might be partial component "src/foo*".
                // But normally '*' is a full component or suffix.
                // Assuming component-wise or simple string prefix if not.
                // Path::strip_prefix is safer.

                // If replacement_prefix ends with separator, it's a dir.
                // If it doesn't, it might be partial component.
                // PathBuf::from("/foo/src/*").parent() -> "/foo/src".

                // Let's rely on strip_prefix if star is component boundary.
                // If replacement is "/src/*", prefix is "/src/".

                if let Ok(relative) = path_to_check.strip_prefix(replacement_prefix_path) {
                    let relative_str = relative.to_string_lossy();
                    let captured = if !replacement_suffix.is_empty() {
                        relative_str.strip_suffix(replacement_suffix)?
                    } else {
                        &relative_str
                    };

                    let alias = if captured.is_empty() {
                        format!("{}{}", pattern_prefix.trim_end_matches('/'), pattern_suffix)
                    } else {
                        format!("{}{}{}", pattern_prefix, captured, pattern_suffix)
                    };
                    return Some(alias);
                }
            } else {
                // No wildcard in replacement (unusual)
                if let Ok(relative) = path_to_check.strip_prefix(replacement) {
                    let relative_str = relative.to_string_lossy();
                    let alias = format!("{}{}{}", pattern_prefix, relative_str, pattern_suffix);
                    return Some(alias);
                }
            }
        } else {
            // Exact match
            let expected_path = replacement.with_extension("");
            if path_to_check == expected_path {
                return Some(pattern.to_string());
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn create_test_tsconfig(dir: &Path, base_url: &str, paths: &[(&str, &[&str])]) -> PathBuf {
        let mut paths_json = String::from("{\n");
        for (i, (pattern, replacements)) in paths.iter().enumerate() {
            let replacements_json = replacements
                .iter()
                .map(|r| format!("\"{}\"", r))
                .collect::<Vec<_>>()
                .join(", ");
            paths_json.push_str(&format!(
                "      \"{}\": [{}]{}",
                pattern,
                replacements_json,
                if i < paths.len() - 1 { ",\n" } else { "\n" }
            ));
        }
        paths_json.push_str("    }");

        let config_json = format!(
            r#"{{
  "compilerOptions": {{
    "baseUrl": "{}",
    "paths": {}
  }}
}}"#,
            base_url, paths_json
        );

        let tsconfig_path = dir.join("tsconfig.json");
        let mut file = std::fs::File::create(&tsconfig_path).unwrap();
        file.write_all(config_json.as_bytes()).unwrap();
        file.flush().unwrap();

        tsconfig_path
    }

    #[test]
    fn test_resolve_sveltekit_lib_alias() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();
        create_test_tsconfig(project_root, ".", &[("$lib/*", &["src/lib/*"])]);

        let src_dir = project_root.join("src");
        std::fs::create_dir_all(&src_dir).unwrap();
        let test_file = src_dir.join("test.ts");
        std::fs::write(&test_file, "").unwrap();

        let resolver = TypeScriptPathAliasResolver::new();
        let resolved = resolver.resolve_alias("$lib/utils", &test_file, project_root);
        assert!(resolved.is_some());
        let resolved_path = resolved.unwrap();
        assert!(resolved_path.contains("src/lib/utils"));
    }

    #[test]
    fn test_resolve_nextjs_at_alias() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();
        create_test_tsconfig(project_root, ".", &[("@/*", &["src/*"])]);

        let test_file = project_root.join("src").join("test.ts");
        std::fs::create_dir_all(test_file.parent().unwrap()).unwrap();
        std::fs::write(&test_file, "").unwrap();

        let resolver = TypeScriptPathAliasResolver::new();
        let resolved = resolver.resolve_alias("@/components/Button", &test_file, project_root);
        assert!(resolved.is_some());
        assert!(resolved.unwrap().contains("src/components/Button"));
    }

    #[test]
    fn test_resolve_with_custom_base_url() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();
        create_test_tsconfig(project_root, "src", &[("@lib/*", &["lib/*"])]);

        let test_file = project_root.join("src").join("test.ts");
        std::fs::create_dir_all(test_file.parent().unwrap()).unwrap();
        std::fs::write(&test_file, "").unwrap();

        let resolver = TypeScriptPathAliasResolver::new();
        let resolved = resolver.resolve_alias("@lib/helpers", &test_file, project_root);
        assert!(resolved.is_some());
        assert!(resolved.unwrap().contains("src/lib/helpers"));
    }

    #[test]
    fn test_exact_match_alias() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();
        create_test_tsconfig(project_root, ".", &[("utils", &["src/utilities"])]);

        let test_file = project_root.join("test.ts");
        std::fs::write(&test_file, "").unwrap();

        let resolver = TypeScriptPathAliasResolver::new();
        let resolved = resolver.resolve_alias("utils", &test_file, project_root);
        assert!(resolved.is_some());
        assert!(resolved.unwrap().contains("src/utilities"));
    }

    #[test]
    fn test_no_match_returns_none() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();
        create_test_tsconfig(project_root, ".", &[("$lib/*", &["src/lib/*"])]);

        let test_file = project_root.join("test.ts");
        std::fs::write(&test_file, "").unwrap();

        let resolver = TypeScriptPathAliasResolver::new();
        let resolved = resolver.resolve_alias("./relative/path", &test_file, project_root);
        assert!(resolved.is_none());
    }

    // Additional tests for extends and jsconfig
    #[test]
    fn test_jsconfig_discovery() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        let jsconfig_path = root.join("jsconfig.json");
        std::fs::write(
            &jsconfig_path,
            r#"{
            "compilerOptions": {
                "baseUrl": ".",
                "paths": { "@/*": ["src/*"] }
            }
        }"#,
        )
        .unwrap();

        let test_file = root.join("test.js");
        std::fs::write(&test_file, "").unwrap();

        let resolver = TypeScriptPathAliasResolver::new();
        let resolved = resolver.resolve_alias("@/app", &test_file, root);

        assert!(resolved.is_some());
        assert!(resolved.unwrap().contains("src/app"));
    }

    #[test]
    fn test_tsconfig_prioritizes_over_jsconfig() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // jsconfig says @ -> js/*
        std::fs::write(
            root.join("jsconfig.json"),
            r#"{
            "compilerOptions": {
                "baseUrl": ".",
                "paths": { "@/*": ["js/*"] }
            }
        }"#,
        )
        .unwrap();

        // tsconfig says @ -> ts/*
        std::fs::write(
            root.join("tsconfig.json"),
            r#"{
            "compilerOptions": {
                "baseUrl": ".",
                "paths": { "@/*": ["ts/*"] }
            }
        }"#,
        )
        .unwrap();

        let test_file = root.join("test.ts");
        std::fs::write(&test_file, "").unwrap();

        let resolver = TypeScriptPathAliasResolver::new();
        let resolved = resolver.resolve_alias("@/app", &test_file, root);

        // Should use tsconfig (ts/*)
        assert!(resolved.is_some());
        assert!(resolved.unwrap().contains("ts/app"));
    }
}
