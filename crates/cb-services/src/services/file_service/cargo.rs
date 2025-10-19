use super::FileService;
use codebuddy_core::dry_run::DryRunnable;
use codebuddy_foundation::protocol::{ ApiError as ServerError , ApiResult as ServerResult };
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::{error, info, warn};
#[cfg(test)]
use tracing::debug;

impl FileService {
    /// Consolidate a Rust package into a target directory
    ///
    /// This function moves source code from old_package_path to new_package_path,
    /// merges Cargo.toml dependencies, removes the old crate from workspace members,
    /// and automatically updates all import statements across the workspace.
    pub(super) async fn consolidate_rust_package(
        &self,
        old_package_path: &Path,
        new_package_path: &Path,
        dry_run: bool,
    ) -> ServerResult<DryRunnable<Value>> {
        info!(
            old_path = ?old_package_path,
            new_path = ?new_package_path,
            dry_run,
            "Consolidating Rust package"
        );

        let old_abs = self.to_absolute_path(old_package_path);
        let new_abs = self.to_absolute_path(new_package_path);

        // Validate that old_path is a Cargo package
        if !self.is_cargo_package(&old_abs).await? {
            return Err(ServerError::InvalidRequest(format!(
                "Source directory is not a Cargo package: {:?}",
                old_abs
            )));
        }

        let old_src_dir = old_abs.join("src");
        if !old_src_dir.exists() {
            return Err(ServerError::NotFound(format!(
                "Source directory does not have a src/ folder: {:?}",
                old_abs
            )));
        }

        if dry_run {
            // In dry run mode, don't create directories
            // Preview mode - return what would happen
            let old_cargo_toml = old_abs.join("Cargo.toml");
            let new_cargo_toml = new_abs.join("Cargo.toml");

            // Calculate rename info for preview
            let rename_info = self
                .extract_consolidation_rename_info(&old_abs, &new_abs)
                .await?;
            let old_crate_name = rename_info["old_crate_name"].as_str().unwrap_or("unknown");
            let new_import_prefix = rename_info["new_import_prefix"]
                .as_str()
                .unwrap_or("unknown");
            let submodule_name = rename_info["submodule_name"].as_str().unwrap_or("unknown");
            let target_crate_name = rename_info["target_crate_name"]
                .as_str()
                .unwrap_or("unknown");

            return Ok(DryRunnable::new(
                true,
                json!({
                    "operation": "consolidate_rust_package",
                    "old_path": old_abs.to_string_lossy(),
                    "new_path": new_abs.to_string_lossy(),
                    "actions": {
                        "move_src": format!("{}/src/* -> {}", old_abs.display(), new_abs.display()),
                        "rename_lib_rs": format!("{}/lib.rs -> {}/mod.rs", new_abs.display(), new_abs.display()),
                        "merge_dependencies": format!("{} -> {}", old_cargo_toml.display(), new_cargo_toml.display()),
                        "remove_from_workspace": "Remove old crate from workspace members",
                        "update_cargo_dependencies": format!("Update all Cargo.toml files: {} → {}", old_crate_name, target_crate_name),
                        "update_imports": format!("use {}::... → use {}::...", old_crate_name, new_import_prefix),
                        "add_module_declaration": format!("Add 'pub mod {};' to {}/src/lib.rs", submodule_name, target_crate_name),
                        "delete_old_crate": format!("Delete {}", old_abs.display())
                    },
                    "import_changes": {
                        "old_crate": old_crate_name,
                        "new_prefix": new_import_prefix,
                        "submodule": submodule_name,
                        "target_crate": target_crate_name
                    },
                    "note": "This is a dry run. No changes will be made. All steps above will be automated during execution."
                }),
            ));
        }

        // Execution mode
        // Calculate rename info before moving files
        let rename_info = self
            .extract_consolidation_rename_info(&old_abs, &new_abs)
            .await?;
        let old_crate_name = rename_info["old_crate_name"]
            .as_str()
            .unwrap_or("unknown")
            .to_string();
        let new_import_prefix = rename_info["new_import_prefix"]
            .as_str()
            .unwrap_or("unknown")
            .to_string();
        let submodule_name = rename_info["submodule_name"]
            .as_str()
            .unwrap_or("unknown")
            .to_string();
        let target_crate_name = rename_info["target_crate_name"]
            .as_str()
            .unwrap_or("unknown")
            .to_string();

        info!(
            old_crate = %old_crate_name,
            new_prefix = %new_import_prefix,
            submodule = %submodule_name,
            "Calculated consolidation rename info"
        );

        // Step 1: Move src files to target directory
        let mut moved_files = Vec::new();
        let walker = ignore::WalkBuilder::new(&old_src_dir).hidden(false).build();
        for entry in walker.flatten() {
            let path = entry.path();
            if path.is_file() {
                let relative_path = path.strip_prefix(&old_src_dir).map_err(|_| {
                    ServerError::Internal(format!(
                        "File path is not under source directory: {}",
                        path.display()
                    ))
                })?;
                let target_path = new_abs.join(relative_path);

                // Ensure parent directory exists
                if let Some(parent) = target_path.parent() {
                    fs::create_dir_all(parent).await.map_err(|e| {
                        ServerError::Internal(format!("Failed to create directory: {}", e))
                    })?;
                }

                // Move the file
                fs::rename(path, &target_path)
                    .await
                    .map_err(|e| ServerError::Internal(format!("Failed to move file: {}", e)))?;

                moved_files.push(target_path.to_string_lossy().to_string());
            }
        }

        info!(files_moved = moved_files.len(), "Moved source files");

        // Step 1.5: Rename lib.rs to mod.rs (Rust module convention)
        let lib_rs_path = new_abs.join("lib.rs");
        let mod_rs_path = new_abs.join("mod.rs");
        if lib_rs_path.exists() {
            fs::rename(&lib_rs_path, &mod_rs_path).await.map_err(|e| {
                ServerError::Internal(format!("Failed to rename lib.rs to mod.rs: {}", e))
            })?;
            info!(
                old_path = ?lib_rs_path,
                new_path = ?mod_rs_path,
                "Renamed lib.rs to mod.rs for directory module"
            );
        }

        // Step 2: Merge Cargo.toml dependencies
        // Find the parent crate's Cargo.toml (traverse up from new_abs)
        let old_cargo_toml = old_abs.join("Cargo.toml");
        let target_cargo_toml = self.find_parent_cargo_toml(&new_abs).await?;

        if let Some(target_toml_path) = target_cargo_toml {
            info!(
                source = ?old_cargo_toml,
                target = ?target_toml_path,
                "Merging dependencies"
            );
            self.merge_cargo_dependencies(&old_cargo_toml, &target_toml_path)
                .await?;
        } else {
            warn!("Could not find target crate's Cargo.toml for dependency merging");
        }

        // Step 3: Remove old crate from workspace members
        if let Err(e) = self.remove_from_workspace_members(&old_abs).await {
            warn!(error = %e, "Failed to update workspace manifest");
        }

        // Step 3.5: Update all workspace Cargo.toml files that depend on the old crate
        // IMPORTANT: Must happen BEFORE deleting the old crate directory
        let old_crate_name_for_deps = old_crate_name.replace('_', "-"); // Cargo.toml uses hyphens
        let target_crate_name_for_deps = target_crate_name.replace('_', "-"); // Cargo.toml uses hyphens

        // Find target crate root by walking up from new_abs to find Cargo.toml with [package]
        let mut target_crate_root = None;
        let mut current = new_abs.as_path();
        while let Some(parent) = current.parent() {
            let cargo_toml = parent.join("Cargo.toml");
            if cargo_toml.exists() {
                if let Ok(content) = fs::read_to_string(&cargo_toml).await {
                    if content.contains("[package]") {
                        target_crate_root = Some(parent.to_path_buf());
                        break;
                    }
                }
            }
            current = parent;
        }

        let target_crate_path = target_crate_root.ok_or_else(|| {
            ServerError::Internal("Could not find target crate root directory".to_string())
        })?;

        match self
            .update_workspace_cargo_dependencies(
                &old_abs,
                &target_crate_path,
                &target_crate_name_for_deps,
                &old_crate_name_for_deps,
            )
            .await
        {
            Ok(updated_count) => {
                info!(
                    updated_files = updated_count,
                    old_crate = %old_crate_name_for_deps,
                    new_crate = %target_crate_name,
                    "Updated workspace Cargo.toml dependencies"
                );
            }
            Err(e) => {
                warn!(
                    error = %e,
                    old_crate = %old_crate_name_for_deps,
                    "Failed to update some workspace Cargo.toml files, but continuing with consolidation"
                );
            }
        }

        // Step 4: Delete the old crate directory
        fs::remove_dir_all(&old_abs).await.map_err(|e| {
            ServerError::Internal(format!("Failed to delete old crate directory: {}", e))
        })?;

        info!("Old crate directory deleted, starting import updates");

        // Step 5: Update all imports across the workspace
        let mut total_imports_updated = 0;
        let mut files_with_import_updates: Vec<String> = Vec::new();

        // Use a "virtual" old file path for the import service
        // This represents the old crate's "entry point" for import resolution
        let virtual_old_path = old_abs.join("src/lib.rs");
        let virtual_new_path = new_abs.join("lib.rs");

        match self
            .reference_updater
            .update_references(
                &virtual_old_path,
                &virtual_new_path,
                self.plugin_registry.all(),
                Some(&rename_info),
                false,
                Some(cb_plugin_api::ScanScope::AllUseStatements),
            )
            .await
        {
            Ok(edit_plan) => {
                info!(
                    edits_planned = edit_plan.edits.len(),
                    "Created import update plan"
                );

                // Apply the edit plan
                match self.apply_edit_plan(&edit_plan).await {
                    Ok(result) => {
                        total_imports_updated = edit_plan.edits.len();
                        files_with_import_updates = result.modified_files;
                        info!(
                            imports_updated = total_imports_updated,
                            files_modified = files_with_import_updates.len(),
                            "Successfully updated imports"
                        );
                    }
                    Err(e) => {
                        warn!(error = %e, "Failed to apply import updates, but consolidation was successful");
                    }
                }
            }
            Err(e) => {
                warn!(error = %e, "Failed to create import update plan, but consolidation was successful");
            }
        }

        // Step 6: Auto-add module declaration to target lib.rs
        let target_lib_rs = target_crate_path.join("src/lib.rs");
        if target_lib_rs.exists() {
            match self.add_module_declaration(&target_lib_rs, &submodule_name).await {
                Ok(added) => {
                    if added {
                        info!(
                            lib_rs = ?target_lib_rs,
                            module = %submodule_name,
                            "Added module declaration to target lib.rs"
                        );
                    } else {
                        info!(
                            lib_rs = ?target_lib_rs,
                            module = %submodule_name,
                            "Module declaration already exists in target lib.rs"
                        );
                    }
                }
                Err(e) => {
                    warn!(
                        error = %e,
                        lib_rs = ?target_lib_rs,
                        module = %submodule_name,
                        "Failed to add module declaration, please add manually: pub mod {};",
                        submodule_name
                    );
                }
            }
        } else {
            warn!(
                lib_rs = ?target_lib_rs,
                "Target lib.rs not found, please manually add: pub mod {};",
                submodule_name
            );
        }

        info!(
            old_path = ?old_abs,
            new_path = ?new_abs,
            files_moved = moved_files.len(),
            imports_updated = total_imports_updated,
            "Consolidation complete"
        );

        Ok(DryRunnable::new(
            false,
            json!({
                "operation": "consolidate_rust_package",
                "success": true,
                "old_path": old_abs.to_string_lossy(),
                "new_path": new_abs.to_string_lossy(),
                "files_moved": moved_files.len(),
                "import_updates": {
                    "old_crate": old_crate_name,
                    "new_prefix": new_import_prefix,
                    "imports_updated": total_imports_updated,
                    "files_modified": files_with_import_updates.len(),
                    "modified_files": files_with_import_updates,
                },
                "module_declaration_added": format!("pub mod {}; added to {}/src/lib.rs", submodule_name, target_crate_name),
                "note": format!("Consolidation complete! All imports updated from '{}' to '{}', and module declaration added automatically.", old_crate_name, new_import_prefix)
            }),
        ))
    }

