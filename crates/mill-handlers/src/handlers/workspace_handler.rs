//! Unified workspace operations tool handler
//!
//! Handles: workspace
//!
//! This handler implements the unified workspace tool for package management
//! and workspace operations. It dispatches to existing handlers based on action:
//! - create_package -> WorkspaceCreateService logic
//! - extract_dependencies -> WorkspaceExtractService logic
//! - find_replace -> find_replace service
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
pub struct WorkspaceHandler;

impl WorkspaceHandler {
    pub fn new() -> Self {
        Self
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
            "update_members" => self.handle_update_members(context, args).await,
            _ => Err(ServerError::invalid_request(format!(
                "Unknown workspace action: {}. Valid actions: create_package, extract_dependencies, find_replace, verify_project, update_members",
                action
            ))),
        }
    }
}

impl WorkspaceHandler {
    /// Handle create_package action - delegates to create package service
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

        // Map params fields to the expected format for create package service
        // Accept multiple field names for flexibility
        if let Some(pkg_path) = params.get("packagePath").and_then(|v| v.as_str()) {
            create_args["packagePath"] = json!(pkg_path);
        } else if let Some(path) = params.get("path").and_then(|v| v.as_str()) {
            create_args["packagePath"] = json!(path);
        } else if let Some(name) = params.get("name").and_then(|v| v.as_str()) {
            create_args["packagePath"] = json!(name);
        }
        if let Some(pkg_type) = params.get("packageType").and_then(|v| v.as_str()) {
            create_args["packageType"] = json!(pkg_type);
        } else if let Some(template) = params.get("template").and_then(|v| v.as_str()) {
            create_args["packageType"] = json!(template);
        }

        // Add options
        create_args["options"] = json!(options);

        // Create a new tool call for the existing handler
        // Delegate to create package service
        let result =
            super::tools::workspace_create::handle_create_package(context, create_args).await?;

