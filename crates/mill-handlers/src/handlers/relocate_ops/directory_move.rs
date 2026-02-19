//! Directory move planning
//!
//! Handles directory move operations with recursive file updates.
//! Uses MoveService for unified planning logic with Cargo package support.

use mill_foundation::errors::MillResult as ServerResult;
use mill_foundation::planning::MovePlan;
use std::path::Path;
use std::time::Instant;
use tracing::{debug, error, info};

use mill_services::services::perf_env::{
    env_truthy, env_u128, DEFAULT_DIRECTORY_MOVE_CONVERT_MS, PERF_ASSERT_STRICT,
    PERF_MAX_DIRECTORY_MOVE_CONVERT_MS,
};
use mill_services::services::perf_metrics::record_metric;

use crate::handlers::tools::extensions::get_concrete_app_state;

use super::converter::editplan_to_moveplan;

fn assert_perf_threshold(metric: &str, observed_ms: u128, threshold_ms: u128) -> ServerResult<()> {
    if observed_ms <= threshold_ms {
        return Ok(());
    }

    tracing::warn!(metric, observed_ms, threshold_ms, "perf threshold exceeded");

    let strict_assert = env_truthy(PERF_ASSERT_STRICT);

    if strict_assert {
        return Err(mill_foundation::errors::MillError::internal(format!(
            "Performance assertion failed for {}: observed {}ms > threshold {}ms",
            metric, observed_ms, threshold_ms
        )));
    }

    Ok(())
}

/// Generate plan for directory move using MoveService
///
/// This function:
/// 1. Creates a MoveService from FileService
/// 2. Calls plan_directory_move to get EditPlan (includes Cargo support!)
/// 3. Converts EditPlan → MovePlan for MCP protocol
pub async fn plan_directory_move(
    old_path: &Path,
    new_path: &Path,
    update_imports: Option<bool>,
    context: &mill_handler_api::ToolHandlerContext,
    operation_id: &str,
) -> ServerResult<MovePlan> {
    let total_start = Instant::now();
    info!(
        operation_id = %operation_id,
        old_path = %old_path.display(),
        new_path = %new_path.display(),
        "Starting directory move planning"
    );

    // Get concrete AppState to access FileService.move_service()
    let concrete_state = get_concrete_app_state(&context.app_state)?;

    // Create MoveService from FileService
    debug!(
        operation_id = %operation_id,
        "Creating MoveService from FileService"
    );
    let move_service = concrete_state.file_service.move_service();

    // Plan the directory move (returns EditPlan with Cargo support)
    debug!(
        operation_id = %operation_id,
        old_path = %old_path.display(),
        new_path = %new_path.display(),
        "Calling MoveService::plan_directory_move (includes Cargo package detection)"
    );

    // Map update_imports to ScanScope
    // If update_imports is explicitly false, use None (conservative)
    // If update_imports is true or None (default), use All to include alias/string refs
    let scan_scope = match update_imports {
        Some(false) => None,
        Some(true) | None => Some(mill_plugin_api::ScanScope::All),
    };

    // Get LSP import finder from context (uses workspace/willRenameFiles for correct import detection)
    // The finder may return empty if the LSP doesn't support willRenameFiles, in which case
    // the plugin-based scanner (TypeScriptReferenceDetector) is used as a fallback.
    let lsp_finder_wrapper = if crate::handlers::common::should_use_lsp_for_refactor(context) {
        let lsp_adapter_guard = context.lsp_adapter.lock().await;
        lsp_adapter_guard
            .as_ref()
            .map(|adapter| crate::handlers::common::LspFinderWrapper(adapter.clone()))
    } else {
        None
    };
    let lsp_finder = lsp_finder_wrapper
        .as_ref()
        .map(|wrapper| wrapper as &dyn mill_services::services::reference_updater::LspImportFinder);

    let planner_start = Instant::now();
    let edit_plan = move_service
        .plan_directory_move(old_path, new_path, scan_scope, lsp_finder)
        .await
        .map_err(|e| {
            error!(
                operation_id = %operation_id,
                error = %e,
                old_path = %old_path.display(),
                new_path = %new_path.display(),
                function = "plan_directory_move",
                "MoveService::plan_directory_move failed"
            );
            e
        })?;

    info!(
        operation_id = %operation_id,
        planner_ms = planner_start.elapsed().as_millis(),
        edits_count = edit_plan.edits.len(),
        "MoveService returned EditPlan, converting to MovePlan"
    );

    // Convert EditPlan → MovePlan
    let convert_start = Instant::now();
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

    let convert_ms = convert_start.elapsed().as_millis();
    info!(
        operation_id = %operation_id,
        convert_ms,
        total_ms = total_start.elapsed().as_millis(),
        affected_files = move_plan.summary.affected_files,
        "perf: directory_move_pipeline"
    );

    record_metric("directory_move_pipeline.convert_ms", convert_ms);

    assert_perf_threshold(
        "directory_move_pipeline.convert_ms",
        convert_ms,
        env_u128(
            PERF_MAX_DIRECTORY_MOVE_CONVERT_MS,
            DEFAULT_DIRECTORY_MOVE_CONVERT_MS,
        ),
    )?;

    Ok(move_plan)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_perf_threshold_with_strict(
        metric: &str,
        observed_ms: u128,
        threshold_ms: u128,
        strict_assert: bool,
    ) -> ServerResult<()> {
        if observed_ms <= threshold_ms {
            return Ok(());
        }
        if strict_assert {
            return Err(mill_foundation::errors::MillError::internal(format!(
                "Performance assertion failed for {}: observed {}ms > threshold {}ms",
                metric, observed_ms, threshold_ms
            )));
        }
        Ok(())
    }

    #[test]
    fn test_assert_perf_threshold_strict_mode_fails() {
        let result =
            assert_perf_threshold_with_strict("directory_move_pipeline.convert_ms", 500, 100, true);
        assert!(result.is_err());
    }
}
