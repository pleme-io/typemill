//! Import path resolution and updating functionality
use std :: collections :: HashMap ;
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

/// Helper function to create TextEdits from ModuleReferences for import path updates
fn create_text_edits_from_references(
    references: &[cb_plugin_api::ModuleReference],
    file_path: &Path,
    old_module_name: &str,
    new_module_name: &str,
) -> Vec<cb_protocol::TextEdit> {
    use cb_protocol::{EditLocation, EditType, TextEdit};

    references
        .iter()
        .map(|refer| TextEdit {
            file_path: Some(file_path.to_string_lossy().to_string()),
            edit_type: EditType::UpdateImport,
            location: EditLocation {
                start_line: (refer.line.saturating_sub(1)) as u32, // Convert to 0-based
                start_column: refer.column as u32,
                end_line: (refer.line.saturating_sub(1)) as u32,
                end_column: (refer.column + refer.length) as u32,
            },
            original_text: refer.text.clone(),
            new_text: refer.text.replace(old_module_name, new_module_name),
            priority: 1,
            description: format!(
                "Update {} reference from '{}' to '{}'",
                match refer.kind {
                    cb_plugin_api::ReferenceKind::Declaration => "import",
                    cb_plugin_api::ReferenceKind::QualifiedPath => "qualified path",
                    cb_plugin_api::ReferenceKind::StringLiteral => "string literal",
                },
                old_module_name,
                new_module_name
            ),
        })
        .collect()
}

/// Find inline fully-qualified crate references in code
///
/// This finds patterns like `old_crate::module::function()` that appear
/// outside of `use` import statements.
///
/// # Arguments
///
/// * `content` - The file content to scan
/// * `file_path` - Path to the file being scanned
/// * `crate_name` - Name of the crate to search for (e.g., "cb_ast")
///
/// # Returns
///
/// Vec of ModuleReference for each inline occurrence found
fn find_inline_crate_references(
    content: &str,
    file_path: &Path,
    crate_name: &str,
) -> Vec<cb_plugin_api::ModuleReference> {
    use cb_plugin_api::{ModuleReference, ReferenceKind};

    let mut references = Vec::new();

    // Pattern to match: `crate_name::` followed by identifiers
    // Regex: \bcrate_name::[\w:]+
    // But we'll use simple string matching for robustness

    for (line_num, line) in content.lines().enumerate() {
        // Skip lines that are import statements (already handled)
        if line.trim_start().starts_with("use ") || line.trim_start().starts_with("pub use ") {
            continue;
        }

        // Skip comment lines
        if line.trim_start().starts_with("//") || line.trim_start().starts_with("/*") {
            continue;
        }

        // Find all occurrences of `crate_name::` in this line
        let search_pattern = format!("{}::", crate_name);
        let mut search_start = 0;

        while let Some(pos) = line[search_start..].find(&search_pattern) {
            let absolute_pos = search_start + pos;

            // Ensure it's a word boundary (not part of a larger identifier)
            let is_word_boundary = if absolute_pos == 0 {
                true
            } else {
                let prev_char = line.chars().nth(absolute_pos - 1).unwrap_or(' ');
                !prev_char.is_alphanumeric() && prev_char != '_'
            };

            if is_word_boundary {
                // Extract the full qualified path (including trailing :: and identifiers)
                let remaining = &line[absolute_pos..];
                let mut path_end = search_pattern.len();

                // Continue while we see `identifier::` or `identifier`
                for ch in remaining[search_pattern.len()..].chars() {
                    if ch.is_alphanumeric() || ch == '_' || ch == ':' {
                        path_end += ch.len_utf8();
                    } else {
                        break;
                    }
                }

                // Trim trailing `::`
                while line[absolute_pos..absolute_pos + path_end].ends_with("::") {
                    path_end -= 2;
                }

                let reference_text = &line[absolute_pos..absolute_pos + path_end];

                debug!(
                    file = ?file_path,
                    line = line_num + 1,
                    column = absolute_pos,
                    text = %reference_text,
                    "Found inline fully-qualified path reference"
                );

                references.push(ModuleReference {
                    line: line_num + 1, // 1-based line numbers
                    column: absolute_pos,
                    length: reference_text.len(),
                    text: reference_text.to_string(),
                    kind: ReferenceKind::QualifiedPath,
                });
            }

            search_start = absolute_pos + search_pattern.len();
        }
    }

    debug!(
        file = ?file_path,
        crate_name = %crate_name,
        references_found = references.len(),
        "Inline crate reference scan complete"
    );

    references
}

