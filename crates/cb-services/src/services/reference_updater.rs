//! Service for updating references in a workspace
//！
//！ This service is responsible for finding all references to a given file or symbol
//！ and updating them to a new path or name. It is language-agnostic and delegates
//！ language-specific logic to plugins.

use cb_protocol::{ApiError as ServerError, ApiResult as ServerResult, EditPlan, EditPlanMetadata, TextEdit, EditLocation, EditType, DependencyUpdate};
use std::path::{Path, PathBuf};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use cb_plugin_api::{LanguagePlugin};

// From path_resolver.rs
/// Cached information about a file's imports
#[derive(Debug, Clone)]
pub struct FileImportInfo {
    /// The files that this file imports
    pub imports: Vec<PathBuf>,
    /// Last modified time when this cache entry was created
    pub last_modified: std::time::SystemTime,
}

/// A service for updating references in a workspace.
pub struct ReferenceUpdater {
    /// Project root directory
    project_root: PathBuf,
    /// Cache of file import information for performance
    /// Maps file path -> (imports, last_modified_time)
    pub(crate) import_cache: Arc<Mutex<HashMap<PathBuf, FileImportInfo>>>,
}

impl ReferenceUpdater {
    /// Creates a new `ReferenceUpdater`.
    pub fn new(project_root: impl AsRef<Path>) -> Self {
        Self {
            project_root: project_root.as_ref().to_path_buf(),
            import_cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Updates all references to `old_path` to point to `new_path`.
    pub async fn update_references(
        &self,
        old_path: &Path,
        new_path: &Path,
        plugins: &[std::sync::Arc<dyn cb_plugin_api::LanguagePlugin>],
        rename_info: Option<&serde_json::Value>,
        _dry_run: bool,
        _scan_scope: Option<cb_plugin_api::ScanScope>,
    ) -> ServerResult<EditPlan> {
        // From edit_builder.rs
        let project_files = find_project_files(&self.project_root, plugins).await?;
        let is_directory_rename = old_path.is_dir();

        let mut affected_files = if is_directory_rename {
            let mut all_affected = HashSet::new();
            let files_in_directory: Vec<&PathBuf> = project_files
                .iter()
                .filter(|f| f.starts_with(old_path) && f.is_file())
                .collect();
            for file_in_dir in files_in_directory {
                let relative_path = file_in_dir.strip_prefix(old_path).unwrap_or(file_in_dir);
                let new_file_path = new_path.join(relative_path);
                let importers = self.find_affected_files_for_rename(file_in_dir, &new_file_path, &project_files, plugins).await?;
                all_affected.extend(importers);
            }
            all_affected.into_iter().collect()
        } else {
            self.find_affected_files_for_rename(old_path, new_path, &project_files, plugins).await?
        };

        if is_directory_rename {
            affected_files.retain(|file| !file.starts_with(old_path));
        }

        let mut all_edits = Vec::new();

        for file_path in affected_files {
            let plugin = if let Some(ext) = file_path.extension() {
                let ext_str = ext.to_str().unwrap_or("");
                plugins.iter().find(|p| p.handles_extension(ext_str))
            } else {
                None
            };

            let plugin = match plugin {
                Some(p) => p,
                None => continue,
            };

            let content = match tokio::fs::read_to_string(&file_path).await {
                Ok(c) => c,
                Err(_) => continue,
            };

            if is_directory_rename {
                // Directory rename logic
                let files_in_directory: Vec<PathBuf> = project_files
                    .iter()
                    .filter(|f| f.starts_with(old_path) && f.is_file())
                    .cloned()
                    .collect();

                let mut current_content = content.clone();
                let mut total_changes = 0;

                for old_file_in_dir in &files_in_directory {
                    let relative_path = old_file_in_dir.strip_prefix(old_path).unwrap_or(old_file_in_dir);
                    let new_file_path = new_path.join(relative_path);

                    let rewrite_result = plugin.rewrite_file_references(
                        &current_content,
                        old_file_in_dir,
                        &new_file_path,
                        &file_path,
                        &self.project_root,
                        rename_info,
                    );
                    if let Some((updated_content, count)) = rewrite_result {
                        if count > 0 && updated_content != current_content {
                            total_changes += count;
                            current_content = updated_content;
                        }
                    }
                }
                if total_changes > 0 && current_content != content {
                    let line_count = current_content.lines().count();
                    let last_line_len = current_content.lines().last().map(|l| l.len()).unwrap_or(0);

                    all_edits.push(TextEdit {
                        file_path: Some(file_path.to_string_lossy().to_string()),
                        edit_type: EditType::UpdateImport,
                        location: EditLocation {
                            start_line: 0,
                            start_column: 0,
                            end_line: line_count.saturating_sub(1) as u32,
                            end_column: last_line_len as u32,
                        },
                        original_text: content,
                        new_text: current_content,
                        priority: 1,
                        description: format!(
                            "Update imports in {} for directory rename ({} files)",
                            file_path.display(),
                            files_in_directory.len()
                        ),
                    });
                }

            } else {
                // File rename logic
                let rewrite_result = plugin.rewrite_file_references(
                    &content,
                    old_path,
                    new_path,
                    &file_path,
                    &self.project_root,
                    rename_info,
                );

                if let Some((updated_content, count)) = rewrite_result {
                    if count > 0 && updated_content != content {
                        let line_count = content.lines().count();
                        let last_line_len = content.lines().last().map(|l| l.len()).unwrap_or(0);

                        all_edits.push(TextEdit {
                            file_path: Some(file_path.to_string_lossy().to_string()),
                            edit_type: EditType::UpdateImport,
                            location: EditLocation {
                                start_line: 0,
                                start_column: 0,
                                end_line: line_count.saturating_sub(1) as u32,
                                end_column: last_line_len as u32,
                            },
                            original_text: content.clone(),
                            new_text: updated_content,
                            priority: 1,
                            description: format!(
                                "Update imports in {} for file rename",
                                file_path.display()
                            ),
                        });
                    }
                }
            }
        }

        Ok(EditPlan {
            source_file: old_path.to_string_lossy().to_string(),
            edits: all_edits,
            dependency_updates: Vec::new(),
            validations: Vec::new(),
            metadata: EditPlanMetadata {
                intent_name: "update_references".to_string(),
                intent_arguments: serde_json::json!({
                    "old_path": old_path.to_string_lossy(),
                    "new_path": new_path.to_string_lossy(),
                }),
                created_at: chrono::Utc::now(),
                complexity: 5,
                impact_areas: vec!["imports".to_string(), "file_references".to_string()],
            },
        })
    }

    pub async fn find_affected_files(
        &self,
        renamed_file: &Path,
        project_files: &[PathBuf],
        plugins: &[std::sync::Arc<dyn cb_plugin_api::LanguagePlugin>],
    ) -> ServerResult<Vec<PathBuf>> {
        let mut affected = Vec::new();

        for file in project_files {
            if file == renamed_file {
                continue;
            }
            if let Ok(content) = tokio::fs::read_to_string(file).await {
                let all_imports = self.get_all_imported_files(&content, file, plugins, project_files);

                // Check if any import resolves to the renamed file
                if all_imports.contains(&renamed_file.to_path_buf()) {
                    affected.push(file.clone());
                }
            }
        }
        Ok(affected)
    }

    /// Find affected files for a rename operation, checking both old and new paths.
    /// This handles the case where the file has already been moved during execution.
    pub async fn find_affected_files_for_rename(
        &self,
        old_path: &Path,
        new_path: &Path,
        project_files: &[PathBuf],
        plugins: &[std::sync::Arc<dyn cb_plugin_api::LanguagePlugin>],
    ) -> ServerResult<Vec<PathBuf>> {
        let mut affected = Vec::new();

        for file in project_files {
            if file == old_path || file == new_path {
                continue;
            }
            if let Ok(content) = tokio::fs::read_to_string(file).await {
                let all_imports = self.get_all_imported_files(&content, file, plugins, project_files);

                // Check if imports reference either the old path (pre-move) or new path (post-move)
                if all_imports.contains(&old_path.to_path_buf()) || all_imports.contains(&new_path.to_path_buf()) {
                    affected.push(file.clone());
                }
            }
        }
        Ok(affected)
    }

    pub(crate) fn get_all_imported_files(
        &self,
        content: &str,
        current_file: &Path,
        plugins: &[std::sync::Arc<dyn cb_plugin_api::LanguagePlugin>],
        project_files: &[PathBuf],
    ) -> Vec<PathBuf> {
        let mut imported_files = Vec::new();
        if let Some(ext) = current_file.extension().and_then(|e| e.to_str()) {
            for plugin in plugins {
                if plugin.handles_extension(ext) {
                    if let Some(import_support) = plugin.import_support() {
                        let import_specifiers = import_support.parse_imports(content);
                        for specifier in import_specifiers {
                            if let Some(resolved) = self.resolve_import_to_file(&specifier, current_file, project_files) {
                                imported_files.push(resolved);
                            }
                        }
                        return imported_files;
                    }
                }
            }
        }
        for line in content.lines() {
            if let Some(specifier) = extract_import_path(line) {
                if let Some(resolved) = self.resolve_import_to_file(&specifier, current_file, project_files) {
                    imported_files.push(resolved);
                }
            }
        }
        imported_files
    }

    /// Resolve an import specifier to a file path
    ///
    /// Delegates to ImportPathResolver for consistent resolution logic.
    pub(crate) fn resolve_import_to_file(
        &self,
        specifier: &str,
        importing_file: &Path,
        project_files: &[PathBuf],
    ) -> Option<PathBuf> {
        let resolver = cb_ast::ImportPathResolver::new(&self.project_root);
        resolver.resolve_import_to_file(specifier, importing_file, project_files)
    }

    pub async fn update_import_reference(
        &self,
        file_path: &Path,
        update: &DependencyUpdate,
        plugins: &[std::sync::Arc<dyn cb_plugin_api::LanguagePlugin>],
    ) -> ServerResult<bool> {
        let extension = match file_path.extension().and_then(|s| s.to_str()) {
            Some(ext) => ext,
            None => return Ok(false),
        };

        let plugin = match plugins.iter().find(|p| p.handles_extension(extension)) {
            Some(p) => p,
            None => {
                return Ok(false);
            }
        };

        let import_support = match plugin.import_support() {
            Some(is) => is,
            None => {
                return Ok(false);
            }
        };

        let content = match tokio::fs::read_to_string(file_path).await {
            Ok(c) => c,
            Err(_) => return Ok(false),
        };

        let original_content = content.clone();
        let updated_content = import_support
            .update_import_reference(file_path, &content, update)
            .map_err(|e| {
                ServerError::Internal(format!("Failed to update import reference: {}", e))
            })?;

        if original_content == updated_content {
            return Ok(false);
        }

        tokio::fs::write(file_path, updated_content).await.map_err(|e| {
            ServerError::Internal(format!(
                "Failed to write updated content to {}: {}",
                file_path.display(),
                e
            ))
        })?;

        Ok(true)
    }
}


/// Find all project files that match the language adapters
pub async fn find_project_files(
    project_root: &Path,
    plugins: &[std::sync::Arc<dyn cb_plugin_api::LanguagePlugin>],
) -> ServerResult<Vec<PathBuf>> {
    let mut files = Vec::new();

    fn collect_files<'a>(
        dir: &'a Path,
        files: &'a mut Vec<PathBuf>,
        plugins: &'a [std::sync::Arc<dyn cb_plugin_api::LanguagePlugin>],
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ServerResult<()>> + Send + 'a>> {
        Box::pin(async move {
            if dir.is_dir() {
                if let Some(dir_name) = dir.file_name() {
                    const IGNORED_DIRS: &[&str] = &[
                        ".build", ".git", ".next", ".pytest_cache", ".tox", ".venv",
                        "__pycache__", "build", "dist", "node_modules", "target", "venv",
                    ];
                    let name = dir_name.to_string_lossy();
                    if IGNORED_DIRS.contains(&name.as_ref()) {
                        return Ok(());
                    }
                }

                let mut read_dir = tokio::fs::read_dir(dir).await.map_err(|e| ServerError::Internal(format!("Failed to read directory: {}", e)))?;
                while let Some(entry) = read_dir.next_entry().await.map_err(|e| ServerError::Internal(format!("Failed to read entry: {}", e)))? {
                    let path = entry.path();
                    if path.is_dir() {
                        collect_files(&path, files, plugins).await?;
                    } else if let Some(ext) = path.extension() {
                        let ext_str = ext.to_str().unwrap_or("");
                        if plugins.iter().any(|plugin| plugin.handles_extension(ext_str)) {
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

pub fn extract_import_path(line: &str) -> Option<String> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio::fs;

    #[test]
    fn test_extract_import_path() {
        assert_eq!(
            extract_import_path("import { foo } from './bar';"),
            Some("./bar".to_string())
        );
        assert_eq!(
            extract_import_path("import { foo } from \"./bar\";"),
            Some("./bar".to_string())
        );
        assert_eq!(
            extract_import_path("const bar = require('./bar');"),
            Some("./bar".to_string())
        );
        assert_eq!(
            extract_import_path("const bar = require(\"./bar\");"),
            Some("./bar".to_string())
        );
        assert_eq!(extract_import_path("let x = 1;"), None);
        assert_eq!(
            extract_import_path("this is from a file"),
            None
        );
    }

    // Helper to create a test harness
    async fn setup_test_harness() -> (TempDir, ReferenceUpdater, Vec<PathBuf>) {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        let updater = ReferenceUpdater::new(root);

        // Create some mock files
        fs::create_dir_all(root.join("src/components")).await.unwrap();
        fs::write(root.join("src/main.ts"), "").await.unwrap();
        fs::write(root.join("src/components/button.ts"), "").await.unwrap();
        fs::write(root.join("src/utils.ts"), "").await.unwrap();
        fs::write(root.join("README.md"), "").await.unwrap();

        let project_files = vec![
            root.join("src/main.ts").canonicalize().unwrap(),
            root.join("src/components/button.ts").canonicalize().unwrap(),
            root.join("src/utils.ts").canonicalize().unwrap(),
            root.join("README.md").canonicalize().unwrap(),
        ];

        (temp_dir, updater, project_files)
    }

    #[tokio::test]
    async fn test_resolve_import_to_file_relative() {
        let (_temp_dir, updater, project_files) = setup_test_harness().await;
        let importing_file = project_files[0].clone(); // src/main.ts

        // ./components/button
        let resolved = updater.resolve_import_to_file("./components/button", &importing_file, &project_files);
        assert_eq!(resolved, Some(project_files[1].clone()));

        // ../utils.ts from components/button.ts
        let importing_file = project_files[1].clone();
        let resolved = updater.resolve_import_to_file("../utils.ts", &importing_file, &project_files);
        assert_eq!(resolved, Some(project_files[2].clone()));
    }

    #[tokio::test]
    async fn test_resolve_import_to_file_bare_specifier() {
        let (_temp_dir, updater, project_files) = setup_test_harness().await;
        let importing_file = project_files[0].clone(); // src/main.ts

        let resolved = updater.resolve_import_to_file("README.md", &importing_file, &project_files);
        assert_eq!(resolved, Some(project_files[3].clone()));
    }

    #[tokio::test]
    async fn test_resolve_import_to_file_not_found() {
        let (_temp_dir, updater, project_files) = setup_test_harness().await;
        let importing_file = project_files[0].clone(); // src/main.ts

        let resolved = updater.resolve_import_to_file("./non-existent", &importing_file, &project_files);
        assert_eq!(resolved, None);
    }
}
