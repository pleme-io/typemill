//! Relocate handler for Magnificent Seven refactoring API
//!
//! Implements the `relocate` tool that wraps the existing MoveHandler functionality
//! with the new unified API shape. This handler delegates to the legacy MoveHandler
//! while providing the standardized WriteResponse envelope.
//!
//! # Tool Overview
//!
//! The `relocate` tool moves symbols, files, or directories to new locations with
//! automatic import and reference updates across the codebase.
//!
//! # API Format
//!
//! ```json
//! {
//!   "target": {
//!     "kind": "symbol" | "file" | "directory",
//!     "filePath": "path/to/file",
//!     "line": 10,        // Required for symbol moves
//!     "character": 5     // Required for symbol moves
//!   },
//!   "destination": "path/to/destination",
//!   "options": {
//!     "dryRun": true  // Default: true (preview mode)
//!   }
//! }
//! ```
//!
//! # Response Format
//!
//! Returns a `WriteResponse` envelope with:
//! - `status`: "success" | "error" | "preview"
//! - `summary`: Human-readable description
//! - `filesChanged`: List of affected file paths
//! - `diagnostics`: Warnings or errors
//! - `changes`: Optional plan or result details
//!
//! # Examples
//!
//! ## Symbol Move (Preview)
//! ```json
//! {
//!   "target": {"kind": "symbol", "filePath": "src/app.rs", "line": 10, "character": 5},
//!   "destination": "src/utils.rs",
//!   "options": {"dryRun": true}
//! }
//! ```
//!
//! ## File Move (Execute)
//! ```json
//! {
//!   "target": {"kind": "file", "filePath": "src/old.rs"},
//!   "destination": "src/new.rs",
//!   "options": {"dryRun": false}
//! }
//! ```
//!
//! ## Directory Move (Preview)
//! ```json
//! {
//!   "target": {"kind": "directory", "filePath": "src/old-dir"},
//!   "destination": "src/new-dir",
//!   "options": {"dryRun": true}
//! }
//! ```

use crate::handlers::r#move::MoveHandler;
use crate::handlers::tool_definitions::{Diagnostic, DiagnosticSeverity, WriteResponse};
use crate::handlers::tools::ToolHandler;
use async_trait::async_trait;
use mill_foundation::core::model::mcp::ToolCall;
use mill_foundation::errors::{MillError as ServerError, MillResult as ServerResult};
use serde::Deserialize;
use serde_json::{json, Value};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Handler for the `relocate` tool
///
/// This handler wraps the existing MoveHandler functionality and adapts it to
/// the new Magnificent Seven API format with WriteResponse envelopes.
pub struct RelocateHandler {
    /// Internal delegate to the existing move handler
    move_handler: MoveHandler,
}

impl RelocateHandler {
    pub fn new() -> Self {
        Self {
            move_handler: MoveHandler::new(),
        }
    }
}

impl Default for RelocateHandler {
    fn default() -> Self {
        Self::new()
    }
}

/// Input parameters for the relocate tool
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RelocateParams {
    /// The target to move
    target: RelocateTarget,
    /// Destination path
    destination: String,
    /// Options for the move operation
    #[serde(default)]
    options: RelocateOptions,
}

/// Target specification for relocation
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RelocateTarget {
    /// Kind of target: "symbol", "file", or "directory"
    kind: String,
    /// Path to the file or directory (or file containing the symbol)
    file_path: String,
    /// 1-based line number (required for symbol moves)
    #[serde(default)]
    line: Option<u32>,
    /// 0-based character offset (required for symbol moves)
    #[serde(default)]
    character: Option<u32>,
}

/// Options for relocation operations
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RelocateOptions {
    /// Preview mode - don't actually apply changes (default: true for safety)
    #[serde(default = "crate::default_true")]
    dry_run: bool,
}

impl Default for RelocateOptions {
    fn default() -> Self {
        Self { dry_run: true }
    }
}

#[async_trait]
impl ToolHandler for RelocateHandler {
    fn tool_names(&self) -> &[&str] {
        &["relocate"]
    }

