//! Generic import-based reference detection
//!
//! Fallback detection for languages without specialized detectors.
//! Uses import path resolution to find affected files.
//!
//! OPTIMIZATION: Uses ImportCache for O(1) lookups of "which files import this path?"
//! instead of scanning all files on every rename operation.

use crate::services::reference_updater::ImportCache;
use mill_plugin_api::LanguagePlugin;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::task::JoinSet;

/// Find affected files using generic import path resolution AND rewrite detection
///
/// If cache is provided and populated, uses reverse index for instant lookup.
/// If cache is provided but empty, populates it while scanning.
/// If no cache, falls back to full scan.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn find_generic_affected_files_cached(
    old_path: &Path,
    new_path: &Path,
    project_root: &Path,
    project_files: &[PathBuf],
    plugins: &[Arc<dyn LanguagePlugin>],
    plugin_map: &HashMap<String, Arc<dyn LanguagePlugin>>,
    rename_info: Option<&serde_json::Value>,
    scan_scope: Option<mill_plugin_api::ScanScope>,
    import_cache: Option<Arc<ImportCache>>,
) -> Vec<PathBuf> {
    let mut affected = HashSet::new();
    let old_path_buf = old_path.to_path_buf();
    let is_directory = old_path.is_dir();
    let renamed_ext = old_path
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_string());

    tracing::info!(
        old_path = %old_path.display(),
        new_path = %new_path.display(),
        project_files_count = project_files.len(),
        cache_available = import_cache.is_some(),
        "find_generic_affected_files_cached called"
    );

    // OPTIMIZATION: If cache is populated, use reverse index for O(1) import lookups
    if let Some(ref cache) = import_cache {
        if cache.is_populated() {
            let (forward, reverse) = cache.stats();
            tracing::info!(
                forward_entries = forward,
                reverse_entries = reverse,
                "Using populated import cache for fast lookup"
            );

            // Get files that import the old path directly (O(1) lookup!)
            let importers = if is_directory {
                cache.get_importers_for_directory(&old_path_buf)
            } else {
                cache.get_importers(&old_path_buf)
            };

            tracing::info!(
                importers_count = importers.len(),
                "Cache lookup found importers"
            );

            affected.extend(importers);

            // Still need METHOD 2 (rewrite detection) for string literals, config paths, etc.
            // But we can skip files that don't need rewrite checking
            // (e.g., .ts files where import detection already covered them)
            let need_rewrite_check: Vec<PathBuf> = project_files
                .iter()
                .filter(|f| {
                    // Skip files already marked as affected
                    if affected.contains(*f) {
                        return false;
                    }
                    // Check file types that may contain alias paths or string refs.
                    f.extension()
                        .and_then(|e| e.to_str())
                        .map(|ext| {
                            let is_doc = matches!(ext, "md" | "markdown" | "toml" | "yaml" | "yml" | "json");
                            let is_web = matches!(ext, "svelte" | "ts" | "tsx" | "js" | "jsx");
                            let allow_rewrite = matches!(scan_scope, Some(mill_plugin_api::ScanScope::All));
                            is_doc || ((is_directory && is_web) && allow_rewrite)
                        })
                        .unwrap_or(false)
                })
                .cloned()
                .collect();

            if !need_rewrite_check.is_empty() {
                let rewrite_affected = check_files_for_rewrite(
                    &need_rewrite_check,
                    old_path,
                    new_path,
                    project_root,
                    plugin_map,
                    rename_info,
                    scan_scope,
                )
                .await;
                affected.extend(rewrite_affected);
            }

            let affected_vec: Vec<PathBuf> = affected.into_iter().collect();
            tracing::info!(
                affected_count = affected_vec.len(),
                "find_generic_affected_files_cached completed (cache hit)"
            );
            return affected_vec;
        }
    }

    // Cache not populated - do full scan and populate cache
    tracing::info!("Cache not populated, doing full scan");

    // Create resolver once for reuse
    let resolver = Arc::new(mill_ast::ImportPathResolver::with_plugins(
        project_root,
        plugins.to_vec(),
    ));

    let plugin_map = Arc::new(plugin_map.clone());
    let rename_info = rename_info.cloned();
    let old_path = old_path.to_path_buf();
    let new_path = new_path.to_path_buf();
    let project_root = project_root.to_path_buf();
    let project_files_arc = Arc::new(project_files.to_vec());
    let renamed_ext = renamed_ext.clone();
    let allow_rewrite = matches!(scan_scope, Some(mill_plugin_api::ScanScope::All));

    let mut join_set = JoinSet::new();
    let concurrency = default_concurrency_limit();
    let semaphore = Arc::new(tokio::sync::Semaphore::new(concurrency));

    for file in project_files {
        if file == &new_path {
            continue;
        }

        let file = file.clone();
        let old_path = old_path.clone();
        let new_path = new_path.clone();
        let project_root = project_root.clone();
        let resolver = resolver.clone();
        let plugin_map = plugin_map.clone();
        let rename_info = rename_info.clone();
        let project_files_arc = project_files_arc.clone();
        let import_cache = import_cache.clone();
        let renamed_ext = renamed_ext.clone();
        let allow_rewrite = allow_rewrite;

        let permit = match semaphore.clone().acquire_owned().await {
            Ok(p) => p,
            Err(_) => continue,
        };

        join_set.spawn(async move {
            let _permit = permit;
            if let Ok(content) = tokio::fs::read_to_string(&file).await {
                let file_clone = file.clone();
                let old_path_clone = old_path.clone();
                let new_path_clone = new_path.clone();
                let project_root_clone = project_root.clone();
                let resolver_clone = resolver.clone();
                let plugin_map_clone = plugin_map.clone();
                let rename_info_clone = rename_info.clone();
                let project_files_clone = project_files_arc.clone();
                let content_clone = content.clone();
                let import_cache_clone = import_cache.clone();
                let renamed_ext = renamed_ext.clone();
                let result = tokio::task::spawn_blocking(move || {
                    // METHOD 1: Import-based detection
                    let all_imports = get_all_imported_files_internal(
                        &content_clone,
                        &file_clone,
                        &plugin_map_clone,
                        &project_files_clone,
                        &resolver_clone,
                    );

                    // CACHE POPULATION: Store the imports for this file
                    if let Some(ref cache) = import_cache_clone {
                        if let Ok(metadata) = std::fs::metadata(&file_clone) {
                            if let Ok(modified) = metadata.modified() {
                                cache.set_imports(file_clone.clone(), all_imports.clone(), modified);
                            }
                        }
                    }

                    // Check if imports reference the old/new path
                    if is_directory {
                        if all_imports
                            .iter()
                            .any(|p| p.starts_with(&old_path_clone) || p.starts_with(&new_path_clone))
                        {
                            return Some(file_clone);
                        }
                    } else if all_imports.contains(&old_path_clone)
                        || all_imports.contains(&new_path_clone)
                    {
                        return Some(file_clone);
                    }

                    // METHOD 2: Rewrite-based detection
                    if let Some(ext) = file_clone.extension().and_then(|e| e.to_str()) {
                        if let Some(plugin) = plugin_map_clone.get(ext) {
                            let target_ext = file_clone.extension().and_then(|e| e.to_str());
                            if !crate::services::reference_updater::is_extension_compatible_for_rewrite(
                                renamed_ext.as_deref(),
                                target_ext,
                            ) {
                                return None;
                            }
                            if !allow_rewrite && is_web_extension(ext) {
                                return None;
                            }
                            if is_directory
                                && is_web_extension(ext)
                                && !content_might_contain_alias_imports(&content_clone)
                            {
                                return None;
                            }

                            let rewrite_result = plugin.rewrite_file_references(
                                &content_clone,
                                &old_path_clone,
                                &new_path_clone,
                                &file_clone,
                                &project_root_clone,
                                rename_info_clone.as_ref(),
                            );

                            if let Some((updated_content, change_count)) = rewrite_result {
                                if change_count > 0 && updated_content != content_clone {
                                    return Some(file_clone);
                                }
                            }
                        } else {
                            #[cfg(feature = "lang-svelte")]
                            if ext == "svelte" {
                                let plugin = mill_lang_svelte::SveltePlugin::new();
                                if !allow_rewrite {
                                    return None;
                                }
                                if is_directory
                                    && !content_might_contain_alias_imports(&content_clone)
                                {
                                    return None;
                                }
                                let rewrite_result = plugin.rewrite_file_references(
                                    &content_clone,
                                    &old_path_clone,
                                    &new_path_clone,
                                    &file_clone,
                                    &project_root_clone,
                                    rename_info_clone.as_ref(),
                                );

                                if let Some((updated_content, change_count)) = rewrite_result {
                                    if change_count > 0 && updated_content != content_clone {
                                        return Some(file_clone);
                                    }
                                }
                            }
                        }
                    }
                    None
                })
                .await;

                match result {
                    Ok(Some(f)) => Some(f),
                    Ok(None) => None,
                    Err(e) => {
                        tracing::error!("Blocking task panicked: {}", e);
                        None
                    }
                }
            } else {
                None
            }
        });
    }

    while let Some(res) = join_set.join_next().await {
        match res {
            Ok(Some(file)) => {
                affected.insert(file);
            }
            Ok(None) => {}
            Err(e) => {
                tracing::error!("Task join error: {}", e);
            }
        }
    }

    let affected_vec: Vec<PathBuf> = affected.into_iter().collect();

    if let Some(ref cache) = import_cache {
        let (forward, reverse) = cache.stats();
        tracing::info!(
            affected_count = affected_vec.len(),
            cache_forward = forward,
            cache_reverse = reverse,
            "find_generic_affected_files_cached completed (cache populated)"
        );
        let _ = cache.save_to_disk(&project_root);
    } else {
        tracing::info!(
            affected_count = affected_vec.len(),
            "find_generic_affected_files_cached completed (no cache)"
        );
    }

    affected_vec
}

