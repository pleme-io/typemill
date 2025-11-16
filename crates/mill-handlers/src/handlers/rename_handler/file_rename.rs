use super::{RenameHandler, RenameOptions, RenameTarget};
use crate::handlers::tools::extensions::get_concrete_app_state;
use mill_foundation::planning::{PlanMetadata, PlanSummary, RenamePlan};
use mill_foundation::errors::{MillError as ServerError, MillResult as ServerResult};
use std::collections::HashMap;
use std::path::Path;
use tracing::debug;

impl RenameHandler {
    /// Generate plan for file rename using MoveService
    pub(crate) async fn plan_file_rename(
        &self,
        target: &RenameTarget,
        new_name: &str,
        options: &RenameOptions,
        context: &mill_handler_api::ToolHandlerContext,
    ) -> ServerResult<RenamePlan> {
        debug!(
            old_path = %target.path,
            new_path = %new_name,
            "Planning file rename"
        );

        // Resolve paths relative to project root (not CWD) BEFORE passing to MoveService
        let old_path = Path::new(&target.path);
        let new_path = Path::new(new_name);
        let abs_old = context
            .app_state
            .file_service
            .to_absolute_path_checked(old_path)?;
        let abs_new = context
            .app_state
            .file_service
            .to_absolute_path_checked(new_path)?;

        // Get scope configuration from options
        let rename_scope = options.to_rename_scope();

        // Get concrete AppState to access move_service()
        let concrete_state = get_concrete_app_state(&context.app_state)?;

        // Call MoveService directly to get the EditPlan (using absolute paths)
        let edit_plan = concrete_state
            .move_service()
            .plan_file_move_with_scope(&abs_old, &abs_new, Some(&rename_scope))
            .await?;

        debug!(
            edits_count = edit_plan.edits.len(),
            dependency_updates_count = edit_plan.dependency_updates.len(),
            "Got EditPlan with text edits for reference updates"
        );

        // DEBUG: Log detailed edit information for same-crate moves
        if !edit_plan.edits.is_empty() {
            tracing::info!(
                edits_count = edit_plan.edits.len(),
                first_edit_file = ?edit_plan.edits.first().and_then(|e| e.file_path.as_ref()),
                first_edit_type = ?edit_plan.edits.first().map(|e| &e.edit_type),
                "plan_file_rename: Received edits from MoveService"
            );
        } else {
            tracing::warn!(
                old_path = %old_path.display(),
                new_path = %new_path.display(),
                "plan_file_rename: No edits received from MoveService for file rename!"
            );
        }

        // Read file content for checksum
        let content = context
            .app_state
            .file_service
            .read_file(&abs_old)
            .await
            .map_err(|e| {
                ServerError::internal(format!("Failed to read file for checksum: {}", e))
            })?;

        // Calculate checksums for all affected files
        let mut file_checksums = HashMap::new();
        file_checksums.insert(
            abs_old.to_string_lossy().to_string(),
            super::utils::calculate_checksum(&content),
        );

        // Add checksums for files being updated
        for edit in &edit_plan.edits {
            if let Some(ref file_path) = edit.file_path {
                let path = Path::new(file_path);
                if path.exists() && path != abs_old.as_path() {
                    if let Ok(content) = context.app_state.file_service.read_file(path).await {
                        file_checksums.insert(
                            path.to_string_lossy().to_string(),
                            super::utils::calculate_checksum(&content),
                        );
                    }
                }
            }
        }

        // Use shared converter to create WorkspaceEdit from EditPlan
        let workspace_edit =
            super::plan_converter::editplan_to_workspace_edit(&edit_plan, &abs_old, &abs_new)?;

        // Build summary from actual edit plan
        let affected_files = 1 + file_checksums.len().saturating_sub(1); // Target file + files being updated

        let summary = PlanSummary {
            affected_files,
            created_files: 1,
            deleted_files: 1,
        };

        // No warnings for simple file rename
        let warnings = Vec::new();

        // Determine language from extension via plugin registry
        let extension = old_path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("unknown");
        let language = context
            .app_state
            .language_plugins
            .get_plugin(extension)
            .map(|p| p.metadata().name.to_string())
            .unwrap_or_else(|| "unknown".to_string());

        // Build metadata
        let metadata = PlanMetadata {
            plan_version: "1.0".to_string(),
            kind: "rename".to_string(),
            language,
            estimated_impact: super::utils::estimate_impact(affected_files),
            created_at: chrono::Utc::now().to_rfc3339(),
        };

        Ok(RenamePlan {
            edits: workspace_edit,
            summary,
            warnings,
            metadata,
            file_checksums,
            is_consolidation: false, // File renames are never consolidations
        })
    }
}
