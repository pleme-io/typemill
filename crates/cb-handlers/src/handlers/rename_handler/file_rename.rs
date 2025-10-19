use crate::handlers::tools::ToolHandlerContext;
use super::{RenamePlanParams, RenameHandler};
use codebuddy_foundation::protocol::{
    refactor_plan::{PlanMetadata, PlanSummary, RenamePlan},
    ApiError as ServerError, ApiResult as ServerResult,
};
use std::collections::HashMap;
use std::path::Path;
use tracing::debug;

impl RenameHandler {
    /// Generate plan for file rename using FileService
    pub(crate) async fn plan_file_rename(
        &self,
        params: &RenamePlanParams,
        context: &ToolHandlerContext,
    ) -> ServerResult<RenamePlan> {
        debug!(
            old_path = %params.target.path,
            new_path = %params.new_name,
            "Planning file rename"
        );

        let old_path = Path::new(&params.target.path);
        let new_path = Path::new(&params.new_name);

        // Get scope configuration from options
        let rename_scope = params.options.to_rename_scope();

        // Call MoveService directly to get the EditPlan
        let edit_plan = context
            .app_state
            .move_service()
            .plan_file_move_with_scope(old_path, new_path, rename_scope.as_ref())
            .await?;

        let abs_old = std::fs::canonicalize(old_path).unwrap_or_else(|_| old_path.to_path_buf());
        let abs_new = std::fs::canonicalize(new_path.parent().unwrap_or(Path::new(".")))
            .unwrap_or_else(|_| new_path.parent().unwrap_or(Path::new(".")).to_path_buf())
            .join(new_path.file_name().unwrap_or(new_path.as_os_str()));

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
                "plan_file_rename: Received edits from FileService"
            );
        } else {
            tracing::warn!(
                old_path = %old_path.display(),
                new_path = %new_path.display(),
                "plan_file_rename: No edits received from FileService for file rename!"
            );
        }

        // Read file content for checksum
        let content = context
            .app_state
            .file_service
            .read_file(&abs_old)
            .await
            .map_err(|e| {
                ServerError::Internal(format!("Failed to read file for checksum: {}", e))
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
        let workspace_edit = super::plan_converter::editplan_to_workspace_edit(
            &edit_plan,
            &abs_old,
            &abs_new,
        )?;

        // Build summary from actual edit plan
        let affected_files = 1 + file_checksums.len().saturating_sub(1); // Target file + files being updated

        let summary = PlanSummary {
            affected_files,
            created_files: 1,
            deleted_files: 1,
        };

        // No warnings for simple file rename
        let warnings = Vec::new();

        // Determine language from extension
        let extension = old_path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("unknown");
        let language = super::utils::extension_to_language(extension);

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