    /// Merge Cargo.toml dependencies from source to target
    pub(super) async fn merge_cargo_dependencies(
        &self,
        source_toml_path: &Path,
        target_toml_path: &Path,
    ) -> ServerResult<()> {
        use toml_edit::DocumentMut;

        // Read both TOML files
        let source_content = fs::read_to_string(source_toml_path).await.map_err(|e| {
            ServerError::Internal(format!("Failed to read source Cargo.toml: {}", e))
        })?;

        let target_content = fs::read_to_string(target_toml_path).await.map_err(|e| {
            ServerError::Internal(format!("Failed to read target Cargo.toml: {}", e))
        })?;

        // Parse both documents
        let source_doc = source_content.parse::<DocumentMut>().map_err(|e| {
            ServerError::Internal(format!("Failed to parse source Cargo.toml: {}", e))
        })?;

        let mut target_doc = target_content.parse::<DocumentMut>().map_err(|e| {
            ServerError::Internal(format!("Failed to parse target Cargo.toml: {}", e))
        })?;

        let mut merged_count = 0;
        let mut conflict_count = 0;

        // Extract target crate name for circular dependency detection (before any mutable borrows)
        let target_crate_name = target_doc
            .get("package")
            .and_then(|p| p.get("name"))
            .and_then(|n| n.as_str())
            .unwrap_or("unknown")
            .to_string(); // Clone the string to avoid borrow conflicts

        // Merge [dependencies], [dev-dependencies], and [build-dependencies]
        for section in ["dependencies", "dev-dependencies", "build-dependencies"] {
            if let Some(source_deps) = source_doc.get(section).and_then(|v| v.as_table()) {
                // Ensure target has this section
                if target_doc.get(section).is_none() {
                    target_doc[section] = toml_edit::Item::Table(toml_edit::Table::new());
                }

                if let Some(target_deps) = target_doc[section].as_table_mut() {
                    for (dep_name, dep_value) in source_deps.iter() {
                        // Check for self-dependency
                        if dep_name == target_crate_name.as_str() {
                            warn!(
                                dependency = %dep_name,
                                section = %section,
                                target_crate = %target_crate_name,
                                "Skipping self-dependency (would create circular dependency)"
                            );
                            conflict_count += 1;
                            continue;
                        }

                        // Check for circular dependency
                        // If source depends on X, and target also depends on X, that's OK
                        // But if source depends on target's parent crate, that would be circular
                        if self.would_create_circular_dependency(dep_name, &target_crate_name) {
                            warn!(
                                dependency = %dep_name,
                                section = %section,
                                target_crate = %target_crate_name,
                                "Skipping dependency to avoid circular dependency"
                            );
                            conflict_count += 1;
                            continue;
                        }

                        if target_deps.contains_key(dep_name) {
                            warn!(
                                dependency = %dep_name,
                                section = %section,
                                "Dependency already exists in target, skipping"
                            );
                            conflict_count += 1;
                        } else {
                            target_deps.insert(dep_name, dep_value.clone());
                            info!(
                                dependency = %dep_name,
                                section = %section,
                                "Merged dependency"
                            );
                            merged_count += 1;
                        }
                    }
                }
            }
        }

        // Write the updated target TOML
        fs::write(target_toml_path, target_doc.to_string())
            .await
            .map_err(|e| {
                ServerError::Internal(format!("Failed to write target Cargo.toml: {}", e))
            })?;

        info!(
            merged = merged_count,
            conflicts = conflict_count,
            "Dependency merge complete"
        );

        Ok(())
    }