    fn is_internal(&self) -> bool {
        false // Public tool in Magnificent Seven API
    }

    async fn handle_tool_call(
        &self,
        context: &mill_handler_api::ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        let operation_id = Uuid::new_v4().to_string();

        info!(
            operation_id = %operation_id,
            tool_name = %tool_call.name,
            "Starting relocate operation"
        );

        // Parse parameters
        let args = tool_call.arguments.clone().ok_or_else(|| {
            error!(
                operation_id = %operation_id,
                "Missing arguments for relocate"
            );
            ServerError::invalid_request("Missing arguments for relocate")
        })?;

        let params: RelocateParams = serde_json::from_value(args.clone()).map_err(|e| {
            error!(
                operation_id = %operation_id,
                error = %e,
                arguments = ?args,
                "Failed to parse relocate parameters"
            );
            ServerError::invalid_request(format!("Invalid relocate parameters: {}", e))
        })?;

        info!(
            operation_id = %operation_id,
            kind = %params.target.kind,
            source_path = %params.target.file_path,
            destination = %params.destination,
            dry_run = params.options.dry_run,
            "Parsed relocate parameters"
        );

        // Validate parameters based on target kind
        if params.target.kind == "symbol" {
            if params.target.line.is_none() || params.target.character.is_none() {
                error!(
                    operation_id = %operation_id,
                    "Symbol move requires line and character position"
                );
                return Err(ServerError::invalid_request(
                    "Symbol move requires 'line' and 'character' fields in target",
                ));
            }
        }

        // Convert to legacy move handler format
        let legacy_args = self.convert_to_legacy_format(&params, &operation_id)?;

        // Create a ToolCall for the legacy handler
        let legacy_tool_call = ToolCall {
            name: "move".to_string(),
            arguments: Some(legacy_args),
        };

        // Call the legacy move handler
        debug!(
            operation_id = %operation_id,
            "Delegating to legacy MoveHandler"
        );

        let legacy_result = self
            .move_handler
            .handle_tool_call(context, &legacy_tool_call)
            .await;

        // Convert legacy response to WriteResponse format
        self.convert_legacy_response(legacy_result, &params, &operation_id)
    }
}

impl RelocateHandler {
    /// Convert new API parameters to legacy MoveHandler format
    fn convert_to_legacy_format(
        &self,
        params: &RelocateParams,
        operation_id: &str,
    ) -> ServerResult<Value> {
        debug!(
            operation_id = %operation_id,
            "Converting relocate parameters to legacy move format"
        );

        let mut legacy_target = json!({
            "kind": params.target.kind,
            "path": params.target.file_path,
        });

        // Add selector for symbol moves
        if params.target.kind == "symbol" {
            if let (Some(line), Some(character)) = (params.target.line, params.target.character) {
                legacy_target["selector"] = json!({
                    "position": {
                        "line": line - 1, // Convert from 1-based to 0-based
                        "character": character
                    }
                });
            }
        }

        Ok(json!({
            "target": legacy_target,
            "destination": params.destination,
            "options": {
                "dry_run": params.options.dry_run
            }
        }))
    }

