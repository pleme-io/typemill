//! Import path resolution and updating functionality

use crate::error::{AstError, AstResult};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tracing::{debug, info, warn};

/// Cached information about a file's imports
#[derive(Debug, Clone)]
pub struct FileImportInfo {
    /// The files that this file imports
    pub imports: Vec<PathBuf>,
    /// Last modified time when this cache entry was created
    pub last_modified: std::time::SystemTime,
}

/// Resolves and updates import paths when files are moved or renamed
pub struct ImportPathResolver {
    /// Project root directory
    project_root: PathBuf,
    /// Cache of file import information for performance
    /// Maps file path -> (imports, last_modified_time)
    import_cache: Arc<Mutex<HashMap<PathBuf, FileImportInfo>>>,
}

impl ImportPathResolver {
    /// Create a new import path resolver
    pub fn new(project_root: impl AsRef<Path>) -> Self {
        Self {
            project_root: project_root.as_ref().to_path_buf(),
            import_cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Create a new resolver with a shared cache (for performance)
    pub fn with_cache(
        project_root: impl AsRef<Path>,
        cache: Arc<Mutex<HashMap<PathBuf, FileImportInfo>>>,
    ) -> Self {
        Self {
            project_root: project_root.as_ref().to_path_buf(),
            import_cache: cache,
        }
    }

    /// Clear the import cache
    pub fn clear_cache(&self) {
        if let Ok(mut cache) = self.import_cache.lock() {
            cache.clear();
            debug!("Cleared import cache");
        }
    }

    /// Get cache statistics for monitoring
    pub fn cache_stats(&self) -> (usize, usize) {
        if let Ok(cache) = self.import_cache.lock() {
            let total = cache.len();
            let valid = cache
                .iter()
                .filter(|(path, info)| {
                    if let Ok(metadata) = std::fs::metadata(path) {
                        if let Ok(modified) = metadata.modified() {
                            return modified == info.last_modified;
                        }
                    }
                    false
                })
                .count();
            (total, valid)
        } else {
            (0, 0)
        }
    }

    /// Calculate the new import path after a file rename
    pub fn calculate_new_import_path(
        &self,
        importing_file: &Path,
        old_target_path: &Path,
        new_target_path: &Path,
        original_import: &str,
    ) -> AstResult<String> {
        // Handle different import styles
        if original_import.starts_with("./") || original_import.starts_with("../") {
            // Relative import - calculate new relative path
            self.calculate_relative_import(importing_file, new_target_path)
        } else if original_import.starts_with("@/") || original_import.starts_with("~/") {
            // Alias import - update the path after the alias
            self.update_alias_import(original_import, old_target_path, new_target_path)
        } else {
            // Absolute or package import - might not need updating
            Ok(original_import.to_string())
        }
    }

    /// Calculate relative import path between two files
    fn calculate_relative_import(&self, from_file: &Path, to_file: &Path) -> AstResult<String> {
        let from_dir = from_file
            .parent()
            .ok_or_else(|| AstError::parse("Invalid source file path"))?;

        let relative = pathdiff::diff_paths(to_file, from_dir)
            .ok_or_else(|| AstError::parse("Cannot calculate relative path"))?;

        // Remove extension for TypeScript/JavaScript imports
        let mut relative_str = relative.to_string_lossy().to_string();
        if let Some(ext) = to_file.extension() {
            let ext_str = ext.to_str().unwrap_or("");
            if matches!(ext_str, "ts" | "tsx" | "js" | "jsx") {
                relative_str = relative_str
                    .trim_end_matches(&format!(".{}", ext_str))
                    .to_string();
            }
        }

        // Ensure relative imports start with ./ or ../
        if !relative_str.starts_with("../") && !relative_str.starts_with("./") {
            relative_str = format!("./{}", relative_str);
        }

        // Convert backslashes to forward slashes for cross-platform compatibility
        Ok(relative_str.replace('\\', "/"))
    }

    /// Update alias-based imports
    fn update_alias_import(
        &self,
        original_import: &str,
        old_path: &Path,
        new_path: &Path,
    ) -> AstResult<String> {
        // Extract the alias prefix (e.g., "@/", "~/")
        let alias_end = original_import.find('/').unwrap_or(original_import.len());
        let alias = &original_import[..alias_end];

        // Get the path after the alias
        let path_after_alias = if alias_end < original_import.len() {
            &original_import[alias_end + 1..]
        } else {
            ""
        };

        // Check if the old path matches this import
        if old_path.to_string_lossy().contains(path_after_alias) {
            // Replace the old path component with the new one
            let new_path_str = new_path.to_string_lossy();
            let new_path_component =
                new_path_str.trim_start_matches(&self.project_root.to_string_lossy().to_string());
            Ok(format!("{}{}", alias, new_path_component))
        } else {
            Ok(original_import.to_string())
        }
    }

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
                            let is_affected = cached_info.imports.contains(&renamed_file.to_path_buf());
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

/// Result of updating imports in multiple files
#[derive(Debug)]
pub struct ImportUpdateResult {
    /// Files that were successfully updated
    pub updated_files: Vec<PathBuf>,
    /// Files that failed to update with error messages
    pub failed_files: Vec<(PathBuf, String)>,
    /// Total number of imports updated
    pub imports_updated: usize,
}

/// Update import paths in all affected files after a file/directory rename
pub async fn update_imports_for_rename(
    old_path: &Path,
    new_path: &Path,
    project_root: &Path,
    adapters: &[std::sync::Arc<dyn crate::language::LanguageAdapter>],
    rename_info: Option<&serde_json::Value>,
    dry_run: bool,
) -> AstResult<ImportUpdateResult> {
    let resolver = ImportPathResolver::new(project_root);

    // Find all files that the adapters handle
    let project_files = find_project_files(project_root, adapters).await?;

    // Find files that import the renamed file
    let affected_files = resolver.find_affected_files(old_path, &project_files).await?;

    info!(
        dry_run = dry_run,
        affected_files = affected_files.len(),
        old_path = ?old_path,
        "Found files potentially affected by rename"
    );

    let mut result = ImportUpdateResult {
        updated_files: Vec::new(),
        failed_files: Vec::new(),
        imports_updated: 0,
    };

    for file_path in affected_files {
        // Find the appropriate adapter for this file
        let adapter = if let Some(ext) = file_path.extension() {
            let ext_str = ext.to_str().unwrap_or("");
            adapters.iter().find(|a| a.handles_extension(ext_str))
        } else {
            None
        };

        let adapter = match adapter {
            Some(a) => a,
            None => {
                debug!(file = ?file_path, "No adapter found for file extension");
                continue;
            }
        };

        // Read file content
        let content = match tokio::fs::read_to_string(&file_path).await {
            Ok(c) => c,
            Err(e) => {
                warn!(error = %e, file = ?file_path, "Failed to read file");
                result.failed_files.push((file_path, e.to_string()));
                continue;
            }
        };

        // Use adapter to rewrite imports
        match adapter.rewrite_imports_for_rename(
            &content,
            old_path,
            new_path,
            &file_path,
            project_root,
            rename_info,
        ) {
            Ok((updated_content, count)) => {
                if count > 0 {
                    if !dry_run {
                        // Write the updated content back to the file
                        if let Err(e) = tokio::fs::write(&file_path, updated_content).await {
                            warn!(error = %e, file = ?file_path, "Failed to write file");
                            result.failed_files.push((file_path, e.to_string()));
                            continue;
                        }
                        debug!(file = ?file_path, "Wrote updated imports to file");
                    } else {
                        debug!(file = ?file_path, changes = count, "[DRY RUN] Would update imports");
                    }

                    result.updated_files.push(file_path.clone());
                    result.imports_updated += count;
                    debug!(
                        imports = count,
                        file = ?file_path,
                        dry_run = dry_run,
                        "Updated imports in file"
                    );
                }
            }
            Err(e) => {
                warn!(error = %e, file = ?file_path, "Failed to update imports");
                result.failed_files.push((file_path, e.to_string()));
            }
        }
    }

    info!(
        files_updated = result.updated_files.len(),
        imports_updated = result.imports_updated,
        dry_run = dry_run,
        "Import update complete"
    );

    Ok(result)
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
    adapters: &[std::sync::Arc<dyn crate::language::LanguageAdapter>],
) -> AstResult<Vec<PathBuf>> {
    let mut files = Vec::new();

    fn collect_files<'a>(
        dir: &'a Path,
        files: &'a mut Vec<PathBuf>,
        adapters: &'a [std::sync::Arc<dyn crate::language::LanguageAdapter>],
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = AstResult<()>> + Send + 'a>> {
        Box::pin(async move {
            if dir.is_dir() {
                // Skip node_modules and other common directories to ignore
                if let Some(dir_name) = dir.file_name() {
                    let name = dir_name.to_string_lossy();
                    if name == "node_modules"
                        || name == ".git"
                        || name == "dist"
                        || name == "build"
                        || name == "target"
                    {
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
                        collect_files(&path, files, adapters).await?;
                    } else if let Some(ext) = path.extension() {
                        let ext_str = ext.to_str().unwrap_or("");
                        // Check if any adapter handles this extension
                        if adapters.iter().any(|adapter| adapter.handles_extension(ext_str)) {
                            files.push(path);
                        }
                    }
                }
            }
            Ok(())
        })
    }

    collect_files(project_root, &mut files, adapters).await?;
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_calculate_relative_import() {
        let temp_dir = TempDir::new().unwrap();
        let resolver = ImportPathResolver::new(temp_dir.path());

        let from_file = temp_dir.path().join("src/components/Button.tsx");
        let to_file = temp_dir.path().join("src/utils/helpers.ts");

        let result = resolver
            .calculate_relative_import(&from_file, &to_file)
            .unwrap();
        assert_eq!(result, "../utils/helpers");
    }

    #[test]
    fn test_extract_import_path() {
        let line1 = "import { Component } from './component';";
        assert_eq!(extract_import_path(line1), Some("./component".to_string()));

        let line2 = "const utils = require('../utils/helpers');";
        assert_eq!(
            extract_import_path(line2),
            Some("../utils/helpers".to_string())
        );

        let line3 = "import React from 'react';";
        assert_eq!(extract_import_path(line3), Some("react".to_string()));
    }

    #[tokio::test]
    async fn test_import_cache_usage() {
        use std::fs;
        use std::io::Write;

        let temp_dir = TempDir::new().unwrap();
        let cache = Arc::new(Mutex::new(HashMap::new()));
        let resolver = ImportPathResolver::with_cache(temp_dir.path(), cache.clone());

        // Create test files (consistent naming with imports)
        let file_a = temp_dir.path().join("fileA.ts");
        let fileb = temp_dir.path().join("fileB.ts");
        let file_c = temp_dir.path().join("fileC.ts");

        // fileA imports fileB using './fileB' path
        fs::write(&file_a, "import { foo } from './fileB';\n").unwrap();
        fs::write(&fileb, "export const foo = 1;\n").unwrap();
        fs::write(&file_c, "import { bar } from './other';\n").unwrap();

        // First call - should populate cache
        let project_files = vec![file_a.clone(), fileb.clone(), file_c.clone()];
        let affected = resolver
            .find_affected_files(&fileb, &project_files)
            .await
            .unwrap();
        assert_eq!(affected.len(), 1);
        assert!(affected.contains(&file_a));

        // Check cache stats - should have entries now
        let (total, valid) = resolver.cache_stats();
        assert!(total > 0, "Cache should have entries after first scan");
        assert!(valid > 0, "Cache should have valid entries");

        // Second call - should use cache (file hasn't been modified)
        let affected2 = resolver
            .find_affected_files(&fileb, &project_files)
            .await
            .unwrap();
        assert_eq!(affected2, affected, "Cached results should match");

        // Modify fileA to invalidate cache
        std::thread::sleep(std::time::Duration::from_millis(10));
        let mut file = fs::OpenOptions::new().append(true).open(&file_a).unwrap();
        file.write_all(b"// comment\n").unwrap();
        drop(file);

        // Third call - cache should be invalidated for fileA
        let affected3 = resolver
            .find_affected_files(&fileb, &project_files)
            .await
            .unwrap();
        assert_eq!(affected3.len(), 1);
        assert!(affected3.contains(&file_a), "Should still detect fileA after modification");
    }
}
