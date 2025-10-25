//! Plan executor service for applying refactoring plans
//!
//! Extracted from WorkspaceApplyHandler to provide reusable execution logic
//! for all refactoring handlers (rename, extract, inline, move, etc.)

use mill_foundation::protocol::{
    ApiError, ApiResult as ServerResult, EditPlan, EditType, RefactorPlan, RefactorPlanExt,
};
use crate::services::file_service::EditPlanResult;
use crate::{
    ChecksumValidator, DryRunGenerator, PlanConverter, PostApplyValidator, ValidationConfig,
    ValidationResult,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, error, info};

/// Options for executing a refactoring plan
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionOptions {
    /// Validate file checksums before applying (prevents stale plans)
    #[serde(default = "default_true")]
    pub validate_checksums: bool,

    /// Automatically rollback all changes if any error occurs
    #[serde(default = "default_true")]
    pub rollback_on_error: bool,

    /// Post-apply validation configuration
    #[serde(default)]
    pub validation: Option<ValidationConfig>,
}

fn default_true() -> bool {
    true
}

impl Default for ExecutionOptions {
    fn default() -> Self {
        Self {
            validate_checksums: true,
            rollback_on_error: true,
            validation: None,
        }
    }
}

/// Result of executing a refactoring plan
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionResult {
    pub success: bool,
    pub applied_files: Vec<String>,
    pub created_files: Vec<String>,
    pub deleted_files: Vec<String>,
    pub warnings: Vec<String>,
    pub validation: Option<ValidationResult>,
    pub rollback_available: bool,
}

/// Service for executing refactoring plans
pub struct PlanExecutor {
    checksum_validator: Arc<ChecksumValidator>,
    dry_run_generator: Arc<DryRunGenerator>,
    post_apply_validator: Arc<PostApplyValidator>,
    plan_converter: Arc<PlanConverter>,
    file_service: Arc<crate::services::FileService>,
}

impl PlanExecutor {
    pub fn new(file_service: Arc<crate::services::FileService>) -> Self {
        Self {
            checksum_validator: Arc::new(ChecksumValidator::new(file_service.clone())),
            dry_run_generator: Arc::new(DryRunGenerator::new()),
            post_apply_validator: Arc::new(PostApplyValidator::new()),
            plan_converter: Arc::new(PlanConverter::new()),
            file_service,
        }
    }

    /// Execute a refactoring plan with the given options
    pub async fn execute_plan(
        &self,
        plan: RefactorPlan,
        options: ExecutionOptions,
    ) -> ServerResult<ExecutionResult> {
        info!(
            plan_type = ?plan,
            validate_checksums = options.validate_checksums,
            "Executing refactoring plan"
        );

        // Step 1: Validate checksums if enabled
        if options.validate_checksums {
            debug!("Validating file checksums");
            self.checksum_validator.validate_checksums(&plan).await?;
        }

        // Step 2: Extract WorkspaceEdit from the discriminated union
        let workspace_edit = plan.workspace_edit();

        // Step 3: Convert LSP WorkspaceEdit to internal EditPlan format
        let mut edit_plan = self
            .plan_converter
            .convert_to_edit_plan(workspace_edit.clone(), &plan)?;

        // Step 4: Handle DeletePlan explicitly by reading from the deletions field
        if let RefactorPlan::DeletePlan(delete_plan) = &plan {
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
                edit_plan.edits.push(mill_foundation::protocol::TextEdit {
                    file_path: Some(target.path.clone()),
                    edit_type: EditType::Delete,
                    location: mill_foundation::protocol::EditLocation {
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

        // Step 5: Apply edits atomically with automatic backup for rollback
        let apply_result = self.file_service.apply_edit_plan(&edit_plan).await;

        match apply_result {
            Ok(result) => {
                // Step 6: Run post-apply validation if specified
                if let Some(validation_config) = options.validation {
                    self.handle_validation(validation_config, result, &edit_plan, &plan)
                        .await
                } else {
                    // No validation - return success immediately
                    Ok(self.create_success_result(result, &edit_plan, &plan, None))
                }
            }
            Err(e) => {
                // Apply failed - FileService already rolled back changes automatically
                error!(error = %e, "Edit plan application failed");
                Err(e)
            }
        }
    }

    /// Handle post-apply validation workflow
    async fn handle_validation(
        &self,
        validation_config: ValidationConfig,
        result: EditPlanResult,
        edit_plan: &EditPlan,
        plan: &RefactorPlan,
    ) -> ServerResult<ExecutionResult> {
        info!(command = %validation_config.command, "Running post-apply validation");

        match self.post_apply_validator.run_validation(&validation_config).await {
            Ok(validation_result) => {
                if validation_result.passed {
                    // Validation passed - return success
                    info!(
                        exit_code = validation_result.exit_code,
                        duration_ms = validation_result.duration_ms,
                        "Post-apply validation passed"
                    );

                    Ok(self.create_success_result(
                        result,
                        edit_plan,
                        plan,
                        Some(validation_result),
                    ))
                } else {
                    // Validation failed - return error with details
                    error!(
                        exit_code = validation_result.exit_code,
                        "Post-apply validation failed"
                    );

                    Err(self.create_validation_failed_error(validation_result))
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

    /// Create a success result
    fn create_success_result(
        &self,
        result: EditPlanResult,
        edit_plan: &EditPlan,
        plan: &RefactorPlan,
        validation: Option<ValidationResult>,
    ) -> ExecutionResult {
        let rollback_available = validation.is_none(); // Save before moving validation

        ExecutionResult {
            success: true,
            applied_files: result.modified_files.clone(),
            created_files: PlanConverter::extract_created_files(edit_plan),
            deleted_files: PlanConverter::extract_deleted_files(edit_plan),
            warnings: plan.warnings().iter().map(|w| w.message.clone()).collect(),
            validation,
            rollback_available, // Validation consumes backup
        }
    }

    /// Create a validation failed error
    fn create_validation_failed_error(&self, validation_result: ValidationResult) -> ApiError {
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
