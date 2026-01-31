#![allow(
    dead_code,
    unused_variables,
    clippy::mutable_key_type,
    clippy::needless_range_loop,
    clippy::ptr_arg,
    clippy::manual_clamp
)]

//! Refactor operation handler - unified handler for extract, inline, and transform operations
//!
//! This handler implements the `refactor` tool which dispatches to internal
//! extract/inline planners based on the action type. The transform
//! action is reserved for future use.
//!
//! ## Supported Actions
//!
//! - **extract**: Extract functions, variables, constants, or modules
//! - **inline**: Inline variables, functions, or constants
//! - **transform**: Code transformations (placeholder for future implementation)
//!
//! ## Response Format
//!
//! All responses use the WriteResponse envelope from tool_definitions.rs:
//! - `dryRun: true` (default) - Returns preview with status="preview"
//! - `dryRun: false` - Executes changes and returns status="success" or "error"

use crate::handlers::refactor_extract::RefactorExtractPlanner;
use crate::handlers::refactor_inline::RefactorInlinePlanner;
use crate::handlers::tool_definitions::{
    Diagnostic, DiagnosticSeverity, WriteResponse, WriteStatus,
};
use crate::handlers::tools::ToolHandler;
use async_trait::async_trait;
use mill_foundation::core::model::mcp::ToolCall;
use mill_foundation::errors::{MillError as ServerError, MillResult as ServerResult};
use mill_foundation::protocol::{RefactorPlan, RefactorPlanExt};
use mill_services::services::ExecutionResult;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::Path;
use tracing::{debug, info};

pub struct RefactorHandler {
    extract_planner: RefactorExtractPlanner,
    inline_planner: RefactorInlinePlanner,
}

impl RefactorHandler {
    pub fn new() -> Self {
        Self {
            extract_planner: RefactorExtractPlanner::new(),
            inline_planner: RefactorInlinePlanner::new(),
        }
    }

    /// Handle refactor() tool call
    async fn handle_refactor(
        &self,
        context: &mill_handler_api::ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        let args = tool_call.arguments.clone().unwrap_or(json!({}));

        // Deserialize parameters
        let params: RefactorParams = serde_json::from_value(args).map_err(|e| {
            ServerError::invalid_request(format!("Invalid refactor parameters: {}", e))
        })?;

        debug!(
            action = %params.action,
            kind = %params.params.kind,
            "Processing refactor operation"
        );

        // Dispatch based on action
        match params.action.as_str() {
            "extract" => self.handle_extract(context, &params).await,
            "inline" => self.handle_inline(context, &params).await,
            "transform" => self.handle_transform(context, &params).await,
            _ => Err(ServerError::invalid_request(format!(
                "Unsupported refactor action: '{}'. Must be one of: extract, inline, transform",
                params.action
            ))),
        }
    }

    /// Handle extract action using extract planner
    async fn handle_extract(
        &self,
        context: &mill_handler_api::ToolHandlerContext,
        params: &RefactorParams,
    ) -> ServerResult<Value> {
        // Validate extract-specific requirements
        let range = params.params.range.as_ref().ok_or_else(|| {
            ServerError::invalid_request("Extract action requires 'range' parameter")
        })?;

        let name = params.params.name.as_ref().ok_or_else(|| {
            ServerError::invalid_request("Extract action requires 'name' parameter")
        })?;

        // Resolve relative paths to absolute using workspace root
        let file_path =
            resolve_file_path(&context.app_state.project_root, &params.params.file_path);

        let extract_params = crate::handlers::refactor_extract::ExtractPlanParams {
            kind: params.params.kind.clone(),
            source: crate::handlers::refactor_extract::SourceRange {
                file_path,
                range: lsp_types::Range {
                    start: lsp_types::Position {
                        line: range.start_line,
                        character: range.start_character,
                    },
                    end: lsp_types::Position {
                        line: range.end_line,
                        character: range.end_character,
                    },
                },
                name: name.clone(),
                destination: params.params.destination.clone(),
            },
            options: crate::handlers::refactor_extract::ExtractOptions {
                dry_run: params.options.dry_run,
                visibility: None,
                destination_path: None,
            },
        };

        info!(
            operation = "extract",
            kind = %params.params.kind,
            dry_run = params.options.dry_run,
            "Building extract plan"
        );

        let plan = self
            .extract_planner
            .build_extract_plan(context, &extract_params)
            .await?;

        let refactor_plan = mill_foundation::protocol::RefactorPlan::ExtractPlan(plan);

        if params.options.dry_run {
            let response = self.parse_plan_response(&refactor_plan, "extract")?;
            Ok(json!({ "content": response }))
        } else {
            let result =
                crate::handlers::common::execute_refactor_plan(context, refactor_plan).await?;
            let response = self.parse_execution_response(&result, "extract")?;
            Ok(json!({ "content": response }))
        }
    }

