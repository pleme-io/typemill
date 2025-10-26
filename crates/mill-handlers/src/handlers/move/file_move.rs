//! File move planning
//!
//! Handles single-file move operations with automatic import updates.
//! Uses MoveService for unified planning logic.

use mill_foundation::protocol::{refactor_plan::MovePlan, ApiResult as ServerResult};
use std::path::Path;
use tracing::{debug, error, info};

use crate::handlers::tools::ToolHandlerContext;

use super::converter::editplan_to_moveplan;

/// Generate plan for file move using MoveService
///
/// This function:
/// 1. Creates a MoveService from FileService
/// 2. Calls plan_file_move to get EditPlan
/// 3. Converts EditPlan → MovePlan for MCP protocol
pub async fn plan_file_move(
    old_path: &Path,
    new_path: &Path,
    context: &ToolHandlerContext,
    operation_id: &str,
) -> ServerResult<MovePlan> {
    info!(
        operation_id = %operation_id,
        old_path = %old_path.display(),
        new_path = %new_path.display(),
        "Starting file move planning"
    );

    // Create MoveService from FileService
    debug!(
        operation_id = %operation_id,
        "Creating MoveService from FileService"
    );
    let move_service = context.app_state.file_service.move_service();

    // Plan the file move (returns EditPlan)
    debug!(
        operation_id = %operation_id,
        old_path = %old_path.display(),
        new_path = %new_path.display(),
        "Calling MoveService::plan_file_move"
    );
    let edit_plan = move_service
        .plan_file_move(old_path, new_path, None)
        .await
        .map_err(|e| {
            error!(
                operation_id = %operation_id,
                error = %e,
                old_path = %old_path.display(),
                new_path = %new_path.display(),
                function = "plan_file_move",
                "MoveService::plan_file_move failed"
            );
            e
        })?;

    info!(
        operation_id = %operation_id,
        edits_count = edit_plan.edits.len(),
        "MoveService returned EditPlan, converting to MovePlan"
    );

    // Convert EditPlan → MovePlan
    let move_plan = editplan_to_moveplan(edit_plan, old_path, new_path, context, operation_id)
        .await
        .map_err(|e| {
            error!(
                operation_id = %operation_id,
                error = %e,
                old_path = %old_path.display(),
                new_path = %new_path.display(),
                function = "editplan_to_moveplan",
                "Failed to convert EditPlan to MovePlan"
            );
            e
        })?;

    info!(
        operation_id = %operation_id,
        affected_files = move_plan.summary.affected_files,
        "File move plan completed successfully"
    );

    Ok(move_plan)
}