        // Convert to WriteResponse format
        self.convert_create_package_response(result, &options).await
    }

    /// Handle extract_dependencies action - delegates to extract service
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
        // Delegate to extract dependencies service
        let result =
            super::tools::workspace_extract::handle_extract_dependencies(context, extract_args)
                .await?;

        // Convert to WriteResponse format
        self.convert_extract_deps_response(result, &options).await
    }

    /// Handle find_replace action - delegates to find_replace service
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
        if let Some(whole_word) = params.get("wholeWord") {
            find_replace_args["wholeWord"] = whole_word.clone();
        }
        if let Some(preserve_case) = params.get("preserveCase") {
            find_replace_args["preserveCase"] = preserve_case.clone();
        }
        if let Some(scope) = params.get("scope") {
            find_replace_args["scope"] = scope.clone();
        }

        // Add dryRun from options
        if let Some(dry_run) = options.get("dryRun") {
            find_replace_args["dryRun"] = dry_run.clone();
        } else {
            find_replace_args["dryRun"] = json!(true); // Default to safe mode
        }

        // Delegate to service
        let result = super::workspace::handle_find_replace(context, find_replace_args).await?;

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

    /// Handle update_members action - add/remove/list workspace members
    async fn handle_update_members(
        &self,
        context: &mill_handler_api::ToolHandlerContext,
        args: &Value,
    ) -> ServerResult<Value> {
        use std::fs;
        use std::path::Path;
        use toml_edit::{DocumentMut, Item};

        debug!("Handling workspace update_members action");

        // Extract params from nested structure
        let params = args
            .get("params")
            .ok_or_else(|| ServerError::invalid_request("Missing 'params' for update_members"))?;

        let options = args
            .get("options")
            .and_then(|v| v.as_object())
            .cloned()
            .unwrap_or_default();

        let dry_run = options
            .get("dryRun")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let create_if_missing = options
            .get("createIfMissing")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // Get the sub-action (add, remove, list)
        let sub_action = params
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                ServerError::invalid_request("Missing 'action' in params (add/remove/list)")
            })?;

        // Get workspace manifest path
        let manifest_path = params
            .get("workspaceManifest")
            .and_then(|v| v.as_str())
            .map(|path_str| {
                if Path::new(path_str).is_absolute() {
                    path_str.to_string()
                } else {
                    let concrete_state = get_concrete_app_state(&context.app_state).ok();
                    concrete_state
                        .map(|state| state.project_root.join(path_str).display().to_string())
                        .unwrap_or_else(|| path_str.to_string())
                }
            })
            .ok_or_else(|| ServerError::invalid_request("Missing 'workspaceManifest' path"))?;

        // Read and parse Cargo.toml
        let cargo_content = fs::read_to_string(&manifest_path).map_err(|e| {
            ServerError::invalid_request(format!(
                "Failed to read workspace manifest '{}': {}",
                manifest_path, e
            ))
        })?;

        let mut doc = cargo_content.parse::<DocumentMut>().map_err(|e| {
            ServerError::invalid_request(format!("Failed to parse workspace manifest: {}", e))
        })?;

        // Check if workspace section exists
        let has_workspace_section = doc.get("workspace").is_some();

        if !has_workspace_section && !create_if_missing {
            return Err(ServerError::invalid_request(
                "Cargo.toml does not contain a [workspace] section. Use createIfMissing: true to create it."
            ));
        }

        // Create workspace section if needed and allowed
        if !has_workspace_section && create_if_missing {
            let mut workspace_table = toml_edit::Table::new();
            workspace_table.insert(
                "members",
                Item::Value(toml_edit::Value::Array(toml_edit::Array::new())),
            );
            doc.insert("workspace", Item::Table(workspace_table));
        }

        // Get current members
        let members_before: Vec<String> = doc
            .get("workspace")
            .and_then(|w| w.get("members"))
            .and_then(|m| m.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        let (members_after, changes_made, workspace_updated) = match sub_action {
            "add" => {
                let new_members: Vec<String> = params
                    .get("members")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| {
                                v.as_str().map(|s| {
                                    // Normalize path separators (backslash to forward slash)
                                    s.replace('\\', "/")
                                })
                            })
                            .collect()
                    })
                    .unwrap_or_default();

                let mut members_after = members_before.clone();
                let mut added = 0;
                for member in &new_members {
                    if !members_after.contains(member) {
                        members_after.push(member.clone());
                        added += 1;
                    }
                }

                if added > 0 && !dry_run {
                    // Update the document
                    let members_array = members_after
                        .iter()
                        .map(|s| toml_edit::Value::from(s.as_str()))
                        .collect::<toml_edit::Array>();

                    if let Some(workspace) = doc.get_mut("workspace") {
                        workspace["members"] = Item::Value(toml_edit::Value::Array(members_array));
                    }

                    fs::write(&manifest_path, doc.to_string()).map_err(|e| {
                        ServerError::invalid_request(format!(
                            "Failed to write workspace manifest: {}",
                            e
                        ))
                    })?;
                }

                (members_after, added, added > 0)
            }
            "remove" => {
                let remove_members: Vec<String> = params
                    .get("members")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect()
                    })
                    .unwrap_or_default();

                let members_after: Vec<String> = members_before
                    .iter()
                    .filter(|m| !remove_members.contains(m))
                    .cloned()
                    .collect();

                let removed = members_before.len() - members_after.len();

                if removed > 0 && !dry_run {
                    // Update the document
                    let members_array = members_after
                        .iter()
                        .map(|s| toml_edit::Value::from(s.as_str()))
                        .collect::<toml_edit::Array>();

                    if let Some(workspace) = doc.get_mut("workspace") {
                        workspace["members"] = Item::Value(toml_edit::Value::Array(members_array));
                    }

                    fs::write(&manifest_path, doc.to_string()).map_err(|e| {
                        ServerError::invalid_request(format!(
                            "Failed to write workspace manifest: {}",
                            e
                        ))
                    })?;
                }

                (members_after, removed, removed > 0)
            }
            "list" => (members_before.clone(), 0, false),
            _ => {
                return Err(ServerError::invalid_request(format!(
                    "Invalid update_members action: {}. Valid: add, remove, list",
                    sub_action
                )));
            }
        };

        let summary = match sub_action {
            "add" => {
                if dry_run {
                    format!("Preview: Would add {} members", changes_made)
                } else {
                    format!("Added {} members to workspace", changes_made)
                }
            }
            "remove" => {
                if dry_run {
                    format!("Preview: Would remove {} members", changes_made)
                } else {
                    format!("Removed {} members from workspace", changes_made)
                }
            }
            "list" => format!("Workspace has {} members", members_before.len()),
            _ => "Unknown operation".to_string(),
        };

        let response = json!({
            "status": if dry_run { "preview" } else { "success" },
            "summary": summary,
            "filesChanged": if workspace_updated && !dry_run { vec![manifest_path] } else { vec![] as Vec<String> },
            "diagnostics": [],
            "result": {
                "action": sub_action,
                "membersBefore": members_before,
                "membersAfter": members_after,
                "changesMade": changes_made,
                "workspaceUpdated": workspace_updated && !dry_run,
                "dryRun": dry_run
            }
        });

        Ok(response)
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
