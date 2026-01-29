//! Internal workspace tool handlers
//!
//! Handles tools that are used by workflows/backend but should not be
//! exposed to AI agents via MCP.
//!
//! Tools: apply_workspace_edit

use super::ToolHandler;
use async_trait::async_trait;
use mill_foundation::core::model::mcp::ToolCall;
use mill_foundation::errors::{MillError as ServerError, MillResult as ServerResult};
use serde_json::Value;

pub struct InternalWorkspaceHandler;

impl InternalWorkspaceHandler {
    pub fn new() -> Self {
        Self
    }

    /// Handle apply_workspace_edit tool call
    /// Applies LSP workspace edits (multi-file refactoring operations) through the MCP protocol
    async fn handle_apply_workspace_edit(
        &self,
        context: &mill_handler_api::ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        use serde_json::json;

        // Extract parameters
        let args = tool_call
            .arguments
            .as_ref()
            .and_then(|v| v.as_object())
            .ok_or_else(|| ServerError::invalid_request("Arguments must be an object"))?;

        let changes = args
            .get("changes")
            .and_then(|v| v.as_object())
            .ok_or_else(|| ServerError::invalid_request("Missing required parameter: changes"))?;

        let dry_run = args
            .get("dryRun")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // Convert changes map to Vec<TextEdit>
        let mut all_edits = Vec::new();
        for (file_path, edits_value) in changes {
            let edits_array = edits_value
                .as_array()
                .ok_or_else(|| ServerError::invalid_request("Edits must be an array"))?;

            for edit_value in edits_array {
                let range = edit_value
                    .get("range")
                    .ok_or_else(|| ServerError::invalid_request("Edit missing range"))?;

                let start_line = range["start"]["line"]
                    .as_u64()
                    .ok_or_else(|| ServerError::invalid_request("Invalid start line"))?
                    as u32;
                let start_char = range["start"]["character"]
                    .as_u64()
                    .ok_or_else(|| ServerError::invalid_request("Invalid start character"))?
                    as u32;
                let end_line = range["end"]["line"]
                    .as_u64()
                    .ok_or_else(|| ServerError::invalid_request("Invalid end line"))?
                    as u32;
                let end_char = range["end"]["character"]
                    .as_u64()
                    .ok_or_else(|| ServerError::invalid_request("Invalid end character"))?
                    as u32;

                let new_text = edit_value
                    .get("newText")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| ServerError::invalid_request("Edit missing newText"))?
                    .to_string();

                all_edits.push(mill_foundation::protocol::TextEdit {
                    file_path: Some(file_path.clone()),
                    edit_type: mill_foundation::protocol::EditType::Replace,
                    location: mill_foundation::protocol::EditLocation {
                        start_line,
                        start_column: start_char,
                        end_line,
                        end_column: end_char,
                    },
                    original_text: String::new(),
                    new_text,
                    priority: 0,
                    description: format!("Workspace edit in {}", file_path),
                });
            }
        }

        // Create EditPlan
        let plan = mill_foundation::planning::EditPlan {
            source_file: String::new(), // Multi-file workspace edit
            edits: all_edits,
            dependency_updates: Vec::new(),
            validations: Vec::new(),
            metadata: mill_foundation::planning::EditPlanMetadata {
                intent_name: "apply_workspace_edit".to_string(),
                intent_arguments: serde_json::Value::Object(args.clone()),
                created_at: chrono::Utc::now(),
                complexity: 1,
                impact_areas: vec!["workspace".to_string()],
                consolidation: None,
            },
        };

        // Apply edits or preview
        if dry_run {
            // Dry run mode - just return what would be modified
            let files_to_modify: Vec<String> = plan
                .edits
                .iter()
                .filter_map(|edit| edit.file_path.clone())
                .collect::<std::collections::HashSet<_>>()
                .into_iter()
                .collect();

            Ok(json!({
                "applied": false,
                "files_modified": files_to_modify,
            }))
        } else {
            // Actually apply the edits
            let result = context
                .app_state
                .file_service
                .apply_edit_plan(&plan)
                .await?;

            Ok(json!({
                "applied": true,
                "files_modified": result.modified_files,
            }))
        }
    }
}

#[async_trait]
impl ToolHandler for InternalWorkspaceHandler {
    fn tool_names(&self) -> &[&str] {
        &["apply_workspace_edit"]
    }

    async fn handle_tool_call(
        &self,
        context: &mill_handler_api::ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        match tool_call.name.as_str() {
            "apply_workspace_edit" => self.handle_apply_workspace_edit(context, tool_call).await,
            _ => Err(ServerError::invalid_request(format!(
                "Unknown internal workspace tool: {}",
                tool_call.name
            ))),
        }
    }
}
