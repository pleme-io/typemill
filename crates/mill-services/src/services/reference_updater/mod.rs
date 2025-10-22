//! Service for updating references in a workspace
//！
//！ This service is responsible for finding all references to a given file or symbol
//！ and updating them to a new path or name. It is language-agnostic and delegates
//！ language-specific logic to plugins.

mod cache;
pub mod detectors;

pub use cache::FileImportInfo;

use mill_foundation::protocol::{ ApiError as ServerError , ApiResult as ServerResult , DependencyUpdate , EditLocation , EditPlan , EditPlanMetadata , EditType , TextEdit , };
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// A service for updating references in a workspace.
pub struct ReferenceUpdater {
    /// Project root directory
    project_root: PathBuf,
    /// Cache of file import information for performance
    /// Maps file path -> (imports, last_modified_time)
    #[allow(dead_code)]
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
    #[allow(clippy::too_many_arguments)]
    pub async fn update_references(
        &self,
        old_path: &Path,
        new_path: &Path,
        plugins: &[std::sync::Arc<dyn mill_plugin_api::LanguagePlugin>],
        rename_info: Option<&serde_json::Value>,
        _dry_run: bool,
        _scan_scope: Option<mill_plugin_api::ScanScope>,
        rename_scope: Option<&mill_foundation::core::rename_scope::RenameScope>,
    ) -> ServerResult<EditPlan> {
        let is_directory_rename = old_path.is_dir();

        // Serialize rename_scope to JSON and merge with existing rename_info
        // This ensures plugins receive BOTH cargo package info AND scope flags
        // (e.g., update_exact_matches, update_comments, update_markdown_prose)
        // Created early so it's available for BOTH detection and rewriting phases
        let merged_rename_info = merge_rename_info(rename_info, rename_scope);

        // From edit_builder.rs
        let mut project_files =
            find_project_files(&self.project_root, plugins, rename_scope).await?;

        // For consolidation moves (detected via rename_info), exclude Cargo.toml files
        // These are handled semantically by consolidate_rust_package, not via generic path updates
        let is_consolidation = rename_info
            .and_then(|info| info.get("submodule_name"))
            .is_some();

        if is_consolidation {
            let before_count = project_files.len();
            project_files
                .retain(|path| path.file_name() != Some(std::ffi::OsStr::new("Cargo.toml")));
            let after_count = project_files.len();
            tracing::info!(
                filtered_count = before_count - after_count,
                "Filtered Cargo.toml files during consolidation (handled semantically)"
            );
        }

        tracing::info!(
            project_files_count = project_files.len(),
            "Found project files"
        );

        // Check if this is a Rust crate rename (directory with Cargo.toml)
        let is_rust_crate_rename = is_directory_rename && old_path.join("Cargo.toml").exists();

        // Check if scope is comprehensive (e.g., --update-all)
        // In comprehensive mode, use ALL files matching scope instead of reference-based detection
        // This ensures 100% coverage by letting plugins decide what to update
        let is_comprehensive = rename_scope.map_or(false, |s| s.is_comprehensive());

        let mut affected_files = if is_comprehensive {
            // Comprehensive mode: scan ALL files in scope
            // Plugins will handle detection based on update_exact_matches, update_markdown_prose, etc.
            tracing::info!(
                project_files_count = project_files.len(),
                "Using comprehensive scope - scanning all files matching scope filters"
            );
            project_files.clone()
        } else if is_rust_crate_rename {
            // For Rust crate renames, call the detector ONCE with the directory paths
            // This allows the detector to scan for crate-level imports
            tracing::info!(
                old_path = %old_path.display(),
                new_path = %new_path.display(),
                "Detected Rust crate rename, using crate-level detection"
            );
            self.find_affected_files_for_rename(old_path, new_path, &project_files, plugins, merged_rename_info.as_ref())
                .await?
        } else if is_directory_rename {
            // For non-Rust directory renames, use BOTH per-file AND directory-level detection
            // 1. Per-file detection: Find files that import specific files in the directory
            // 2. Directory-level detection: Find files with string literals referencing the directory
            let mut all_affected = HashSet::new();
            let mut importer_to_imported_files: HashMap<PathBuf, HashSet<(PathBuf, PathBuf)>> =
                HashMap::new();

            // FIRST: Directory-level detection for string literals (e.g., "config/settings.toml")
            // This is essential for catching path references that aren't imports
            tracing::info!(
                old_path = %old_path.display(),
                new_path = %new_path.display(),
                "Running directory-level detection for string literals"
            );
            let directory_level_affected = self
                .find_affected_files_for_rename(old_path, new_path, &project_files, plugins, merged_rename_info.as_ref())
                .await?;

            for file in directory_level_affected {
                all_affected.insert(file);
            }

            // SECOND: Per-file detection for import-based references
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
                        merged_rename_info.as_ref(),
                    )
                    .await?;

                // Track which files in the directory each importer references
                for importer in importers {
                    all_affected.insert(importer.clone());
                    importer_to_imported_files
                        .entry(importer)
                        .or_default()
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
                "Directory rename: found affected files (including string literals)"
            );

            affected_vec
        } else {
            self.find_affected_files_for_rename(old_path, new_path, &project_files, plugins, merged_rename_info.as_ref())
                .await?
        };

