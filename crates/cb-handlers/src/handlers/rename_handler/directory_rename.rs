use crate::handlers::common::calculate_checksums_for_directory_rename;
use crate::handlers::tools::ToolHandlerContext;
use super::{RenamePlanParams, RenameHandler};
use codebuddy_foundation::protocol::{
    refactor_plan::{PlanMetadata, PlanSummary, PlanWarning, RenamePlan},
    ApiResult as ServerResult,
};
use std::path::Path;
use tracing::{debug, info};

impl RenameHandler {
    /// Auto-detect if this is a consolidation move
    ///
    /// Detects when moving a Rust crate into another crate's src/ directory.
    /// Pattern: crates/source-crate → crates/target-crate/src/module
    fn is_consolidation_move(old_path: &Path, new_path: &Path) -> bool {
        // Check if source is a Cargo package
        let has_source_cargo = old_path.join("Cargo.toml").exists();

        // Check if target path is inside another crate's src/ directory
        let mut target_in_src = false;
        let mut parent_has_cargo = false;

        for ancestor in new_path.ancestors() {
            if ancestor.file_name().and_then(|n| n.to_str()) == Some("src") {
                target_in_src = true;
                // Check if this src's parent has Cargo.toml
                if let Some(crate_root) = ancestor.parent() {
                    if crate_root.join("Cargo.toml").exists() {
                        parent_has_cargo = true;
                        break;
                    }
                }
            }
        }

        has_source_cargo && target_in_src && parent_has_cargo
    }

    /// Generate plan for directory rename using FileService
    pub(crate) async fn plan_directory_rename(
        &self,
        params: &RenamePlanParams,
        context: &ToolHandlerContext,
    ) -> ServerResult<RenamePlan> {
        debug!(
            old_path = %params.target.path,
            new_path = %params.new_name,
            "Planning directory rename"
        );

        // Resolve paths against workspace root, not CWD
        let workspace_root = &context.app_state.project_root;
        let old_path = if Path::new(&params.target.path).is_absolute() {
            Path::new(&params.target.path).to_path_buf()
        } else {
            workspace_root.join(&params.target.path)
        };
        let new_path = if Path::new(&params.new_name).is_absolute() {
            Path::new(&params.new_name).to_path_buf()
        } else {
            workspace_root.join(&params.new_name)
        };

        // Determine if this is a consolidation (explicit flag or auto-detect)
        let is_consolidation = params.options.consolidate
            .unwrap_or_else(|| Self::is_consolidation_move(&old_path, &new_path));

        if is_consolidation {
            info!(
                old_path = %old_path.display(),
                new_path = %new_path.display(),
                "Detected consolidation move - will merge Cargo.toml and update imports"
            );
        }

        // Get scope configuration from options
        let rename_scope = params.options.to_rename_scope();

        // Get the EditPlan with import updates (call MoveService directly)
        let edit_plan = context
            .app_state
            .move_service()
            .plan_directory_move_with_scope(&old_path, &new_path, rename_scope.as_ref())
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
        let is_cargo_package = old_path.join("Cargo.toml").exists();

        // For directory rename, we need to calculate checksums for all files being moved
        // Paths are already resolved against workspace root, so canonicalize directly
        let abs_old = std::fs::canonicalize(&old_path).unwrap_or_else(|_| old_path.clone());

        // Calculate abs_new early so we can use it for checksum fallback logic
        // new_path is already resolved against workspace root or is absolute
        let abs_new = if new_path.exists() {
            std::fs::canonicalize(&new_path).unwrap_or_else(|_| new_path.clone())
        } else {
            // For non-existent paths, canonicalize parent and join filename
            let parent = new_path.parent().unwrap_or(workspace_root);
            let parent_abs = std::fs::canonicalize(parent)
                .unwrap_or_else(|_| parent.to_path_buf());
            parent_abs.join(new_path.file_name().unwrap_or(new_path.as_os_str()))
        };

        // Calculate checksums for all affected files using shared utility
        // IMPORTANT: Checksums are stored with paths at the OLD/CURRENT location.
        // Validation happens BEFORE the rename, so files exist at their old location.
        let file_checksums =
            calculate_checksums_for_directory_rename(&abs_old, &edit_plan.edits, context).await?;

        // Use shared converter to create WorkspaceEdit from EditPlan
        let workspace_edit = super::plan_converter::editplan_to_workspace_edit(
            &edit_plan,
            &abs_old,
            &abs_new,
        )?;

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
            let target_crate_root = new_path
                .ancestors()
                .find(|p| {
                    p.file_name()
                        .and_then(|n| n.to_str())
                        .map(|n| n == "src")
                        .unwrap_or(false)
                        && p.parent()
                            .map(|parent| parent.join("Cargo.toml").exists())
                            .unwrap_or(false)
                })
                .and_then(|src_dir| src_dir.parent())
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
