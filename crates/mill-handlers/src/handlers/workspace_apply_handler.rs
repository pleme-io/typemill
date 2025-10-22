//! Unified workspace apply handler for all refactoring plans
//!
//! This is the ONLY command that writes files from refactoring plans.
//! It supports ALL 7 plan types with checksum validation, atomic apply,
//! rollback support, and post-apply validation.
//!
//! This handler has been refactored to use focused service classes:
//! - ChecksumValidator: Validates file checksums
//! - PlanConverter: Converts LSP WorkspaceEdit to EditPlan
//! - DryRunGenerator: Creates preview results
//! - PostApplyValidator: Runs post-apply validation commands

use crate::handlers::tools::{ToolHandler, ToolHandlerContext};
use async_trait::async_trait;
use cb_services::{
    services::file_service::EditPlanResult, ChecksumValidator, DryRunGenerator, PlanConverter,
    PostApplyValidator, ValidationConfig, ValidationResult,
};
use codebuddy_foundation::core::model::mcp::ToolCall;
use codebuddy_foundation::protocol::{
    ApiError, ApiResult as ServerResult, RefactorPlan, RefactorPlanExt,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::{debug, error, info, warn};

pub struct WorkspaceApplyHandler;

impl WorkspaceApplyHandler {
    pub fn new() -> Self {
        Self
    }

    /// Get or create shared service instances (lazy singletons)
    fn get_services(context: &ToolHandlerContext) -> WorkspaceApplyServices {
        // Services are created once per handler instance
        WorkspaceApplyServices {
            checksum_validator: std::sync::Arc::new(ChecksumValidator::new(
                context.app_state.file_service.clone(),
            )),
            plan_converter: std::sync::Arc::new(PlanConverter::new()),
            dry_run_generator: std::sync::Arc::new(DryRunGenerator::new()),
            post_apply_validator: std::sync::Arc::new(PostApplyValidator::new()),
        }
    }
}

impl Default for WorkspaceApplyHandler {
    fn default() -> Self {
        Self::new()
    }
}

/// Bundle of services used by WorkspaceApplyHandler
struct WorkspaceApplyServices {
    checksum_validator: std::sync::Arc<ChecksumValidator>,
    plan_converter: std::sync::Arc<PlanConverter>,
    dry_run_generator: std::sync::Arc<DryRunGenerator>,
    post_apply_validator: std::sync::Arc<PostApplyValidator>,
}

/// Parameters for workspace.apply_edit command
#[derive(Debug, Deserialize)]
struct ApplyEditParams {
    plan: RefactorPlan,
    #[serde(default)]
    options: ApplyOptions,
}

/// Options for applying a refactoring plan
#[derive(Debug, Deserialize)]
#[serde(default)]
struct ApplyOptions {
    /// Preview mode - don't actually apply changes
    dry_run: bool,
    /// Validate file checksums before applying (prevents stale plans)
    validate_checksums: bool,
    /// Automatically rollback all changes if any error occurs
    rollback_on_error: bool,
    /// Post-apply validation configuration
    validation: Option<ValidationConfig>,
}

impl Default for ApplyOptions {
    fn default() -> Self {
        Self {
            dry_run: false,
            validate_checksums: true,
            rollback_on_error: true,
            validation: None,
        }
    }
}

/// Result of applying a refactoring plan
#[derive(Debug, Serialize)]
struct ApplyResult {
    /// Whether the apply operation succeeded
    success: bool,
    /// Files that were modified
    applied_files: Vec<String>,
    /// Files that were created
    created_files: Vec<String>,
    /// Files that were deleted
    deleted_files: Vec<String>,
    /// Warnings encountered during apply
    warnings: Vec<String>,
    /// Validation result (if validation was performed)
    validation: Option<ValidationResult>,
    /// Whether rollback is still available (false if validation consumed backup)
    rollback_available: bool,
}

#[async_trait]
impl ToolHandler for WorkspaceApplyHandler {
    fn tool_names(&self) -> &[&str] {
        &["workspace.apply_edit"]
    }

    fn is_internal(&self) -> bool {
        false // Public tool - AI agents use this to execute plans
    }

    async fn handle_tool_call(
        &self,
        context: &ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        // Create service instances once at start
        let services = Self::get_services(context);

        let args = tool_call
            .arguments
            .as_ref()
            .ok_or_else(|| ApiError::InvalidRequest("Missing arguments".to_string()))?;

        let params: ApplyEditParams = serde_json::from_value(args.clone())
            .map_err(|e| ApiError::InvalidRequest(format!("Invalid parameters: {}", e)))?;

        info!(
            plan_type = ?params.plan,
            dry_run = params.options.dry_run,
            validate_checksums = params.options.validate_checksums,
            "Applying refactoring plan"
        );

        // Step 1: Validate checksums if enabled
        if params.options.validate_checksums {
            debug!("Validating file checksums");
            services
                .checksum_validator
                .validate_checksums(&params.plan)
                .await?;
        }

        // Step 2: Extract WorkspaceEdit from the discriminated union
        let workspace_edit = params.plan.workspace_edit();

        // Step 3: Convert LSP WorkspaceEdit to internal EditPlan format
        let mut edit_plan = services
            .plan_converter
            .convert_to_edit_plan(workspace_edit.clone(), &params.plan)?;

        // Handle DeletePlan explicitly by reading from the deletions field
        if let RefactorPlan::DeletePlan(delete_plan) = &params.plan {
            debug!(
                deletion_count = delete_plan.deletions.len(),
                "Adding delete operations from DeletePlan"
            );

            for target in &delete_plan.deletions {
                debug!(
                    path = %target.path,
                    kind = %target.kind,
                    "Adding delete operation"
                );
                edit_plan
                    .edits
                    .push(codebuddy_foundation::protocol::TextEdit {
                        file_path: Some(target.path.clone()),
                        edit_type: codebuddy_foundation::protocol::EditType::Delete,
                        location: codebuddy_foundation::protocol::EditLocation {
                            start_line: 0,
                            start_column: 0,
                            end_line: 0,
                            end_column: 0,
                        },
                        original_text: String::new(),
                        new_text: String::new(),
                        priority: 0,
                        description: format!("Delete {}: {}", target.kind, target.path),
                    });
            }
        }

        // Step 4: Dry run preview
        if params.options.dry_run {
            let warnings: Vec<String> = params
                .plan
                .warnings()
                .iter()
                .map(|w| w.message.clone())
                .collect();

            let result = services
                .dry_run_generator
                .create_dry_run_result(&edit_plan, warnings);
            let result_json = serde_json::to_value(result).unwrap();
            return Ok(serde_json::json!({
                "content": result_json
            }));
        }

        // Step 5: Apply edits atomically with automatic backup for rollback
        let apply_result = context
            .app_state
            .file_service
            .apply_edit_plan(&edit_plan)
            .await;

        match apply_result {
            Ok(result) => {
                // Step 6: Run post-apply validation if specified
                if let Some(validation_config) = params.options.validation {
                    Self::handle_validation(
                        &services.post_apply_validator,
                        validation_config,
                        result,
                        &edit_plan,
                        &params.plan,
                    )
                    .await
                } else {
                    // No validation - return success immediately
                    Ok(Self::create_success_result(
                        result,
                        &edit_plan,
                        &params.plan,
                        None,
                    ))
                }
            }
            Err(e) => {
                // Apply failed - FileService already rolled back changes automatically
                error!(error = %e, "Edit plan application failed");
                Err(e)
            }
        }
    }
}

impl WorkspaceApplyHandler {
    /// Handle post-apply validation workflow
    async fn handle_validation(
        post_apply_validator: &PostApplyValidator,
        validation_config: ValidationConfig,
        result: EditPlanResult,
        edit_plan: &codebuddy_foundation::protocol::EditPlan,
        plan: &RefactorPlan,
    ) -> ServerResult<Value> {
        info!(command = %validation_config.command, "Running post-apply validation");

        match post_apply_validator
            .run_validation(&validation_config)
            .await
        {
            Ok(validation_result) => {
                if validation_result.passed {
                    // Validation passed - return success
                    info!(
                        exit_code = validation_result.exit_code,
                        duration_ms = validation_result.duration_ms,
                        "Post-apply validation passed"
                    );

                    Ok(Self::create_success_result(
                        result,
                        edit_plan,
                        plan,
                        Some(validation_result),
                    ))
                } else {
                    // Validation failed - return error with details
                    warn!(
                        exit_code = validation_result.exit_code,
                        "Post-apply validation failed"
                    );

                    Err(Self::create_validation_failed_error(validation_result))
                }
            }
            Err(e) => {
                // Validation execution failed (timeout, command not found, etc.)
                error!(error = %e, "Validation execution failed");

                Err(ApiError::Internal(format!(
                    "Post-apply validation execution failed: {}. \
                     Changes were applied but could not validate.",
                    e
                )))
            }
        }
    }

    /// Create a success result JSON response
    fn create_success_result(
        result: EditPlanResult,
        edit_plan: &codebuddy_foundation::protocol::EditPlan,
        plan: &RefactorPlan,
        validation: Option<ValidationResult>,
    ) -> Value {
        let rollback_available = validation.is_none(); // Save before moving validation
        let result_json = serde_json::to_value(ApplyResult {
            success: true,
            applied_files: result.modified_files.clone(),
            created_files: PlanConverter::extract_created_files(edit_plan),
            deleted_files: PlanConverter::extract_deleted_files(edit_plan),
            warnings: plan.warnings().iter().map(|w| w.message.clone()).collect(),
            validation,
            rollback_available, // Validation consumes backup
        })
        .unwrap();

        serde_json::json!({
            "content": result_json
        })
    }

    /// Create a validation failed error
    fn create_validation_failed_error(validation_result: ValidationResult) -> ApiError {
        ApiError::Internal(format!(
            "Post-apply validation failed (exit code {}). \
             Changes were applied but validation command failed.\n\
             Command: {}\n\
             Stdout: {}\n\
             Stderr: {}\n\
             \n\
             Please manually revert changes if needed.",
            validation_result.exit_code,
            validation_result.command,
            validation_result.stdout,
            validation_result.stderr
        ))
    }
}
