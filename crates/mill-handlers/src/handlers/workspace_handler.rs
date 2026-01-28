//! Unified workspace operations tool handler
//!
//! Handles: workspace
//!
//! This handler implements the unified workspace tool for package management
//! and workspace operations. It dispatches to existing handlers based on action:
//! - create_package -> WorkspaceCreateHandler logic
//! - extract_dependencies -> WorkspaceExtractDepsHandler logic
//! - find_replace -> FindReplaceHandler logic
//! - verify_project -> health_check style verification

use super::tools::{extensions::get_concrete_app_state, ToolHandler};
use async_trait::async_trait;
use mill_foundation::core::model::mcp::ToolCall;
use mill_foundation::errors::{MillError as ServerError, MillResult as ServerResult};
use serde_json::{json, Value};
use tracing::{debug, info};

use crate::handlers::tool_definitions::{
    Diagnostic, DiagnosticSeverity, WriteResponse, WriteStatus,
};

/// Handler for unified workspace operations
pub struct WorkspaceHandler {
    workspace_create_handler: super::tools::workspace_create::WorkspaceCreateHandler,
    workspace_extract_deps_handler:
        super::tools::workspace_extract_deps::WorkspaceExtractDepsHandler,
    find_replace_handler: super::workspace::find_replace_handler::FindReplaceHandler,
}

impl WorkspaceHandler {
    pub fn new() -> Self {
        Self {
            workspace_create_handler: super::tools::workspace_create::WorkspaceCreateHandler::new(),
            workspace_extract_deps_handler:
                super::tools::workspace_extract_deps::WorkspaceExtractDepsHandler::new(),
            find_replace_handler: super::workspace::find_replace_handler::FindReplaceHandler::new(),
        }
    }
}

impl Default for WorkspaceHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ToolHandler for WorkspaceHandler {
    fn tool_names(&self) -> &[&str] {
        &["workspace"]
    }

    async fn handle_tool_call(
        &self,
        context: &mill_handler_api::ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        debug!(tool_name = %tool_call.name, "Handling unified workspace operation");

        // Parse the action from arguments
        let args = tool_call
            .arguments
            .as_ref()
            .ok_or_else(|| ServerError::invalid_request("Missing arguments"))?;

        let action = args
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ServerError::invalid_request("Missing 'action' parameter"))?;

        debug!(action = %action, "Dispatching workspace action");

        match action {
            "create_package" => self.handle_create_package(context, args).await,
            "extract_dependencies" => self.handle_extract_dependencies(context, args).await,
            "find_replace" => self.handle_find_replace(context, args).await,
            "verify_project" => self.handle_verify_project(context).await,
            _ => Err(ServerError::invalid_request(format!(
                "Unknown workspace action: {}",
                action
            ))),
        }
    }
}

impl WorkspaceHandler {
    /// Handle create_package action - delegates to WorkspaceCreateHandler
    async fn handle_create_package(
        &self,
        context: &mill_handler_api::ToolHandlerContext,
        args: &Value,
    ) -> ServerResult<Value> {
        debug!("Handling workspace create_package action");

        // Extract params from nested structure
        let params = args
            .get("params")
            .ok_or_else(|| ServerError::invalid_request("Missing 'params' for create_package"))?;

        let options = args
            .get("options")
            .and_then(|v| v.as_object())
            .cloned()
            .unwrap_or_default();

        // Build arguments for the existing handler
        let mut create_args = json!({});

        // Map params fields to the expected format for WorkspaceCreateHandler
        if let Some(name) = params.get("name").and_then(|v| v.as_str()) {
            create_args["packagePath"] = json!(name);
        }
        if let Some(path) = params.get("path").and_then(|v| v.as_str()) {
            create_args["packagePath"] = json!(path);
        }
        if let Some(template) = params.get("template").and_then(|v| v.as_str()) {
            create_args["packageType"] = json!(template);
        }

        // Add options
        create_args["options"] = json!(options);

        // Create a new tool call for the existing handler
        let delegate_tool_call = ToolCall {
            name: "workspace.create_package".to_string(),
            arguments: Some(create_args),
        };

        // Delegate to existing handler
        let result = self
            .workspace_create_handler
            .handle_tool_call(context, &delegate_tool_call)
            .await?;

        // Convert to WriteResponse format
        self.convert_create_package_response(result, &options).await
    }

