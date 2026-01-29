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
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
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

        let extract_params = crate::handlers::refactor_extract::ExtractPlanParams {
            kind: params.params.kind.clone(),
            source: crate::handlers::refactor_extract::SourceRange {
                file_path: params.params.file_path.clone(),
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
            let plan_json = serde_json::to_value(&refactor_plan)
                .map_err(|e| ServerError::internal(format!("Failed to serialize plan: {}", e)))?;
            let response = self.parse_plan_response(&plan_json, "extract")?;
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

        let inline_params = crate::handlers::refactor_inline::InlinePlanParams {
            kind: params.params.kind.clone(),
            target: crate::handlers::refactor_inline::InlineTarget {
                file_path: params.params.file_path.clone(),
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
            let plan_json = serde_json::to_value(&refactor_plan)
                .map_err(|e| ServerError::internal(format!("Failed to serialize plan: {}", e)))?;
            let response = self.parse_plan_response(&plan_json, "inline")?;
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

    /// Parse RefactorPlan response and convert to WriteResponse
    fn parse_plan_response(&self, content: &Value, operation: &str) -> ServerResult<WriteResponse> {
        // Extract plan details from the RefactorPlan variant
        let plan_data = if let Some(extract_plan) = content.get("ExtractPlan") {
            extract_plan
        } else if let Some(inline_plan) = content.get("InlinePlan") {
            inline_plan
        } else {
            return Err(ServerError::internal(
                "Unexpected plan format: missing ExtractPlan or InlinePlan",
            ));
        };

        // Extract affected files from the summary
        let summary = plan_data
            .get("summary")
            .ok_or_else(|| ServerError::internal("Plan response missing 'summary' field"))?;

        let affected_files = summary
            .get("affected_files")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        let created_files = summary
            .get("created_files")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        let deleted_files = summary
            .get("deleted_files")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        // Extract warnings and convert to diagnostics
        let warnings = plan_data
            .get("warnings")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|w| w.as_str())
                    .map(|msg| Diagnostic {
                        severity: DiagnosticSeverity::Warning,
                        message: msg.to_string(),
                        file_path: None,
                        line: None,
                    })
                    .collect()
            })
            .unwrap_or_default();

        // Extract file checksums to get affected file paths
        let files_changed: Vec<String> = plan_data
            .get("file_checksums")
            .and_then(|v| v.as_object())
            .map(|obj| obj.keys().cloned().collect())
            .unwrap_or_default();

        // Generate summary message
        let summary_msg = format!(
            "{} {} (preview): {} file(s) affected, {} created, {} deleted",
            operation,
            capitalize_first(
                plan_data
                    .get("metadata")
                    .and_then(|m| m.get("kind"))
                    .and_then(|k| k.as_str())
                    .unwrap_or(operation)
            ),
            affected_files,
            created_files,
            deleted_files
        );

        Ok(WriteResponse {
            status: WriteStatus::Preview,
            summary: summary_msg,
            files_changed,
            diagnostics: warnings,
            changes: Some(content.clone()),
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
