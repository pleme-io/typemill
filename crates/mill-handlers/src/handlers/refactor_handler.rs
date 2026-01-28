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
//! This handler implements the `refactor` tool which dispatches to existing
//! ExtractHandler and InlineHandler based on the action type. The transform
//! action is reserved for future use.
//!
//! ## Supported Actions
//!
//! - **extract**: Extract functions, variables, constants, or modules (delegates to ExtractHandler)
//! - **inline**: Inline variables, functions, or constants (delegates to InlineHandler)
//! - **transform**: Code transformations (placeholder for future implementation)
//!
//! ## Response Format
//!
//! All responses use the WriteResponse envelope from tool_definitions.rs:
//! - `dryRun: true` (default) - Returns preview with status="preview"
//! - `dryRun: false` - Executes changes and returns status="success" or "error"

use crate::handlers::extract_handler::ExtractHandler;
use crate::handlers::inline_handler::InlineHandler;
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
    extract_handler: ExtractHandler,
    inline_handler: InlineHandler,
}

impl RefactorHandler {
    pub fn new() -> Self {
        Self {
            extract_handler: ExtractHandler::new(),
            inline_handler: InlineHandler::new(),
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

    /// Handle extract action by delegating to ExtractHandler
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

        // Convert RefactorParams to ExtractHandler's expected format
        let extract_args = json!({
            "kind": params.params.kind,
            "source": {
                "filePath": params.params.file_path,
                "range": {
                    "start": {
                        "line": range.start_line,
                        "character": range.start_character
                    },
                    "end": {
                        "line": range.end_line,
                        "character": range.end_character
                    }
                },
                "name": name,
                "destination": params.params.destination
            },
            "options": {
                "dryRun": params.options.dry_run
            }
        });

        let extract_call = ToolCall {
            name: "extract".to_string(),
            arguments: Some(extract_args),
        };

        info!(
            operation = "extract",
            kind = %params.params.kind,
            dry_run = params.options.dry_run,
            "Delegating to ExtractHandler"
        );

        // Call ExtractHandler and convert response to WriteResponse
        let result = self
            .extract_handler
            .handle_tool_call(context, &extract_call)
            .await?;

        self.convert_to_write_response(result, params.options.dry_run, "extract")
    }

    /// Handle inline action by delegating to InlineHandler
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

        // Convert RefactorParams to InlineHandler's expected format
        let inline_args = json!({
            "kind": params.params.kind,
            "target": {
                "filePath": params.params.file_path,
                "position": {
                    "line": line,
                    "character": character
                }
            },
            "options": {
                "dryRun": params.options.dry_run
            }
        });

        let inline_call = ToolCall {
            name: "inline".to_string(),
            arguments: Some(inline_args),
        };

        info!(
            operation = "inline",
            kind = %params.params.kind,
            dry_run = params.options.dry_run,
            "Delegating to InlineHandler"
        );

        // Call InlineHandler and convert response to WriteResponse
        let result = self
            .inline_handler
            .handle_tool_call(context, &inline_call)
            .await?;

        self.convert_to_write_response(result, params.options.dry_run, "inline")
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

    /// Convert handler response to WriteResponse envelope
    fn convert_to_write_response(
        &self,
        result: Value,
        dry_run: bool,
        operation: &str,
    ) -> ServerResult<Value> {
        // Extract the content field if it exists
        let content = result
            .get("content")
            .ok_or_else(|| ServerError::internal("Handler response missing 'content' field"))?;

        // Check if it's already a RefactorPlan or ExecutionResult
        let response = if dry_run {
            // Parse the RefactorPlan to extract metadata
            self.parse_plan_response(content, operation)?
        } else {
            // Parse the ExecutionResult
            self.parse_execution_response(content, operation)?
        };

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
                &plan_data
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

    fn is_internal(&self) -> bool {
        false // Public tool
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
    /// Line number (for inline)
    #[serde(default)]
    line: Option<u32>,
    /// Character offset (for inline)
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
    /// 1-based start line
    start_line: u32,
    /// 0-based start character
    start_character: u32,
    /// 1-based end line
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
}

impl Default for RefactorOptions {
    fn default() -> Self {
        Self { dry_run: true }
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