    /// Handle extract_dependencies action - delegates to WorkspaceExtractDepsHandler
    async fn handle_extract_dependencies(
        &self,
        context: &mill_handler_api::ToolHandlerContext,
        args: &Value,
    ) -> ServerResult<Value> {
        debug!("Handling workspace extract_dependencies action");

        // Extract params from nested structure
        let params = args.get("params").ok_or_else(|| {
            ServerError::invalid_request("Missing 'params' for extract_dependencies")
        })?;

        let options = args
            .get("options")
            .and_then(|v| v.as_object())
            .cloned()
            .unwrap_or_default();

        // Build arguments for the existing handler
        let mut extract_args = json!({});

        // Map params fields
        if let Some(source) = params.get("sourceManifest") {
            extract_args["sourceManifest"] = source.clone();
        }
        if let Some(target) = params.get("targetManifest") {
            extract_args["targetManifest"] = target.clone();
        }
        if let Some(deps) = params.get("dependencies") {
            extract_args["dependencies"] = deps.clone();
        }

        // Add options
        extract_args["options"] = json!(options);

        // Create a new tool call for the existing handler
        let delegate_tool_call = ToolCall {
            name: "workspace.extract_dependencies".to_string(),
            arguments: Some(extract_args),
        };

        // Delegate to existing handler
        let result = self
            .workspace_extract_deps_handler
            .handle_tool_call(context, &delegate_tool_call)
            .await?;

        // Convert to WriteResponse format
        self.convert_extract_deps_response(result, &options).await
    }

    /// Handle find_replace action - delegates to FindReplaceHandler
    async fn handle_find_replace(
        &self,
        context: &mill_handler_api::ToolHandlerContext,
        args: &Value,
    ) -> ServerResult<Value> {
        debug!("Handling workspace find_replace action");

        // Extract params from nested structure
        let params = args
            .get("params")
            .ok_or_else(|| ServerError::invalid_request("Missing 'params' for find_replace"))?;

        let options = args
            .get("options")
            .and_then(|v| v.as_object())
            .cloned()
            .unwrap_or_default();

        // Build arguments for the existing handler
        let mut find_replace_args = json!({});

        // Map params fields
        if let Some(pattern) = params.get("pattern") {
            find_replace_args["pattern"] = pattern.clone();
        }
        if let Some(replacement) = params.get("replacement") {
            find_replace_args["replacement"] = replacement.clone();
        }
        if let Some(mode) = params.get("mode") {
            find_replace_args["mode"] = mode.clone();
        }
        if let Some(glob) = params.get("glob") {
            // Map glob to scope.includePatterns
            find_replace_args["scope"] = json!({
                "includePatterns": [glob]
            });
        }

        // Add dryRun from options
        if let Some(dry_run) = options.get("dryRun") {
            find_replace_args["dryRun"] = dry_run.clone();
        } else {
            find_replace_args["dryRun"] = json!(true); // Default to safe mode
        }

        // Create a new tool call for the existing handler
        let delegate_tool_call = ToolCall {
            name: "workspace.find_replace".to_string(),
            arguments: Some(find_replace_args),
        };

        // Delegate to existing handler
        let result = self
            .find_replace_handler
            .handle_tool_call(context, &delegate_tool_call)
            .await?;

        // Convert to WriteResponse format
        self.convert_find_replace_response(result, &options).await
    }

    /// Handle verify_project action - similar to health_check
    async fn handle_verify_project(
        &self,
        context: &mill_handler_api::ToolHandlerContext,
    ) -> ServerResult<Value> {
        info!("Handling workspace verify_project action");

        let _concrete_state = get_concrete_app_state(&context.app_state)?;

        // Get plugin count from plugin manager
        let plugin_count = context
            .plugin_manager
            .get_all_tool_definitions()
            .await
            .len();

        // Get detailed metrics and statistics
        let metrics = context.plugin_manager.get_metrics().await;
        let stats = context.plugin_manager.get_registry_statistics().await;

        // Calculate success rate
        let success_rate = if metrics.total_requests > 0 {
            (metrics.successful_requests as f64 / metrics.total_requests as f64) * 100.0
        } else {
            100.0
        };

        // Determine overall status
        let status = if success_rate >= 95.0 {
            WriteStatus::Success
        } else if success_rate >= 75.0 {
            WriteStatus::Preview // Using Preview as "warning"
        } else {
            WriteStatus::Error
        };

        let summary = format!(
            "Project verified: {} plugins loaded, {:.1}% success rate",
            plugin_count, success_rate
        );

        let response = WriteResponse {
            status,
            summary,
            files_changed: vec![],
            diagnostics: vec![],
            changes: Some(json!({
                "plugins": {
                    "loaded": plugin_count,
                    "total_plugins": stats.total_plugins,
                    "supported_extensions": stats.supported_extensions,
                    "supported_methods": stats.supported_methods,
                },
                "metrics": {
                    "total_requests": metrics.total_requests,
                    "successful_requests": metrics.successful_requests,
                    "failed_requests": metrics.failed_requests,
                    "success_rate": format!("{:.2}%", success_rate),
                    "average_processing_time_ms": metrics.average_processing_time_ms,
                }
            })),
        };

        Ok(serde_json::to_value(response)?)
    }

