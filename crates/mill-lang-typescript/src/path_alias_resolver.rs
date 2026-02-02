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
use regex::Regex;
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
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

    /// Cache of Svelte config path lookups (keyed by directory)
    svelte_config_path_cache: Arc<Mutex<HashMap<PathBuf, Option<PathBuf>>>>,
    /// Cache of parsed Svelte alias maps (keyed by config path)
    svelte_alias_cache: Arc<Mutex<HashMap<PathBuf, Arc<IndexMap<String, Vec<PathBuf>>>>>>,

    /// Cache of Vite config path lookups (keyed by directory)
    vite_config_path_cache: Arc<Mutex<HashMap<PathBuf, Option<PathBuf>>>>,
    /// Cache of parsed Vite alias maps (keyed by config path)
    vite_alias_cache: Arc<Mutex<HashMap<PathBuf, Arc<IndexMap<String, Vec<PathBuf>>>>>>,
}

impl TypeScriptPathAliasResolver {
    /// Create a new TypeScript path alias resolver
    pub fn new() -> Self {
        Self {
            tsconfig_cache: Arc::new(Mutex::new(HashMap::new())),
            tsconfig_path_cache: Arc::new(Mutex::new(HashMap::new())),
            svelte_config_path_cache: Arc::new(Mutex::new(HashMap::new())),
            svelte_alias_cache: Arc::new(Mutex::new(HashMap::new())),
            vite_config_path_cache: Arc::new(Mutex::new(HashMap::new())),
            vite_alias_cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn find_nearest_config_with_cache(
        &self,
        importing_file: &Path,
        names: &[&str],
        cache: &Arc<Mutex<HashMap<PathBuf, Option<PathBuf>>>>,
    ) -> Option<PathBuf> {
        let mut current = importing_file.parent()?;
        let mut ancestors_to_cache: Vec<PathBuf> = Vec::new();

        loop {
            {
                let cache_guard = cache.lock().ok()?;
                if let Some(cached_result) = cache_guard.get(current) {
                    let result = cached_result.clone();
                    drop(cache_guard);
                    if !ancestors_to_cache.is_empty() {
                        if let Ok(mut cache_guard) = cache.lock() {
                            for ancestor in &ancestors_to_cache {
                                cache_guard.insert(ancestor.clone(), result.clone());
                            }
                        }
                    }
                    return result;
                }
            }

            let mut found: Option<PathBuf> = None;
            for name in names {
                let candidate = current.join(name);
                if candidate.exists() {
                    found = Some(candidate);
                    break;
                }
            }

            if let Some(result) = found {
                if let Ok(mut cache_guard) = cache.lock() {
                    cache_guard.insert(current.to_path_buf(), Some(result.clone()));
                    for ancestor in &ancestors_to_cache {
                        cache_guard.insert(ancestor.clone(), Some(result.clone()));
                    }
                }
                return Some(result);
            }

            ancestors_to_cache.push(current.to_path_buf());
            current = current.parent()?;
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

    /// Find nearest SvelteKit config (svelte.config.js/ts)
    fn find_nearest_svelte_config(&self, importing_file: &Path) -> Option<PathBuf> {
        self.find_nearest_config_with_cache(
            importing_file,
            &["svelte.config.js", "svelte.config.ts", "svelte.config.mjs", "svelte.config.cjs"],
            &self.svelte_config_path_cache,
        )
    }

    /// Find nearest Vite config (vite.config.*)
    fn find_nearest_vite_config(&self, importing_file: &Path) -> Option<PathBuf> {
        self.find_nearest_config_with_cache(
            importing_file,
            &[
                "vite.config.ts",
                "vite.config.js",
                "vite.config.mjs",
                "vite.config.cjs",
            ],
            &self.vite_config_path_cache,
        )
    }

    fn load_aliases_from_svelte_config(
        &self,
        config_path: &Path,
    ) -> Option<Arc<IndexMap<String, Vec<PathBuf>>>> {
        {
            let cache = self.svelte_alias_cache.lock().ok()?;
            if let Some(map) = cache.get(config_path) {
                return Some(Arc::clone(map));
            }
        }

        let config_dir = config_path.parent()?;
        let map = if let Some(runtime_map) = load_aliases_from_config_runtime(
            config_path,
            config_dir,
            ConfigKind::Svelte,
        ) {
            Arc::new(runtime_map)
        } else {
            let content = std::fs::read_to_string(config_path).ok()?;
            Arc::new(parse_aliases_from_config(&content, config_dir))
        };

        if let Ok(mut cache) = self.svelte_alias_cache.lock() {
            cache.insert(config_path.to_path_buf(), Arc::clone(&map));
        }

        Some(map)
    }

    fn load_aliases_from_vite_config(
        &self,
        config_path: &Path,
    ) -> Option<Arc<IndexMap<String, Vec<PathBuf>>>> {
        {
            let cache = self.vite_alias_cache.lock().ok()?;
            if let Some(map) = cache.get(config_path) {
                return Some(Arc::clone(map));
            }
        }

        let config_dir = config_path.parent()?;
        let map = if let Some(runtime_map) =
            load_aliases_from_config_runtime(config_path, config_dir, ConfigKind::Vite)
        {
            Arc::new(runtime_map)
        } else {
            let content = std::fs::read_to_string(config_path).ok()?;
            Arc::new(parse_aliases_from_config(&content, config_dir))
        };

        if let Ok(mut cache) = self.vite_alias_cache.lock() {
            cache.insert(config_path.to_path_buf(), Arc::clone(&map));
        }

        Some(map)
    }

    fn resolve_svelte_lib_alias(
        &self,
        specifier: &str,
        importing_file: &Path,
        project_root: &Path,
    ) -> Option<String> {
        if specifier != "$lib" && !specifier.starts_with("$lib/") {
            return None;
        }

        let svelte_root = self
            .find_nearest_svelte_config(importing_file)
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .unwrap_or_else(|| project_root.to_path_buf());

        let mut resolved = svelte_root.join("src").join("lib");
        if let Some(rest) = specifier.strip_prefix("$lib/") {
            if !rest.is_empty() {
                resolved = resolved.join(rest);
            }
        }

        if self.path_exists_with_extensions(&resolved) {
            Some(resolved.to_string_lossy().to_string())
        } else {
            Some(resolved.to_string_lossy().to_string())
        }
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
        let tsconfig_path = self.find_nearest_tsconfig(importing_file);

        if let Some(tsconfig_path) = tsconfig_path {
            // 2. Load and parse tsconfig
            if let Some(config) = self.load_tsconfig(&tsconfig_path) {
                // 3. Use paths from resolved config
                // base_url is implicit in resolved paths now
                if let Some(resolved) = self.match_path_alias(specifier, &config.paths) {
                    return Some(resolved);
                }
            }
        }

        // Fallback: Svelte/Vite alias config
        if let Some(resolved) = self.resolve_alias_from_extra_configs(specifier, importing_file) {
            return Some(resolved);
        }

        // Fallback: SvelteKit $lib alias without tsconfig paths
        self.resolve_svelte_lib_alias(specifier, importing_file, _project_root)
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
        // 1. Try tsconfig.json paths
        if let Some(tsconfig_path) = self.find_nearest_tsconfig(importing_file) {
            if let Some(config) = self.load_tsconfig(&tsconfig_path) {
                for (pattern, replacements) in &config.paths {
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
            }
        }

        // 2. Fallback to Svelte/Vite alias configs if available
        if let Some(alias) = self.path_to_alias_from_extra_configs(
            absolute_path,
            importing_file,
        ) {
            return Some(alias);
        }

        // 3. Fallback: SvelteKit $lib alias even without tsconfig paths
        if let Some(alias) =
            self.path_to_svelte_lib_alias(absolute_path, importing_file, project_root)
        {
            return Some(alias);
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
        let normalized_replacement = normalize_path(replacement);
        let replacement_str = normalized_replacement.to_string_lossy();

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
                if let Ok(relative) = path_to_check.strip_prefix(&normalized_replacement) {
                    let relative_str = relative.to_string_lossy();
                    let alias = format!("{}{}{}", pattern_prefix, relative_str, pattern_suffix);
                    return Some(alias);
                }
            }
        } else {
            // Exact match
            let expected_path = normalized_replacement.with_extension("");
            if path_to_check == expected_path {
                return Some(pattern.to_string());
            }
        }

        None
    }
}

impl TypeScriptPathAliasResolver {
    fn resolve_alias_from_extra_configs(
        &self,
        specifier: &str,
        importing_file: &Path,
    ) -> Option<String> {
        if let Some(config_path) = self.find_nearest_svelte_config(importing_file) {
            if let Some(map) = self.load_aliases_from_svelte_config(&config_path) {
                if let Some(resolved) = resolve_alias_from_map(specifier, &map) {
                    return Some(resolved);
                }
            }
        }

        if let Some(config_path) = self.find_nearest_vite_config(importing_file) {
            if let Some(map) = self.load_aliases_from_vite_config(&config_path) {
                if let Some(resolved) = resolve_alias_from_map(specifier, &map) {
                    return Some(resolved);
                }
            }
        }

        None
    }

    fn path_to_alias_from_extra_configs(
        &self,
        absolute_path: &Path,
        importing_file: &Path,
    ) -> Option<String> {
        if let Some(config_path) = self.find_nearest_svelte_config(importing_file) {
            if let Some(map) = self.load_aliases_from_svelte_config(&config_path) {
                if let Some(alias) = path_to_alias_from_map(absolute_path, &map) {
                    return Some(alias);
                }
            }
        }

        if let Some(config_path) = self.find_nearest_vite_config(importing_file) {
            if let Some(map) = self.load_aliases_from_vite_config(&config_path) {
                if let Some(alias) = path_to_alias_from_map(absolute_path, &map) {
                    return Some(alias);
                }
            }
        }

        None
    }

    fn path_to_svelte_lib_alias(
        &self,
        absolute_path: &Path,
        importing_file: &Path,
        project_root: &Path,
    ) -> Option<String> {
        let svelte_root = self
            .find_nearest_svelte_config(importing_file)
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .unwrap_or_else(|| project_root.to_path_buf());

        let lib_root = svelte_root.join("src").join("lib");
        let mut path_to_check = absolute_path.to_path_buf();
        if let Some(ext) = path_to_check.extension().and_then(|e| e.to_str()) {
            if ext == "ts" || ext == "tsx" || ext == "js" || ext == "jsx" {
                path_to_check = path_to_check.with_extension("");
            }
        }

        let relative = path_to_check.strip_prefix(&lib_root).ok()?;
        let relative_str = relative.to_string_lossy().replace('\\', "/");

        if relative_str.is_empty() {
            Some("$lib".to_string())
        } else {
            Some(format!("$lib/{}", relative_str))
        }
    }
}

fn parse_aliases_from_config(
    content: &str,
    config_dir: &Path,
) -> IndexMap<String, Vec<PathBuf>> {
    let mut map: IndexMap<String, Vec<PathBuf>> = IndexMap::new();
    let alias_re = Regex::new(r"\balias\s*:").expect("alias regex should be valid");

    for m in alias_re.find_iter(content) {
        if let Some((block, block_type)) = extract_alias_block(content, m.end()) {
            match block_type {
                AliasBlockType::Object => {
                    collect_alias_object_pairs(block, config_dir, &mut map);
                }
                AliasBlockType::Array => {
                    collect_alias_array_pairs(block, config_dir, &mut map);
                }
            }
        }
    }

    map
}

enum AliasBlockType {
    Object,
    Array,
}

#[derive(Copy, Clone)]
enum ConfigKind {
    Svelte,
    Vite,
}

fn load_aliases_from_config_runtime(
    config_path: &Path,
    config_dir: &Path,
    kind: ConfigKind,
) -> Option<IndexMap<String, Vec<PathBuf>>> {
    let ext = config_path.extension().and_then(|e| e.to_str()).unwrap_or("");

    let absolute = config_path.canonicalize().ok()?;
    let file_arg = absolute.to_string_lossy().to_string();

    let output = if ext == "mjs" {
        run_node_alias_script(&file_arg, kind, NodeMode::Esm, config_dir)
    } else if ext == "cjs" {
        run_node_alias_script(&file_arg, kind, NodeMode::Cjs, config_dir)
    } else if ext == "ts" {
        run_node_alias_script(&file_arg, kind, NodeMode::EsmTsx, config_dir)
            .or_else(|| run_node_alias_script(&file_arg, kind, NodeMode::EsmTsxNearest, config_dir))
            .or_else(|| run_node_alias_script(&file_arg, kind, NodeMode::NpxTsx, config_dir))
    } else {
        run_node_alias_script(&file_arg, kind, NodeMode::Esm, config_dir)
            .or_else(|| run_node_alias_script(&file_arg, kind, NodeMode::Cjs, config_dir))
    }?;

    let alias_values: Vec<(String, String)> = parse_alias_json(&output)?;

    let mut map: IndexMap<String, Vec<PathBuf>> = IndexMap::new();
    for (find, replacement) in alias_values {
        if let Some(path) = normalize_alias_path(&replacement, config_dir) {
            map.entry(find).or_default().push(path);
        }
    }

    if map.is_empty() {
        None
    } else {
        Some(map)
    }
}

#[derive(Copy, Clone)]
enum NodeMode {
    Esm,
    Cjs,
    EsmTsx,
    EsmTsxNearest,
    NpxTsx,
}

fn run_node_alias_script(
    file_arg: &str,
    kind: ConfigKind,
    mode: NodeMode,
    config_dir: &Path,
) -> Option<String> {
    let kind_str = match kind {
        ConfigKind::Svelte => "svelte",
        ConfigKind::Vite => "vite",
    };

    let (arg0, arg1, use_npx, cwd_override) = match mode {
        NodeMode::Esm => (
            "--input-type=module",
            r#"import { pathToFileURL } from "url";
const file = process.argv[1];
const kind = process.argv[2];
const mod = await import(pathToFileURL(file).href);
const cfg = mod?.default ?? mod;
const resolved = typeof cfg === "function" ? await cfg({ command: "build", mode: "production" }) : cfg;
const alias = kind === "svelte" ? resolved?.kit?.alias : resolved?.resolve?.alias;
const normalized = [];
const pushEntry = (find, replacement) => {
  if (!find || !replacement) return;
  if (typeof replacement === "string") normalized.push([find, replacement]);
  else if (Array.isArray(replacement)) replacement.forEach(r => { if (typeof r === "string") normalized.push([find, r]); });
};
if (Array.isArray(alias)) {
  alias.forEach(entry => {
    if (!entry) return;
    if (typeof entry.find === "string" && typeof entry.replacement === "string") {
      pushEntry(entry.find, entry.replacement);
    }
  });
} else if (alias && typeof alias === "object") {
  for (const [key, value] of Object.entries(alias)) {
    pushEntry(key, value);
  }
}
process.stdout.write(JSON.stringify(normalized));"#,
            false,
            None,
        ),
        NodeMode::Cjs => (
            "",
            r#"const file = process.argv[1];
const kind = process.argv[2];
const mod = require(file);
const cfg = mod?.default ?? mod;
const resolved = typeof cfg === "function" ? cfg({ command: "build", mode: "production" }) : cfg;
const alias = kind === "svelte" ? resolved?.kit?.alias : resolved?.resolve?.alias;
const normalized = [];
const pushEntry = (find, replacement) => {
  if (!find || !replacement) return;
  if (typeof replacement === "string") normalized.push([find, replacement]);
  else if (Array.isArray(replacement)) replacement.forEach(r => { if (typeof r === "string") normalized.push([find, r]); });
};
if (Array.isArray(alias)) {
  alias.forEach(entry => {
    if (!entry) return;
    if (typeof entry.find === "string" && typeof entry.replacement === "string") {
      pushEntry(entry.find, entry.replacement);
    }
  });
} else if (alias && typeof alias === "object") {
  for (const [key, value] of Object.entries(alias)) {
    pushEntry(key, value);
  }
}
process.stdout.write(JSON.stringify(normalized));"#,
            false,
            None,
        ),
        NodeMode::EsmTsx => (
            "--loader",
            "tsx",
            false,
            None,
        ),
        NodeMode::EsmTsxNearest => (
            "--loader",
            "tsx",
            false,
            find_nearest_node_modules(config_dir),
        ),
        NodeMode::NpxTsx => (
            "",
            "",
            true,
            None,
        ),
    };

    let output = if use_npx {
        let script = r#"import { pathToFileURL } from "url";
const file = process.argv[1];
const kind = process.argv[2];
const mod = await import(pathToFileURL(file).href);
const cfg = mod?.default ?? mod;
const resolved = typeof cfg === "function" ? await cfg({ command: "build", mode: "production" }) : cfg;
const alias = kind === "svelte" ? resolved?.kit?.alias : resolved?.resolve?.alias;
const normalized = [];
const pushEntry = (find, replacement) => {
  if (!find || !replacement) return;
  if (typeof replacement === "string") normalized.push([find, replacement]);
  else if (Array.isArray(replacement)) replacement.forEach(r => { if (typeof r === "string") normalized.push([find, r]); });
};
if (Array.isArray(alias)) {
  alias.forEach(entry => {
    if (!entry) return;
    if (typeof entry.find === "string" && typeof entry.replacement === "string") {
      pushEntry(entry.find, entry.replacement);
    }
  });
} else if (alias && typeof alias === "object") {
  for (const [key, value] of Object.entries(alias)) {
    pushEntry(key, value);
  }
}
process.stdout.write(JSON.stringify(normalized));"#;

        let mut cmd = Command::new("npx");
        cmd.arg("--yes")
            .arg("tsx")
            .arg("-e")
            .arg(script)
            .arg(file_arg)
            .arg(kind_str)
            .current_dir(config_dir);
        cmd.output().ok()?
    } else if matches!(mode, NodeMode::EsmTsx | NodeMode::EsmTsxNearest) {
        let script = r#"import { pathToFileURL } from "url";
const file = process.argv[1];
const kind = process.argv[2];
const mod = await import(pathToFileURL(file).href);
const cfg = mod?.default ?? mod;
const resolved = typeof cfg === "function" ? await cfg({ command: "build", mode: "production" }) : cfg;
const alias = kind === "svelte" ? resolved?.kit?.alias : resolved?.resolve?.alias;
const normalized = [];
const pushEntry = (find, replacement) => {
  if (!find || !replacement) return;
  if (typeof replacement === "string") normalized.push([find, replacement]);
  else if (Array.isArray(replacement)) replacement.forEach(r => { if (typeof r === "string") normalized.push([find, r]); });
};
if (Array.isArray(alias)) {
  alias.forEach(entry => {
    if (!entry) return;
    if (typeof entry.find === "string" && typeof entry.replacement === "string") {
      pushEntry(entry.find, entry.replacement);
    }
  });
} else if (alias && typeof alias === "object") {
  for (const [key, value] of Object.entries(alias)) {
    pushEntry(key, value);
  }
}
process.stdout.write(JSON.stringify(normalized));"#;

        let mut cmd = Command::new("node");
        cmd.arg(arg0)
            .arg(arg1)
            .arg("--input-type=module")
            .arg("-e")
            .arg(script)
            .arg(file_arg)
            .arg(kind_str);
        if let Some(cwd) = cwd_override.as_ref() {
            cmd.current_dir(cwd);
        } else {
            cmd.current_dir(config_dir);
        }
        cmd.output().ok()?
    } else {
        let mut cmd = Command::new("node");
        if !arg0.is_empty() {
            cmd.arg(arg0);
        }
        let output = cmd
            .arg("-e")
            .arg(arg1)
            .arg(file_arg)
            .arg(kind_str)
            .current_dir(config_dir)
            .output()
            .ok()?;
        output
    };

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if stdout.is_empty() {
        None
    } else {
        Some(stdout)
    }
}

fn find_nearest_node_modules(start_dir: &Path) -> Option<PathBuf> {
    let mut current = start_dir;
    loop {
        let candidate = current.join("node_modules").join("tsx");
        if candidate.exists() {
            return Some(current.to_path_buf());
        }
        match current.parent() {
            Some(parent) => current = parent,
            None => break,
        }
    }
    None
}

fn parse_alias_json(json_str: &str) -> Option<Vec<(String, String)>> {
    let value: Value = serde_json::from_str(json_str).ok()?;
    let array = value.as_array()?;
    let mut result = Vec::new();
    for entry in array {
        let pair = entry.as_array()?;
        if pair.len() != 2 {
            continue;
        }
        let find = pair[0].as_str()?.to_string();
        let replacement = pair[1].as_str()?.to_string();
        result.push((find, replacement));
    }
    Some(result)
}

fn extract_alias_block(content: &str, start: usize) -> Option<(&str, AliasBlockType)> {
    let bytes = content.as_bytes();
    let mut idx = start;

    while idx < bytes.len() && bytes[idx].is_ascii_whitespace() {
        idx += 1;
    }

    let (open, close, block_type) = match bytes.get(idx) {
        Some(b'{') => (b'{', b'}', AliasBlockType::Object),
        Some(b'[') => (b'[', b']', AliasBlockType::Array),
        _ => return None,
    };

    let mut depth = 0usize;
    let mut i = idx;
    let mut in_string = false;
    let mut string_delim = b'\0';
    let mut escape_next = false;

    while i < bytes.len() {
        let ch = bytes[i];

        if in_string {
            if escape_next {
                escape_next = false;
            } else if ch == b'\\' {
                escape_next = true;
            } else if ch == string_delim {
                in_string = false;
            }
            i += 1;
            continue;
        }

        if ch == b'\'' || ch == b'"' || ch == b'`' {
            in_string = true;
            string_delim = ch;
            i += 1;
            continue;
        }

        if ch == open {
            depth += 1;
        } else if ch == close {
            depth -= 1;
            if depth == 0 {
                let block = &content[idx + 1..i];
                return Some((block, block_type));
            }
        }

        i += 1;
    }

    None
}

fn collect_alias_object_pairs(
    block: &str,
    config_dir: &Path,
    map: &mut IndexMap<String, Vec<PathBuf>>,
) {
    let single_quoted_pair =
        Regex::new(r#"(?m)'([^']+)'\s*:\s*'([^']+)'"#)
            .expect("single-quoted alias regex should be valid");
    let double_quoted_pair =
        Regex::new(r#"(?m)"([^"]+)"\s*:\s*"([^"]+)""#)
            .expect("double-quoted alias regex should be valid");
    let ident_single_pair =
        Regex::new(r#"(?m)([$A-Za-z_][\w$]*)\s*:\s*'([^']+)'"#)
            .expect("ident single alias regex should be valid");
    let ident_double_pair =
        Regex::new(r#"(?m)([$A-Za-z_][\w$]*)\s*:\s*"([^"]+)""#)
            .expect("ident double alias regex should be valid");

    for caps in single_quoted_pair.captures_iter(block) {
        let key = caps.get(1).map(|m| m.as_str()).unwrap_or("");
        let value = caps.get(2).map(|m| m.as_str()).unwrap_or("");
        if let Some(path) = normalize_alias_path(value, config_dir) {
            map.entry(key.to_string()).or_default().push(path);
        }
    }

    for caps in double_quoted_pair.captures_iter(block) {
        let key = caps.get(1).map(|m| m.as_str()).unwrap_or("");
        let value = caps.get(2).map(|m| m.as_str()).unwrap_or("");
        if let Some(path) = normalize_alias_path(value, config_dir) {
            map.entry(key.to_string()).or_default().push(path);
        }
    }

    for caps in ident_single_pair.captures_iter(block) {
        let key = caps.get(1).map(|m| m.as_str()).unwrap_or("");
        let value = caps.get(2).map(|m| m.as_str()).unwrap_or("");
        if let Some(path) = normalize_alias_path(value, config_dir) {
            map.entry(key.to_string()).or_default().push(path);
        }
    }

    for caps in ident_double_pair.captures_iter(block) {
        let key = caps.get(1).map(|m| m.as_str()).unwrap_or("");
        let value = caps.get(2).map(|m| m.as_str()).unwrap_or("");
        if let Some(path) = normalize_alias_path(value, config_dir) {
            map.entry(key.to_string()).or_default().push(path);
        }
    }
}

fn collect_alias_array_pairs(
    block: &str,
    config_dir: &Path,
    map: &mut IndexMap<String, Vec<PathBuf>>,
) {
    let object_re = Regex::new(r#"(?s)\{[^}]*\}"#).expect("alias object regex should be valid");
    let find_re = Regex::new(r#"find\s*:\s*(['"])([^'"]+)\1"#)
        .expect("find alias regex should be valid");
    let replacement_re = Regex::new(r#"replacement\s*:\s*(['"])([^'"]+)\1"#)
        .expect("replacement alias regex should be valid");

    for obj in object_re.find_iter(block) {
        let obj_str = obj.as_str();
        let find = find_re
            .captures(obj_str)
            .and_then(|c| c.get(2))
            .map(|m| m.as_str());
        let replacement = replacement_re
            .captures(obj_str)
            .and_then(|c| c.get(2))
            .map(|m| m.as_str());

        if let (Some(find), Some(replacement)) = (find, replacement) {
            if let Some(path) = normalize_alias_path(replacement, config_dir) {
                map.entry(find.to_string()).or_default().push(path);
            }
        }
    }
}

fn normalize_alias_path(value: &str, config_dir: &Path) -> Option<PathBuf> {
    if value.starts_with("http://") || value.starts_with("https://") {
        return None;
    }
    let path = if value.starts_with('/') {
        PathBuf::from(value)
    } else {
        config_dir.join(value)
    };
    Some(path)
}

fn resolve_alias_from_map(
    specifier: &str,
    map: &IndexMap<String, Vec<PathBuf>>,
) -> Option<String> {
    for (pattern, replacements) in map {
        if let Some(resolved) = resolve_alias_against_pattern(specifier, pattern, replacements) {
            return Some(resolved);
        }
    }
    None
}

fn resolve_alias_against_pattern(
    specifier: &str,
    pattern: &str,
    replacements: &[PathBuf],
) -> Option<String> {
    if pattern.contains('*') {
        return match_path_alias_with_replacements(specifier, pattern, replacements);
    }

    if specifier == pattern {
        return replacements
            .first()
            .map(|p| p.to_string_lossy().to_string());
    }

    if specifier.starts_with(pattern) && specifier.as_bytes().get(pattern.len()) == Some(&b'/') {
        let suffix = &specifier[pattern.len() + 1..];
        if let Some(base) = replacements.first() {
            let resolved = base.join(suffix);
            return Some(resolved.to_string_lossy().to_string());
        }
    }

    None
}

fn match_path_alias_with_replacements(
    specifier: &str,
    pattern: &str,
    replacements: &[PathBuf],
) -> Option<String> {
    // Reuse logic from tsconfig matching by building a temporary map
    let mut map = IndexMap::new();
    map.insert(pattern.to_string(), replacements.to_vec());
    let resolver = TypeScriptPathAliasResolver::new();
    resolver.match_path_alias(specifier, &map)
}

fn path_to_alias_from_map(
    absolute_path: &Path,
    map: &IndexMap<String, Vec<PathBuf>>,
) -> Option<String> {
    let resolver = TypeScriptPathAliasResolver::new();

    let mut path_to_check = absolute_path.to_path_buf();
    if let Some(ext) = path_to_check.extension().and_then(|e| e.to_str()) {
        if ext == "ts" || ext == "tsx" || ext == "js" || ext == "jsx" {
            path_to_check = path_to_check.with_extension("");
        }
    }

    let mut best: Option<(usize, String)> = None;

    for (pattern, replacements) in map {
        for replacement in replacements {
            if !pattern.contains('*') {
                if let Ok(relative) = path_to_check.strip_prefix(replacement) {
                    let relative_str = relative.to_string_lossy().replace('\\', "/");
                    let alias = if relative_str.is_empty() {
                        pattern.to_string()
                    } else {
                        format!("{}/{}", pattern.trim_end_matches('/'), relative_str)
                    };
                    let score = replacement.components().count();
                    if best.as_ref().map_or(true, |(best_score, _)| score > *best_score) {
                        best = Some((score, alias));
                    }
                }
            }

            if let Some(alias) = resolver.try_convert_path_to_alias(
                absolute_path,
                pattern,
                replacement,
                Path::new("."),
                Path::new("."),
            ) {
                let score = replacement.components().count();
                if best.as_ref().map_or(true, |(best_score, _)| score > *best_score) {
                    best = Some((score, alias));
                }
            }
        }
    }

    best.map(|(_, alias)| alias)
}

fn normalize_path(path: &Path) -> PathBuf {
    use std::path::Component;
    let mut result = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                result.pop();
            }
            _ => result.push(component.as_os_str()),
        }
    }
    if result.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        result
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
    fn test_resolve_alias_from_svelte_config() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();

        let config = r#"
export default {
  kit: {
    alias: {
      "@": "src",
      "$utils": "src/utils",
      "$lib/*": "src/lib/*"
    }
  }
};
"#;
        std::fs::write(project_root.join("svelte.config.js"), config).unwrap();

        let src_dir = project_root.join("src");
        std::fs::create_dir_all(src_dir.join("utils")).unwrap();
        std::fs::create_dir_all(src_dir.join("lib")).unwrap();
        let importing_file = src_dir.join("routes").join("+page.svelte");
        std::fs::create_dir_all(importing_file.parent().unwrap()).unwrap();
        std::fs::write(&importing_file, "").unwrap();

        let resolver = TypeScriptPathAliasResolver::new();
        let resolved = resolver.resolve_alias("@/utils/format", &importing_file, project_root);
        assert!(resolved.is_some());
        let resolved_path = resolved.unwrap();
        assert!(resolved_path.ends_with("src/utils/format"));

        let resolved = resolver.resolve_alias("$lib/components/Button", &importing_file, project_root);
        assert!(resolved.is_some());
        let resolved_path = resolved.unwrap();
        assert!(resolved_path.ends_with("src/lib/components/Button"));
    }

    #[test]
    fn test_path_to_alias_from_svelte_config() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();

        let config = r#"
export default {
  kit: {
    alias: {
      "@": "src",
      "$utils": "src/utils"
    }
  }
};
"#;
        std::fs::write(project_root.join("svelte.config.js"), config).unwrap();

        let src_dir = project_root.join("src");
        std::fs::create_dir_all(src_dir.join("utils")).unwrap();
        std::fs::create_dir_all(src_dir.join("components")).unwrap();
        let importing_file = src_dir.join("routes").join("+page.svelte");
        std::fs::create_dir_all(importing_file.parent().unwrap()).unwrap();
        std::fs::write(&importing_file, "").unwrap();

        let resolver = TypeScriptPathAliasResolver::new();
        let abs_path = src_dir.join("utils").join("format.ts");
        let alias = resolver.path_to_alias(&abs_path, &importing_file, project_root);
        assert_eq!(alias, Some("$utils/format".to_string()));

        let abs_path = src_dir.join("components").join("Button.ts");
        let alias = resolver.path_to_alias(&abs_path, &importing_file, project_root);
        assert_eq!(alias, Some("@/components/Button".to_string()));
    }

    #[test]
    fn test_path_to_alias_with_sveltekit_extends() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();

        let web_dir = project_root.join("web");
        let svelte_kit_dir = web_dir.join(".svelte-kit");
        std::fs::create_dir_all(&svelte_kit_dir).unwrap();

        let svelte_tsconfig = r#"
{
  "compilerOptions": {
    "paths": {
      "$lib/*": ["../src/lib/*"]
    }
  }
}
"#;
        std::fs::write(svelte_kit_dir.join("tsconfig.json"), svelte_tsconfig).unwrap();

        let root_tsconfig = r#"
{
  "extends": "./.svelte-kit/tsconfig.json",
  "compilerOptions": {
    "moduleResolution": "bundler"
  }
}
"#;
        std::fs::write(web_dir.join("tsconfig.json"), root_tsconfig).unwrap();

        let src_dir = web_dir.join("src").join("lib").join("utils");
        std::fs::create_dir_all(&src_dir).unwrap();
        let importing_file = web_dir.join("src").join("routes").join("+page.svelte");
        std::fs::create_dir_all(importing_file.parent().unwrap()).unwrap();
        std::fs::write(&importing_file, "").unwrap();

        let resolver = TypeScriptPathAliasResolver::new();
        let abs_path = src_dir.join("text.ts");
        let alias = resolver.path_to_alias(&abs_path, &importing_file, project_root);
        assert_eq!(alias, Some("$lib/utils/text".to_string()));
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
