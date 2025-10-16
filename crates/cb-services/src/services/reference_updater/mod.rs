//! Service for updating references in a workspace
//！
//！ This service is responsible for finding all references to a given file or symbol
//！ and updating them to a new path or name. It is language-agnostic and delegates
//！ language-specific logic to plugins.

mod cache;
pub mod detectors;

pub use cache::FileImportInfo;

use cb_protocol::{
    ApiError as ServerError, ApiResult as ServerResult, DependencyUpdate, EditLocation, EditPlan,
    EditPlanMetadata, EditType, TextEdit,
};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

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
        tracing::info!(
            old_path = %old_path.display(),
            new_path = %new_path.display(),
            project_root = %self.project_root.display(),
            plugins_count = plugins.len(),
            "update_references called"
        );

        // From edit_builder.rs
        let project_files = find_project_files(&self.project_root, plugins).await?;
        tracing::info!(
            project_files_count = project_files.len(),
            "Found project files"
        );
        let is_directory_rename = old_path.is_dir();

        // Check if this is a Rust crate rename (directory with Cargo.toml)
        let is_rust_crate_rename = is_directory_rename && old_path.join("Cargo.toml").exists();

        let mut affected_files = if is_rust_crate_rename {
            // For Rust crate renames, call the detector ONCE with the directory paths
            // This allows the detector to scan for crate-level imports
            tracing::info!(
                old_path = %old_path.display(),
                new_path = %new_path.display(),
                "Detected Rust crate rename, using crate-level detection"
            );
            self.find_affected_files_for_rename(old_path, new_path, &project_files, plugins)
                .await?
        } else if is_directory_rename {
            // For non-Rust directory renames, use per-file detection
            // Build a mapping of importer -> set of files in directory it imports
            let mut all_affected = HashSet::new();
            let mut importer_to_imported_files: HashMap<PathBuf, HashSet<(PathBuf, PathBuf)>> = HashMap::new();

            let files_in_directory: Vec<&PathBuf> = project_files
                .iter()
                .filter(|f| f.starts_with(old_path) && f.is_file())
                .collect();

            let files_in_dir_count = files_in_directory.len();

            for file_in_dir in &files_in_directory {
                let relative_path = file_in_dir.strip_prefix(old_path).unwrap_or(file_in_dir);
                let new_file_path = new_path.join(relative_path);
                let importers = self
                    .find_affected_files_for_rename(
                        file_in_dir,
                        &new_file_path,
                        &project_files,
                        plugins,
                    )
                    .await?;

                // Track which files in the directory each importer references
                for importer in importers {
                    all_affected.insert(importer.clone());
                    importer_to_imported_files
                        .entry(importer)
                        .or_insert_with(HashSet::new)
                        .insert(((*file_in_dir).clone(), new_file_path.clone()));
                }
            }

            // Store the mapping for use in rewriting phase
            // We'll need to pass this to the rewriting logic
            let affected_vec: Vec<PathBuf> = all_affected.into_iter().collect();

            // Store mapping in a way we can access during rewriting
            // For now, we'll process affected files differently for directory renames
            tracing::info!(
                affected_files_count = affected_vec.len(),
                files_in_directory_count = files_in_dir_count,
                "Directory rename: found affected files"
            );

            affected_vec
        } else {
            self.find_affected_files_for_rename(old_path, new_path, &project_files, plugins)
                .await?
        };

        if is_directory_rename {
            affected_files.retain(|file| !file.starts_with(old_path));
        }

        let mut all_edits = Vec::new();

        tracing::info!(
            affected_files_count = affected_files.len(),
            "Processing affected files for reference updates"
        );

        for file_path in affected_files {
            tracing::debug!(
                file_path = %file_path.display(),
                "Processing affected file"
            );
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

            if is_rust_crate_rename {
                // For Rust crate renames, use simple file rename logic with rename_info
                // The rename_info contains old_crate_name and new_crate_name which the plugin uses
                tracing::debug!(
                    file_path = %file_path.display(),
                    old_path = %old_path.display(),
                    new_path = %new_path.display(),
                    "Rewriting imports for Rust crate rename"
                );

                let rewrite_result = plugin.rewrite_file_references(
                    &content,
                    old_path,  // Pass the directory path (crate root)
                    new_path,  // Pass the new directory path
                    &file_path,
                    &self.project_root,
                    rename_info,  // This contains old_crate_name and new_crate_name
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
                                "Update imports in {} for crate rename",
                                file_path.display()
                            ),
                        });
                    }
                }
            } else if is_directory_rename {
                // Directory rename logic (for non-Rust crate directories)
                // For each file in the directory, call plugin.rewrite_file_references
                // This allows plugins to update imports for all files being moved
                tracing::debug!(
                    file_path = %file_path.display(),
                    old_path = %old_path.display(),
                    new_path = %new_path.display(),
                    "Rewriting references for directory rename"
                );

                // Get all files within the moved directory
                let files_in_directory: Vec<&PathBuf> = project_files
                    .iter()
                    .filter(|f| f.starts_with(old_path) && f.is_file())
                    .collect();

                let mut combined_content = content.clone();
                let mut total_changes = 0;

                // Process each file in the directory that might be referenced
                for file_in_dir in &files_in_directory {
                    let relative_path = file_in_dir.strip_prefix(old_path).unwrap_or(file_in_dir);
                    let new_file_path = new_path.join(relative_path);

                    tracing::debug!(
                        importer = %file_path.display(),
                        old_imported_file = %file_in_dir.display(),
                        new_imported_file = %new_file_path.display(),
                        "Checking if importer references file in moved directory"
                    );

                    // Call plugin to rewrite references for this specific file
                    let rewrite_result = plugin.rewrite_file_references(
                        &combined_content,
                        file_in_dir,  // Old path of specific file in directory
                        &new_file_path,  // New path of specific file in directory
                        &file_path,
                        &self.project_root,
                        rename_info,
                    );

                    if let Some((updated_content, count)) = rewrite_result {
                        if count > 0 && updated_content != combined_content {
                            tracing::debug!(
                                changes = count,
                                importer = %file_path.display(),
                                moved_file = %file_in_dir.display(),
                                "Applied {} import updates for file in moved directory",
                                count
                            );
                            combined_content = updated_content;
                            total_changes += count;
                        }
                    }
                }

                // If any changes were made, add a single edit for this file
                if total_changes > 0 && combined_content != content {
                    let line_count = content.lines().count();
                    let last_line_len = content.lines().last().map(|l| l.len()).unwrap_or(0);

                    tracing::info!(
                        file_path = %file_path.display(),
                        total_changes,
                        "Adding edit for directory rename with {} total changes",
                        total_changes
                    );

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
                        new_text: combined_content,
                        priority: 1,
                        description: format!(
                            "Update {} imports in {} for directory rename",
                            total_changes,
                            file_path.display()
                        ),
                    });
                }
            } else {
                // File rename logic
                tracing::info!(
                    file_path = %file_path.display(),
                    old_path = %old_path.display(),
                    new_path = %new_path.display(),
                    content_length = content.len(),
                    "Calling plugin rewrite_file_references"
                );

                let rewrite_result = plugin.rewrite_file_references(
                    &content,
                    old_path,
                    new_path,
                    &file_path,
                    &self.project_root,
                    rename_info,
                );

                tracing::info!(
                    result = ?rewrite_result,
                    "Plugin rewrite_file_references returned"
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

        tracing::info!(
            all_edits_count = all_edits.len(),
            "Returning EditPlan with edits"
        );

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
                let all_imports =
                    self.get_all_imported_files(&content, file, plugins, project_files);

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
        // Rust-specific cross-crate move detection
        // Rust uses crate-qualified imports (e.g., "use common::utils::foo") which the generic
        // ImportPathResolver cannot resolve. We need special handling for cross-crate moves.

        // Check if this is a Rust file OR a Rust crate directory (contains Cargo.toml)
        let is_rust_file = old_path.extension().and_then(|e| e.to_str()) == Some("rs");
        let is_rust_crate = old_path.is_dir() && old_path.join("Cargo.toml").exists();

        if is_rust_file || is_rust_crate {
            let rust_affected = detectors::find_rust_affected_files(
                old_path,
                new_path,
                &self.project_root,
                project_files,
            )
            .await;

            if !rust_affected.is_empty() {
                // Return early - we've found all affected Rust files
                return Ok(rust_affected);
            }
        }

        // Fallback to generic import-based detection for non-Rust or when no Rust files affected
        let generic_affected = detectors::find_generic_affected_files(
            old_path,
            new_path,
            &self.project_root,
            project_files,
            plugins,
        );

        Ok(generic_affected)
    }

    pub(crate) fn get_all_imported_files(
        &self,
        content: &str,
        current_file: &Path,
        plugins: &[std::sync::Arc<dyn cb_plugin_api::LanguagePlugin>],
        project_files: &[PathBuf],
    ) -> Vec<PathBuf> {
        detectors::get_all_imported_files(
            content,
            current_file,
            plugins,
            project_files,
            &self.project_root,
        )
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

        tokio::fs::write(file_path, updated_content)
            .await
            .map_err(|e| {
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

                let mut read_dir = tokio::fs::read_dir(dir).await.map_err(|e| {
                    ServerError::Internal(format!("Failed to read directory: {}", e))
                })?;
                while let Some(entry) = read_dir
                    .next_entry()
                    .await
                    .map_err(|e| ServerError::Internal(format!("Failed to read entry: {}", e)))?
                {
                    let path = entry.path();
                    if path.is_dir() {
                        collect_files(&path, files, plugins).await?;
                    } else if let Some(ext) = path.extension() {
                        let ext_str = ext.to_str().unwrap_or("");
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio::fs;

    /// Test Rust cross-crate move detection (Issue fix verification)
    #[tokio::test]
    async fn test_rust_cross_crate_move_detection() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        let updater = ReferenceUpdater::new(root);

        // Create Rust workspace structure matching test fixture
        fs::create_dir_all(root.join("common/src")).await.unwrap();
        fs::create_dir_all(root.join("my_crate/src")).await.unwrap();
        fs::create_dir_all(root.join("new_utils/src"))
            .await
            .unwrap();

        // Write fixture files
        fs::write(root.join("common/src/lib.rs"), "pub mod utils;")
            .await
            .unwrap();

        fs::write(root.join("common/src/utils.rs"), "pub fn do_stuff() {}")
            .await
            .unwrap();

        fs::write(
            root.join("my_crate/src/main.rs"),
            "use common::utils::do_stuff;\nfn main() { do_stuff(); }",
        )
        .await
        .unwrap();

        fs::write(
            root.join("common/Cargo.toml"),
            "[package]\nname = \"common\"\nversion = \"0.1.0\"\n",
        )
        .await
        .unwrap();

        fs::write(
            root.join("new_utils/Cargo.toml"),
            "[package]\nname = \"new_utils\"\nversion = \"0.1.0\"\n",
        )
        .await
        .unwrap();

        // Simulate move: common/src/utils.rs → new_utils/src/lib.rs
        let old_path = root.join("common/src/utils.rs");
        let new_path = root.join("new_utils/src/lib.rs");

        // Get all Rust files
        let project_files = vec![
            root.join("common/src/lib.rs"),
            root.join("common/src/utils.rs"),
            root.join("my_crate/src/main.rs"),
        ];

        // Create Rust plugin (needed for file detection)
        let rust_plugin = cb_lang_rust::RustPlugin::new();
        let plugins: Vec<std::sync::Arc<dyn cb_plugin_api::LanguagePlugin>> =
            vec![std::sync::Arc::from(rust_plugin)];

        // Test: find_affected_files_for_rename should detect my_crate/src/main.rs
        let affected = updater
            .find_affected_files_for_rename(&old_path, &new_path, &project_files, &plugins)
            .await
            .unwrap();

        // Verify that my_crate/src/main.rs was detected
        assert!(
            affected.contains(&root.join("my_crate/src/main.rs")),
            "Expected my_crate/src/main.rs to be detected as affected file. Found: {:?}",
            affected
        );

        // Should find exactly 1 affected file
        assert_eq!(
            affected.len(),
            1,
            "Expected 1 affected file, found {}: {:?}",
            affected.len(),
            affected
        );
    }

    /// Test Rust same-crate move detection (New test for same-crate moves)
    #[tokio::test]
    async fn test_rust_same_crate_move_detection() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        let updater = ReferenceUpdater::new(root);

        // Create Rust crate structure with files in the same crate
        fs::create_dir_all(root.join("common/src")).await.unwrap();

        // Write Cargo.toml for the crate
        fs::write(
            root.join("common/Cargo.toml"),
            "[package]\nname = \"common\"\nversion = \"0.1.0\"\n",
        )
        .await
        .unwrap();

        // Create source files
        fs::write(root.join("common/src/lib.rs"), "pub mod utils;\npub mod helpers;\npub mod processor;")
            .await
            .unwrap();

        fs::write(root.join("common/src/utils.rs"), "pub fn calculate(x: i32) -> i32 { x * 2 }")
            .await
            .unwrap();

        fs::write(root.join("common/src/helpers.rs"), "// Helper functions")
            .await
            .unwrap();

        // Processor file that imports from utils
        fs::write(
            root.join("common/src/processor.rs"),
            "use common::utils::calculate;\n\npub fn process(x: i32) -> i32 {\n    calculate(x)\n}",
        )
        .await
        .unwrap();

        // Simulate same-crate move: common/src/utils.rs → common/src/helpers.rs
        let old_path = root.join("common/src/utils.rs");
        let new_path = root.join("common/src/helpers.rs");

        // Get all Rust files
        let project_files = vec![
            root.join("common/src/lib.rs"),
            root.join("common/src/utils.rs"),
            root.join("common/src/helpers.rs"),
            root.join("common/src/processor.rs"),
        ];

        // Create Rust plugin
        let rust_plugin = cb_lang_rust::RustPlugin::new();
        let plugins: Vec<std::sync::Arc<dyn cb_plugin_api::LanguagePlugin>> =
            vec![std::sync::Arc::from(rust_plugin)];

        // Test: find_affected_files_for_rename should detect common/src/processor.rs
        let affected = updater
            .find_affected_files_for_rename(&old_path, &new_path, &project_files, &plugins)
            .await
            .unwrap();

        // Verify that common/src/processor.rs was detected
        assert!(
            affected.contains(&root.join("common/src/processor.rs")),
            "Expected common/src/processor.rs to be detected as affected file for same-crate move. Found: {:?}",
            affected
        );

        // Should find exactly 1 affected file (processor.rs)
        assert_eq!(
            affected.len(),
            1,
            "Expected 1 affected file for same-crate move, found {}: {:?}",
            affected.len(),
            affected
        );
    }
}
