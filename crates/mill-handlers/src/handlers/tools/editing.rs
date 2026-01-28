//! Editing tool handlers
//!
//! Handles: edit_file

use super::ToolHandler;
use async_trait::async_trait;
use mill_foundation::core::model::mcp::ToolCall;
use mill_foundation::errors::{MillError as ServerError, MillResult as ServerResult};
use mill_foundation::protocol::{
    EditLocation, EditPlan, EditPlanMetadata, EditType, TextEdit, ValidationRule, ValidationType,
};
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;

pub struct EditingToolsHandler;

impl EditingToolsHandler {
    pub fn new() -> Self {
        Self
    }
}

impl Default for EditingToolsHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Deserialize)]
struct EditFileParams {
    path: String,
    edits: Vec<SimpleEdit>,
    #[serde(default)]
    dry_run: bool,
}

#[derive(Debug, Deserialize)]
struct SimpleEdit {
    start_line: u32,
    start_column: u32,
    end_line: u32,
    end_column: u32,
    new_text: String,
}

#[async_trait]
impl ToolHandler for EditingToolsHandler {
    fn tool_names(&self) -> &[&str] {
        &["edit_file"]
    }

    fn is_internal(&self) -> bool {
        // edit_file is an internal tool - low-level text editing at line/column positions.
        // AI agents should use higher-level refactoring tools (rename, extract, etc.) instead.
        true
    }

    async fn handle_tool_call(
        &self,
        context: &mill_handler_api::ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        if tool_call.name != "edit_file" {
            return Err(ServerError::invalid_request(format!(
                "Unknown editing tool: {}",
                tool_call.name
            )));
        }

        let params: EditFileParams = serde_json::from_value(
            tool_call
                .arguments
                .clone()
                .unwrap_or(serde_json::Value::Null),
        )
        .map_err(|e| {
            ServerError::invalid_request(format!("Failed to parse edit_file params: {}", e))
        })?;

        let source_file = params.path.clone();

        if params.dry_run {
            return Err(ServerError::invalid_request(
                "Dry run is not currently supported for edit_file tool",
            ));
        }

        let edits: Vec<TextEdit> = params
            .edits
            .into_iter()
            .enumerate()
            .map(|(idx, edit)| TextEdit {
                file_path: Some(source_file.clone()),
                edit_type: EditType::Replace,
                location: EditLocation {
                    start_line: edit.start_line,
                    start_column: edit.start_column,
                    end_line: edit.end_line,
                    end_column: edit.end_column,
                },
                original_text: String::new(), // Not required for application
                new_text: edit.new_text,
                priority: (idx as u32) + 1,
                description: "Manual edit via edit_file".to_string(),
            })
            .collect();

        let plan = EditPlan {
            source_file: source_file.clone(),
            edits,
            dependency_updates: Vec::new(),
            validations: vec![ValidationRule {
                rule_type: ValidationType::SyntaxCheck,
                description: "Verify syntax after editing".to_string(),
                parameters: HashMap::new(),
            }],
            metadata: EditPlanMetadata {
                intent_name: "edit_file".to_string(),
                intent_arguments: tool_call
                    .arguments
                    .clone()
                    .unwrap_or(serde_json::Value::Null),
                created_at: chrono::Utc::now(),
                complexity: 1,
                impact_areas: vec!["editing".to_string()],
                consolidation: None,
            },
        };

        let result = context
            .app_state
            .file_service
            .apply_edit_plan(&plan)
            .await?;

        Ok(serde_json::to_value(result).unwrap_or(serde_json::Value::Null))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_params() {
        let params_json = json!({
            "path": "test.rs",
            "edits": [
                {
                    "start_line": 0,
                    "start_column": 0,
                    "end_line": 1,
                    "end_column": 0,
                    "new_text": "hello"
                }
            ],
            "dry_run": true
        });

        let params: EditFileParams = serde_json::from_value(params_json).unwrap();
        assert_eq!(params.path, "test.rs");
        assert_eq!(params.edits.len(), 1);
        assert!(params.dry_run);
    }

    #[test]
    fn test_parse_params_defaults() {
        let params_json = json!({
            "path": "test.rs",
            "edits": []
        });

        let params: EditFileParams = serde_json::from_value(params_json).unwrap();
        assert!(!params.dry_run);
    }
}
