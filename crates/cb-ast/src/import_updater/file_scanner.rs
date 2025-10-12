use crate::error::{AstError, AstResult};
use crate::import_updater::path_resolver::{FileImportInfo, ImportPathResolver};
use std::path::{Path, PathBuf};
use tracing::debug;

impl ImportPathResolver {
    /// Find all files that need import updates after a rename
    /// Uses caching to avoid re-scanning files that haven't changed
    pub async fn find_affected_files(
        &self,
        renamed_file: &Path,
        project_files: &[PathBuf],
    ) -> AstResult<Vec<PathBuf>> {
        let mut affected = Vec::new();
        let renamed_stem = renamed_file
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("");

        for file in project_files {
            if file == renamed_file {
                continue; // Skip the renamed file itself
            }

            // Check cache first (release lock immediately)
            let cache_result = {
                if let Ok(cache) = self.import_cache.lock() {
                    cache.get(file).cloned()
                } else {
                    None
                }
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
                    if self.file_imports_target(&content, renamed_file) {
                        affected.push(file.clone());

                        // Update cache with this file's imports
                        if let Ok(metadata) = tokio::fs::metadata(file).await {
                            if let Ok(modified) = metadata.modified() {
                                if let Ok(mut cache) = self.import_cache.lock() {
                                    cache.insert(
                                        file.clone(),
                                        FileImportInfo {
                                            imports: vec![renamed_file.to_path_buf()],
                                            last_modified: modified,
                                        },
                                    );
                                }
                            }
                        }
                    } else if !renamed_stem.is_empty() && !content.contains(renamed_stem) {
                        // File definitely doesn't import the target - cache this negative result
                        if let Ok(metadata) = tokio::fs::metadata(file).await {
                            if let Ok(modified) = metadata.modified() {
                                if let Ok(mut cache) = self.import_cache.lock() {
                                    cache.insert(
                                        file.clone(),
                                        FileImportInfo {
                                            imports: vec![],
                                            last_modified: modified,
                                        },
                                    );
                                }
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

    /// Check if a file's content imports a target file
    fn file_imports_target(&self, content: &str, target: &Path) -> bool {
        let target_stem = target.file_stem().and_then(|s| s.to_str()).unwrap_or("");

        // Check for various import patterns that might reference this file
        // For simplicity, just check if the file stem appears in import statements
        if !content.contains(target_stem) {
            return false;
        }

        // Check for ES6 imports: import ... from './target'
        if content.contains("import") && content.contains("from") {
            for line in content.lines() {
                if line.contains("import") && line.contains("from") && line.contains(target_stem) {
                    return true;
                }
            }
        }

        // Check for CommonJS: require('./target')
        if content.contains("require") {
            for line in content.lines() {
                if line.contains("require") && line.contains(target_stem) {
                    return true;
                }
            }
        }

        false
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