    /// Convert legacy MoveHandler response to WriteResponse format
    fn convert_legacy_response(
        &self,
        legacy_result: ServerResult<Value>,
        params: &RelocateParams,
        operation_id: &str,
    ) -> ServerResult<Value> {
        match legacy_result {
            Ok(legacy_value) => {
                debug!(
                    operation_id = %operation_id,
                    "Successfully received legacy move result"
                );

                // Extract the content from legacy response
                let content = legacy_value.get("content").ok_or_else(|| {
                    error!(
                        operation_id = %operation_id,
                        "Legacy response missing 'content' field"
                    );
                    ServerError::internal("Legacy response missing content")
                })?;

                // Determine if this is a preview or execution result
                let response = if params.options.dry_run {
                    self.build_preview_response(content, params, operation_id)?
                } else {
                    self.build_execution_response(content, params, operation_id)?
                };

                let response_json = serde_json::to_value(&response).map_err(|e| {
                    error!(
                        operation_id = %operation_id,
                        error = %e,
                        "Failed to serialize WriteResponse"
                    );
                    ServerError::internal(format!("Failed to serialize response: {}", e))
                })?;

                info!(
                    operation_id = %operation_id,
                    status = ?response.status,
                    files_changed = response.files_changed.len(),
                    "Relocate operation completed successfully"
                );

                Ok(response_json)
            }
            Err(e) => {
                error!(
                    operation_id = %operation_id,
                    error = %e,
                    "Legacy move handler failed"
                );

                let error_response = WriteResponse::error(
                    format!("Failed to relocate {}: {}", params.target.kind, e),
                    vec![Diagnostic {
                        severity: DiagnosticSeverity::Error,
                        message: e.to_string(),
                        file_path: Some(params.target.file_path.clone()),
                        line: params.target.line,
                    }],
                );

                let response_json = serde_json::to_value(&error_response).map_err(|e| {
                    error!(
                        operation_id = %operation_id,
                        error = %e,
                        "Failed to serialize error response"
                    );
                    ServerError::internal(format!("Failed to serialize error response: {}", e))
                })?;

                Ok(response_json)
            }
        }
    }

    /// Build a preview response from the move plan
    fn build_preview_response(
        &self,
        content: &Value,
        params: &RelocateParams,
        operation_id: &str,
    ) -> ServerResult<WriteResponse> {
        debug!(
            operation_id = %operation_id,
            "Building preview response from move plan"
        );

        // Try to parse as RefactorPlan::MovePlan
        let plan_type = content
            .get("type")
            .and_then(|t| t.as_str())
            .unwrap_or("unknown");

        if plan_type != "MovePlan" {
            warn!(
                operation_id = %operation_id,
                plan_type = %plan_type,
                "Unexpected plan type in preview response"
            );
        }

        // Extract affected files from the plan
        let affected_files = self.extract_affected_files(content, operation_id);
        let warnings = self.extract_warnings(content, operation_id);

        let summary = format!(
            "Preview: Would move {} from '{}' to '{}' (affects {} file(s))",
            params.target.kind,
            params.target.file_path,
            params.destination,
            affected_files.len()
        );

        let mut response = WriteResponse::preview(summary, affected_files, content.clone());

        // Add warnings as diagnostics
        for warning in warnings {
            response = response.with_warning(warning);
        }

        Ok(response)
    }

    /// Build an execution response from the execution result
    fn build_execution_response(
        &self,
        content: &Value,
        params: &RelocateParams,
        operation_id: &str,
    ) -> ServerResult<WriteResponse> {
        debug!(
            operation_id = %operation_id,
            "Building execution response from move result"
        );

        // Extract applied files from execution result
        let applied_files = content
            .get("applied_files")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let success = content
            .get("success")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        if success {
            let summary = format!(
                "Successfully moved {} from '{}' to '{}' ({} file(s) modified)",
                params.target.kind,
                params.target.file_path,
                params.destination,
                applied_files.len()
            );

            let mut response = WriteResponse::success(summary, applied_files);

            // Add any warnings from execution
            let warnings = self.extract_warnings(content, operation_id);
            for warning in warnings {
                response = response.with_warning(warning);
            }

            Ok(response)
        } else {
            let error_msg = content
                .get("error")
                .and_then(|v| v.as_str())
                .unwrap_or("Move operation failed");

            warn!(
                operation_id = %operation_id,
                error = %error_msg,
                "Move execution reported failure"
            );

            Ok(WriteResponse::error(
                format!("Failed to move {}: {}", params.target.kind, error_msg),
                vec![Diagnostic {
                    severity: DiagnosticSeverity::Error,
                    message: error_msg.to_string(),
                    file_path: Some(params.target.file_path.clone()),
                    line: params.target.line,
                }],
            ))
        }
    }