/// Check a subset of files for rewrite-based detection only
///
/// Used when cache is populated for import detection but we still need
/// to check config/doc files for string literal references.
async fn check_files_for_rewrite(
    files: &[PathBuf],
    old_path: &Path,
    new_path: &Path,
    project_root: &Path,
    plugin_map: &HashMap<String, Arc<dyn LanguagePlugin>>,
    rename_info: Option<&serde_json::Value>,
    scan_scope: Option<mill_plugin_api::ScanScope>,
) -> Vec<PathBuf> {
    let mut affected = Vec::new();
    let plugin_map = Arc::new(plugin_map.clone());
    let rename_info = rename_info.cloned();
    let old_path = old_path.to_path_buf();
    let new_path = new_path.to_path_buf();
    let project_root = project_root.to_path_buf();
    let is_directory = old_path.is_dir();
    let renamed_ext = old_path
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_string());

    let allow_rewrite = matches!(scan_scope, Some(mill_plugin_api::ScanScope::All));

    let mut join_set = JoinSet::new();
    let concurrency = default_concurrency_limit();
    let semaphore = Arc::new(tokio::sync::Semaphore::new(concurrency));

    for file in files {
        let file = file.clone();
        let old_path = old_path.clone();
        let new_path = new_path.clone();
        let project_root = project_root.clone();
        let plugin_map = plugin_map.clone();
        let rename_info = rename_info.clone();
        let renamed_ext = renamed_ext.clone();

        let permit = match semaphore.clone().acquire_owned().await {
            Ok(p) => p,
            Err(_) => continue,
        };

        join_set.spawn(async move {
            let _permit = permit;
            if let Ok(content) = tokio::fs::read_to_string(&file).await {
                let file_clone = file.clone();
                let renamed_ext = renamed_ext.clone();
                let result = tokio::task::spawn_blocking(move || {
                    if let Some(ext) = file_clone.extension().and_then(|e| e.to_str()) {
                        if !crate::services::reference_updater::is_extension_compatible_for_rewrite(
                            renamed_ext.as_deref(),
                            Some(ext),
                        ) {
                            return None;
                        }
                        if !allow_rewrite && is_web_extension(ext) {
                            return None;
                        }
                        if is_directory
                            && is_web_extension(ext)
                            && !content_might_contain_alias_imports(&content)
                        {
                            return None;
                        }
                        if let Some(plugin) = plugin_map.get(ext) {
                            let rewrite_result = plugin.rewrite_file_references(
                                &content,
                                &old_path,
                                &new_path,
                                &file_clone,
                                &project_root,
                                rename_info.as_ref(),
                            );

                            if let Some((updated_content, change_count)) = rewrite_result {
                                if change_count > 0 && updated_content != content {
                                    return Some(file_clone);
                                }
                            }
                        }
                    }
                    None
                })
                .await;

                match result {
                    Ok(Some(f)) => Some(f),
                    _ => None,
                }
            } else {
                None
            }
        });
    }

    while let Some(res) = join_set.join_next().await {
        if let Ok(Some(file)) = res {
            affected.push(file);
        }
    }

    affected
}

