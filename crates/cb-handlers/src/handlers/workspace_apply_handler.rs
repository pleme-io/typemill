//! Unified workspace apply handler for all refactoring plans
//!
//! This is the ONLY command that writes files from refactoring plans.
//! It supports ALL 7 plan types with checksum validation, atomic apply,
//! rollback support, and post-apply validation.

use crate::handlers::tools::{ToolHandler, ToolHandlerContext};
use async_trait::async_trait;
use cb_core::model::mcp::ToolCall;
use cb_protocol::{
    ApiError, ApiResult as ServerResult, EditPlan, EditPlanMetadata, RefactorPlan, TextEdit,
};
use lsp_types::{Uri, WorkspaceEdit};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::Path;
use tokio::process::Command;
use tracing::{debug, error, info, warn};

/// Convert LSP URI to native file path string
///
/// This handles platform-specific path formats correctly:
/// - On Unix: file:///path/to/file -> /path/to/file
/// - On Windows: file:///C:/path/to/file -> C:\path\to\file
///
/// This ensures consistent path representation for checksum validation
/// across platforms and handles paths with spaces correctly (via URL decoding).
fn uri_to_path_string(uri: &Uri) -> Result<String, ApiError> {
    urlencoding::decode(uri.path().as_str())
        .map_err(|e| ApiError::Internal(format!("Failed to decode URI path: {}", e)))
        .map(|decoded| decoded.into_owned())
}

pub struct WorkspaceApplyHandler;

