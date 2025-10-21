use crate::error::{AstError, AstResult};
use crate::import_updater::path_resolver::{FileImportInfo, ImportPathResolver};
use std::path::{Path, PathBuf};
use tracing::debug;

/// Check if import cache is enabled via environment variables
/// Returns true if cache should be used, false if disabled
fn is_import_cache_enabled() -> bool {
    // Check master switch first
    if let Ok(val) = std::env::var("CODEBUDDY_DISABLE_CACHE") {
        if val == "1" || val.to_lowercase() == "true" {
            return false;
        }
    }

    // Check import-specific switch
    if let Ok(val) = std::env::var("CODEBUDDY_DISABLE_IMPORT_CACHE") {
        if val == "1" || val.to_lowercase() == "true" {
            return false;
        }
    }

    true
}

impl ImportPathResolver {
    /// Find all files that need import updates after a rename
    /// Uses caching to avoid re-scanning files that haven't changed
    pub async fn find_affected_files(
        &self,
        renamed_file: &Path,
        project_files: &[PathBuf],
        plugins: &[std::sync::Arc<dyn cb_plugin_api::LanguagePlugin>],
    ) -> AstResult<Vec<PathBuf>> {
        let mut affected = Vec::new();

        for file in project_files {
            if file == renamed_file {
                continue; // Skip the renamed file itself
            }

            // Check cache first (release lock immediately)
            // Can be disabled via environment variables:
            // - CODEBUDDY_DISABLE_CACHE=1 (disables all caches)
            // - CODEBUDDY_DISABLE_IMPORT_CACHE=1 (disables only import cache)
            let cache_enabled = is_import_cache_enabled();

            let cache_result = if cache_enabled {
                if let Ok(cache) = self.import_cache.lock() {
                    cache.get(file).cloned()
                } else {
                    None
                }
            } else {
                debug!("Import cache disabled via CODEBUDDY_DISABLE_IMPORT_CACHE");
                None
            };

            let (should_scan, is_cached_hit) = if let Some(cached_info) = cache_result {
                // Verify cache is still valid by checking file modification time
                if let Ok(metadata) = tokio::fs::metadata(file).await {
                    if let Ok(modified) = metadata.modified() {
                        if modified == cached_info.last_modified {
                            // Cache hit - check if renamed file is in cached imports
                            debug!(file = ?file, "Cache hit for import check");
                            let is_affected =
                                cached_info.imports.contains(&renamed_file.to_path_buf());
                            (false, is_affected)
                        } else {
                            // Cache stale - need to re-scan
                            debug!(file = ?file, "Cache stale, re-scanning");
                            (true, false)
                        }
                    } else {
                        (true, false)
                    }
                } else {
                    (true, false)
                }
            } else {
                // Cache miss - need to scan
                debug!(file = ?file, "Cache miss for import check");
                (true, false)
            };

            if is_cached_hit {
                affected.push(file.clone());
                continue;
            }

            if should_scan {
                // Read file and check for imports
                if let Ok(content) = tokio::fs::read_to_string(file).await {
                    // Parse ALL imports from this file for proper caching
                    let all_imports =
                        self.get_all_imported_files(&content, file, plugins, project_files);

                    let is_affected = all_imports.contains(&renamed_file.to_path_buf());

                    if is_affected {
                        affected.push(file.clone());
                    }

                    // Update cache with this file's COMPLETE import list
                    if let Ok(metadata) = tokio::fs::metadata(file).await {
                        if let Ok(modified) = metadata.modified() {
                            if let Ok(mut cache) = self.import_cache.lock() {
                                cache.insert(
                                    file.clone(),
                                    FileImportInfo {
                                        imports: all_imports,
                                        last_modified: modified,
                                    },
                                );
                            }
                        }
                    }
                }
            }
        }

        debug!(
            affected_files = affected.len(),
            total_files = project_files.len(),
            "Found affected files"
        );

        Ok(affected)
    }

    /// Get all files imported by a file, resolved to absolute paths
    pub(crate) fn get_all_imported_files(
        &self,
        content: &str,
        current_file: &Path,
        plugins: &[std::sync::Arc<dyn cb_plugin_api::LanguagePlugin>],
        project_files: &[PathBuf],
    ) -> Vec<PathBuf> {
        let mut imported_files = Vec::new();

        // Try to use plugin-specific import parsing
        if let Some(ext) = current_file.extension().and_then(|e| e.to_str()) {
            for plugin in plugins {
                if plugin.handles_extension(ext) {
                    if let Some(import_parser) = plugin.import_parser() {
                        // Parse all imports using the plugin
                        let import_specifiers =
                            cb_plugin_api::ImportParser::parse_imports(import_parser, content);

                        // Resolve each import specifier to an absolute file path
                        for specifier in import_specifiers {
                            if let Some(resolved) =
                                self.resolve_import_to_file(&specifier, current_file, project_files)
                            {
                                imported_files.push(resolved);
                            }
                        }
                        return imported_files;
                    }
                }
            }
        }

        // Fallback: parse imports manually if no plugin found
        for line in content.lines() {
            if let Some(specifier) = extract_import_path(line) {
                if let Some(resolved) =
                    self.resolve_import_to_file(&specifier, current_file, project_files)
                {
                    imported_files.push(resolved);
                }
            }
        }

        imported_files
    }