/// Update import paths in all affected files after a file/directory rename
///
/// Returns an EditPlan that can be applied via FileService.apply_edit_plan()
pub async fn update_imports_for_rename(
    old_path: &Path,
    new_path: &Path,
    project_root: &Path,
    plugins: &[std::sync::Arc<dyn cb_plugin_api::LanguagePlugin>],
    rename_info: Option<&serde_json::Value>,
    dry_run: bool,
    scan_scope: Option<cb_plugin_api::ScanScope>,
) -> AstResult<cb_protocol::EditPlan> {
    let resolver = ImportPathResolver::new(project_root);

    // Find all files that the plugins handle
    let project_files = find_project_files(project_root, plugins).await?;

    // Find files that import the renamed file
    let mut affected_files = resolver
        .find_affected_files(old_path, &project_files)
        .await?;

    // If scan_scope is provided, use enhanced scanning to find additional references
    if let Some(scope) = scan_scope {
        use std::collections::HashSet;

        // Get module name from old path for searching
        let module_name = old_path.file_stem().and_then(|s| s.to_str()).unwrap_or("");

        // Use HashSet to avoid duplicates
        let mut all_affected: HashSet<PathBuf> = affected_files.iter().cloned().collect();

        debug!(
            scan_scope = ?scope,
            module_name = %module_name,
            "Using enhanced scanning to find additional references"
        );

        // Scan all project files for module references
        for file_path in &project_files {
            // Find the appropriate plugin for this file
            let plugin = if let Some(ext) = file_path.extension() {
                let ext_str = ext.to_str().unwrap_or("");
                plugins.iter().find(|p| p.handles_extension(ext_str))
            } else {
                None
            };

            if let Some(plugin) = plugin {
                // Read file content
                if let Ok(content) = tokio::fs::read_to_string(file_path).await {
                    // Find module references using the enhanced scanner
                    // We need to downcast to concrete plugin types to access find_module_references
                    use cb_lang_rust::RustPlugin;
                    use cb_lang_typescript::TypeScriptPlugin;
                    use cb_lang_go::GoPlugin;

                    let refs_opt = if let Some(rust_plugin) = plugin.as_any().downcast_ref::<RustPlugin>() {
                        rust_plugin.find_module_references(&content, module_name, scope).ok()
                    } else if let Some(ts_plugin) = plugin.as_any().downcast_ref::<TypeScriptPlugin>() {
                        Some(ts_plugin.find_module_references(&content, module_name, scope))
                    } else if let Some(go_plugin) = plugin.as_any().downcast_ref::<GoPlugin>() {
                        go_plugin.find_module_references(&content, module_name, scope).ok()
                    } else {
                        None
                    };

                    if let Some(refs) = refs_opt {
                        if !refs.is_empty() {
                            debug!(
                                file = ?file_path,
                                references = refs.len(),
                                "Found module references via enhanced scanning"
                            );
                            all_affected.insert(file_path.clone());
                        }
                    }
                }
            }
        }

        affected_files = all_affected.into_iter().collect();
    }

    info!(
        dry_run = dry_run,
        affected_files = affected_files.len(),
        old_path = ?old_path,
        scan_scope = ?scan_scope,
        "Found files potentially affected by rename"
    );

    // Get module names for reference replacement
    let old_module_name = old_path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    let new_module_name = new_path.file_stem().and_then(|s| s.to_str()).unwrap_or("");

    // For Rust crate renames, also extract crate names for inline reference updates
    let (old_crate_name, new_crate_name) = if let Some(info) = rename_info {
        let old_crate = info
            .get("old_crate_name")
            .and_then(|v| v.as_str())
            .unwrap_or(old_module_name);
        let new_crate = info
            .get("new_crate_name")
            .and_then(|v| v.as_str())
            .unwrap_or(new_module_name);
        (old_crate.to_string(), new_crate.to_string())
    } else {
        (old_module_name.to_string(), new_module_name.to_string())
    };

    debug!(
        old_module = %old_module_name,
        new_module = %new_module_name,
        old_crate = %old_crate_name,
        new_crate = %new_crate_name,
        "Extracted rename information for import and inline reference updates"
    );

    let mut all_edits = Vec::new();
    let mut edited_file_count = 0;

    // Build TextEdits for each affected file
    for file_path in affected_files {
        // Find the appropriate plugin for this file
        let plugin = if let Some(ext) = file_path.extension() {
            let ext_str = ext.to_str().unwrap_or("");
            plugins.iter().find(|p| p.handles_extension(ext_str))
        } else {
            None
        };

        let plugin = match plugin {
            Some(p) => p,
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
                continue;
            }
        };

        // If scan_scope is provided, use find_module_references for precise edits
        if let Some(scope) = scan_scope {
            // Downcast to concrete plugin types to access find_module_references
            use cb_lang_rust::RustPlugin;
            use cb_lang_typescript::TypeScriptPlugin;
            use cb_lang_go::GoPlugin;

            let refs_opt = if let Some(rust_plugin) = plugin.as_any().downcast_ref::<RustPlugin>() {
                rust_plugin.find_module_references(&content, old_module_name, scope).ok()
            } else if let Some(ts_plugin) = plugin.as_any().downcast_ref::<TypeScriptPlugin>() {
                Some(ts_plugin.find_module_references(&content, old_module_name, scope))
            } else if let Some(go_plugin) = plugin.as_any().downcast_ref::<GoPlugin>() {
                go_plugin.find_module_references(&content, old_module_name, scope).ok()
            } else {
                None
            };

            if let Some(refs) = refs_opt {
                if !refs.is_empty() {
                    let edits = create_text_edits_from_references(
                        &refs,
                        &file_path,
                        old_module_name,
                        new_module_name,
                    );
                    debug!(
                        file = ?file_path,
                        edits = edits.len(),
                        "Created precise TextEdits from module references"
                    );
                    all_edits.extend(edits);
                    edited_file_count += 1;
                }
            }

            // ADDITIONAL SCAN: Find inline fully-qualified paths
            // This catches references like `old_crate::module::function()`
            // that are NOT in import statements
            if old_crate_name != new_crate_name {
                let inline_refs =
                    find_inline_crate_references(&content, &file_path, &old_crate_name);

                if !inline_refs.is_empty() {
                    debug!(
                        file = ?file_path,
                        inline_references = inline_refs.len(),
                        old_crate = %old_crate_name,
                        "Found inline fully-qualified path references"
                    );

                    // Create text edits for inline references
                    let inline_edits = create_text_edits_from_references(
                        &inline_refs,
                        &file_path,
                        &old_crate_name,
                        &new_crate_name,
                    );

                    if !inline_edits.is_empty() {
                        all_edits.extend(inline_edits);
                    }
                }
            }
        } else {
            // Fallback to the old rewrite logic
            // Downcast to concrete plugin types to access rewrite_imports_for_rename
            use cb_lang_rust::RustPlugin;
            use cb_lang_typescript::TypeScriptPlugin;
            use cb_lang_go::GoPlugin;

            let rewrite_result = if let Some(rust_plugin) = plugin.as_any().downcast_ref::<RustPlugin>() {
                rust_plugin.rewrite_imports_for_rename(&content, old_path, new_path, &file_path, project_root, rename_info).ok()
            } else if let Some(ts_plugin) = plugin.as_any().downcast_ref::<TypeScriptPlugin>() {
                ts_plugin.rewrite_imports_for_rename(&content, old_path, new_path, &file_path, project_root, rename_info).ok()
            } else if let Some(go_plugin) = plugin.as_any().downcast_ref::<GoPlugin>() {
                go_plugin.rewrite_imports_for_rename(&content, old_path, new_path, &file_path, project_root, rename_info).ok()
            } else {
                None
            };

            match rewrite_result {
                Some((updated_content, count)) => {
                    if count > 0 && updated_content != content {
                        // Create a single TextEdit for the entire file content replacement
                        use cb_protocol::{EditLocation, EditType, TextEdit};
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
                                "Update imports in {} (legacy rewrite)",
                                file_path.display()
                            ),
                        });
                        edited_file_count += 1;
                        debug!(
                            file = ?file_path,
                            imports_updated = count,
                            "Created full-file TextEdit from legacy rewrite"
                        );
                    }
                }
                None => {
                    warn!(file = ?file_path, "Plugin does not support rewrite_imports_for_rename");
                }
            }
        }
    }

    info!(
        files_affected = edited_file_count,
        edits_created = all_edits.len(),
        dry_run = dry_run,
        scan_scope = ?scan_scope,
        "Built EditPlan for import updates"
    );

    // Build and return the EditPlan
    use cb_protocol::{EditPlan, EditPlanMetadata};

    Ok(EditPlan {
        source_file: old_path.to_string_lossy().to_string(),
        edits: all_edits,
        dependency_updates: Vec::new(),
        validations: Vec::new(),
        metadata: EditPlanMetadata {
            intent_name: "rename_file_or_directory".to_string(),
            intent_arguments: serde_json::json!({
                "old_path": old_path.to_string_lossy(),
                "new_path": new_path.to_string_lossy(),
                "scan_scope": scan_scope.map(|s| format!("{:?}", s)),
                "dry_run": dry_run,
            }),
            created_at: chrono::Utc::now(),
            complexity: if scan_scope.is_some() { 7 } else { 5 },
            impact_areas: vec!["imports".to_string(), "file_references".to_string()],
        },
    })
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
        assert!(
            affected3.contains(&file_a),
            "Should still detect fileA after modification"
        );
    }
}