/// Get all files imported by the given file content
///
/// Uses plugin import support if available, falls back to regex-based extraction.
pub(crate) fn get_all_imported_files(
    content: &str,
    current_file: &Path,
    plugins: &[Arc<dyn LanguagePlugin>],
    plugin_map: &HashMap<String, Arc<dyn LanguagePlugin>>,
    project_files: &[PathBuf],
    project_root: &Path,
) -> Vec<PathBuf> {
    let resolver = mill_ast::ImportPathResolver::with_plugins(project_root, plugins.to_vec());
    get_all_imported_files_internal(content, current_file, plugin_map, project_files, &resolver)
}

/// Internal helper that takes a pre-created resolver
fn get_all_imported_files_internal(
    content: &str,
    current_file: &Path,
    plugin_map: &HashMap<String, Arc<dyn LanguagePlugin>>,
    project_files: &[PathBuf],
    resolver: &mill_ast::ImportPathResolver,
) -> Vec<PathBuf> {
    let mut imported_files = Vec::new();

    if let Some(ext) = current_file.extension().and_then(|e| e.to_str()) {
        if let Some(plugin) = plugin_map.get(ext) {
            if let Some(import_parser) = plugin.import_parser() {
                let import_specifiers = import_parser.parse_imports(content);
                for specifier in import_specifiers {
                    if let Some(resolved) =
                        resolve_import_to_file(&specifier, current_file, project_files, resolver)
                    {
                        imported_files.push(resolved);
                    }
                }
                return imported_files;
            }
        }
    }

    // Fallback: use regex-based extraction
    for line in content.lines() {
        if let Some(specifier) = extract_import_path(line) {
            if let Some(resolved) =
                resolve_import_to_file(&specifier, current_file, project_files, resolver)
            {
                imported_files.push(resolved);
            }
        }
    }

    imported_files
}