    /// Extract affected file paths from a plan or result
    fn extract_affected_files(&self, content: &Value, operation_id: &str) -> Vec<String> {
        // Try to extract from MovePlan structure
        if let Some(edits) = content.get("edits") {
            if let Some(changes) = edits.get("changes") {
                if let Some(obj) = changes.as_object() {
                    return obj.keys().map(|k| k.to_string()).collect();
                }
            }
            if let Some(doc_changes) = edits.get("documentChanges") {
                if let Some(arr) = doc_changes.as_array() {
                    return arr
                        .iter()
                        .filter_map(|v| {
                            v.get("textDocument")
                                .and_then(|td| td.get("uri"))
                                .and_then(|uri| uri.as_str())
                                .map(String::from)
                        })
                        .collect();
                }
            }
        }

        // Try summary.affected_files
        if let Some(summary) = content.get("summary") {
            if let Some(affected) = summary.get("affected_files").and_then(|v| v.as_u64()) {
                debug!(
                    operation_id = %operation_id,
                    affected_files = affected,
                    "Found affected_files count in summary, but no file paths available"
                );
            }
        }

        Vec::new()
    }

    /// Extract warning messages from a plan or result
    fn extract_warnings(&self, content: &Value, operation_id: &str) -> Vec<String> {
        let warnings = content
            .get("warnings")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|w| w.get("message").and_then(|m| m.as_str()).map(String::from))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        if !warnings.is_empty() {
            debug!(
                operation_id = %operation_id,
                warnings_count = warnings.len(),
                "Extracted warnings from response"
            );
        }

        warnings
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_relocate_handler_tool_names() {
        let handler = RelocateHandler::new();
        assert_eq!(handler.tool_names(), &["relocate"]);
    }

    #[test]
    fn test_relocate_handler_is_public() {
        let handler = RelocateHandler::new();
        assert!(!handler.is_internal());
    }

    #[test]
    fn test_relocate_params_deserialization() {
        let json = json!({
            "target": {
                "kind": "file",
                "filePath": "src/old.rs"
            },
            "destination": "src/new.rs"
        });

        let params: RelocateParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.target.kind, "file");
        assert_eq!(params.target.file_path, "src/old.rs");
        assert_eq!(params.destination, "src/new.rs");
        assert!(params.options.dry_run); // Default is true
    }

    #[test]
    fn test_relocate_params_with_options() {
        let json = json!({
            "target": {
                "kind": "symbol",
                "filePath": "src/app.rs",
                "line": 10,
                "character": 5
            },
            "destination": "src/utils.rs",
            "options": {
                "dryRun": false
            }
        });

        let params: RelocateParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.target.kind, "symbol");
        assert_eq!(params.target.line, Some(10));
        assert_eq!(params.target.character, Some(5));
        assert!(!params.options.dry_run);
    }

    #[test]
    fn test_legacy_format_conversion_file() {
        let handler = RelocateHandler::new();
        let params = RelocateParams {
            target: RelocateTarget {
                kind: "file".to_string(),
                file_path: "src/old.rs".to_string(),
                line: None,
                character: None,
            },
            destination: "src/new.rs".to_string(),
            options: RelocateOptions { dry_run: true },
        };

        let legacy = handler
            .convert_to_legacy_format(&params, "test-op")
            .unwrap();
        assert_eq!(legacy["target"]["kind"], "file");
        assert_eq!(legacy["target"]["path"], "src/old.rs");
        assert_eq!(legacy["destination"], "src/new.rs");
        assert_eq!(legacy["options"]["dry_run"], true);
    }

    #[test]
    fn test_legacy_format_conversion_symbol() {
        let handler = RelocateHandler::new();
        let params = RelocateParams {
            target: RelocateTarget {
                kind: "symbol".to_string(),
                file_path: "src/app.rs".to_string(),
                line: Some(10),
                character: Some(5),
            },
            destination: "src/utils.rs".to_string(),
            options: RelocateOptions { dry_run: false },
        };

        let legacy = handler
            .convert_to_legacy_format(&params, "test-op")
            .unwrap();
        assert_eq!(legacy["target"]["kind"], "symbol");
        assert_eq!(legacy["target"]["path"], "src/app.rs");
        assert_eq!(legacy["target"]["selector"]["position"]["line"], 9); // Converted to 0-based
        assert_eq!(legacy["target"]["selector"]["position"]["character"], 5);
        assert_eq!(legacy["options"]["dry_run"], false);
    }
}
