use super::{RenameHandler, RenameOptions, RenameTarget};
use crate::handlers::common::calculate_checksums_for_directory_rename;
use crate::handlers::tools::extensions::get_concrete_app_state;
use mill_foundation::errors::MillResult as ServerResult;
use mill_foundation::planning::{PlanMetadata, PlanSummary, PlanWarning, RenamePlan};
use std::path::{Path, PathBuf};
use tracing::{debug, info};

impl RenameHandler {
    /// Helper to find the crate root (parent of src/) for a path
    async fn find_target_crate_root(path: &Path) -> Option<PathBuf> {
        for p in path.ancestors() {
            if p.file_name().and_then(|n| n.to_str()) == Some("src") {
                if let Some(parent) = p.parent() {
                    if tokio::fs::try_exists(parent.join("Cargo.toml"))
                        .await
                        .unwrap_or(false)
                    {
                        return Some(parent.to_path_buf());
                    }
                }
            }
        }
        None
    }

    /// Auto-detect if this is a consolidation move
    ///
    /// Detects when moving a Rust crate into another crate's src/ directory.
    /// Pattern: crates/source-crate → crates/target-crate/src/module
    async fn is_consolidation_move(old_path: &Path, new_path: &Path) -> bool {
        // Check if source is a Cargo package
        let has_source_cargo = tokio::fs::try_exists(old_path.join("Cargo.toml"))
            .await
            .unwrap_or(false);

        // Check if target path is inside another crate's src/ directory
        let has_target_crate = Self::find_target_crate_root(new_path).await.is_some();

        has_source_cargo && has_target_crate
    }

