//! TypeScript Path Alias Resolver
//!
//! Implements path alias resolution for TypeScript projects using tsconfig.json
//! path mappings. Supports common patterns like:
//! - SvelteKit: `$lib/*` → `src/lib/*`
//! - Next.js: `@/*` → `src/*`
//! - Vite: `~/*` → `./*`

use crate::tsconfig::TsConfig;
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
    tsconfig_cache: Arc<Mutex<HashMap<PathBuf, Arc<TsConfig>>>>,

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

    /// Find nearest tsconfig.json with caching
    ///
    /// # Arguments
    ///
    /// * `importing_file` - File to start searching from
    ///
    /// # Returns
    ///
    /// Path to nearest tsconfig.json if found
    fn find_nearest_tsconfig(&self, importing_file: &Path) -> Option<PathBuf> {
        // Use parent directory as cache key
        let dir = importing_file.parent()?;

        // Check cache first
        {
            let cache = self.tsconfig_path_cache.lock().ok()?;
            if let Some(cached_result) = cache.get(dir) {
                return cached_result.clone();
            }
        }

        // Not cached - perform filesystem walk
        let result = TsConfig::find_nearest(importing_file);

        // Cache result (including None results to avoid repeated failed lookups)
        {
            if let Ok(mut cache) = self.tsconfig_path_cache.lock() {
                cache.insert(dir.to_path_buf(), result.clone());
            }
        }

        result
    }

    /// Load tsconfig.json with caching
    ///
    /// # Arguments
    ///
    /// * `tsconfig_path` - Path to the tsconfig.json file
    ///
    /// # Returns
    ///
    /// Parsed TsConfig wrapped in Arc if successful, None on error
    fn load_tsconfig(&self, tsconfig_path: &Path) -> Option<Arc<TsConfig>> {
        // Check cache first
        {
            let cache = self.tsconfig_cache.lock().ok()?;
            if let Some(config) = cache.get(tsconfig_path) {
                // Cheap Arc clone - just increments reference count
                return Some(Arc::clone(config));
            }
        }

        // Parse and cache
        let config = Arc::new(TsConfig::from_file(tsconfig_path).ok()?);
        {
            let mut cache = self.tsconfig_cache.lock().ok()?;
            cache.insert(tsconfig_path.to_path_buf(), Arc::clone(&config));
        }

        Some(config)
    }

    /// Try to match specifier against path mapping patterns
    ///
    /// # Arguments
    ///
    /// * `specifier` - Import specifier (e.g., "$lib/utils")
    /// * `paths` - Path mappings from tsconfig.json (IndexMap preserves order)
    /// * `base_url` - Base URL directory for resolving paths
    ///
    /// # Returns
    ///
    /// Resolved path if match found, None otherwise
    ///
    /// # Pattern Matching Order
    ///
    /// Patterns are matched in declaration order (IndexMap preserves insertion order).
    /// This matches TypeScript's behavior: the first matching pattern wins.
    /// This is critical for overlapping patterns like:
    /// - "@api/models/*" → "src/api/models/*"  (more specific)
    /// - "@api/*" → "src/api-v2/*"             (less specific)
    fn match_path_alias(
        &self,
        specifier: &str,
        paths: &IndexMap<String, Vec<String>>,
        base_url: &Path,
    ) -> Option<String> {
        // IndexMap iteration preserves insertion order, so patterns are tried
        // in the order they appear in tsconfig.json
        for (pattern, replacements) in paths {
            if let Some(resolved) = self.try_match_pattern(specifier, pattern, replacements, base_url) {
                return Some(resolved);
            }
        }
        None
    }

    /// Try to match a single pattern
    ///
    /// Handles both exact matches and wildcard patterns (e.g., "$lib/*")
    ///
    /// # Phase 1 Implementation
    ///
    /// This implementation handles:
    /// - Exact matches (no wildcards)
    /// - Simple `/*` suffix patterns (most common case)
    ///
    /// Future phases can add:
    /// - Multiple wildcards
    /// - Wildcard in middle of pattern
    /// - Complex glob patterns
    fn try_match_pattern(
        &self,
        specifier: &str,
        pattern: &str,
        replacements: &[String],
        base_url: &Path,
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

                // Note: The prefix/suffix matching already handles separator requirements
                // E.g., "@api/models/*" has prefix "@api/models/" which won't match "@apiModels"

                // Try each replacement path in order (TypeScript behavior)
                // TypeScript tries replacements sequentially until one resolves
                // CRITICAL: Always loop through ALL replacements, not just when len > 1
                for replacement in replacements {
                    let resolved_path = replacement.replace('*', captured);
                    let resolved = base_url.join(&resolved_path);

                    // Return first replacement that exists on disk
                    if self.path_exists_with_extensions(&resolved) {
                        return Some(resolved.to_string_lossy().to_string());
                    }
                }

                // If none of the replacements exist, fall back to the first one
                // This allows resolution to continue downstream (e.g., for dry-run scenarios)
                if let Some(replacement) = replacements.first() {
                    let resolved_path = replacement.replace('*', captured);
                    let resolved = base_url.join(&resolved_path);
                    return Some(resolved.to_string_lossy().to_string());
                }
            }
        } else if pattern == specifier {
            // Exact match (no wildcard)
            // CRITICAL: Always loop through ALL replacements, not just when len > 1
            for replacement in replacements {
                let resolved = base_url.join(replacement);

                // Return first replacement that exists on disk
                if self.path_exists_with_extensions(&resolved) {
                    return Some(resolved.to_string_lossy().to_string());
                }
            }

            // If none of the replacements exist, fall back to the first one
            if let Some(replacement) = replacements.first() {
                let resolved = base_url.join(replacement);
                return Some(resolved.to_string_lossy().to_string());
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
        let index_files = ["index.ts", "index.tsx", "index.js", "index.jsx", "index.d.ts"];
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

        // 3. Extract compiler options
        let compiler_options = config.compiler_options.as_ref()?;
        let paths = compiler_options.paths.as_ref()?;

        // 4. Determine base URL (relative to tsconfig.json directory)
        let tsconfig_dir = tsconfig_path.parent()?;
        let base_url = config.get_base_url(tsconfig_dir);

        // 5. Try to match specifier against path mappings
        self.match_path_alias(specifier, paths, &base_url)
    }

    fn is_potential_alias(&self, specifier: &str) -> bool {
        // Common TypeScript alias patterns
        specifier.starts_with('$')       // SvelteKit
            || specifier.starts_with('@')    // Next.js, common
            || specifier.starts_with('~')    // Vite, Nuxt
            || (!specifier.starts_with('.') && !specifier.starts_with('/')) // Could be bare specifier
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn create_test_tsconfig(dir: &Path, base_url: &str, paths: &[(&str, &[&str])]) -> PathBuf {
        // Manually construct JSON to preserve insertion order
        // (serde_json::Map uses HashMap which doesn't preserve order)
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

        // Create tsconfig.json with SvelteKit $lib mapping
        create_test_tsconfig(project_root, ".", &[("$lib/*", &["src/lib/*"])]);

        // Create a test file
        let src_dir = project_root.join("src");
        std::fs::create_dir_all(&src_dir).unwrap();
        let test_file = src_dir.join("test.ts");
        std::fs::write(&test_file, "").unwrap();

        let resolver = TypeScriptPathAliasResolver::new();

        // Test resolution
        let resolved = resolver.resolve_alias("$lib/utils", &test_file, project_root);
        assert!(resolved.is_some());

        let resolved_path = resolved.unwrap();
        assert!(resolved_path.contains("src/lib/utils") || resolved_path.ends_with("src/lib/utils"));
    }

    #[test]
    fn test_resolve_nextjs_at_alias() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();

        // Create tsconfig.json with Next.js @ mapping
        create_test_tsconfig(project_root, ".", &[("@/*", &["src/*"])]);

        let test_file = project_root.join("src").join("test.ts");
        std::fs::create_dir_all(test_file.parent().unwrap()).unwrap();
        std::fs::write(&test_file, "").unwrap();

        let resolver = TypeScriptPathAliasResolver::new();

        let resolved = resolver.resolve_alias("@/components/Button", &test_file, project_root);
        assert!(resolved.is_some());

        let resolved_path = resolved.unwrap();
        assert!(
            resolved_path.contains("src/components/Button")
                || resolved_path.ends_with("src/components/Button")
        );
    }

    #[test]
    fn test_resolve_with_custom_base_url() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();

        // Create tsconfig.json with custom baseUrl
        create_test_tsconfig(project_root, "src", &[("@lib/*", &["lib/*"])]);

        let test_file = project_root.join("src").join("test.ts");
        std::fs::create_dir_all(test_file.parent().unwrap()).unwrap();
        std::fs::write(&test_file, "").unwrap();

        let resolver = TypeScriptPathAliasResolver::new();

        let resolved = resolver.resolve_alias("@lib/helpers", &test_file, project_root);
        assert!(resolved.is_some());

        let resolved_path = resolved.unwrap();
        // Should resolve relative to baseUrl (src)
        assert!(resolved_path.contains("src/lib/helpers") || resolved_path.ends_with("src/lib/helpers"));
    }

    #[test]
    fn test_exact_match_alias() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();

        // Create tsconfig.json with exact match alias (no wildcard)
        create_test_tsconfig(project_root, ".", &[("utils", &["src/utilities"])]);

        let test_file = project_root.join("test.ts");
        std::fs::write(&test_file, "").unwrap();

        let resolver = TypeScriptPathAliasResolver::new();

        let resolved = resolver.resolve_alias("utils", &test_file, project_root);
        assert!(resolved.is_some());

        let resolved_path = resolved.unwrap();
        assert!(
            resolved_path.contains("src/utilities") || resolved_path.ends_with("src/utilities")
        );
    }

    #[test]
    fn test_no_match_returns_none() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();

        create_test_tsconfig(project_root, ".", &[("$lib/*", &["src/lib/*"])]);

        let test_file = project_root.join("test.ts");
        std::fs::write(&test_file, "").unwrap();

        let resolver = TypeScriptPathAliasResolver::new();

        // Try to resolve a non-alias specifier
        let resolved = resolver.resolve_alias("./relative/path", &test_file, project_root);
        assert!(resolved.is_none());
    }

    #[test]
    fn test_is_potential_alias() {
        let resolver = TypeScriptPathAliasResolver::new();

        // Should recognize common alias patterns
        assert!(resolver.is_potential_alias("$lib/utils"));
        assert!(resolver.is_potential_alias("@/components"));
        assert!(resolver.is_potential_alias("~/helpers"));

        // Bare specifiers might be aliases
        assert!(resolver.is_potential_alias("utils"));

        // Relative paths are not aliases
        assert!(!resolver.is_potential_alias("./utils"));
        assert!(!resolver.is_potential_alias("../utils"));
        assert!(!resolver.is_potential_alias("/absolute/path"));
    }

    #[test]
    fn test_caching_works() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();

        create_test_tsconfig(project_root, ".", &[("$lib/*", &["src/lib/*"])]);

        let test_file = project_root.join("test.ts");
        std::fs::write(&test_file, "").unwrap();

        let resolver = TypeScriptPathAliasResolver::new();

        // First resolution - should parse and cache
        let resolved1 = resolver.resolve_alias("$lib/utils", &test_file, project_root);
        assert!(resolved1.is_some());

        // Second resolution - should use cache
        let resolved2 = resolver.resolve_alias("$lib/helpers", &test_file, project_root);
        assert!(resolved2.is_some());

        // Cache should have one entry
        let cache = resolver.tsconfig_cache.lock().unwrap();
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn test_missing_tsconfig_returns_none() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();

        // No tsconfig.json created
        let test_file = project_root.join("test.ts");
        std::fs::write(&test_file, "").unwrap();

        let resolver = TypeScriptPathAliasResolver::new();

        let resolved = resolver.resolve_alias("$lib/utils", &test_file, project_root);
        assert!(resolved.is_none());
    }

    #[test]
    fn test_multiple_replacements_uses_first() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();

        // Create tsconfig with multiple replacement paths
        create_test_tsconfig(
            project_root,
            ".",
            &[("@lib/*", &["src/lib/*", "src/shared/*"])],
        );

        let test_file = project_root.join("test.ts");
        std::fs::write(&test_file, "").unwrap();

        let resolver = TypeScriptPathAliasResolver::new();

        let resolved = resolver.resolve_alias("@lib/utils", &test_file, project_root);
        assert!(resolved.is_some());

        // Should use first replacement (Phase 1 behavior)
        let resolved_path = resolved.unwrap();
        assert!(resolved_path.contains("src/lib/utils") || resolved_path.ends_with("src/lib/utils"));
    }

    #[test]
    fn test_nested_path_resolution() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();

        create_test_tsconfig(project_root, ".", &[("$lib/*", &["src/lib/*"])]);

        let test_file = project_root.join("src").join("routes").join("test.ts");
        std::fs::create_dir_all(test_file.parent().unwrap()).unwrap();
        std::fs::write(&test_file, "").unwrap();

        let resolver = TypeScriptPathAliasResolver::new();

        // Should still find tsconfig.json by walking up
        let resolved = resolver.resolve_alias("$lib/server/core/orchestrator", &test_file, project_root);
        assert!(resolved.is_some());

        let resolved_path = resolved.unwrap();
        assert!(
            resolved_path.contains("src/lib/server/core/orchestrator")
                || resolved_path.ends_with("src/lib/server/core/orchestrator")
        );
    }

    #[test]
    fn test_overlapping_aliases_preserves_order() {
        // This test verifies the fix for the HashMap ordering bug
        // When multiple patterns overlap, TypeScript uses the FIRST matching pattern
        // IndexMap preserves insertion order, so patterns are matched in declaration order

        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();

        // Create tsconfig with overlapping patterns
        // "@api/models/*" is more specific and should match first
        // "@api/*" is less specific and should only match if "@api/models/*" doesn't
        create_test_tsconfig(
            project_root,
            ".",
            &[
                ("@api/models/*", &["src/api/models/*"]),  // More specific (first)
                ("@api/*", &["src/api-v2/*"]),              // Less specific (second)
            ],
        );

        let test_file = project_root.join("src").join("test.ts");
        std::fs::create_dir_all(test_file.parent().unwrap()).unwrap();
        std::fs::write(&test_file, "").unwrap();

        let resolver = TypeScriptPathAliasResolver::new();

        // Test 1: "@api/models/User" should match first pattern
        let resolved = resolver.resolve_alias("@api/models/User", &test_file, project_root);
        assert!(resolved.is_some());
        let resolved_path = resolved.unwrap();

        // Should resolve to src/api/models/User, NOT src/api-v2/models/User
        assert!(
            resolved_path.contains("src/api/models/User")
                || resolved_path.ends_with("src/api/models/User"),
            "Expected 'src/api/models/User' but got: {}",
            resolved_path
        );
        assert!(
            !resolved_path.contains("api-v2"),
            "Should not match second pattern for @api/models/*: {}",
            resolved_path
        );

        // Test 2: "@api/other" should match second pattern
        let resolved = resolver.resolve_alias("@api/other", &test_file, project_root);
        assert!(resolved.is_some());
        let resolved_path = resolved.unwrap();

        // Should resolve to src/api-v2/other
        assert!(
            resolved_path.contains("src/api-v2/other")
                || resolved_path.ends_with("src/api-v2/other"),
            "Expected 'src/api-v2/other' but got: {}",
            resolved_path
        );
    }

    #[test]
    fn test_specific_pattern_wins_over_generic() {
        // Another test for overlapping patterns with different specificity
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();

        // Patterns ordered from specific to generic (TypeScript convention)
        create_test_tsconfig(
            project_root,
            ".",
            &[
                ("@lib/server/core/*", &["src/lib/server/core/*"]),  // Most specific
                ("@lib/server/*", &["src/lib/server/*"]),             // Medium specific
                ("@lib/*", &["src/lib/*"]),                           // Least specific
            ],
        );

        let test_file = project_root.join("src").join("test.ts");
        std::fs::create_dir_all(test_file.parent().unwrap()).unwrap();
        std::fs::write(&test_file, "").unwrap();

        let resolver = TypeScriptPathAliasResolver::new();

        // Test 1: "@lib/server/core/orchestrator" should match first (most specific)
        let resolved = resolver.resolve_alias("@lib/server/core/orchestrator", &test_file, project_root);
        assert!(resolved.is_some());
        assert!(resolved.unwrap().contains("src/lib/server/core/orchestrator"));

        // Test 2: "@lib/server/providers" should match second pattern
        let resolved = resolver.resolve_alias("@lib/server/providers", &test_file, project_root);
        assert!(resolved.is_some());
        assert!(resolved.unwrap().contains("src/lib/server/providers"));

        // Test 3: "@lib/components" should match third pattern
        let resolved = resolver.resolve_alias("@lib/components", &test_file, project_root);
        assert!(resolved.is_some());
        assert!(resolved.unwrap().contains("src/lib/components"));
    }

    #[test]
    fn test_overlapping_with_different_targets() {
        // Test when overlapping patterns map to completely different directories
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();

        create_test_tsconfig(
            project_root,
            ".",
            &[
                ("@legacy/auth/*", &["old/auth-system/*"]),  // Specific legacy path
                ("@legacy/*", &["legacy/*"]),                 // Generic legacy path
                ("@/*", &["src/*"]),                          // Current code
            ],
        );

        let test_file = project_root.join("src").join("test.ts");
        std::fs::create_dir_all(test_file.parent().unwrap()).unwrap();
        std::fs::write(&test_file, "").unwrap();

        let resolver = TypeScriptPathAliasResolver::new();

        // Each pattern should resolve to its own distinct directory
        let auth_resolved = resolver.resolve_alias("@legacy/auth/login", &test_file, project_root);
        assert!(auth_resolved.is_some());
        assert!(auth_resolved.unwrap().contains("old/auth-system/login"));

        let legacy_resolved = resolver.resolve_alias("@legacy/utils", &test_file, project_root);
        assert!(legacy_resolved.is_some());
        assert!(legacy_resolved.unwrap().contains("legacy/utils"));

        let current_resolved = resolver.resolve_alias("@/components", &test_file, project_root);
        assert!(current_resolved.is_some());
        assert!(current_resolved.unwrap().contains("src/components"));
    }

    #[test]
    fn test_pattern_requires_slash_separator() {
        // This test verifies the fix for the pattern matching bug
        // "@api/models/*" should NOT match "@apiModels" (missing separator)
        // TypeScript requires an actual '/' after the prefix

        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();

        create_test_tsconfig(
            project_root,
            ".",
            &[
                ("@api/models/*", &["src/api/models/*"]),
                ("@apiModels", &["src/api-models-package"]),  // Exact match for package
            ],
        );

        let test_file = project_root.join("src").join("test.ts");
        std::fs::create_dir_all(test_file.parent().unwrap()).unwrap();
        std::fs::write(&test_file, "").unwrap();

        let resolver = TypeScriptPathAliasResolver::new();

        // Test 1: "@api/models/User" should match the wildcard pattern
        let resolved = resolver.resolve_alias("@api/models/User", &test_file, project_root);
        assert!(resolved.is_some());
        assert!(
            resolved.unwrap().contains("src/api/models/User"),
            "Should match wildcard pattern with separator"
        );

        // Test 2: "@apiModels" should NOT match "@api/models/*"
        // It should match the exact pattern "@apiModels" instead
        let resolved = resolver.resolve_alias("@apiModels", &test_file, project_root);
        assert!(resolved.is_some());
        assert!(
            resolved.unwrap().contains("src/api-models-package"),
            "Should match exact pattern, not wildcard without separator"
        );

        // Test 3: "@api/models" (no slash after) should NOT match "@api/models/*"
        let resolved = resolver.resolve_alias("@api/models", &test_file, project_root);
        assert!(
            resolved.is_none(),
            "Should not match wildcard when there's no suffix after prefix"
        );
    }

    #[test]
    fn test_pattern_rejects_missing_separator() {
        // More explicit test for the separator requirement
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();

        create_test_tsconfig(
            project_root,
            ".",
            &[("$lib/*", &["src/lib/*"])],
        );

        let test_file = project_root.join("src").join("test.ts");
        std::fs::create_dir_all(test_file.parent().unwrap()).unwrap();
        std::fs::write(&test_file, "").unwrap();

        let resolver = TypeScriptPathAliasResolver::new();

        // Valid: has separator
        assert!(resolver.resolve_alias("$lib/utils", &test_file, project_root).is_some());
        assert!(resolver.resolve_alias("$lib/server/core", &test_file, project_root).is_some());

        // Invalid: no separator after prefix
        assert!(
            resolver.resolve_alias("$library", &test_file, project_root).is_none(),
            "$library should not match $lib/* (no separator)"
        );
        assert!(
            resolver.resolve_alias("$lib", &test_file, project_root).is_none(),
            "$lib should not match $lib/* (no suffix)"
        );
        assert!(
            resolver.resolve_alias("$libextra", &test_file, project_root).is_none(),
            "$libextra should not match $lib/* (no separator)"
        );
    }

    #[test]
    fn test_multiple_replacements_in_order() {
        // This test verifies that we try all replacement paths in order
        // TypeScript tries each replacement until one resolves

        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();

        // Pattern with multiple replacement candidates
        create_test_tsconfig(
            project_root,
            ".",
            &[(
                "@shared/*",
                &[
                    "platform/web/*",      // First candidate (web-specific)
                    "platform/mobile/*",   // Second candidate (mobile fallback)
                    "shared/*"             // Third candidate (common code)
                ],
            )],
        );

        let test_file = project_root.join("src").join("test.ts");
        std::fs::create_dir_all(test_file.parent().unwrap()).unwrap();
        std::fs::write(&test_file, "").unwrap();

        let resolver = TypeScriptPathAliasResolver::new();

        // Should return first candidate (even if it doesn't exist)
        // In a full implementation, we'd check existence and try next
        let resolved = resolver.resolve_alias("@shared/utils", &test_file, project_root);
        assert!(resolved.is_some());

        let resolved_path = resolved.unwrap();
        // Should use first replacement
        assert!(
            resolved_path.contains("platform/web/utils"),
            "Should use first replacement in order: {}",
            resolved_path
        );
    }

    #[test]
    fn test_exact_match_vs_wildcard_priority() {
        // Verify exact matches don't get confused with wildcard patterns
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();

        create_test_tsconfig(
            project_root,
            ".",
            &[
                ("utils", &["src/utilities"]),         // Exact match (no wildcard)
                ("utils/*", &["src/utilities/v2/*"]),  // Wildcard pattern
            ],
        );

        let test_file = project_root.join("src").join("test.ts");
        std::fs::create_dir_all(test_file.parent().unwrap()).unwrap();
        std::fs::write(&test_file, "").unwrap();

        let resolver = TypeScriptPathAliasResolver::new();

        // "utils" should match exact pattern (first)
        let resolved = resolver.resolve_alias("utils", &test_file, project_root);
        assert!(resolved.is_some());
        assert!(resolved.unwrap().contains("src/utilities"));

        // "utils/format" should match wildcard pattern (second)
        let resolved = resolver.resolve_alias("utils/format", &test_file, project_root);
        assert!(resolved.is_some());
        assert!(resolved.unwrap().contains("src/utilities/v2/format"));
    }

    #[test]
    fn test_wildcard_in_middle_of_pattern() {
        // This test verifies wildcards can appear anywhere in the pattern
        // TypeScript supports patterns like "libs/*/src" or "packages/*/index"
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();

        // Create directory structure
        std::fs::create_dir_all(project_root.join("libs/mylib/src")).unwrap();
        std::fs::write(project_root.join("libs/mylib/src/index.ts"), "export {}").unwrap();

        create_test_tsconfig(
            project_root,
            ".",
            &[("libs/*/src", &["libs/*/src"])],  // Wildcard in middle
        );

        let test_file = project_root.join("app.ts");
        std::fs::write(&test_file, "").unwrap();

        let resolver = TypeScriptPathAliasResolver::new();

        // "libs/mylib/src" should match pattern and substitute wildcard
        let resolved = resolver.resolve_alias("libs/mylib/src", &test_file, project_root);
        assert!(resolved.is_some());
        let resolved_path = resolved.unwrap();
        assert!(
            resolved_path.contains("libs/mylib/src"),
            "Should substitute wildcard in middle: {}",
            resolved_path
        );
    }

    #[test]
    fn test_multiple_replacements_with_fallback() {
        // This test verifies that we try all replacements until one exists
        // TypeScript tries each replacement in order until it finds a file
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();

        // Create only the fallback paths (not the first)
        std::fs::create_dir_all(project_root.join("shared")).unwrap();
        std::fs::write(project_root.join("shared/utils.ts"), "export {}").unwrap();

        create_test_tsconfig(
            project_root,
            ".",
            &[(
                "@shared/*",
                &[
                    "platform/web/*",    // First - doesn't exist
                    "platform/mobile/*", // Second - doesn't exist
                    "shared/*"           // Third - exists!
                ],
            )],
        );

        let test_file = project_root.join("app.ts");
        std::fs::write(&test_file, "").unwrap();

        let resolver = TypeScriptPathAliasResolver::new();

        // Should fall back to third replacement (shared/*)
        let resolved = resolver.resolve_alias("@shared/utils", &test_file, project_root);
        assert!(resolved.is_some());
        let resolved_path = resolved.unwrap();
        assert!(
            resolved_path.contains("shared/utils"),
            "Should fallback to third replacement: {}",
            resolved_path
        );
        assert!(
            !resolved_path.contains("platform/web"),
            "Should not use first non-existent path: {}",
            resolved_path
        );
    }

    #[test]
    fn test_file_existence_checking_with_extensions() {
        // Verify that we check for files with various extensions
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();

        // Create file with .ts extension only
        std::fs::create_dir_all(project_root.join("src/lib")).unwrap();
        std::fs::write(project_root.join("src/lib/utils.ts"), "export {}").unwrap();

        create_test_tsconfig(
            project_root,
            ".",
            &[("$lib/*", &["src/lib/*"])],
        );

        let test_file = project_root.join("app.ts");
        std::fs::write(&test_file, "").unwrap();

        let resolver = TypeScriptPathAliasResolver::new();

        // Should find utils.ts even though specifier doesn't include extension
        let resolved = resolver.resolve_alias("$lib/utils", &test_file, project_root);
        assert!(resolved.is_some());
        assert!(resolved.unwrap().contains("src/lib/utils"));
    }

    #[test]
    fn test_wildcard_substitution_in_replacement() {
        // Test that wildcards in replacement paths are correctly substituted
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();

        // Create package structure
        std::fs::create_dir_all(project_root.join("packages/foo/src")).unwrap();
        std::fs::write(project_root.join("packages/foo/src/index.ts"), "export {}").unwrap();

        create_test_tsconfig(
            project_root,
            ".",
            &[("@packages/*", &["packages/*/src"])],  // Wildcard in replacement
        );

        let test_file = project_root.join("app.ts");
        std::fs::write(&test_file, "").unwrap();

        let resolver = TypeScriptPathAliasResolver::new();

        // "@packages/foo" should resolve to "packages/foo/src"
        let resolved = resolver.resolve_alias("@packages/foo", &test_file, project_root);
        assert!(resolved.is_some());
        let resolved_path = resolved.unwrap();
        assert!(
            resolved_path.contains("packages/foo/src"),
            "Should substitute wildcard in replacement: {}",
            resolved_path
        );
    }

    #[test]
    fn test_index_file_resolution() {
        // Verify that directory/index.ts is found when importing directory
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();

        // Create directory with index.ts
        std::fs::create_dir_all(project_root.join("src/lib/components")).unwrap();
        std::fs::write(project_root.join("src/lib/components/index.ts"), "export {}").unwrap();

        create_test_tsconfig(
            project_root,
            ".",
            &[("$lib/*", &["src/lib/*"])],
        );

        let test_file = project_root.join("app.ts");
        std::fs::write(&test_file, "").unwrap();

        let resolver = TypeScriptPathAliasResolver::new();

        // Should find components/index.ts when importing "components"
        let resolved = resolver.resolve_alias("$lib/components", &test_file, project_root);
        assert!(resolved.is_some());
        assert!(resolved.unwrap().contains("src/lib/components"));
    }

    #[test]
    fn test_fallback_to_second_replacement() {
        // Verify that when first replacement doesn't exist, we try second
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();

        // Create ONLY the second path (not the first)
        std::fs::create_dir_all(project_root.join("platform/mobile")).unwrap();
        std::fs::write(project_root.join("platform/mobile/utils.ts"), "export {}").unwrap();

        create_test_tsconfig(
            project_root,
            ".",
            &[(
                "@shared/*",
                &[
                    "platform/web/*",     // First - doesn't exist
                    "platform/mobile/*",  // Second - exists!
                    "shared/*"            // Third - doesn't exist
                ],
            )],
        );

        let test_file = project_root.join("app.ts");
        std::fs::write(&test_file, "").unwrap();

        let resolver = TypeScriptPathAliasResolver::new();

        // Should skip first and use second replacement
        let resolved = resolver.resolve_alias("@shared/utils", &test_file, project_root);
        assert!(resolved.is_some(), "Should resolve to second replacement");

        let resolved_path = resolved.unwrap();
        assert!(
            resolved_path.contains("platform/mobile/utils"),
            "Should use second replacement that exists: {}",
            resolved_path
        );
        assert!(
            !resolved_path.contains("platform/web"),
            "Should NOT use first non-existent replacement: {}",
            resolved_path
        );
    }

    #[test]
    fn test_fallback_to_third_replacement() {
        // Verify that we can fallback all the way to third option
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();

        // Create ONLY the third path (first two don't exist)
        std::fs::create_dir_all(project_root.join("shared")).unwrap();
        std::fs::write(project_root.join("shared/utils.ts"), "export {}").unwrap();

        create_test_tsconfig(
            project_root,
            ".",
            &[(
                "@shared/*",
                &[
                    "platform/web/*",     // First - doesn't exist
                    "platform/mobile/*",  // Second - doesn't exist
                    "shared/*"            // Third - exists!
                ],
            )],
        );

        let test_file = project_root.join("app.ts");
        std::fs::write(&test_file, "").unwrap();

        let resolver = TypeScriptPathAliasResolver::new();

        let resolved = resolver.resolve_alias("@shared/utils", &test_file, project_root);
        assert!(resolved.is_some(), "Should resolve to third replacement");

        let resolved_path = resolved.unwrap();
        assert!(
            resolved_path.contains("shared/utils"),
            "Should use third replacement: {}",
            resolved_path
        );
    }

    #[test]
    fn test_libs_star_src_monorepo_pattern() {
        // Test monorepo pattern: libs/*/src
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();

        // Create monorepo structure
        std::fs::create_dir_all(project_root.join("libs/auth/src")).unwrap();
        std::fs::write(project_root.join("libs/auth/src/index.ts"), "export {}").unwrap();

        std::fs::create_dir_all(project_root.join("libs/database/src")).unwrap();
        std::fs::write(project_root.join("libs/database/src/index.ts"), "export {}").unwrap();

        create_test_tsconfig(
            project_root,
            ".",
            &[("libs/*/src", &["libs/*/src"])],  // Wildcard in middle
        );

        let test_file = project_root.join("app.ts");
        std::fs::write(&test_file, "").unwrap();

        let resolver = TypeScriptPathAliasResolver::new();

        // Test auth library
        let resolved = resolver.resolve_alias("libs/auth/src", &test_file, project_root);
        assert!(resolved.is_some(), "Should resolve libs/auth/src");
        assert!(resolved.unwrap().contains("libs/auth/src"));

        // Test database library
        let resolved = resolver.resolve_alias("libs/database/src", &test_file, project_root);
        assert!(resolved.is_some(), "Should resolve libs/database/src");
        assert!(resolved.unwrap().contains("libs/database/src"));
    }

    #[test]
    fn test_packages_star_index_monorepo_pattern() {
        // Test monorepo pattern: packages/*/index
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();

        std::fs::create_dir_all(project_root.join("packages/ui")).unwrap();
        std::fs::write(project_root.join("packages/ui/index.ts"), "export {}").unwrap();

        create_test_tsconfig(
            project_root,
            ".",
            &[("packages/*/index", &["packages/*/index"])],
        );

        let test_file = project_root.join("app.ts");
        std::fs::write(&test_file, "").unwrap();

        let resolver = TypeScriptPathAliasResolver::new();

        let resolved = resolver.resolve_alias("packages/ui/index", &test_file, project_root);
        assert!(resolved.is_some(), "Should resolve packages/ui/index");
        assert!(resolved.unwrap().contains("packages/ui/index"));
    }
}