    /// Convert create_package result to WriteResponse
    async fn convert_create_package_response(
        &self,
        result: Value,
        options: &serde_json::Map<String, Value>,
    ) -> ServerResult<Value> {
        let dry_run = options
            .get("dryRun")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        // Parse the original result
        let created_files = result
            .get("createdFiles")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let package_name = result
            .get("packageInfo")
            .and_then(|pi| pi.get("name"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        let summary = if dry_run {
            format!(
                "Preview: Would create package '{}' with {} files",
                package_name,
                created_files.len()
            )
        } else {
            format!(
                "Created package '{}' with {} files",
                package_name,
                created_files.len()
            )
        };

        let status = if dry_run {
            WriteStatus::Preview
        } else {
            WriteStatus::Success
        };

        let response = WriteResponse {
            status,
            summary,
            files_changed: created_files,
            diagnostics: vec![],
            changes: Some(result),
        };

        Ok(serde_json::to_value(response)?)
    }

    /// Convert extract_dependencies result to WriteResponse
    async fn convert_extract_deps_response(
        &self,
        result: Value,
        options: &serde_json::Map<String, Value>,
    ) -> ServerResult<Value> {
        let dry_run = options
            .get("dryRun")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let deps_extracted = result
            .get("dependenciesExtracted")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        let target_updated = result
            .get("targetManifestUpdated")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let warnings = result
            .get("warnings")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(|msg| Diagnostic {
                        severity: DiagnosticSeverity::Warning,
                        message: msg.to_string(),
                        file_path: None,
                        line: None,
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let summary = if dry_run {
            format!("Preview: Would extract {} dependencies", deps_extracted)
        } else if target_updated {
            format!("Extracted {} dependencies", deps_extracted)
        } else {
            format!("No dependencies extracted (already present or not found)")
        };

        let status = if dry_run {
            WriteStatus::Preview
        } else if target_updated {
            WriteStatus::Success
        } else {
            WriteStatus::Success // Still success, just nothing to do
        };

        let files_changed = if target_updated && !dry_run {
            // Extract target manifest path from result if available
            vec!["Cargo.toml".to_string()] // Simplified - in real usage would extract actual path
        } else {
            vec![]
        };

        let response = WriteResponse {
            status,
            summary,
            files_changed,
            diagnostics: warnings,
            changes: Some(result),
        };

        Ok(serde_json::to_value(response)?)
    }

    /// Convert find_replace result to WriteResponse
    async fn convert_find_replace_response(
        &self,
        result: Value,
        _options: &serde_json::Map<String, Value>,
    ) -> ServerResult<Value> {
        // Check if this is an EditPlan (dry run) or ApplyResult (executed)
        if result.get("edits").is_some() {
            // This is an EditPlan (preview mode)
            let edits = result
                .get("edits")
                .and_then(|v| v.as_array())
                .map(|arr| arr.len())
                .unwrap_or(0);

            // Extract unique file paths
            let files_changed = result
                .get("edits")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    let mut files = std::collections::HashSet::new();
                    for edit in arr {
                        if let Some(file_path) = edit.get("filePath").and_then(|v| v.as_str()) {
                            files.insert(file_path.to_string());
                        }
                    }
                    files.into_iter().collect::<Vec<_>>()
                })
                .unwrap_or_default();

            let summary = format!(
                "Preview: Would replace {} matches in {} files",
                edits,
                files_changed.len()
            );

            let response = WriteResponse {
                status: WriteStatus::Preview,
                summary,
                files_changed,
                diagnostics: vec![],
                changes: Some(result),
            };

            Ok(serde_json::to_value(response)?)
        } else {
            // This is an ApplyResult (execution mode)
            let files_modified = result
                .get("filesModified")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();

            let matches_replaced = result
                .get("matchesReplaced")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);

            let summary = format!(
                "Replaced {} matches in {} files",
                matches_replaced,
                files_modified.len()
            );

            let response = WriteResponse {
                status: WriteStatus::Success,
                summary,
                files_changed: files_modified,
                diagnostics: vec![],
                changes: Some(result),
            };

            Ok(serde_json::to_value(response)?)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handler_tool_names() {
        let handler = WorkspaceHandler::new();
        assert_eq!(handler.tool_names(), &["workspace"]);
    }
}
