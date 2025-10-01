//! Import path resolution and updating functionality

use crate::error::{AstError, AstResult};
// TODO: Re-add when import graph caching is implemented
// use crate::parser::ImportGraph;
// use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

/// Resolves and updates import paths when files are moved or renamed
pub struct ImportPathResolver {
    /// Project root directory
    project_root: PathBuf,
    // TODO: Add import graph caching for performance
    // import_cache: HashMap<PathBuf, ImportGraph>,
}

impl ImportPathResolver {
    /// Create a new import path resolver
    pub fn new(project_root: impl AsRef<Path>) -> Self {
        Self {
            project_root: project_root.as_ref().to_path_buf(),
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
    pub async fn find_affected_files(
        &self,
        renamed_file: &Path,
        project_files: &[PathBuf],
    ) -> AstResult<Vec<PathBuf>> {
        let mut affected = Vec::new();
        let _renamed_str = renamed_file.to_string_lossy();

        for file in project_files {
            if file == renamed_file {
                continue; // Skip the renamed file itself
            }

            // Check if this file might import the renamed file
            if let Ok(content) = tokio::fs::read_to_string(file).await {
                if self.file_imports_target(&content, renamed_file) {
                    affected.push(file.clone());
                }
            }
        }

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

/// Update import paths in all affected files after a file rename
pub async fn update_import_paths(
    old_path: &Path,
    new_path: &Path,
    project_root: &Path,
    dry_run: bool,
) -> AstResult<ImportUpdateResult> {
    let resolver = ImportPathResolver::new(project_root);

    // Find all TypeScript/JavaScript files in the project
    let project_files = find_project_files(project_root).await?;

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
        match update_imports_in_file(&file_path, old_path, new_path, &resolver, dry_run).await {
            Ok(count) => {
                if count > 0 {
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

/// Update imports in a single file
async fn update_imports_in_file(
    file_path: &Path,
    old_target: &Path,
    new_target: &Path,
    resolver: &ImportPathResolver,
    dry_run: bool,
) -> AstResult<usize> {
    let content = tokio::fs::read_to_string(file_path)
        .await
        .map_err(|e| AstError::parse(format!("Failed to read file: {}", e)))?;

    let mut updated_content = String::new();
    let mut updates_count = 0;
    let old_target_stem = old_target
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("");

    for line in content.lines() {
        if line.contains("import") || line.contains("require") {
            if line.contains(old_target_stem) {
                // This line likely contains an import that needs updating
                if let Some(updated_line) =
                    update_import_line(line, file_path, old_target, new_target, resolver)
                {
                    updated_content.push_str(&updated_line);
                    updates_count += 1;
                } else {
                    updated_content.push_str(line);
                }
            } else {
                updated_content.push_str(line);
            }
        } else {
            updated_content.push_str(line);
        }
        updated_content.push('\n');
    }

    if updates_count > 0 && !dry_run {
        // Write the updated content back to the file only if not in dry run mode
        tokio::fs::write(file_path, updated_content.trim_end())
            .await
            .map_err(|e| AstError::transformation(format!("Failed to write file: {}", e)))?;
        debug!(file = ?file_path, "Wrote updated imports to file");
    } else if updates_count > 0 && dry_run {
        debug!(file = ?file_path, changes = updates_count, "[DRY RUN] Would update imports");
    }

    Ok(updates_count)
}

/// Update a single import line
fn update_import_line(
    line: &str,
    importing_file: &Path,
    old_target: &Path,
    new_target: &Path,
    resolver: &ImportPathResolver,
) -> Option<String> {
    // Extract the import path from the line
    let import_path = extract_import_path(line)?;

    // Calculate the new import path
    if let Ok(new_import_path) =
        resolver.calculate_new_import_path(importing_file, old_target, new_target, &import_path)
    {
        // Replace the old import path with the new one
        Some(line.replace(&import_path, &new_import_path))
    } else {
        None
    }
}

/// Extract import path from an import/require statement
fn extract_import_path(line: &str) -> Option<String> {
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

/// Find all TypeScript/JavaScript files in a project
async fn find_project_files(project_root: &Path) -> AstResult<Vec<PathBuf>> {
    let mut files = Vec::new();
    let extensions = ["ts", "tsx", "js", "jsx", "mjs", "cjs"];

    fn collect_files<'a>(
        dir: &'a Path,
        files: &'a mut Vec<PathBuf>,
        extensions: &'a [&str],
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = AstResult<()>> + Send + 'a>> {
        Box::pin(async move {
            if dir.is_dir() {
                // Skip node_modules and other common directories to ignore
                if let Some(dir_name) = dir.file_name() {
                    let name = dir_name.to_string_lossy();
                    if name == "node_modules" || name == ".git" || name == "dist" || name == "build"
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
                        collect_files(&path, files, extensions).await?;
                    } else if let Some(ext) = path.extension() {
                        if extensions.contains(&ext.to_str().unwrap_or("")) {
                            files.push(path);
                        }
                    }
                }
            }
            Ok(())
        })
    }

    collect_files(project_root, &mut files, &extensions).await?;
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
}