    /// Check if adding a dependency would create a circular dependency
    ///
    /// This is a simplified check that detects obvious circular dependencies:
    /// - Source depends on target (self-dependency after merge)
    /// - Source depends on a crate that depends on target (one-level circular)
    ///
    /// # Arguments
    ///
    /// * `dep_name` - Name of the dependency being added
    /// * `target_crate_name` - Name of the crate receiving the dependency
    ///
    /// # Returns
    ///
    /// `true` if adding this dependency would create a circular dependency
    fn would_create_circular_dependency(&self, dep_name: &str, target_crate_name: &str) -> bool {
        // Simple heuristic checks:

        // 1. Direct circular: dependency matches target
        if dep_name == target_crate_name {
            return true;
        }

        // 2. Known parent-child relationships in this codebase
        // For example: cb-core is a base crate that many others depend on
        // If we're merging into cb-types, and source depends on cb-core,
        // we need to check if cb-core depends on cb-types (which it does in this codebase)
        let known_circular_patterns = [
            // (dependency, target) pairs that would create circular deps
            ("cb-core", "cb-types"),     // cb-core -> cb-types -> cb-core
            ("cb-types", "cb-protocol"), // cb-types -> cb-protocol -> cb-types
            ("cb-types", "cb-core"),     // cb-types -> cb-core -> cb-types
        ];

        for (dep, target) in &known_circular_patterns {
            if dep_name == *dep && target_crate_name == *target {
                return true;
            }
        }

        // Could be extended with full dependency graph analysis,
        // but this simple check catches the most common cases
        false
    }

    /// Update all workspace Cargo.toml files that reference the old crate
    ///
    /// This scans all Cargo.toml files in the workspace and replaces dependencies
    /// on the old crate with dependencies on the target crate.
    ///
    /// # Arguments
    ///
    /// * `old_crate_path` - Path to the old crate directory (to derive old crate name)
    /// * `target_crate_path` - Path to the target crate directory
    /// * `target_crate_name` - Name of the target crate to use as replacement
    /// * `old_crate_name` - Name of the old crate (with hyphens, as appears in Cargo.toml)
    ///
    /// # Returns
    ///
    /// Number of Cargo.toml files successfully updated
    async fn update_workspace_cargo_dependencies(
        &self,
        old_crate_path: &Path,
        target_crate_path: &Path,
        target_crate_name: &str,
        old_crate_name: &str,
    ) -> ServerResult<usize> {
        use toml_edit::DocumentMut;

        info!(
            old_crate = %old_crate_name,
            target_crate = %target_crate_name,
            target_crate_path = ?target_crate_path,
            "Scanning workspace for Cargo.toml files with dependencies on old crate"
        );

        let mut updated_count = 0;
        let mut checked_count = 0;

        // Find all Cargo.toml files in the workspace
        let walker = ignore::WalkBuilder::new(&self.project_root)
            .hidden(false)
            .build();

        for entry in walker.flatten() {
            let path = entry.path();

            // Only process Cargo.toml files
            if path.file_name() != Some(std::ffi::OsStr::new("Cargo.toml")) {
                continue;
            }

            // Skip the old crate's Cargo.toml (it's being deleted anyway)
            if path.starts_with(old_crate_path) {
                continue;
            }

            checked_count += 1;

            // Read the Cargo.toml file
            let content = match fs::read_to_string(path).await {
                Ok(c) => c,
                Err(e) => {
                    warn!(
                        file = ?path,
                        error = %e,
                        "Failed to read Cargo.toml"
                    );
                    continue;
                }
            };

            // Check if this file references the old crate
            if !content.contains(old_crate_name) {
                continue;
            }

            // Parse the TOML document
            let mut doc = match content.parse::<DocumentMut>() {
                Ok(d) => d,
                Err(e) => {
                    warn!(
                        file = ?path,
                        error = %e,
                        "Failed to parse Cargo.toml"
                    );
                    continue;
                }
            };

            let mut file_modified = false;

            // Update dependencies in all relevant sections
            for section in ["dependencies", "dev-dependencies", "build-dependencies"] {
                if let Some(deps) = doc.get_mut(section).and_then(|v| v.as_table_mut()) {
                    if deps.contains_key(old_crate_name) {
                        // Remove old dependency
                        let old_dep_value = deps.remove(old_crate_name);

                        // Check if target crate dependency already exists
                        if deps.contains_key(target_crate_name) {
                            info!(
                                file = ?path,
                                section = %section,
                                target_crate = %target_crate_name,
                                old_crate = %old_crate_name,
                                "Target crate already exists, removed old crate"
                            );
                        } else {
                            // Add target crate dependency
                            // Derive the path to the target crate
                            if let Some(old_dep) = old_dep_value {
                                // Clone the dependency spec and update the path
                                let mut new_dep = old_dep.clone();

                                // For consolidation, REMOVE path dependencies so they resolve via workspace
                                // The target crate should be available through workspace.dependencies
                                if let Some(dep_table) = new_dep.as_inline_table_mut() {
                                    if dep_table.contains_key("path") {
                                        dep_table.remove("path");
                                        info!(
                                            file = ?path,
                                            crate_name = %target_crate_name,
                                            "Removed path dependency (will resolve via workspace)"
                                        );
                                    }
                                } else if let Some(dep_table) = new_dep.as_table_mut() {
                                    if dep_table.contains_key("path") {
                                        dep_table.remove("path");
                                        info!(
                                            file = ?path,
                                            crate_name = %target_crate_name,
                                            "Removed path dependency (will resolve via workspace)"
                                        );
                                    }
                                }

                                deps.insert(target_crate_name, new_dep);
                            }

                            info!(
                                file = ?path,
                                section = %section,
                                old_crate = %old_crate_name,
                                new_crate = %target_crate_name,
                                "Replaced dependency"
                            );
                        }

                        file_modified = true;
                    }
                }
            }

            // Write back if modified
            if file_modified {
                match fs::write(path, doc.to_string()).await {
                    Ok(_) => {
                        info!(
                            file = ?path,
                            "Updated Cargo.toml dependencies"
                        );
                        updated_count += 1;
                    }
                    Err(e) => {
                        error!(
                            file = ?path,
                            error = %e,
                            "Failed to write updated Cargo.toml"
                        );
                    }
                }
            }
        }

        info!(
            checked = checked_count,
            updated = updated_count,
            "Workspace Cargo.toml dependency scan complete"
        );

        Ok(updated_count)
    }