fn is_web_extension(ext: &str) -> bool {
    matches!(ext, "svelte" | "ts" | "tsx" | "js" | "jsx")
}

fn content_might_contain_alias_imports(content: &str) -> bool {
    if !(content.contains("import") || content.contains("require")) {
        return false;
    }
    content.contains("$lib/")
        || content.contains("@/")
        || content.contains("~/")
        || content.contains("$")
        || content.contains("@")
        || content.contains("~")
}

fn default_concurrency_limit() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get().saturating_mul(2))
        .unwrap_or(8)
        .clamp(4, 64)
}

/// Resolve an import specifier to a file path
fn resolve_import_to_file(
    specifier: &str,
    importing_file: &Path,
    project_files: &[PathBuf],
    resolver: &mill_ast::ImportPathResolver,
) -> Option<PathBuf> {
    resolver.resolve_import_to_file(specifier, importing_file, project_files)
}

/// Extract import path from a line of code using regex patterns
pub(crate) fn extract_import_path(line: &str) -> Option<String> {
    if line.contains("from") {
        if let Some(start) = line.find(['\'', '"']) {
            let quote_char = &line[start..=start];
            let path_start = start + 1;
            if let Some(end) = line[path_start..].find(quote_char) {
                return Some(line[path_start..path_start + end].to_string());
            }
        }
    }
    if line.contains("require") {
        if let Some(start) = line.find(['\'', '"', '(']) {
            let path_start = if &line[start..=start] == "(" {
                start + 1
            } else {
                start
            };
            if let Some(quote_start) = line[path_start..].find(['\'', '"']) {
                let actual_start = path_start + quote_start + 1;
                let quote_char = &line[path_start + quote_start..path_start + quote_start + 1];
                if let Some(end) = line[actual_start..].find(quote_char) {
                    return Some(line[actual_start..actual_start + end].to_string());
                }
            }
        }
    }
    None
}