    /// Handle inline action using inline planner
    async fn handle_inline(
        &self,
        context: &mill_handler_api::ToolHandlerContext,
        params: &RefactorParams,
    ) -> ServerResult<Value> {
        // Validate inline-specific requirements
        let line = params.params.line.ok_or_else(|| {
            ServerError::invalid_request("Inline action requires 'line' parameter")
        })?;

        let character = params.params.character.ok_or_else(|| {
            ServerError::invalid_request("Inline action requires 'character' parameter")
        })?;

        // Resolve relative paths to absolute using workspace root
        let file_path =
            resolve_file_path(&context.app_state.project_root, &params.params.file_path);

        let inline_params = crate::handlers::refactor_inline::InlinePlanParams {
            kind: params.params.kind.clone(),
            target: crate::handlers::refactor_inline::InlineTarget {
                file_path,
                position: lsp_types::Position { line, character },
            },
            options: crate::handlers::refactor_inline::InlineOptions {
                dry_run: params.options.dry_run,
                inline_all: params.options.inline_all,
            },
        };

        info!(
            operation = "inline",
            kind = %params.params.kind,
            dry_run = params.options.dry_run,
            "Building inline plan"
        );

        let plan = self
            .inline_planner
            .build_inline_plan(context, &inline_params)
            .await?;

        let refactor_plan = mill_foundation::protocol::RefactorPlan::InlinePlan(plan);

        if params.options.dry_run {
            let response = self.parse_plan_response(&refactor_plan, "inline")?;
            Ok(json!({ "content": response }))
        } else {
            let result =
                crate::handlers::common::execute_refactor_plan(context, refactor_plan).await?;
            let response = self.parse_execution_response(&result, "inline")?;
            Ok(json!({ "content": response }))
        }
    }

    /// Handle transform action (placeholder for future implementation)
    async fn handle_transform(
        &self,
        _context: &mill_handler_api::ToolHandlerContext,
        params: &RefactorParams,
    ) -> ServerResult<Value> {
        info!(
            operation = "transform",
            kind = %params.params.kind,
            "Transform action not yet implemented"
        );

        // Return a WriteResponse with not_supported error
        let response = WriteResponse::error(
            "Transform action is not yet implemented".to_string(),
            vec![Diagnostic {
                severity: DiagnosticSeverity::Error,
                message: "The 'transform' action is reserved for future use. Use 'extract' or 'inline' for current refactoring needs.".to_string(),
                file_path: Some(params.params.file_path.clone()),
                line: None,
            }],
        );

        Ok(json!({ "content": response }))
    }

    /// Parse RefactorPlan response and convert to WriteResponse
    fn parse_plan_response(
        &self,
        plan: &RefactorPlan,
        operation: &str,
    ) -> ServerResult<WriteResponse> {
        // Access fields directly via pattern matching
        let (summary, metadata) = match plan {
            RefactorPlan::ExtractPlan(p) => (&p.summary, &p.metadata),
            RefactorPlan::InlinePlan(p) => (&p.summary, &p.metadata),
            RefactorPlan::TransformPlan(p) => (&p.summary, &p.metadata),
            RefactorPlan::RenamePlan(p) => (&p.summary, &p.metadata),
            RefactorPlan::MovePlan(p) => (&p.summary, &p.metadata),
            RefactorPlan::ReorderPlan(p) => (&p.summary, &p.metadata),
            RefactorPlan::DeletePlan(p) => (&p.summary, &p.metadata),
        };

        // Extract affected files from summary
        let affected_files = summary.affected_files;
        let created_files = summary.created_files;
        let deleted_files = summary.deleted_files;

        // Extract warnings via RefactorPlanExt trait
        let diagnostics = plan
            .warnings()
            .iter()
            .map(|w| Diagnostic {
                severity: DiagnosticSeverity::Warning,
                message: w.message.clone(),
                file_path: None,
                line: None,
            })
            .collect();

        // Extract file checksums via RefactorPlanExt trait
        let files_changed: Vec<String> = plan.checksums().keys().cloned().collect();

        // Generate summary message
        let summary_msg = format!(
            "{} {} (preview): {} file(s) affected, {} created, {} deleted",
            operation,
            capitalize_first(&metadata.kind),
            affected_files,
            created_files,
            deleted_files
        );

        // Serialize plan for the changes field (this is the only serialization needed)
        let changes = serde_json::to_value(plan)
            .map_err(|e| ServerError::internal(format!("Failed to serialize plan: {}", e)))?;

        Ok(WriteResponse {
            status: WriteStatus::Preview,
            summary: summary_msg,
            files_changed,
            diagnostics,
            changes: Some(changes),
        })
    }