        // For directory renames, exclude files inside the renamed directory UNLESS it's a Rust crate rename
        // For Rust crate renames, we need to process files inside the crate to update self-referencing imports
        if is_directory_rename {
            if is_rust_crate_rename {
                // For Rust crate renames, INCLUDE files inside the crate for self-reference updates
                // Add all Rust files inside the renamed crate to affected_files
                tracing::info!(
                    "Rust crate rename detected - including files inside crate for self-reference updates"
                );

                let files_in_crate: Vec<PathBuf> = project_files
                    .iter()
                    .filter(|f| f.starts_with(old_path) && f.extension().and_then(|e| e.to_str()) == Some("rs"))
                    .cloned()
                    .collect();

                tracing::info!(
                    files_in_crate_count = files_in_crate.len(),
                    "Found Rust files inside renamed crate"
                );

                for file in files_in_crate {
                    if !affected_files.contains(&file) {
                        affected_files.push(file);
                    }
                }
            } else {
                // For non-Rust directory renames, exclude files inside the directory
                affected_files.retain(|file| !file.starts_with(old_path));
            }
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
                    old_path, // Pass the directory path (crate root)
                    new_path, // Pass the new directory path
                    &file_path,
                    &self.project_root,
                    merged_rename_info.as_ref(), // Contains both cargo info AND scope flags
                );