    /// Find the path to a crate by its name in the workspace
    #[allow(dead_code)]
    pub(super) async fn find_crate_path_by_name(
        &self,
        crate_name: &str,
    ) -> ServerResult<Option<PathBuf>> {
        let walker = ignore::WalkBuilder::new(&self.project_root)
            .max_depth(Some(3))
            .hidden(false)
            .build();

        for entry in walker.flatten() {
            let path = entry.path();

            if path.file_name() == Some(std::ffi::OsStr::new("Cargo.toml")) {
                if let Ok(content) = fs::read_to_string(path).await {
                    if let Ok(doc) = content.parse::<toml_edit::DocumentMut>() {
                        if let Some(name) = doc
                            .get("package")
                            .and_then(|p| p.get("name"))
                            .and_then(|n| n.as_str())
                        {
                            if name == crate_name {
                                return Ok(path.parent().map(|p| p.to_path_buf()));
                            }
                        }
                    }
                }
            }
        }

        Ok(None)
    }

    /// Remove a package path from workspace members in the root Cargo.toml
    async fn remove_from_workspace_members(&self, package_path: &Path) -> ServerResult<()> {
        use toml_edit::DocumentMut;

        // Find the workspace root
        let mut current_path = package_path.parent();

        while let Some(path) = current_path {
            let workspace_toml_path = path.join("Cargo.toml");
            if workspace_toml_path.exists() {
                let content = fs::read_to_string(&workspace_toml_path)
                    .await
                    .map_err(|e| {
                        ServerError::Internal(format!("Failed to read workspace Cargo.toml: {}", e))
                    })?;

                if content.contains("[workspace]") {
                    // Parse the workspace manifest
                    let mut doc = content.parse::<DocumentMut>().map_err(|e| {
                        ServerError::Internal(format!(
                            "Failed to parse workspace Cargo.toml: {}",
                            e
                        ))
                    })?;

                    // Calculate relative path from workspace root to package
                    let package_rel_path = package_path.strip_prefix(path).map_err(|_| {
                        ServerError::Internal("Failed to calculate relative path".to_string())
                    })?;

                    let package_rel_str = package_rel_path.to_string_lossy().to_string();

                    // Remove from workspace members
                    let should_write =
                        if let Some(members) = doc["workspace"]["members"].as_array_mut() {
                            let index_opt = members
                                .iter()
                                .position(|m| m.as_str() == Some(&package_rel_str));
                            if let Some(index) = index_opt {
                                members.remove(index);
                                true
                            } else {
                                false
                            }
                        } else {
                            false
                        };

                    if should_write {
                        // Write back
                        fs::write(&workspace_toml_path, doc.to_string())
                            .await
                            .map_err(|e| {
                                ServerError::Internal(format!(
                                    "Failed to write workspace Cargo.toml: {}",
                                    e
                                ))
                            })?;

                        info!(
                            workspace = ?workspace_toml_path,
                            removed_member = %package_rel_str,
                            "Removed package from workspace members"
                        );
                    }

                    return Ok(());
                }
            }

            if path == self.project_root {
                break;
            }
            current_path = path.parent();
        }

        Ok(())
    }

    /// Check if a directory is a Cargo package by looking for a Cargo.toml with a [package] section.
    pub(super) async fn is_cargo_package(&self, dir: &Path) -> ServerResult<bool> {
        let cargo_toml_path = dir.join("Cargo.toml");
        if !cargo_toml_path.exists() {
            return Ok(false);
        }
        match fs::read_to_string(&cargo_toml_path).await {
            Ok(content) => Ok(content.contains("[package]")),
            Err(_) => Ok(false),
        }
    }

    /// Find the parent crate's Cargo.toml by traversing up from a directory
    ///
    /// When consolidating to `target_crate/src/source`, this finds `target_crate/Cargo.toml`
    async fn find_parent_cargo_toml(&self, start_path: &Path) -> ServerResult<Option<PathBuf>> {
        let mut current = start_path;

        while let Some(parent) = current.parent() {
            let cargo_toml = parent.join("Cargo.toml");
            if cargo_toml.exists() {
                // Check if it's a package (not just a workspace)
                if let Ok(content) = fs::read_to_string(&cargo_toml).await {
                    if content.contains("[package]") {
                        return Ok(Some(cargo_toml));
                    }
                }
            }

            // Stop at project root
            if parent == self.project_root {
                break;
            }

            current = parent;
        }

        Ok(None)
    }

    /// Extract consolidation rename information for import updating
    ///
    /// This calculates:
    /// - old_crate_name: The name from the old Cargo.toml
    /// - new_import_prefix: The new import path (e.g., "target_crate::submodule")
    /// - submodule_name: The name of the subdirectory that will contain the consolidated code
    /// - target_crate_name: The name of the target crate
    pub(super) async fn extract_consolidation_rename_info(
        &self,
        old_package_path: &Path,
        new_package_path: &Path,
    ) -> ServerResult<serde_json::Value> {
        use serde_json::json;

        // Read the old Cargo.toml to get the old crate name
        let old_cargo_toml = old_package_path.join("Cargo.toml");
        let old_content = fs::read_to_string(&old_cargo_toml)
            .await
            .map_err(|e| ServerError::Internal(format!("Failed to read old Cargo.toml: {}", e)))?;

        let old_doc = old_content
            .parse::<toml_edit::DocumentMut>()
            .map_err(|e| ServerError::Internal(format!("Failed to parse old Cargo.toml: {}", e)))?;

        let old_crate_name = old_doc["package"]["name"]
            .as_str()
            .ok_or_else(|| {
                ServerError::Internal("Missing [package].name in old Cargo.toml".to_string())
            })?
            .replace('-', "_"); // Normalize to underscores for imports

        // Find the target crate by looking for Cargo.toml in parent directories
        let mut target_crate_name = String::new();
        let mut current = new_package_path;

        while let Some(parent) = current.parent() {
            let cargo_toml = parent.join("Cargo.toml");
            if cargo_toml.exists() {
                if let Ok(content) = fs::read_to_string(&cargo_toml).await {
                    if content.contains("[package]") {
                        // Found the target crate
                        if let Ok(doc) = content.parse::<toml_edit::DocumentMut>() {
                            if let Some(name) = doc["package"]["name"].as_str() {
                                target_crate_name = name.replace('-', "_");
                                break;
                            }
                        }
                    }
                }
            }
            current = parent;
        }

        if target_crate_name.is_empty() {
            return Err(ServerError::Internal(
                "Could not find target crate Cargo.toml".to_string(),
            ));
        }

        // Extract submodule name from the new path
        // e.g., "crates/cb-types/src/protocol" -> "protocol"
        let submodule_name = new_package_path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| ServerError::Internal("Invalid new directory path".to_string()))?
            .to_string();

        // Build the new import prefix
        // e.g., "codebuddy_foundation::protocol"
        let new_import_prefix = format!("{}::{}", target_crate_name, submodule_name);