    /// Parse ExecutionResult response and convert to WriteResponse
    fn parse_execution_response(
        &self,
        result: &ExecutionResult,
        operation: &str,
    ) -> ServerResult<WriteResponse> {
        let success = result.success;
        let applied_files = result.applied_files.clone();

        // Extract warnings and errors
        let mut diagnostics = Vec::new();

        for warning in &result.warnings {
            diagnostics.push(Diagnostic {
                severity: DiagnosticSeverity::Warning,
                message: warning.clone(),
                file_path: None,
                line: None,
            });
        }

        // Validation errors are not present in ExecutionResult on success,
        // and if validation fails, execute_refactor_plan returns an error.
        // Additionally, ValidationResult does not have an 'errors' field.

        let summary = if success {
            format!(
                "{} completed successfully: {} file(s) modified",
                capitalize_first(operation),
                applied_files.len()
            )
        } else {
            format!("{} failed", capitalize_first(operation))
        };

        let status = if success {
            WriteStatus::Success
        } else {
            WriteStatus::Error
        };

        // Serialize result for the changes field
        let changes = serde_json::to_value(result)
            .map_err(|e| ServerError::internal(format!("Failed to serialize result: {}", e)))?;

        Ok(WriteResponse {
            status,
            summary,
            files_changed: applied_files,
            diagnostics,
            changes: Some(changes),
        })
    }
}

impl Default for RefactorHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ToolHandler for RefactorHandler {
    fn tool_names(&self) -> &[&str] {
        &["refactor"]
    }

    async fn handle_tool_call(
        &self,
        context: &mill_handler_api::ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        match tool_call.name.as_str() {
            "refactor" => self.handle_refactor(context, tool_call).await,
            _ => Err(ServerError::not_supported(format!(
                "Unknown refactor operation: {}",
                tool_call.name
            ))),
        }
    }
}

// =============================================================================
// Parameter Structures
// =============================================================================

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct RefactorParams {
    /// The refactoring action: extract, inline, or transform
    action: String,
    /// Action-specific parameters
    params: RefactorActionParams,
    /// Refactoring options
    #[serde(default)]
    options: RefactorOptions,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct RefactorActionParams {
    /// Kind of element to refactor (function, variable, constant, module)
    kind: String,
    /// Source file path
    file_path: String,
    /// Code range (for extract)
    #[serde(default)]
    range: Option<RefactorRange>,
    /// Line number (0-based, for inline)
    #[serde(default)]
    line: Option<u32>,
    /// Character offset (0-based, for inline)
    #[serde(default)]
    character: Option<u32>,
    /// Name for extracted element (for extract)
    #[serde(default)]
    name: Option<String>,
    /// Destination path (for extract module)
    #[serde(default)]
    destination: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct RefactorRange {
    /// 0-based start line
    start_line: u32,
    /// 0-based start character
    start_character: u32,
    /// 0-based end line
    end_line: u32,
    /// 0-based end character
    end_character: u32,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct RefactorOptions {
    /// Preview mode - don't actually apply changes (default: true for safety)
    #[serde(default = "crate::default_true")]
    dry_run: bool,
    /// Inline all usages vs current only (inline action only)
    #[serde(default)]
    inline_all: Option<bool>,
}

impl Default for RefactorOptions {
    fn default() -> Self {
        Self {
            dry_run: true,
            inline_all: None,
        }
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Resolve a file path to absolute, using workspace root for relative paths
fn resolve_file_path(workspace_root: &Path, path: &str) -> String {
    let path_buf = Path::new(path);
    if path_buf.is_absolute() {
        path.to_string()
    } else {
        workspace_root.join(path).to_string_lossy().to_string()
    }
}

/// Capitalize the first character of a string
fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capitalize_first() {
        assert_eq!(capitalize_first("extract"), "Extract");
        assert_eq!(capitalize_first("inline"), "Inline");
        assert_eq!(capitalize_first("transform"), "Transform");
        assert_eq!(capitalize_first(""), "");
    }

    #[test]
    fn test_refactor_params_deserialization() {
        let json = json!({
            "action": "extract",
            "params": {
                "kind": "function",
                "filePath": "src/main.rs",
                "range": {
                    "startLine": 10,
                    "startCharacter": 5,
                    "endLine": 20,
                    "endCharacter": 10
                },
                "name": "extracted_function"
            },
            "options": {
                "dryRun": true
            }
        });

        let params: RefactorParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.action, "extract");
        assert_eq!(params.params.kind, "function");
        assert_eq!(params.params.file_path, "src/main.rs");
        assert_eq!(params.params.name.as_ref().unwrap(), "extracted_function");
        assert!(params.options.dry_run);
    }

    #[test]
    fn test_inline_params_deserialization() {
        let json = json!({
            "action": "inline",
            "params": {
                "kind": "variable",
                "filePath": "src/lib.rs",
                "line": 42,
                "character": 8
            }
        });

        let params: RefactorParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.action, "inline");
        assert_eq!(params.params.kind, "variable");
        assert_eq!(params.params.line.unwrap(), 42);
        assert_eq!(params.params.character.unwrap(), 8);
        assert!(params.options.dry_run); // Should default to true
    }
}