    /// Resolve an import specifier (like './utils' or '../api') to an absolute file path
    ///
    /// Handles:
    /// - Relative paths (./foo, ../foo)
    /// - Absolute paths (/foo)
    /// - Bare specifiers for markdown links (API_REFERENCE.md, docs/file.md)
    /// - Extension inference (.ts, .tsx, .js, .jsx, .rs)
    pub fn resolve_import_to_file(
        &self,
        specifier: &str,
        importing_file: &Path,
        project_files: &[PathBuf],
    ) -> Option<PathBuf> {
        // Try explicit relative/absolute paths first (./foo, ../foo, /foo)
        if specifier.starts_with("./") || specifier.starts_with("../") || specifier.starts_with('/')
        {
            let importing_dir = importing_file.parent()?;
            let candidate = importing_dir.join(specifier);
            let extensions = ["", ".ts", ".tsx", ".js", ".jsx", ".rs"];
            for ext in extensions {
                let candidate_with_ext = if ext.is_empty() {
                    candidate.clone()
                } else {
                    candidate.with_extension(&ext[1..])
                };
                if let Ok(abs_candidate) = candidate_with_ext.canonicalize() {
                    if project_files.contains(&abs_candidate) {
                        return Some(abs_candidate);
                    }
                }
            }
        }

        // For bare specifiers (e.g., "API_REFERENCE.md"), try project-relative paths
        // This supports markdown links like [text](API_REFERENCE.md) or [text](docs/file.md)
        let project_relative_candidate = self.project_root().join(specifier);

        // First try canonical path if file exists (works for dry-run before file is moved)
        if let Ok(abs_candidate) = project_relative_candidate.canonicalize() {
            if project_files.contains(&abs_candidate) {
                return Some(abs_candidate);
            }
        }

        // If canonicalization fails (file has been moved/deleted), try matching by basename
        // against project_files. This handles the execution path where the file has been moved.
        // We need to check if any file in project_files matches the specifier basename OR
        // the expected path structure.
        if let Some(candidate_filename) = Path::new(specifier).file_name() {
            // Try exact path match first (e.g., "docs/API_REFERENCE.md")
            for project_file in project_files {
                // Check if project file ends with the specifier path
                if let Ok(relative) = project_file.strip_prefix(self.project_root()) {
                    if relative.to_string_lossy() == specifier {
                        return Some(project_file.clone());
                    }
                }
            }

            // Fall back to basename matching (e.g., "API_REFERENCE.md" matches any file with that name)
            for project_file in project_files {
                if project_file.file_name() == Some(candidate_filename) {
                    return Some(project_file.clone());
                }
            }
        }

        None
    }
}

/// Extract import path from an import/require statement
pub fn extract_import_path(line: &str) -> Option<String> {
    // Handle ES6 imports: import ... from 'path'
    if line.contains("from") {
        if let Some(start) = line.find(['\'', '"']) {
            let quote_char = &line[start..=start];
            let path_start = start + 1;
            if let Some(end) = line[path_start..].find(quote_char) {
                return Some(line[path_start..path_start + end].to_string());
            }
        }
    }

    // Handle CommonJS: require('path')
    if line.contains("require") {
        if let Some(start) = line.find(['\'', '"']) {
            let quote_char = &line[start..=start];
            let path_start = start + 1;
            if let Some(end) = line[path_start..].find(quote_char) {
                return Some(line[path_start..path_start + end].to_string());
            }
        }
    }

    None
}

/// Find all project files that match the language adapters
pub async fn find_project_files(
    project_root: &Path,
    plugins: &[std::sync::Arc<dyn cb_plugin_api::LanguagePlugin>],
) -> AstResult<Vec<PathBuf>> {
    let mut files = Vec::new();

    fn collect_files<'a>(
        dir: &'a Path,
        files: &'a mut Vec<PathBuf>,
        plugins: &'a [std::sync::Arc<dyn cb_plugin_api::LanguagePlugin>],
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = AstResult<()>> + Send + 'a>> {
        Box::pin(async move {
            if dir.is_dir() {
                // Skip common build/cache directories
                if let Some(dir_name) = dir.file_name() {
                    const IGNORED_DIRS: &[&str] = &[
                        ".build",
                        ".git",
                        ".next",
                        ".pytest_cache",
                        ".tox",
                        ".venv",
                        "__pycache__",
                        "build",
                        "dist",
                        "node_modules",
                        "target",
                        "venv",
                    ];

                    let name = dir_name.to_string_lossy();
                    if IGNORED_DIRS.contains(&name.as_ref()) {
                        return Ok(());
                    }
                }

                let mut read_dir = tokio::fs::read_dir(dir)
                    .await
                    .map_err(|e| AstError::parse(format!("Failed to read directory: {}", e)))?;

                while let Some(entry) = read_dir
                    .next_entry()
                    .await
                    .map_err(|e| AstError::parse(format!("Failed to read entry: {}", e)))?
                {
                    let path = entry.path();
                    if path.is_dir() {
                        collect_files(&path, files, plugins).await?;
                    } else if let Some(ext) = path.extension() {
                        let ext_str = ext.to_str().unwrap_or("");
                        // Check if any plugin handles this extension
                        if plugins
                            .iter()
                            .any(|plugin| plugin.handles_extension(ext_str))
                        {
                            files.push(path);
                        }
                    }
                }
            }
            Ok(())
        })
    }

    collect_files(project_root, &mut files, plugins).await?;
    Ok(files)
}