    /// Generate plan for directory rename using FileService
    pub(crate) async fn plan_directory_rename(
        &self,
        target: &RenameTarget,
        new_name: &str,
        options: &RenameOptions,
        context: &mill_handler_api::ToolHandlerContext,
    ) -> ServerResult<RenamePlan> {
        debug!(
            old_path = %target.path,
            new_path = %new_name,
            "Planning directory rename"
        );

        // Resolve paths against workspace root, not CWD
        let workspace_root = &context.app_state.project_root;
        let old_path = if Path::new(&target.path).is_absolute() {
            Path::new(&target.path).to_path_buf()
        } else {
            workspace_root.join(&target.path)
        };
        let new_path = if Path::new(new_name).is_absolute() {
            Path::new(new_name).to_path_buf()
        } else {
            workspace_root.join(new_name)
        };

        // Determine if this is a consolidation (explicit flag or auto-detect)
        let is_consolidation = match options.consolidate {
            Some(val) => val,
            None => Self::is_consolidation_move(&old_path, &new_path).await,
        };

        if is_consolidation {
            info!(
                old_path = %old_path.display(),
                new_path = %new_path.display(),
                "Detected consolidation move - will merge Cargo.toml and update imports"
            );

            // Validate that consolidation won't create circular dependencies
            // Find target crate root (the parent of src/ directory)
            let target_crate_root = Self::find_target_crate_root(&new_path)
                .await
                .ok_or_else(|| mill_foundation::errors::MillError::InvalidRequest {
                    message: "Could not find target crate root for consolidation".to_string(),
                    parameter: Some("newName".to_string()),
                })?;

            // Validate circular dependencies using Rust-specific analysis
            debug!(
                source = %old_path.display(),
                target = %target_crate_root.display(),
                "Validating consolidation for circular dependencies"
            );

            #[cfg(feature = "lang-rust")]
            {
                use mill_lang_rust::dependency_analysis::validate_no_circular_dependencies;

                match validate_no_circular_dependencies(
                    &old_path,
                    target_crate_root,
                    workspace_root,
                )
                .await
                {
                    // Only reject if there are ACTUAL problematic modules that would create circular imports.
                    // It's normal for target to depend on source (e.g., app → lib) during consolidation.
                    // The key question is: are there specific modules in source that would create
                    // circular imports after being merged into target? If problematic_modules is empty,
                    // the consolidation is safe.
                    Ok(analysis)
                        if analysis.has_circular_dependency
                            && !analysis.problematic_modules.is_empty() =>
                    {
                        return Err(mill_foundation::errors::MillError::InvalidRequest {
                            message: format!(
                                "Cannot consolidate {} into {}: would create circular dependency.\n\
                                 Dependency chain: {}\n\
                                 Problematic modules: {}",
                                analysis.source_crate,
                                analysis.target_crate,
                                analysis.dependency_chain.join(" → "),
                                analysis.problematic_modules.len()
                            ),
                            parameter: Some("target".to_string()),
                        });
                    }
                    Ok(_) => {
                        info!("Circular dependency validation passed");
                    }
                    Err(e) => {
                        // Log validation error but don't fail the plan
                        // This allows consolidation to proceed if validation itself fails
                        tracing::warn!(
                            error = %e,
                            "Failed to validate circular dependencies, proceeding anyway"
                        );
                    }
                }
            }

            #[cfg(not(feature = "lang-rust"))]
            {
                // Rust language support not compiled in, skip validation
                debug!(
                    "Rust language support not available, skipping circular dependency validation"
                );
            }
        }

        // Get scope configuration from options
        let mut rename_scope = options.to_rename_scope();

        // For consolidation moves, exclude Cargo.toml files from generic path updates
        // The semantic Cargo.toml changes (merging dependencies, updating workspace members)
        // are handled during execution, not in the plan
        if is_consolidation {
            rename_scope
                .exclude_patterns
                .push("**/Cargo.toml".to_string());
        }

        // Get concrete AppState to access move_service()
        let concrete_state = get_concrete_app_state(&context.app_state)?;

        // Get the EditPlan with import updates (call MoveService directly)
        let edit_plan = concrete_state
            .move_service()
            .plan_directory_move_with_scope(&old_path, &new_path, Some(&rename_scope))
            .await?;

        debug!(
            edits_count = edit_plan.edits.len(),
            "Got EditPlan with text edits for import updates"
        );

        // Calculate files_to_move by walking the directory
        let mut files_to_move = 0;
        let walker = ignore::WalkBuilder::new(&old_path).hidden(false).build();
        for entry in walker.flatten() {
            if entry.path().is_file() {
                files_to_move += 1;
            }
        }

        // Check if this is a Cargo package
        let is_cargo_package = tokio::fs::try_exists(old_path.join("Cargo.toml"))
            .await
            .unwrap_or(false);

        // For directory rename, we need to calculate checksums for all files being moved
        // Paths are already resolved against workspace root, so canonicalize directly
        let abs_old = tokio::fs::canonicalize(&old_path)
            .await
            .unwrap_or_else(|_| old_path.clone());

        // Calculate abs_new early so we can use it for checksum fallback logic
        // new_path is already resolved against workspace root or is absolute
        let abs_new = if tokio::fs::try_exists(&new_path).await.unwrap_or(false) {
            tokio::fs::canonicalize(&new_path)
                .await
                .unwrap_or_else(|_| new_path.clone())
        } else {
            // For non-existent paths, canonicalize parent and join filename
            let parent = new_path.parent().unwrap_or(workspace_root);
            let parent_abs = tokio::fs::canonicalize(parent)
                .await
                .unwrap_or_else(|_| parent.to_path_buf());
            parent_abs.join(new_path.file_name().unwrap_or(new_path.as_os_str()))
        };

        // Calculate checksums for all affected files using shared utility
        // IMPORTANT: Checksums are stored with paths at the OLD/CURRENT location.
        // Validation happens BEFORE the rename, so files exist at their old location.
        let file_checksums =
            calculate_checksums_for_directory_rename(&abs_old, &edit_plan.edits, context).await?;

        // Use shared converter to create WorkspaceEdit from EditPlan
        let workspace_edit =
            super::plan_converter::editplan_to_workspace_edit(&edit_plan, &abs_old, &abs_new)?;

        // Build summary
        let summary = PlanSummary {
            affected_files: files_to_move,
            created_files: files_to_move,
            deleted_files: files_to_move,
        };

        // Add warning if this is a Cargo package
        let mut warnings = Vec::new();
        if is_cargo_package {
            warnings.push(PlanWarning {
                code: "CARGO_PACKAGE_RENAME".to_string(),
                message: "Renaming a Cargo package will update workspace members and dependencies"
                    .to_string(),
                candidates: None,
            });
        }

        // Add consolidation-specific warning
        if is_consolidation {
            let target_crate_root = Self::find_target_crate_root(&new_path)
                .await
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "target crate".to_string());

            let module_name = new_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("module");

            warnings.push(PlanWarning {
                code: "CONSOLIDATION_MANUAL_STEP".to_string(),
                message: format!(
                    "After consolidation, manually add 'pub mod {};' to {}/src/lib.rs to expose the consolidated code",
                    module_name, target_crate_root
                ),
                candidates: None,
            });
        }

        // Build metadata
        let metadata = PlanMetadata {
            plan_version: "1.0".to_string(),
            kind: "rename".to_string(),
            language: "rust".to_string(), // Assume Rust for directory renames with Cargo
            estimated_impact: super::utils::estimate_impact(files_to_move),
            created_at: chrono::Utc::now().to_rfc3339(),
        };

        debug!(
            checksum_count = file_checksums.len(),
            "Generated file checksums for rename plan"
        );

        Ok(RenamePlan {
            edits: workspace_edit,
            summary,
            warnings,
            metadata,
            file_checksums,
            is_consolidation,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_is_consolidation_move() {
        // Setup directory structure
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create source crate
        let src_crate = root.join("source_crate");
        fs::create_dir(&src_crate).unwrap();
        fs::write(src_crate.join("Cargo.toml"), "[package]").unwrap();

        // Create target crate
        let target_crate = root.join("target_crate");
        fs::create_dir(&target_crate).unwrap();
        fs::write(target_crate.join("Cargo.toml"), "[package]").unwrap();
        let target_src = target_crate.join("src");
        fs::create_dir(&target_src).unwrap();

        // Case 1: True consolidation
        let old_path = src_crate.clone();
        let new_path = target_src.join("module_name");
        assert!(RenameHandler::is_consolidation_move(&old_path, &new_path).await);

        // Case 2: Not consolidation (no cargo.toml in source)
        let other_dir = root.join("other_dir");
        fs::create_dir(&other_dir).unwrap();
        assert!(!RenameHandler::is_consolidation_move(&other_dir, &new_path).await);
    }
}