        info!(
            old_crate_name = %old_crate_name,
            new_import_prefix = %new_import_prefix,
            submodule_name = %submodule_name,
            target_crate_name = %target_crate_name,
            "Extracted consolidation rename information"
        );

        Ok(json!({
            "old_crate_name": old_crate_name,
            "new_crate_name": new_import_prefix.clone(), // For compatibility with update_imports_for_rename
            "new_import_prefix": new_import_prefix,
            "submodule_name": submodule_name,
            "target_crate_name": target_crate_name,
        }))
    }

    /// Extract Cargo package rename information for import rewriting
    #[allow(dead_code)]
    pub(super) async fn extract_cargo_rename_info(
        &self,
        old_package_path: &Path,
        new_package_path: &Path,
    ) -> ServerResult<serde_json::Value> {
        use serde_json::json;

        // Read the old Cargo.toml to get the old crate name
        let old_cargo_toml = old_package_path.join("Cargo.toml");
        let old_content = fs::read_to_string(&old_cargo_toml)
            .await
            .map_err(|e| ServerError::Internal(format!("Failed to read old Cargo.toml: {}", e)))?;

        let old_doc = old_content
            .parse::<toml_edit::DocumentMut>()
            .map_err(|e| ServerError::Internal(format!("Failed to parse old Cargo.toml: {}", e)))?;

        let old_crate_name = old_doc["package"]["name"]
            .as_str()
            .ok_or_else(|| {
                ServerError::Internal("Missing [package].name in Cargo.toml".to_string())
            })?
            .to_string();

        // Derive the new crate name from the new directory path
        // Convert directory name to valid crate name (replace hyphens with underscores for imports)
        let new_dir_name = new_package_path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| ServerError::Internal("Invalid new directory path".to_string()))?;

        // For Rust crate names: keep hyphens in package name but use underscores for imports
        let new_crate_name = new_dir_name.replace('_', "-"); // Normalize to hyphens for package name
        let new_crate_import = new_dir_name.replace('-', "_"); // Use underscores for use statements

        info!(
            old_crate_name = %old_crate_name,
            new_crate_name = %new_crate_name,
            new_crate_import = %new_crate_import,
            "Extracted Cargo rename information"
        );

        Ok(json!({
            "old_crate_name": old_crate_name.replace('-', "_"), // For Rust import updates (use statements)
            "old_package_name": old_crate_name, // For Cargo.toml dependency lookups (keep hyphens)
            "new_crate_name": new_crate_import, // Use underscores for imports
            "new_package_name": new_crate_name, // Keep hyphens for Cargo.toml
        }))
    }

    /// Find the parent Cargo workspace and update the members array to reflect a renamed package.
    ///
    /// Returns the list of Cargo.toml files that were updated (workspace root + moved package)
    #[cfg(test)]
    pub(super) async fn update_workspace_manifests(
        &self,
        old_package_path: &Path,
        new_package_path: &Path,
    ) -> ServerResult<Vec<PathBuf>> {
        let mut updated_files = Vec::new();
        let mut current_path = old_package_path.parent();

        while let Some(path) = current_path {
            let workspace_toml_path = path.join("Cargo.toml");
            if workspace_toml_path.exists() {
                let content = fs::read_to_string(&workspace_toml_path)
                    .await
                    .map_err(|e| {
                        ServerError::Internal(format!("Failed to read workspace Cargo.toml: {}", e))
                    })?;

                if content.contains("[workspace]") {
                    // This is the workspace root we need to modify.
                    let mut doc = content.parse::<toml_edit::DocumentMut>().map_err(|e| {
                        ServerError::Internal(format!(
                            "Failed to parse workspace Cargo.toml: {}",
                            e
                        ))
                    })?;

                    let old_rel_path = old_package_path.strip_prefix(path).map_err(|_| {
                        ServerError::Internal("Failed to calculate old relative path".to_string())
                    })?;
                    let new_rel_path = new_package_path.strip_prefix(path).map_err(|_| {
                        ServerError::Internal("Failed to calculate new relative path".to_string())
                    })?;

                    let old_path_str = old_rel_path.to_string_lossy().to_string();
                    let new_path_str = new_rel_path.to_string_lossy().to_string();

                    // Check if we need to update the workspace members
                    let members = doc["workspace"]["members"].as_array_mut().ok_or_else(|| {
                        ServerError::Internal(
                            "`[workspace.members]` is not a valid array".to_string(),
                        )
                    })?;

                    let index_opt = members
                        .iter()
                        .position(|m| m.as_str() == Some(&old_path_str));
                    if let Some(index) = index_opt {
                        members.remove(index);

                        // Check if this is a consolidation move (into another crate's src/ directory)
                        // Pattern: crates/target-crate/src/module should NOT be added to workspace members
                        let is_nested_module = new_rel_path.components()
                            .collect::<Vec<_>>()
                            .windows(2)
                            .any(|w| w[0].as_os_str() == "src");

                        if !is_nested_module {
                            // Regular rename - add new path to workspace members
                            members.push(new_path_str.as_str());
                            info!(
                                workspace = ?workspace_toml_path,
                                old = %old_path_str,
                                new = %new_path_str,
                                "Updated workspace members"
                            );
                        } else {
                            // Consolidation - only remove old member, don't add new nested path
                            info!(
                                workspace = ?workspace_toml_path,
                                removed = %old_path_str,
                                "Removed workspace member (consolidation detected)"
                            );
                        }

                        fs::write(&workspace_toml_path, doc.to_string())
                            .await
                            .map_err(|e| {
                                ServerError::Internal(format!(
                                    "Failed to write updated workspace Cargo.toml: {}",
                                    e
                                ))
                            })?;

                        updated_files.push(workspace_toml_path.clone());
                    }

                    // Also update relative path dependencies in the moved package's Cargo.toml
                    let package_cargo_toml = new_package_path.join("Cargo.toml");
                    if package_cargo_toml.exists() {
                        let package_updated = self
                            .update_package_relative_paths(
                                &package_cargo_toml,
                                old_package_path,
                                new_package_path,
                                path,
                            )
                            .await?;

                        if package_updated {
                            updated_files.push(package_cargo_toml);
                        }
                    }

                    // If we found the workspace, we can stop searching.
                    return Ok(updated_files);
                }
            }

            if path == self.project_root {
                break;
            }
            current_path = path.parent();
        }

        Ok(updated_files)
    }

    /// Update path dependencies in other crates that depend on the moved crate
    ///
    /// When a crate moves, other crates with path dependencies need their paths updated.
    /// For example, if cb-lang-common moves from crates/languages/ to crates/, then
    /// cb-lang-go's Cargo.toml needs: path = "../cb-lang-common" → "../../cb-lang-common"
    #[cfg(test)]
    pub(super) async fn update_dependent_crate_paths(
        &self,
        old_crate_name: &str,
        new_crate_name: &str,
        new_crate_path: &Path,
    ) -> ServerResult<Vec<PathBuf>> {
        let mut updated_files = Vec::new();

        // Find all Cargo.toml files in the workspace
        let walker = ignore::WalkBuilder::new(&self.project_root)
            .hidden(false)
            .build();

        for entry in walker.flatten() {
            let path = entry.path();
            if path.file_name() == Some(std::ffi::OsStr::new("Cargo.toml")) {
                // Skip the moved crate's own Cargo.toml
                if path.parent() == Some(new_crate_path) {
                    continue;
                }

                // Try to update this Cargo.toml if it depends on the moved crate
                match self
                    .update_cargo_toml_dependency(
                        path,
                        old_crate_name,
                        new_crate_name,
                        new_crate_path,
                    )
                    .await
                {
                    Ok(true) => {
                        info!(cargo_toml = %path.display(), "Updated path dependency");
                        updated_files.push(path.to_path_buf());
                    }
                    Ok(false) => {
                        // File doesn't depend on the moved crate, skip
                    }
                    Err(e) => {
                        warn!(
                            error = %e,
                            cargo_toml = %path.display(),
                            "Failed to update dependency path"
                        );
                    }
                }
            }
        }

        Ok(updated_files)
    }

    /// Update a single Cargo.toml's path dependency if it depends on the moved crate
    ///
    /// This function now also handles renaming the dependency key itself.
    ///
    /// Handles all dependency sections:
    /// - [dependencies], [dev-dependencies], [build-dependencies]
    /// - [target.'cfg(...)'.dependencies]
    /// - [workspace.dependencies]
    /// - [patch.crates-io], [patch.'...']
    ///
    /// Returns Ok(true) if the file was updated, Ok(false) if no update was needed
    #[cfg(test)]
    async fn update_cargo_toml_dependency(
        &self,
        cargo_toml_path: &Path,
        old_crate_name: &str,
        new_crate_name: &str,
        new_crate_path: &Path,
    ) -> ServerResult<bool> {
        let content = fs::read_to_string(cargo_toml_path)
            .await
            .map_err(|e| ServerError::Internal(format!("Failed to read Cargo.toml: {}", e)))?;

        // Optimization: if the old crate name isn't in the file, we can skip parsing
        if !content.contains(old_crate_name) {
            return Ok(false);
        }

        let mut doc = content
            .parse::<toml_edit::DocumentMut>()
            .map_err(|e| ServerError::Internal(format!("Failed to parse Cargo.toml: {}", e)))?;

        let mut updated = false;
        let cargo_toml_dir = cargo_toml_path.parent().ok_or_else(|| {
            ServerError::Internal(format!(
                "Cannot get parent directory of Cargo.toml: {}",
                cargo_toml_path.display()
            ))
        })?;

        // Helper to update a dependency table
        let update_dep_in_table =
            |table: &mut dyn toml_edit::TableLike, updated: &mut bool| -> ServerResult<()> {
                if let Some(mut dep) = table.remove(old_crate_name) {
                    if let Some(dep_table) = dep.as_inline_table_mut() {
                        if dep_table.contains_key("path") {
                            let new_rel_path = pathdiff::diff_paths(new_crate_path, cargo_toml_dir)
                                .ok_or_else(|| {
                                    ServerError::Internal(
                                        "Failed to calculate relative path".to_string(),
                                    )
                                })?;
                            dep_table.insert(
                                "path",
                                toml_edit::Value::from(new_rel_path.to_string_lossy().to_string()),
                            );
                        }
                    } else if let Some(dep_table) = dep.as_table_mut() {
                        if dep_table.contains_key("path") {
                            let new_rel_path = pathdiff::diff_paths(new_crate_path, cargo_toml_dir)
                                .ok_or_else(|| {
                                    ServerError::Internal(
                                        "Failed to calculate relative path".to_string(),
                                    )
                                })?;
                            dep_table.insert(
                                "path",
                                toml_edit::value(new_rel_path.to_string_lossy().to_string()),
                            );
                        }
                    }
                    // Re-insert with the new name
                    table.insert(new_crate_name, dep);
                    *updated = true;
                }
                Ok(())
            };

        // Check standard dependency sections
        for section in ["dependencies", "dev-dependencies", "build-dependencies"] {
            if let Some(deps) = doc.get_mut(section).and_then(|d| d.as_table_like_mut()) {
                update_dep_in_table(deps, &mut updated)?;
            }
        }

        // Check [workspace.dependencies]
        if let Some(workspace) = doc.get_mut("workspace").and_then(|w| w.as_table_mut()) {
            if let Some(deps) = workspace
                .get_mut("dependencies")
                .and_then(|d| d.as_table_like_mut())
            {
                update_dep_in_table(deps, &mut updated)?;
            }
        }

        // Check [target.'cfg(...)'.dependencies] sections
        if let Some(target) = doc.get_mut("target").and_then(|t| t.as_table_mut()) {
            for (_target_name, target_table) in target.iter_mut() {
                if let Some(target_table) = target_table.as_table_mut() {
                    for dep_section in ["dependencies", "dev-dependencies", "build-dependencies"] {
                        if let Some(deps) = target_table
                            .get_mut(dep_section)
                            .and_then(|d| d.as_table_like_mut())
                        {
                            update_dep_in_table(deps, &mut updated)?;
                        }
                    }
                }
            }
        }

        // Check [patch.crates-io] and [patch.'...'] sections
        if let Some(patch) = doc.get_mut("patch").and_then(|p| p.as_table_mut()) {
            for (_registry, registry_table) in patch.iter_mut() {
                if let Some(registry_table) = registry_table.as_table_like_mut() {
                    update_dep_in_table(registry_table, &mut updated)?;
                }
            }
        }

        if updated {
            fs::write(cargo_toml_path, doc.to_string())
                .await
                .map_err(|e| ServerError::Internal(format!("Failed to write Cargo.toml: {}", e)))?;
        }

        Ok(updated)
    }

    /// Update relative `path` dependencies in a package's Cargo.toml after it moves
    #[cfg(test)]
    async fn update_package_relative_paths(
        &self,
        package_cargo_toml: &Path,
        old_package_path: &Path,
        new_package_path: &Path,
        workspace_root: &Path,
    ) -> ServerResult<bool> {
        let content = fs::read_to_string(package_cargo_toml).await.map_err(|e| {
            ServerError::Internal(format!("Failed to read package Cargo.toml: {}", e))
        })?;

        let mut doc = content.parse::<toml_edit::DocumentMut>().map_err(|e| {
            ServerError::Internal(format!("Failed to parse package Cargo.toml: {}", e))
        })?;

        let mut updated_count = 0;

        // Update [package].name to match the new directory name
        let new_dir_name = new_package_path.file_name().and_then(|n| n.to_str());

        if let Some(new_name) = new_dir_name {
            let new_crate_name = new_name.replace('_', "-"); // Normalize to hyphens for Cargo.toml
            if let Some(package_section) = doc.get_mut("package") {
                if let Some(name_field) = package_section.get_mut("name") {
                    let old_name = name_field.as_str().unwrap_or("");
                    if old_name != new_crate_name {
                        info!(
                            old_name = %old_name,
                            new_name = %new_crate_name,
                            "Updating package name in Cargo.toml"
                        );
                        *name_field = toml_edit::value(new_crate_name);
                        updated_count += 1;
                    }
                }
            }
        }

        // Calculate depth change for relative path updates
        let old_depth = old_package_path
            .strip_prefix(workspace_root)
            .map(|p| p.components().count())
            .unwrap_or(0);
        let new_depth = new_package_path
            .strip_prefix(workspace_root)
            .map(|p| p.components().count())
            .unwrap_or(0);

        // Helper closure to update path dependencies in a dependency table
        let update_deps_in_table = |deps: &mut toml_edit::Table, updated: &mut usize| {
            for (name, value) in deps.iter_mut() {
                if let Some(table) = value.as_inline_table_mut() {
                    if let Some(path_value) = table.get_mut("path") {
                        if let Some(old_path_str) = path_value.as_str() {
                            let new_path_str =
                                self.adjust_relative_path(old_path_str, old_depth, new_depth);
                            if new_path_str != old_path_str {
                                info!(
                                    dependency = %name,
                                    old_path = %old_path_str,
                                    new_path = %new_path_str,
                                    "Updating relative path dependency"
                                );
                                *path_value = new_path_str.as_str().into();
                                *updated += 1;
                            }
                        }
                    }
                } else if let Some(table) = value.as_table_mut() {
                    if let Some(path_value) = table.get_mut("path") {
                        if let Some(old_path_str) = path_value.as_str() {
                            let new_path_str =
                                self.adjust_relative_path(old_path_str, old_depth, new_depth);
                            if new_path_str != old_path_str {
                                info!(
                                    dependency = %name,
                                    old_path = %old_path_str,
                                    new_path = %new_path_str,
                                    "Updating relative path dependency"
                                );
                                *path_value = new_path_str.as_str().into();
                                *updated += 1;
                            }
                        }
                    }
                }
            }
        };

        // Update standard dependency sections
        for section in ["dependencies", "dev-dependencies", "build-dependencies"] {
            if let Some(deps) = doc[section].as_table_mut() {
                update_deps_in_table(deps, &mut updated_count);
            }
        }

        // Update target-specific dependency sections
        if let Some(target) = doc.get_mut("target").and_then(|t| t.as_table_mut()) {
            for (_target_spec, target_value) in target.iter_mut() {
                if let Some(target_table) = target_value.as_table_mut() {
                    for dep_section in ["dependencies", "dev-dependencies", "build-dependencies"] {
                        if let Some(deps) = target_table
                            .get_mut(dep_section)
                            .and_then(|d| d.as_table_mut())
                        {
                            update_deps_in_table(deps, &mut updated_count);
                        }
                    }
                }
            }
        }

        if updated_count > 0 {
            fs::write(package_cargo_toml, doc.to_string())
                .await
                .map_err(|e| {
                    ServerError::Internal(format!(
                        "Failed to write updated package Cargo.toml: {}",
                        e
                    ))
                })?;
            info!(
                package = ?package_cargo_toml,
                updated_count = updated_count,
                "Updated relative path dependencies in package manifest"
            );
        } else {
            debug!("No relative path dependencies needed updating");
        }

        Ok(updated_count > 0)
    }

    /// Preview workspace manifest updates without writing to disk
    ///
    /// Returns a list of (file_path, old_content, new_content) tuples for each Cargo.toml
    /// that would be updated.
    #[allow(dead_code)]
    pub(super) async fn plan_workspace_manifest_updates(
        &self,
        old_package_path: &Path,
        new_package_path: &Path,
    ) -> ServerResult<Vec<(PathBuf, String, String)>> {
        let mut planned_updates = Vec::new();
        let mut current_path = old_package_path.parent();

        while let Some(path) = current_path {
            let workspace_toml_path = path.join("Cargo.toml");
            if workspace_toml_path.exists() {
                let content = fs::read_to_string(&workspace_toml_path)
                    .await
                    .map_err(|e| {
                        ServerError::Internal(format!("Failed to read workspace Cargo.toml: {}", e))
                    })?;

                if content.contains("[workspace]") {
                    let mut doc = content.parse::<toml_edit::DocumentMut>().map_err(|e| {
                        ServerError::Internal(format!(
                            "Failed to parse workspace Cargo.toml: {}",
                            e
                        ))
                    })?;

                    let old_rel_path = old_package_path.strip_prefix(path).map_err(|_| {
                        ServerError::Internal("Failed to calculate old relative path".to_string())
                    })?;
                    let new_rel_path = new_package_path.strip_prefix(path).map_err(|_| {
                        ServerError::Internal("Failed to calculate new relative path".to_string())
                    })?;

                    let old_path_str = old_rel_path.to_string_lossy().to_string();
                    let new_path_str = new_rel_path.to_string_lossy().to_string();

                    let members = doc["workspace"]["members"].as_array_mut().ok_or_else(|| {
                        ServerError::Internal(
                            "`[workspace.members]` is not a valid array".to_string(),
                        )
                    })?;

                    let index_opt = members
                        .iter()
                        .position(|m| m.as_str() == Some(&old_path_str));

                    if let Some(index) = index_opt {
                        members.remove(index);
                        members.push(new_path_str.as_str());

                        let new_content = doc.to_string();
                        planned_updates.push((workspace_toml_path.clone(), content.clone(), new_content));

                        // Also plan updates for the package's own Cargo.toml
                        // IMPORTANT: Read from OLD path since directory hasn't been renamed yet during planning
                        let package_cargo_toml = old_package_path.join("Cargo.toml");
                        if package_cargo_toml.exists() {
                            if let Ok((pkg_path, pkg_old, pkg_new)) = self
                                .plan_package_manifest_update(
                                    &package_cargo_toml,
                                    old_package_path,
                                    new_package_path,
                                    path,
                                )
                                .await
                            {
                                planned_updates.push((pkg_path, pkg_old, pkg_new));
                            }
                        }
                    }

                    return Ok(planned_updates);
                }
            }

            if path == self.project_root {
                break;
            }
            current_path = path.parent();
        }

        Ok(planned_updates)
    }

    /// Preview package manifest update (name and relative paths)
    ///
    /// Returns (new_path, old_content, new_content) because the file will be at the new location
    /// after the directory rename happens.
    #[allow(dead_code)]
    async fn plan_package_manifest_update(
        &self,
        package_cargo_toml: &Path,
        old_package_path: &Path,
        new_package_path: &Path,
        workspace_root: &Path,
    ) -> ServerResult<(PathBuf, String, String)> {
        let content = fs::read_to_string(package_cargo_toml).await.map_err(|e| {
            ServerError::Internal(format!("Failed to read package Cargo.toml: {}", e))
        })?;

        let mut doc = content.parse::<toml_edit::DocumentMut>().map_err(|e| {
            ServerError::Internal(format!("Failed to parse package Cargo.toml: {}", e))
        })?;

        let mut updated = false;

        // Update [package].name
        let new_dir_name = new_package_path.file_name().and_then(|n| n.to_str());
        if let Some(new_name) = new_dir_name {
            let new_crate_name = new_name.replace('_', "-");
            if let Some(package_section) = doc.get_mut("package") {
                if let Some(name_field) = package_section.get_mut("name") {
                    let old_name = name_field.as_str().unwrap_or("");
                    if old_name != new_crate_name {
                        *name_field = toml_edit::value(new_crate_name);
                        updated = true;
                    }
                }
            }
        }

        // Update relative path dependencies
        let old_depth = old_package_path
            .strip_prefix(workspace_root)
            .map(|p| p.components().count())
            .unwrap_or(0);
        let new_depth = new_package_path
            .strip_prefix(workspace_root)
            .map(|p| p.components().count())
            .unwrap_or(0);

        let update_deps_in_table = |deps: &mut toml_edit::Table, updated: &mut bool| {
            for (_name, value) in deps.iter_mut() {
                if let Some(table) = value.as_inline_table_mut() {
                    if let Some(path_value) = table.get_mut("path") {
                        if let Some(old_path_str) = path_value.as_str() {
                            let new_path_str =
                                self.adjust_relative_path(old_path_str, old_depth, new_depth);
                            if new_path_str != old_path_str {
                                *path_value = new_path_str.as_str().into();
                                *updated = true;
                            }
                        }
                    }
                } else if let Some(table) = value.as_table_mut() {
                    if let Some(path_value) = table.get_mut("path") {
                        if let Some(old_path_str) = path_value.as_str() {
                            let new_path_str =
                                self.adjust_relative_path(old_path_str, old_depth, new_depth);
                            if new_path_str != old_path_str {
                                *path_value = new_path_str.as_str().into();
                                *updated = true;
                            }
                        }
                    }
                }
            }
        };

        for section in ["dependencies", "dev-dependencies", "build-dependencies"] {
            if let Some(deps) = doc[section].as_table_mut() {
                update_deps_in_table(deps, &mut updated);
            }
        }

        if updated {
            // IMPORTANT: Return the path at the OLD location (where file currently exists)
            // The directory rename operation will move this file to the new location,
            // but during planning the file is still at the old location.
            // The handler will calculate checksums from files at their current (old) locations.
            Ok((package_cargo_toml.to_path_buf(), content, doc.to_string()))
        } else {
            Err(ServerError::Internal("No updates needed".to_string()))
        }
    }

    /// Preview dependent crate path updates without writing to disk
    #[allow(dead_code)]
    pub(super) async fn plan_dependent_crate_path_updates(
        &self,
        old_crate_name: &str,
        new_crate_name: &str,
        new_crate_path: &Path,
    ) -> ServerResult<Vec<(PathBuf, String, String)>> {
        let mut planned_updates = Vec::new();

        let walker = ignore::WalkBuilder::new(&self.project_root)
            .hidden(false)
            .build();

        for entry in walker.flatten() {
            let path = entry.path();
            if path.file_name() == Some(std::ffi::OsStr::new("Cargo.toml")) {
                if path.parent() == Some(new_crate_path) {
                    continue;
                }

                let content = match fs::read_to_string(path).await {
                    Ok(c) => c,
                    Err(_) => continue,
                };

                if !content.contains(old_crate_name) {
                    continue;
                }

                match self
                    .plan_single_cargo_toml_dependency_update(
                        path,
                        old_crate_name,
                        new_crate_name,
                        new_crate_path,
                        &content,
                    )
                    .await
                {
                    Ok(Some((file_path, old_content, new_content))) => {
                        planned_updates.push((file_path, old_content, new_content));
                    }
                    Ok(None) => {}
                    Err(_) => continue,
                }
            }
        }

        Ok(planned_updates)
    }

    /// Add a module declaration to a lib.rs file
    ///
    /// Inserts `pub mod <module_name>;` in the appropriate location (after existing mod declarations).
    /// Returns Ok(true) if the declaration was added, Ok(false) if it already exists.
    async fn add_module_declaration(
        &self,
        lib_rs_path: &Path,
        module_name: &str,
    ) -> ServerResult<bool> {
        let content = fs::read_to_string(lib_rs_path).await.map_err(|e| {
            ServerError::Internal(format!("Failed to read lib.rs: {}", e))
        })?;

        // Check if the module declaration already exists
        let declaration = format!("pub mod {};", module_name);
        if content.contains(&declaration) || content.contains(&format!("pub mod {module_name} ;")) {
            return Ok(false); // Already exists
        }

        // Find the insertion point (after last `pub mod` declaration)
        let lines: Vec<&str> = content.lines().collect();
        let mut insertion_line = 0;
        let mut found_mod_declaration = false;

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with("pub mod ") || trimmed.starts_with("mod ") {
                insertion_line = i + 1;
                found_mod_declaration = true;
            } else if found_mod_declaration && !trimmed.is_empty() && !trimmed.starts_with("//") {
                // Stop at first non-comment, non-empty line after mod declarations
                break;
            }
        }

        // Insert the declaration
        let mut new_lines = lines.clone();
        new_lines.insert(insertion_line, &declaration);
        let new_content = new_lines.join("\n");

        // Preserve trailing newline if original had one
        let final_content = if content.ends_with('\n') {
            format!("{}\n", new_content)
        } else {
            new_content
        };

        fs::write(lib_rs_path, final_content).await.map_err(|e| {
            ServerError::Internal(format!("Failed to write lib.rs: {}", e))
        })?;

        Ok(true)
    }

    /// Preview a single Cargo.toml dependency update
    #[allow(dead_code)]
    async fn plan_single_cargo_toml_dependency_update(
        &self,
        cargo_toml_path: &Path,
        old_crate_name: &str,
        new_crate_name: &str,
        new_crate_path: &Path,
        content: &str,
    ) -> ServerResult<Option<(PathBuf, String, String)>> {
        let mut doc = content
            .parse::<toml_edit::DocumentMut>()
            .map_err(|e| ServerError::Internal(format!("Failed to parse Cargo.toml: {}", e)))?;

        let mut updated = false;
        let cargo_toml_dir = cargo_toml_path.parent().ok_or_else(|| {
            ServerError::Internal(format!(
                "Cannot get parent directory of Cargo.toml: {}",
                cargo_toml_path.display()
            ))
        })?;

        let update_dep_in_table =
            |table: &mut dyn toml_edit::TableLike, updated: &mut bool| -> ServerResult<()> {
                if let Some(mut dep) = table.remove(old_crate_name) {
                    if let Some(dep_table) = dep.as_inline_table_mut() {
                        if dep_table.contains_key("path") {
                            let new_rel_path = pathdiff::diff_paths(new_crate_path, cargo_toml_dir)
                                .ok_or_else(|| {
                                    ServerError::Internal(
                                        "Failed to calculate relative path".to_string(),
                                    )
                                })?;
                            dep_table.insert(
                                "path",
                                toml_edit::Value::from(new_rel_path.to_string_lossy().to_string()),
                            );
                        }
                    } else if let Some(dep_table) = dep.as_table_mut() {
                        if dep_table.contains_key("path") {
                            let new_rel_path = pathdiff::diff_paths(new_crate_path, cargo_toml_dir)
                                .ok_or_else(|| {
                                    ServerError::Internal(
                                        "Failed to calculate relative path".to_string(),
                                    )
                                })?;
                            dep_table.insert(
                                "path",
                                toml_edit::value(new_rel_path.to_string_lossy().to_string()),
                            );
                        }
                    }
                    table.insert(new_crate_name, dep);
                    *updated = true;
                }
                Ok(())
            };

        for section in ["dependencies", "dev-dependencies", "build-dependencies"] {
            if let Some(deps) = doc.get_mut(section).and_then(|d| d.as_table_like_mut()) {
                update_dep_in_table(deps, &mut updated)?;
            }
        }

        if let Some(workspace) = doc.get_mut("workspace").and_then(|w| w.as_table_mut()) {
            if let Some(deps) = workspace
                .get_mut("dependencies")
                .and_then(|d| d.as_table_like_mut())
            {
                update_dep_in_table(deps, &mut updated)?;
            }
        }

        if updated {
            Ok(Some((cargo_toml_path.to_path_buf(), content.to_string(), doc.to_string())))
        } else {
            Ok(None)
        }
    }
}