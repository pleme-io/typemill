//! Internal workspace tool handlers
//!
//! Handles tools that are used by workflows/backend but should not be
//! exposed to AI agents via MCP.
//!
//! Tools: apply_workspace_edit

use super::{ToolHandler, ToolHandlerContext};
use async_trait::async_trait;
use codebuddy_foundation::core::model::mcp::ToolCall;
use codebuddy_foundation::protocol::{ApiError, ApiResult as ServerResult};
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
        context: &ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        use serde_json::json;

        // Extract parameters
        let args = tool_call
            .arguments
            .as_ref()
            .and_then(|v| v.as_object())
            .ok_or_else(|| ApiError::InvalidRequest("Arguments must be an object".to_string()))?;

        let changes = args
            .get("changes")
            .and_then(|v| v.as_object())
            .ok_or_else(|| {
                ApiError::InvalidRequest("Missing required parameter: changes".to_string())
            })?;

        let dry_run = args
            .get("dry_run")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // Convert changes map to Vec<TextEdit>
        let mut all_edits = Vec::new();
        for (file_path, edits_value) in changes {
            let edits_array = edits_value
                .as_array()
                .ok_or_else(|| ApiError::InvalidRequest("Edits must be an array".to_string()))?;

            for edit_value in edits_array {
                let range = edit_value
                    .get("range")
                    .ok_or_else(|| ApiError::InvalidRequest("Edit missing range".to_string()))?;

                let start_line = range["start"]["line"]
                    .as_u64()
                    .ok_or_else(|| ApiError::InvalidRequest("Invalid start line".to_string()))?
                    as u32;
                let start_char = range["start"]["character"].as_u64().ok_or_else(|| {
                    ApiError::InvalidRequest("Invalid start character".to_string())
                })? as u32;
                let end_line = range["end"]["line"]
                    .as_u64()
                    .ok_or_else(|| ApiError::InvalidRequest("Invalid end line".to_string()))?
                    as u32;
                let end_char = range["end"]["character"]
                    .as_u64()
                    .ok_or_else(|| ApiError::InvalidRequest("Invalid end character".to_string()))?
                    as u32;

                let new_text = edit_value
                    .get("newText")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| ApiError::InvalidRequest("Edit missing newText".to_string()))?
                    .to_string();

                all_edits.push(codebuddy_foundation::protocol::TextEdit {
                    file_path: Some(file_path.clone()),
                    edit_type: codebuddy_foundation::protocol::EditType::Replace,
                    location: codebuddy_foundation::protocol::EditLocation {
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
        let plan = codebuddy_foundation::protocol::EditPlan {
            source_file: String::new(), // Multi-file workspace edit
            edits: all_edits,
            dependency_updates: Vec::new(),
            validations: Vec::new(),
            metadata: codebuddy_foundation::protocol::EditPlanMetadata {
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

    fn is_internal(&self) -> bool {
        // This tool is internal - it's used by the workflow planner to apply
        // LSP workspace edits. AI agents should use high-level tools like
        // rename.plan which internally may trigger workspace edits.
        true
    }

    async fn handle_tool_call(
        &self,
        context: &ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        match tool_call.name.as_str() {
            "apply_workspace_edit" => self.handle_apply_workspace_edit(context, tool_call).await,
            _ => Err(ApiError::InvalidRequest(format!(
                "Unknown internal workspace tool: {}",
                tool_call.name
            ))),
        }
    }
}