                if let Some((updated_content, count)) = rewrite_result {
                    if count > 0 && updated_content != content {
                        let line_count = content.lines().count();
                        let last_line_len = content.lines().last().map(|l| l.len()).unwrap_or(0);

                        // For directory renames, files inside the renamed directory need to use the NEW path
                        // For file renames, all affected files are outside the renamed file, so use original paths
                        let edit_file_path = if is_directory_rename && file_path.starts_with(old_path) {
                            // File is inside the renamed directory - compute new path
                            let relative_path = file_path.strip_prefix(old_path).unwrap_or(&file_path);
                            new_path.join(relative_path)
                        } else {
                            // File is outside the renamed item (or it's a file rename) - use original path
                            file_path.clone()
                        };

                        all_edits.push(TextEdit {
                            file_path: Some(edit_file_path.to_string_lossy().to_string()),
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
                                edit_file_path.display()
                            ),
                        });
                    }
                }
            } else if is_directory_rename {
                // Directory rename logic (for non-Rust crate directories)
                // Step 1: Call with directory paths to update mod declarations
                // Step 2: Call with individual file paths to update imports
                tracing::debug!(
                    file_path = %file_path.display(),
                    old_path = %old_path.display(),
                    new_path = %new_path.display(),
                    "Rewriting references for directory rename"
                );

                let mut combined_content = content.clone();
                let mut total_changes = 0;

                // Step 1: Update mod declarations by calling with directory paths
                // This allows language plugins (especially Rust) to detect and update
                // mod declarations like "mod utils;" -> "mod helpers;"
                let mod_decl_result = plugin.rewrite_file_references(
                    &combined_content,
                    old_path, // Directory path
                    new_path, // Directory path
                    &file_path,
                    &self.project_root,
                    merged_rename_info.as_ref(),
                );

                if let Some((updated_content, count)) = mod_decl_result {
                    if count > 0 && updated_content != combined_content {
                        tracing::debug!(
                            changes = count,
                            importer = %file_path.display(),
                            "Applied {} mod declaration updates for directory rename",
                            count
                        );
                        combined_content = updated_content;
                        total_changes += count;
                    }
                }

                // Step 2: Update imports by calling with individual file paths
                // Get all files within the moved directory
                let files_in_directory: Vec<&PathBuf> = project_files
                    .iter()
                    .filter(|f| f.starts_with(old_path) && f.is_file())
                    .collect();

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
                        file_in_dir,    // Old path of specific file in directory
                        &new_file_path, // New path of specific file in directory
                        &file_path,
                        &self.project_root,
                        merged_rename_info.as_ref(),
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

                // Add this log right before the plugin method is called
                tracing::debug!(
                    plugin_name = plugin.metadata().name,
                    current_file = %file_path.display(),
                    "Attempting to call rewrite_file_references on selected plugin"
                );

                let rewrite_result = plugin.rewrite_file_references(
                    &content,
                    old_path,
                    new_path,
                    &file_path,
                    &self.project_root,
                    merged_rename_info.as_ref(),
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
                consolidation: None,
            },
        })
    }

    pub async fn find_affected_files(
        &self,
        renamed_file: &Path,
        project_files: &[PathBuf],
        plugins: &[std::sync::Arc<dyn mill_plugin_api::LanguagePlugin>],
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
    ///
    /// # Arguments
    ///
    /// * `rename_info` - Optional JSON containing scope flags and cargo package info.
    ///   Passed to generic detector so plugins can use flags like update_exact_matches.
    pub async fn find_affected_files_for_rename(
        &self,
        old_path: &Path,
        new_path: &Path,
        project_files: &[PathBuf],
        plugins: &[std::sync::Arc<dyn mill_plugin_api::LanguagePlugin>],
        rename_info: Option<&serde_json::Value>,
    ) -> ServerResult<Vec<PathBuf>> {
        // Rust-specific cross-crate move detection
        // Rust uses crate-qualified imports (e.g., "use common::utils::foo") which the generic
        // ImportPathResolver cannot resolve. We need special handling for cross-crate moves.

        // Check if this is a Rust file OR a Rust crate directory (contains Cargo.toml)
        let is_rust_file = old_path.extension().and_then(|e| e.to_str()) == Some("rs");
        let is_rust_crate = old_path.is_dir() && old_path.join("Cargo.toml").exists();

        let mut all_affected = Vec::new();

        if is_rust_file || is_rust_crate {
            // Call Rust plugin's reference detector via plugin registry
            if let Some(rust_plugin) = plugins.iter().find(|p| p.handles_extension("rs")) {
                if let Some(detector) = rust_plugin.reference_detector() {
                    let rust_affected = detector
                        .find_affected_files(old_path, new_path, &self.project_root, project_files)
                        .await;

                    // Add Rust files to the affected list
                    all_affected.extend(rust_affected);
                }
            }

            // ALSO run generic detection to find non-Rust files (markdown/TOML/YAML)
            // Filter out .rs files since they're already handled by Rust detector
            let non_rust_files: Vec<PathBuf> = project_files
                .iter()
                .filter(|f| f.extension().and_then(|e| e.to_str()) != Some("rs"))
                .cloned()
                .collect();

            let generic_affected = detectors::find_generic_affected_files(
                old_path,
                new_path,
                &self.project_root,
                &non_rust_files,
                plugins,
                rename_info,
            );

            all_affected.extend(generic_affected);
        } else {
            // Not a Rust file/crate - use generic detection for everything
            let generic_affected = detectors::find_generic_affected_files(
                old_path,
                new_path,
                &self.project_root,
                project_files,
                plugins,
                rename_info,
            );

            all_affected.extend(generic_affected);
        }

        // Sort and dedup just in case (shouldn't be needed now but safe)
        all_affected.sort();
        all_affected.dedup();

        Ok(all_affected)
    }

    pub(crate) fn get_all_imported_files(
        &self,
        content: &str,
        current_file: &Path,
        plugins: &[std::sync::Arc<dyn mill_plugin_api::LanguagePlugin>],
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
        plugins: &[std::sync::Arc<dyn mill_plugin_api::LanguagePlugin>],
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

        let import_advanced_support = match plugin.import_advanced_support() {
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
        let updated_content = import_advanced_support
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

/// Merges existing rename_info (cargo package names) with serialized RenameScope (scope flags)
///
/// # Arguments
///
/// * `rename_info` - Optional JSON containing cargo package info (old_crate_name, new_crate_name)
/// * `rename_scope` - Optional RenameScope with flags (update_comments, update_exact_matches, etc.)
///
/// # Returns
///
/// Merged JSON Value containing both cargo info and scope flags, or None if both inputs are None
///
/// # Example
///
/// Input rename_info:
/// ```json
/// {
///   "old_crate_name": "cb_client",
///   "new_crate_name": "mill_client"
/// }
/// ```
///
/// Input rename_scope (serialized):
/// ```json
/// {
///   "update_comments": true,
///   "update_exact_matches": true,
///   "update_markdown_prose": true
/// }
/// ```
///
/// Output:
/// ```json
/// {
///   "old_crate_name": "cb_client",
///   "new_crate_name": "mill_client",
///   "update_comments": true,
///   "update_exact_matches": true,
///   "update_markdown_prose": true
/// }
/// ```
fn merge_rename_info(
    rename_info: Option<&serde_json::Value>,
    rename_scope: Option<&mill_foundation::core::rename_scope::RenameScope>,
) -> Option<serde_json::Value> {
    match (rename_info, rename_scope) {
        (Some(info), Some(scope)) => {
            // Both exist - merge them
            let mut merged = info.clone();
            if let Some(scope_json) = serde_json::to_value(scope).ok() {
                if let (Some(merged_obj), Some(scope_obj)) = (merged.as_object_mut(), scope_json.as_object()) {
                    // Merge scope fields into the existing rename_info
                    for (key, value) in scope_obj {
                        merged_obj.insert(key.clone(), value.clone());
                    }
                }
            }
            Some(merged)
        }
        (Some(info), None) => {
            // Only rename_info exists
            Some(info.clone())
        }
        (None, Some(scope)) => {
            // Only rename_scope exists - serialize it
            serde_json::to_value(scope).ok()
        }
        (None, None) => {
            // Neither exists
            None
        }
    }
}

/// Find all project files that should be scanned for reference updates
///
/// # File Filtering Strategy
///
/// This function supports two modes of file filtering:
///
/// ## 1. RenameScope-based filtering (when `rename_scope` is Some)
///
/// Uses the provided `RenameScope` to determine which files to include based on:
/// - `update_code`: Include code files (.rs, .ts, .tsx, .js, .jsx)
/// - `update_docs`: Include documentation files (.md, .markdown)
/// - `update_configs`: Include configuration files (.toml, .yaml, .yml)
/// - `exclude_patterns`: Glob patterns to exclude specific files/directories
///
/// This mode enables **comprehensive rename coverage** - when updating references after a
/// rename/move operation, all relevant file types (code, docs, configs) can be scanned and
/// updated, ensuring 100% coverage of affected references.
///
/// **Example:** Renaming `old-dir/` → `new-dir/` with `update_docs=true` and `update_configs=true`
/// will scan and update:
/// - Code files: `src/main.rs` (imports, qualified paths)
/// - Documentation: `README.md` (markdown links, path mentions)
/// - Configs: `Cargo.toml`, `.github/workflows/ci.yml` (path references)
///
/// ## 2. Plugin-based filtering (when `rename_scope` is None - backward compatibility)
///
/// Uses language plugins to determine which files to include. A file is included if any
/// registered plugin handles its extension. This mode maintains backward compatibility with
/// code that doesn't use RenameScope.
///
/// # Bug Fix (2025-10-20)
///
/// **Previous behavior:** Only scanned files that language plugins handle, which excluded
/// .md, .toml, .yaml files even when RenameScope specified they should be updated.
///
/// **Root cause:** The function only checked `plugin.handles_extension()`, ignoring the
/// `RenameScope` settings.
///
/// **Fix:** When RenameScope is provided, use `scope.should_include_file()` to determine
/// inclusion, which respects all scope settings including file type flags and exclude patterns.
///
/// **Impact:** Enables comprehensive rename coverage (100% of affected references) when using
/// RenameScope with appropriate flags.
///
/// # Arguments
///
/// * `project_root` - Root directory to scan
/// * `plugins` - Registered language plugins
/// * `rename_scope` - Optional scope controlling which file types to include
///
/// # Returns
///
/// Vector of absolute paths to files that should be scanned
pub async fn find_project_files(
    project_root: &Path,
    plugins: &[std::sync::Arc<dyn mill_plugin_api::LanguagePlugin>],
    rename_scope: Option<&mill_foundation::core::rename_scope::RenameScope>,
) -> ServerResult<Vec<PathBuf>> {
    let mut files = Vec::new();

    fn collect_files<'a>(
        dir: &'a Path,
        files: &'a mut Vec<PathBuf>,
        plugins: &'a [std::sync::Arc<dyn mill_plugin_api::LanguagePlugin>],
        rename_scope: Option<&'a mill_foundation::core::rename_scope::RenameScope>,
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
                        collect_files(&path, files, plugins, rename_scope).await?;
                    } else {
                        // If RenameScope is provided, use it to determine file inclusion
                        // Otherwise, fall back to plugin-based filtering for backward compatibility
                        let should_include = if let Some(scope) = rename_scope {
                            scope.should_include_file(&path)
                        } else if let Some(ext) = path.extension() {
                            let ext_str = ext.to_str().unwrap_or("");
                            plugins
                                .iter()
                                .any(|plugin| plugin.handles_extension(ext_str))
                        } else {
                            false
                        };

                        if should_include {
                            files.push(path);
                        }
                    }
                }
            }
            Ok(())
        })
    }

    collect_files(project_root, &mut files, plugins, rename_scope).await?;
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

        // Get plugins from registry
        let plugin_registry = crate::services::registry_builder::build_language_plugin_registry();
        let plugins = plugin_registry.all();

        // Test: find_affected_files_for_rename should detect my_crate/src/main.rs
        let affected = updater
            .find_affected_files_for_rename(&old_path, &new_path, &project_files, plugins, None)
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
        fs::write(
            root.join("common/src/lib.rs"),
            "pub mod utils;\npub mod helpers;\npub mod processor;",
        )
        .await
        .unwrap();

        fs::write(
            root.join("common/src/utils.rs"),
            "pub fn calculate(x: i32) -> i32 { x * 2 }",
        )
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

        // Get plugins from registry
        let plugin_registry = crate::services::registry_builder::build_language_plugin_registry();
        let plugins = plugin_registry.all();

        // Test: find_affected_files_for_rename should detect common/src/processor.rs
        let affected = updater
            .find_affected_files_for_rename(&old_path, &new_path, &project_files, plugins, None)
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

    /// Test that find_project_files respects RenameScope for documentation files
    #[tokio::test]
    async fn test_find_project_files_with_rename_scope_docs() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create test files
        fs::create_dir_all(root.join("src")).await.unwrap();
        fs::write(root.join("src/main.rs"), "fn main() {}")
            .await
            .unwrap();
        fs::write(root.join("README.md"), "# Project")
            .await
            .unwrap();
        fs::write(root.join("CHANGELOG.md"), "# Changes")
            .await
            .unwrap();

        let plugin_registry = crate::services::registry_builder::build_language_plugin_registry();
        let plugins = plugin_registry.all();

        // Test WITHOUT RenameScope - uses plugin-based filtering (includes all plugin-supported files)
        let files_without_scope = find_project_files(root, plugins, None).await.unwrap();
        // With language plugins for Rust and Markdown, all 3 files are included
        assert_eq!(
            files_without_scope.len(),
            3,
            "Without RenameScope, uses plugin-based filtering. Found: {:?}",
            files_without_scope
        );

        // Test WITH RenameScope - update_docs=false should exclude .md files
        let scope_no_docs = mill_foundation::core::rename_scope::RenameScope {
            update_code: true,
            update_docs: false, // Exclude docs
            update_configs: false,
            update_string_literals: false,
            update_comments: false,
            update_markdown_prose: false,
            update_exact_matches: false,
            update_all: false,
            exclude_patterns: vec![],
        };

        let files_no_docs = find_project_files(root, plugins, Some(&scope_no_docs))
            .await
            .unwrap();
        assert_eq!(
            files_no_docs.len(),
            1,
            "With RenameScope(update_docs=false), should exclude .md files. Found: {:?}",
            files_no_docs
        );
        assert!(files_no_docs
            .iter()
            .any(|p| p.file_name().unwrap() == "main.rs"));
        assert!(!files_no_docs
            .iter()
            .any(|p| p.file_name().unwrap() == "README.md"));
        assert!(!files_no_docs
            .iter()
            .any(|p| p.file_name().unwrap() == "CHANGELOG.md"));
    }

    /// Test that find_project_files respects RenameScope for config files
    #[tokio::test]
    async fn test_find_project_files_with_rename_scope_configs() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create test files
        fs::create_dir_all(root.join("src")).await.unwrap();
        fs::write(root.join("src/main.rs"), "fn main() {}")
            .await
            .unwrap();
        fs::write(root.join("Cargo.toml"), "[package]")
            .await
            .unwrap();
        fs::write(root.join("config.yaml"), "key: value")
            .await
            .unwrap();
        fs::write(root.join("settings.yml"), "setting: true")
            .await
            .unwrap();

        let plugin_registry = crate::services::registry_builder::build_language_plugin_registry();
        let plugins = plugin_registry.all();

        // Test WITHOUT RenameScope - uses plugin-based filtering (includes all plugin-supported files)
        let files_without_scope = find_project_files(root, plugins, None).await.unwrap();
        // With language plugins for Rust, TOML, and YAML, all 4 files are included
        assert_eq!(
            files_without_scope.len(),
            4,
            "Without RenameScope, uses plugin-based filtering. Found: {:?}",
            files_without_scope
        );

        // Test WITH RenameScope - update_configs=false should exclude config files
        let scope_no_configs = mill_foundation::core::rename_scope::RenameScope {
            update_code: true,
            update_docs: false,
            update_configs: false, // Exclude configs
            update_string_literals: false,
            update_comments: false,
            update_markdown_prose: false,
            update_exact_matches: false,
            update_all: false,
            exclude_patterns: vec![],
        };

        let files_no_configs = find_project_files(root, plugins, Some(&scope_no_configs))
            .await
            .unwrap();
        assert_eq!(
            files_no_configs.len(),
            1,
            "With RenameScope(update_configs=false), should exclude .toml, .yaml, .yml files. Found: {:?}",
            files_no_configs
        );
        assert!(files_no_configs
            .iter()
            .any(|p| p.file_name().unwrap() == "main.rs"));
        assert!(!files_no_configs
            .iter()
            .any(|p| p.file_name().unwrap() == "Cargo.toml"));
        assert!(!files_no_configs
            .iter()
            .any(|p| p.file_name().unwrap() == "config.yaml"));
        assert!(!files_no_configs
            .iter()
            .any(|p| p.file_name().unwrap() == "settings.yml"));
    }

    /// Test that find_project_files respects RenameScope exclude patterns
    #[tokio::test]
    async fn test_find_project_files_with_rename_scope_excludes() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create test files
        fs::create_dir_all(root.join("src")).await.unwrap();
        fs::create_dir_all(root.join("tests")).await.unwrap();
        fs::write(root.join("src/main.rs"), "fn main() {}")
            .await
            .unwrap();
        fs::write(root.join("tests/test.rs"), "#[test] fn test() {}")
            .await
            .unwrap();
        fs::write(root.join("README.md"), "# Project")
            .await
            .unwrap();
        fs::write(root.join("CONTRIBUTING.md"), "# Contributing")
            .await
            .unwrap();

        let plugin_registry = crate::services::registry_builder::build_language_plugin_registry();
        let plugins = plugin_registry.all();

        // Test WITH RenameScope and exclude patterns
        let scope = mill_foundation::core::rename_scope::RenameScope {
            update_code: true,
            update_docs: true,
            update_configs: false,
            update_string_literals: false,
            update_comments: false,
            update_markdown_prose: false,
            update_exact_matches: false,
            update_all: false,
            exclude_patterns: vec![
                String::from("**/tests/**"),
                String::from("**/CONTRIBUTING.md"), // Must match full path
            ],
        };

        let files_with_scope = find_project_files(root, plugins, Some(&scope))
            .await
            .unwrap();

        // Should include src/main.rs and README.md
        // Should exclude tests/test.rs and CONTRIBUTING.md
        assert!(
            files_with_scope
                .iter()
                .any(|p| p.file_name().unwrap() == "main.rs"),
            "Should include src/main.rs"
        );
        assert!(
            files_with_scope
                .iter()
                .any(|p| p.file_name().unwrap() == "README.md"),
            "Should include README.md"
        );
        assert!(
            !files_with_scope
                .iter()
                .any(|p| p.file_name().unwrap() == "test.rs"),
            "Should exclude tests/test.rs based on exclude pattern"
        );
        assert!(
            !files_with_scope
                .iter()
                .any(|p| p.file_name().unwrap() == "CONTRIBUTING.md"),
            "Should exclude CONTRIBUTING.md based on exclude pattern"
        );
    }

    /// Test that find_project_files with comprehensive RenameScope
    #[tokio::test]
    async fn test_find_project_files_comprehensive_scope() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create diverse set of files
        fs::create_dir_all(root.join("src")).await.unwrap();
        fs::write(root.join("src/main.rs"), "fn main() {}")
            .await
            .unwrap();
        fs::write(root.join("README.md"), "# Project")
            .await
            .unwrap();
        fs::write(root.join("Cargo.toml"), "[package]")
            .await
            .unwrap();
        fs::write(root.join("config.yaml"), "key: value")
            .await
            .unwrap();

        let plugin_registry = crate::services::registry_builder::build_language_plugin_registry();
        let plugins = plugin_registry.all();

        // Test WITH comprehensive RenameScope - all flags true
        let scope = mill_foundation::core::rename_scope::RenameScope {
            update_code: true,
            update_docs: true,
            update_configs: true,
            update_string_literals: true,
            update_comments: true,
            update_markdown_prose: true,
            update_exact_matches: false,
            update_all: false,
            exclude_patterns: vec![],
        };

        let files_with_scope = find_project_files(root, plugins, Some(&scope))
            .await
            .unwrap();
        assert_eq!(
            files_with_scope.len(),
            4,
            "With comprehensive RenameScope, should find all files. Found: {:?}",
            files_with_scope
        );
        assert!(files_with_scope
            .iter()
            .any(|p| p.file_name().unwrap() == "main.rs"));
        assert!(files_with_scope
            .iter()
            .any(|p| p.file_name().unwrap() == "README.md"));
        assert!(files_with_scope
            .iter()
            .any(|p| p.file_name().unwrap() == "Cargo.toml"));
        assert!(files_with_scope
            .iter()
            .any(|p| p.file_name().unwrap() == "config.yaml"));
    }
}