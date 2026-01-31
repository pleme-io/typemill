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
use mill_foundation::protocol::RefactorPlan;
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
            let response = self.create_preview_response(&refactor_plan, "extract")?;
            Ok(json!({ "content": response }))
        } else {
            let result =
                crate::handlers::common::execute_refactor_plan(context, refactor_plan).await?;
            let result_json = serde_json::to_value(&result).map_err(|e| {
                ServerError::internal(format!("Failed to serialize execution result: {}", e))
            })?;
            let response = self.parse_execution_response(&result_json, "extract")?;
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
            let response = self.create_preview_response(&refactor_plan, "inline")?;
            Ok(json!({ "content": response }))
        } else {
            let result =
                crate::handlers::common::execute_refactor_plan(context, refactor_plan).await?;
            let result_json = serde_json::to_value(&result).map_err(|e| {
                ServerError::internal(format!("Failed to serialize execution result: {}", e))
            })?;
            let response = self.parse_execution_response(&result_json, "inline")?;
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

    /// Create a preview response directly from the RefactorPlan
    ///
    /// This method is optimized to avoid unnecessary JSON serialization/deserialization for metadata
    /// and fixes a bug where warnings were not being correctly parsed.
    fn create_preview_response(
        &self,
        plan: &RefactorPlan,
        operation: &str,
    ) -> ServerResult<WriteResponse> {
        let (summary, warnings, file_checksums, metadata) = match plan {
            RefactorPlan::ExtractPlan(p) => (
                &p.summary,
                &p.warnings,
                &p.file_checksums,
                &p.metadata,
            ),
            RefactorPlan::InlinePlan(p) => (
                &p.summary,
                &p.warnings,
                &p.file_checksums,
                &p.metadata,
            ),
            RefactorPlan::RenamePlan(p) => (
                &p.summary,
                &p.warnings,
                &p.file_checksums,
                &p.metadata,
            ),
            RefactorPlan::MovePlan(p) => (
                &p.summary,
                &p.warnings,
                &p.file_checksums,
                &p.metadata,
            ),
            RefactorPlan::ReorderPlan(p) => (
                &p.summary,
                &p.warnings,
                &p.file_checksums,
                &p.metadata,
            ),
            RefactorPlan::TransformPlan(p) => (
                &p.summary,
                &p.warnings,
                &p.file_checksums,
                &p.metadata,
            ),
            RefactorPlan::DeletePlan(p) => (
                &p.summary,
                &p.warnings,
                &p.file_checksums,
                &p.metadata,
            ),
        };

        let diagnostics: Vec<Diagnostic> = warnings
            .iter()
            .map(|w| Diagnostic {
                severity: DiagnosticSeverity::Warning,
                message: w.message.clone(),
                file_path: None,
                line: None,
            })
            .collect();

        let files_changed: Vec<String> = file_checksums.keys().cloned().collect();

        let summary_msg = format!(
            "{} {} (preview): {} file(s) affected, {} created, {} deleted",
            operation,
            capitalize_first(&metadata.kind),
            summary.affected_files,
            summary.created_files,
            summary.deleted_files
        );

        // Serialize the full plan to JSON for the 'changes' field
        let changes_json = serde_json::to_value(plan)
            .map_err(|e| ServerError::internal(format!("Failed to serialize plan: {}", e)))?;

        Ok(WriteResponse {
            status: WriteStatus::Preview,
            summary: summary_msg,
            files_changed,
            diagnostics,
            changes: Some(changes_json),
        })
    }

    /// Parse ExecutionResult response and convert to WriteResponse
    fn parse_execution_response(
        &self,
        content: &Value,
        operation: &str,
    ) -> ServerResult<WriteResponse> {
        let success = content
            .get("success")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let applied_files: Vec<String> = content
            .get("applied_files")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        // Extract warnings and errors
        let mut diagnostics = Vec::new();

        if let Some(warnings) = content.get("warnings").and_then(|v| v.as_array()) {
            for warning in warnings {
                if let Some(msg) = warning.as_str() {
                    diagnostics.push(Diagnostic {
                        severity: DiagnosticSeverity::Warning,
                        message: msg.to_string(),
                        file_path: None,
                        line: None,
                    });
                }
            }
        }

        if let Some(validation) = content.get("validation") {
            if let Some(errors) = validation.get("errors").and_then(|v| v.as_array()) {
                for error in errors {
                    if let Some(msg) = error.as_str() {
                        diagnostics.push(Diagnostic {
                            severity: DiagnosticSeverity::Error,
                            message: msg.to_string(),
                            file_path: None,
                            line: None,
                        });
                    }
                }
            }
        }

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

        Ok(WriteResponse {
            status,
            summary,
            files_changed: applied_files,
            diagnostics,
            changes: Some(content.clone()),
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

    #[test]
    fn test_create_preview_response_warnings_fix() {
        use mill_foundation::protocol::{ExtractPlan, PlanMetadata, PlanSummary, PlanWarning, RefactorPlan};
        use lsp_types::WorkspaceEdit;
        use std::collections::HashMap;

        let handler = RefactorHandler::new();

        let plan = RefactorPlan::ExtractPlan(ExtractPlan {
            edits: WorkspaceEdit::default(),
            summary: PlanSummary {
                affected_files: 1,
                created_files: 0,
                deleted_files: 0,
            },
            warnings: vec![PlanWarning {
                code: "W001".to_string(),
                message: "Test warning".to_string(),
                candidates: None,
            }],
            metadata: PlanMetadata {
                plan_version: "1.0".to_string(),
                kind: "extract".to_string(),
                language: "rust".to_string(),
                estimated_impact: "low".to_string(),
                created_at: "2024-01-01T00:00:00Z".to_string(),
            },
            file_checksums: HashMap::new(),
        });

        let response = handler.create_preview_response(&plan, "extract").unwrap();

        // Verify warnings are now correctly preserved
        assert_eq!(response.diagnostics.len(), 1, "Warnings should be preserved");
        assert_eq!(response.diagnostics[0].message, "Test warning");
    }

    #[test]
    fn test_create_preview_response_performance() {
        use mill_foundation::protocol::{ExtractPlan, PlanMetadata, PlanSummary, RefactorPlan};
        use lsp_types::WorkspaceEdit;
        use std::collections::HashMap;

        let handler = RefactorHandler::new();

        // Create a plan with significant data to measure serialization/parsing cost
        let mut checksums = HashMap::new();
        for i in 0..1000 {
            checksums.insert(format!("file_{}.rs", i), "abcdef1234567890".to_string());
        }

        let plan = RefactorPlan::ExtractPlan(ExtractPlan {
            edits: WorkspaceEdit::default(),
            summary: PlanSummary {
                affected_files: 1000,
                created_files: 0,
                deleted_files: 0,
            },
            warnings: vec![],
            metadata: PlanMetadata {
                plan_version: "1.0".to_string(),
                kind: "extract".to_string(),
                language: "rust".to_string(),
                estimated_impact: "high".to_string(),
                created_at: "2024-01-01T00:00:00Z".to_string(),
            },
            file_checksums: checksums,
        });

        let start = std::time::Instant::now();

        // This simulates the optimized flow
        for _ in 0..100 {
            let _ = handler.create_preview_response(&plan, "extract").unwrap();
        }

        let duration = start.elapsed();
        println!("Optimized Performance (100 iterations): {:?}", duration);
    }