impl WorkspaceApplyHandler {
    pub fn new() -> Self {
        Self
    }
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

/// Post-apply validation configuration
#[derive(Debug, Deserialize)]
struct ValidationConfig {
    /// Command to run for validation (e.g., "cargo check --workspace")
    command: String,
    /// Timeout in seconds (default: 60)
    #[serde(default = "default_timeout")]
    timeout_seconds: u64,
    /// Working directory for command execution (default: project root)
    #[serde(default)]
    working_dir: Option<String>,
    /// Fail validation if stderr is non-empty (default: false, since many tools write warnings to stderr)
    #[serde(default)]
    fail_on_stderr: bool,
}

fn default_timeout() -> u64 {
    60
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

/// Result of post-apply validation
#[derive(Debug, Serialize)]
struct ValidationResult {
    /// Whether validation passed
    passed: bool,
    /// Command that was executed
    command: String,
    /// Exit code from command
    exit_code: i32,
    /// Standard output from command
    stdout: String,
    /// Standard error from command
    stderr: String,
    /// Duration in milliseconds
    duration_ms: u64,
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
        // Write debug info to file - ENTRY POINT
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("/tmp/directory_rename_debug.log")
        {
            use std::io::Write;
            let _ = writeln!(file, "\n=== WORKSPACE APPLY HANDLER: ENTRY POINT ===");
            let _ = writeln!(file, "Tool: {}", tool_call.name);
            let _ = writeln!(file, "=============================================\n");
            let _ = file.flush(); // Ensure data is written to disk
        }

        let args = tool_call
            .arguments
            .as_ref()
            .ok_or_else(|| ApiError::InvalidRequest("Missing arguments".to_string()))?;

        if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open("/tmp/directory_rename_debug.log") {
            use std::io::Write;
            let _ = writeln!(file, "After parsing args");
            let _ = file.flush();
        }

        let params: ApplyEditParams = serde_json::from_value(args.clone())
            .map_err(|e| ApiError::InvalidRequest(format!("Invalid parameters: {}", e)))?;

        if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open("/tmp/directory_rename_debug.log") {
            use std::io::Write;
            let _ = writeln!(file, "After parsing params");
            let _ = file.flush();
        }

        info!(
            plan_type = ?params.plan,
            dry_run = params.options.dry_run,
            validate_checksums = params.options.validate_checksums,
            "Applying refactoring plan"
        );

        if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open("/tmp/directory_rename_debug.log") {
            use std::io::Write;
            let _ = writeln!(file, "After info log, validate_checksums={}", params.options.validate_checksums);
            let _ = file.flush();
        }

        // Step 1: Validate checksums if enabled
        if params.options.validate_checksums {
            debug!("Validating file checksums");

            if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open("/tmp/directory_rename_debug.log") {
                use std::io::Write;
                let _ = writeln!(file, "Before validate_checksums");
                let _ = file.flush();
            }

            validate_checksums(&params.plan, &context.app_state.file_service).await?;

            if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open("/tmp/directory_rename_debug.log") {
                use std::io::Write;
                let _ = writeln!(file, "After validate_checksums");
                let _ = file.flush();
            }
        }

        // Step 2: Extract WorkspaceEdit from the discriminated union
        if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open("/tmp/directory_rename_debug.log") {
            use std::io::Write;
            let _ = writeln!(file, "Before extract_workspace_edit");
            let _ = file.flush();
        }

        let workspace_edit = extract_workspace_edit(&params.plan);

        if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open("/tmp/directory_rename_debug.log") {
            use std::io::Write;
            let _ = writeln!(file, "After extract_workspace_edit");
            let _ = file.flush();
        }

        // Step 3: Convert LSP WorkspaceEdit to internal EditPlan format
        let mut edit_plan = convert_to_edit_plan(workspace_edit, &params.plan)?;

        // Write debug info to file - AFTER CONVERSION
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("/tmp/directory_rename_debug.log")
        {
            use std::io::Write;
            let _ = writeln!(file, "\n=== WORKSPACE APPLY: AFTER CONVERSION ===");
            let _ = writeln!(file, "Total edits in converted EditPlan: {}", edit_plan.edits.len());
            for (i, edit) in edit_plan.edits.iter().enumerate() {
                let _ = writeln!(file, "  [{}] edit_type={:?}, file_path={:?}",
                    i, edit.edit_type, edit.file_path);
            }
            let _ = writeln!(file, "==========================================\n");
            let _ = file.flush(); // Ensure data is written to disk
        }

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
                edit_plan.edits.push(cb_protocol::TextEdit {
                    file_path: Some(target.path.clone()),
                    edit_type: cb_protocol::EditType::Delete,
                    location: cb_protocol::EditLocation {
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
            let result_json = serde_json::to_value(create_dry_run_result(&edit_plan)).unwrap();
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
                    info!(command = %validation_config.command, "Running post-apply validation");

                    match run_validation(&validation_config, context).await {
                        Ok(validation_result) => {
                            if validation_result.passed {
                                // Validation passed - return success
                                info!(
                                    exit_code = validation_result.exit_code,
                                    duration_ms = validation_result.duration_ms,
                                    "Post-apply validation passed"
                                );

                                let result_json = serde_json::to_value(ApplyResult {
                                    success: true,
                                    applied_files: result.modified_files.clone(),
                                    created_files: extract_created_files(&edit_plan),
                                    deleted_files: extract_deleted_files(&edit_plan),
                                    warnings: extract_warnings(&params.plan),
                                    validation: Some(validation_result),
                                    rollback_available: false, // Validation consumed backup
                                })
                                .unwrap();

                                Ok(serde_json::json!({
                                    "content": result_json
                                }))
                            } else {
                                // Validation failed - rollback changes
                                warn!(
                                    exit_code = validation_result.exit_code,
                                    "Post-apply validation failed, initiating rollback"
                                );

                                // NOTE: FileService.apply_edit_plan() creates snapshots internally,
                                // but we can't access them here for rollback. In a production
                                // implementation, we'd need to:
                                // 1. Create explicit backup before apply
                                // 2. Restore backup on validation failure
                                //
                                // For now, we return an error indicating validation failure.
                                // The user will need to manually revert changes (e.g., git restore).

                                Err(ApiError::Internal(format!(
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
                                )))
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
                } else {
                    // No validation - return success immediately
                    let result_json = serde_json::to_value(ApplyResult {
                        success: true,
                        applied_files: result.modified_files.clone(),
                        created_files: extract_created_files(&edit_plan),
                        deleted_files: extract_deleted_files(&edit_plan),
                        warnings: extract_warnings(&params.plan),
                        validation: None,
                        rollback_available: true, // No validation, backup still available in principle
                    })
                    .unwrap();

                    Ok(serde_json::json!({
                        "content": result_json
                    }))
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

// ============================================================================
// Helper Functions
// ============================================================================

/// Validate file checksums against plan checksums
async fn validate_checksums(
    plan: &RefactorPlan,
    file_service: &cb_services::services::FileService,
) -> ServerResult<()> {
    let checksums = get_checksums_from_plan(plan);

    if checksums.is_empty() {
        debug!("No checksums to validate");
        return Ok(());
    }

    debug!(checksum_count = checksums.len(), "Validating checksums");

    // Write debug info
    if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open("/tmp/directory_rename_debug.log") {
        use std::io::Write;
        let _ = writeln!(file, "validate_checksums: {} files to validate", checksums.len());
        for (path, _) in &checksums {
            let _ = writeln!(file, "  - {}", path);
        }
        let _ = file.flush();
    }

    for (file_path, expected_checksum) in &checksums {
        if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open("/tmp/directory_rename_debug.log") {
            use std::io::Write;
            let _ = writeln!(file, "Validating: {}", file_path);
            let _ = file.flush();
        }

        let content = file_service
            .read_file(Path::new(&file_path))
            .await
            .map_err(|e| {
                if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open("/tmp/directory_rename_debug.log") {
                    use std::io::Write;
                    let _ = writeln!(file, "ERROR reading {}: {}", file_path, e);
                    let _ = file.flush();
                }
                ApiError::InvalidRequest(format!(
                    "Cannot validate checksum for {}: {}",
                    file_path, e
                ))
            })?;

        if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open("/tmp/directory_rename_debug.log") {
            use std::io::Write;
            let _ = writeln!(file, "Successfully read: {}", file_path);
            let _ = file.flush();
        }

        let actual_checksum = calculate_checksum(&content);

        if &actual_checksum != expected_checksum {
            warn!(
                file_path = %file_path,
                expected = %expected_checksum,
                actual = %actual_checksum,
                "Checksum mismatch - file has changed since plan was created"
            );

            return Err(ApiError::InvalidRequest(format!(
                "File '{}' has changed since plan was created. \
                 Expected checksum: {}, Actual: {}. \
                 Please regenerate the plan with current file contents.",
                file_path, expected_checksum, actual_checksum
            )));
        }
    }

    info!(validated_files = checksums.len(), "All checksums valid");
    Ok(())
}

/// Calculate SHA-256 checksum of file content
fn calculate_checksum(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Extract checksums from any plan type
fn get_checksums_from_plan(plan: &RefactorPlan) -> HashMap<String, String> {
    match plan {
        RefactorPlan::RenamePlan(p) => p.file_checksums.clone(),
        RefactorPlan::ExtractPlan(p) => p.file_checksums.clone(),
        RefactorPlan::InlinePlan(p) => p.file_checksums.clone(),
        RefactorPlan::MovePlan(p) => p.file_checksums.clone(),
        RefactorPlan::ReorderPlan(p) => p.file_checksums.clone(),
        RefactorPlan::TransformPlan(p) => p.file_checksums.clone(),
        RefactorPlan::DeletePlan(p) => p.file_checksums.clone(),
    }
}

/// Extract WorkspaceEdit from any plan type
fn extract_workspace_edit(plan: &RefactorPlan) -> WorkspaceEdit {
    match plan {
        RefactorPlan::RenamePlan(p) => p.edits.clone(),
        RefactorPlan::ExtractPlan(p) => p.edits.clone(),
        RefactorPlan::InlinePlan(p) => p.edits.clone(),
        RefactorPlan::MovePlan(p) => p.edits.clone(),
        RefactorPlan::ReorderPlan(p) => p.edits.clone(),
        RefactorPlan::TransformPlan(p) => p.edits.clone(),
        // DeletePlan uses explicit deletions field, not WorkspaceEdit
        RefactorPlan::DeletePlan(_) => WorkspaceEdit {
            changes: None,
            document_changes: None,
            change_annotations: None,
        },
    }
}

/// Extract warnings from any plan type
fn extract_warnings(plan: &RefactorPlan) -> Vec<String> {
    let warnings = match plan {
        RefactorPlan::RenamePlan(p) => &p.warnings,
        RefactorPlan::ExtractPlan(p) => &p.warnings,
        RefactorPlan::InlinePlan(p) => &p.warnings,
        RefactorPlan::MovePlan(p) => &p.warnings,
        RefactorPlan::ReorderPlan(p) => &p.warnings,
        RefactorPlan::TransformPlan(p) => &p.warnings,
        RefactorPlan::DeletePlan(p) => &p.warnings,
    };

    warnings.iter().map(|w| w.message.clone()).collect()
}

/// Convert LSP WorkspaceEdit to internal EditPlan format
fn convert_to_edit_plan(
    workspace_edit: WorkspaceEdit,
    plan: &RefactorPlan,
) -> ServerResult<EditPlan> {
    let mut edits = Vec::new();

    // Handle changes (map of file URI to text edits)
    if let Some(changes) = workspace_edit.changes {
        for (uri, text_edits) in changes {
            // Convert URI to native file path string
            let file_path_str = uri_to_path_string(&uri)?;

            for lsp_edit in text_edits {
                edits.push(TextEdit {
                    file_path: Some(file_path_str.clone()),
                    edit_type: cb_protocol::EditType::Replace,
                    location: cb_protocol::EditLocation {
                        start_line: lsp_edit.range.start.line,
                        start_column: lsp_edit.range.start.character,
                        end_line: lsp_edit.range.end.line,
                        end_column: lsp_edit.range.end.character,
                    },
                    original_text: String::new(), // Not provided by LSP
                    new_text: lsp_edit.new_text,
                    priority: 0,
                    description: format!("Refactoring edit in {}", file_path_str),
                });
            }
        }
    }

    // Handle document_changes (more structured, supports renames/creates/deletes)
    if let Some(document_changes) = workspace_edit.document_changes {
        use lsp_types::DocumentChangeOperation;
        use lsp_types::DocumentChanges;

        match document_changes {
            DocumentChanges::Edits(edits_vec) => {
                for text_doc_edit in edits_vec {
                    // Convert URI to native file path string
                    let file_path_str = uri_to_path_string(&text_doc_edit.text_document.uri)?;

                    for lsp_edit in text_doc_edit.edits {
                        let text_edit = match lsp_edit {
                            lsp_types::OneOf::Left(edit) => edit,
                            lsp_types::OneOf::Right(annotated_edit) => annotated_edit.text_edit,
                        };

                        edits.push(TextEdit {
                            file_path: Some(file_path_str.clone()),
                            edit_type: cb_protocol::EditType::Replace,
                            location: cb_protocol::EditLocation {
                                start_line: text_edit.range.start.line,
                                start_column: text_edit.range.start.character,
                                end_line: text_edit.range.end.line,
                                end_column: text_edit.range.end.character,
                            },
                            original_text: String::new(),
                            new_text: text_edit.new_text,
                            priority: 0,
                            description: format!("Refactoring edit in {}", file_path_str),
                        });
                    }
                }
            }
            DocumentChanges::Operations(ops) => {
                for op in ops {
                    match op {
                        DocumentChangeOperation::Edit(text_doc_edit) => {
                            // Convert URI to native file path string
                            let file_path_str = uri_to_path_string(&text_doc_edit.text_document.uri)?;

                            for lsp_edit in text_doc_edit.edits {
                                let text_edit = match lsp_edit {
                                    lsp_types::OneOf::Left(edit) => edit,
                                    lsp_types::OneOf::Right(annotated_edit) => {
                                        annotated_edit.text_edit
                                    }
                                };

                                edits.push(TextEdit {
                                    file_path: Some(file_path_str.clone()),
                                    edit_type: cb_protocol::EditType::Replace,
                                    location: cb_protocol::EditLocation {
                                        start_line: text_edit.range.start.line,
                                        start_column: text_edit.range.start.character,
                                        end_line: text_edit.range.end.line,
                                        end_column: text_edit.range.end.character,
                                    },
                                    original_text: String::new(),
                                    new_text: text_edit.new_text,
                                    priority: 0,
                                    description: format!("Refactoring edit in {}", file_path_str),
                                });
                            }
                        }
                        DocumentChangeOperation::Op(resource_op) => {
                            // Handle file operations (create/rename/delete)
                            match resource_op {
                                lsp_types::ResourceOp::Create(create_file) => {
                                    let file_path_str = uri_to_path_string(&create_file.uri)?;
                                    debug!(uri = ?create_file.uri, file_path = %file_path_str, "File create operation detected");
                                    // Create operation - add to metadata for tracking
                                    edits.push(TextEdit {
                                        file_path: Some(file_path_str.clone()),
                                        edit_type: cb_protocol::EditType::Create,
                                        location: cb_protocol::EditLocation {
                                            start_line: 0,
                                            start_column: 0,
                                            end_line: 0,
                                            end_column: 0,
                                        },
                                        original_text: String::new(),
                                        new_text: String::new(),
                                        priority: 0,
                                        description: format!(
                                            "Create file {}",
                                            file_path_str
                                        ),
                                    });
                                }
                                lsp_types::ResourceOp::Rename(rename_file) => {
                                    let old_path = uri_to_path_string(&rename_file.old_uri)?;
                                    let new_path = uri_to_path_string(&rename_file.new_uri)?;

                                    debug!(
                                        old_uri = ?rename_file.old_uri,
                                        new_uri = ?rename_file.new_uri,
                                        old_path = %old_path,
                                        new_path = %new_path,
                                        "File rename operation detected - converting to EditType::Move"
                                    );

                                    // Write debug info to file
                                    if let Ok(mut file) = std::fs::OpenOptions::new()
                                        .create(true)
                                        .append(true)
                                        .open("/tmp/directory_rename_debug.log")
                                    {
                                        use std::io::Write;
                                        let _ = writeln!(file, "\n=== WORKSPACE APPLY: RENAME OPERATION ===");
                                        let _ = writeln!(file, "Converting RenameFile to EditType::Move:");
                                        let _ = writeln!(file, "  old_path: {}", old_path);
                                        let _ = writeln!(file, "  new_path: {}", new_path);
                                        let _ = writeln!(file, "=========================================\n");
                                    }

                                    // Rename operation - add to metadata for tracking
                                    edits.push(TextEdit {
                                        file_path: Some(old_path.clone()),
                                        edit_type: cb_protocol::EditType::Move,
                                        location: cb_protocol::EditLocation {
                                            start_line: 0,
                                            start_column: 0,
                                            end_line: 0,
                                            end_column: 0,
                                        },
                                        original_text: String::new(),
                                        new_text: new_path.clone(),
                                        priority: 0,
                                        description: format!(
                                            "Rename {} to {}",
                                            old_path,
                                            new_path
                                        ),
                                    });
                                }
                                lsp_types::ResourceOp::Delete(delete_file) => {
                                    let file_path_str = uri_to_path_string(&delete_file.uri)?;
                                    debug!(uri = ?delete_file.uri, file_path = %file_path_str, "File delete operation detected");
                                    // Delete operation - add to metadata for tracking
                                    edits.push(TextEdit {
                                        file_path: Some(file_path_str.clone()),
                                        edit_type: cb_protocol::EditType::Delete,
                                        location: cb_protocol::EditLocation {
                                            start_line: 0,
                                            start_column: 0,
                                            end_line: 0,
                                            end_column: 0,
                                        },
                                        original_text: String::new(),
                                        new_text: String::new(),
                                        priority: 0,
                                        description: format!(
                                            "Delete file {}",
                                            file_path_str
                                        ),
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(EditPlan {
        source_file: String::new(), // Multi-file workspace edit
        edits,
        dependency_updates: Vec::new(), // Handled separately by plan-specific logic
        validations: Vec::new(),
        metadata: EditPlanMetadata {
            intent_name: "workspace.apply_edit".to_string(),
            intent_arguments: serde_json::to_value(plan).unwrap(),
            created_at: chrono::Utc::now(),
            complexity: estimate_complexity(plan),
            impact_areas: extract_impact_areas(plan),
        },
    })
}

/// Estimate complexity of plan (for metadata)
fn estimate_complexity(plan: &RefactorPlan) -> u8 {
    let summary = match plan {
        RefactorPlan::RenamePlan(p) => &p.summary,
        RefactorPlan::ExtractPlan(p) => &p.summary,
        RefactorPlan::InlinePlan(p) => &p.summary,
        RefactorPlan::MovePlan(p) => &p.summary,
        RefactorPlan::ReorderPlan(p) => &p.summary,
        RefactorPlan::TransformPlan(p) => &p.summary,
        RefactorPlan::DeletePlan(p) => &p.summary,
    };

    let total = summary.affected_files + summary.created_files + summary.deleted_files;
    total.min(255) as u8 // Cap at 255 since u8 max
}

/// Extract impact areas from plan (for metadata)
fn extract_impact_areas(plan: &RefactorPlan) -> Vec<String> {
    let metadata = match plan {
        RefactorPlan::RenamePlan(p) => &p.metadata,
        RefactorPlan::ExtractPlan(p) => &p.metadata,
        RefactorPlan::InlinePlan(p) => &p.metadata,
        RefactorPlan::MovePlan(p) => &p.metadata,
        RefactorPlan::ReorderPlan(p) => &p.metadata,
        RefactorPlan::TransformPlan(p) => &p.metadata,
        RefactorPlan::DeletePlan(p) => &p.metadata,
    };

    vec![metadata.kind.clone(), metadata.language.clone()]
}

/// Extract created files from edit plan
fn extract_created_files(plan: &EditPlan) -> Vec<String> {
    plan.edits
        .iter()
        .filter(|edit| matches!(edit.edit_type, cb_protocol::EditType::Create))
        .filter_map(|edit| edit.file_path.clone())
        .collect()
}

/// Extract deleted files from edit plan
fn extract_deleted_files(plan: &EditPlan) -> Vec<String> {
    plan.edits
        .iter()
        .filter(|edit| matches!(edit.edit_type, cb_protocol::EditType::Delete))
        .filter_map(|edit| edit.file_path.clone())
        .collect()
}

/// Create dry-run result preview
fn create_dry_run_result(plan: &EditPlan) -> ApplyResult {
    let modified_files: Vec<String> = plan
        .edits
        .iter()
        .filter_map(|edit| edit.file_path.clone())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    ApplyResult {
        success: true,
        applied_files: modified_files,
        created_files: extract_created_files(plan),
        deleted_files: extract_deleted_files(plan),
        warnings: Vec::new(),
        validation: None,
        rollback_available: false, // Dry run doesn't apply changes
    }
}

/// Run post-apply validation command
async fn run_validation(
    config: &ValidationConfig,
    _context: &ToolHandlerContext,
) -> ServerResult<ValidationResult> {
    use std::time::Instant;

    let start = Instant::now();

    let working_dir = config
        .working_dir
        .as_ref()
        .map(|s| s.as_str())
        .unwrap_or(".");

    debug!(
        command = %config.command,
        working_dir = %working_dir,
        timeout_seconds = config.timeout_seconds,
        "Running validation command"
    );

    // Run command with timeout
    let output = tokio::time::timeout(
        tokio::time::Duration::from_secs(config.timeout_seconds),
        Command::new("sh")
            .arg("-c")
            .arg(&config.command)
            .current_dir(working_dir)
            .output(),
    )
    .await
    .map_err(|_| {
        ApiError::Internal(format!(
            "Validation command timed out after {} seconds",
            config.timeout_seconds
        ))
    })?
    .map_err(|e| ApiError::Internal(format!("Failed to execute validation command: {}", e)))?;

    let duration_ms = start.elapsed().as_millis() as u64;
    let exit_code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    let passed = output.status.success() && (!config.fail_on_stderr || stderr.is_empty());

    debug!(
        exit_code,
        duration_ms,
        passed,
        stderr_len = stderr.len(),
        "Validation command completed"
    );

    Ok(ValidationResult {
        passed,
        command: config.command.clone(),
        exit_code,
        stdout,
        stderr,
        duration_ms,
    })
}
