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
use std::path::PathBuf;

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
        &["edit_file", "insert_after_symbol"]
    }

    async fn handle_tool_call(
        &self,
        context: &mill_handler_api::ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        if tool_call.name == "insert_after_symbol" {
            return self.handle_insert_after_symbol(context, tool_call).await;
        }

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

impl EditingToolsHandler {
    async fn handle_insert_after_symbol(
        &self,
        context: &mill_handler_api::ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        let args = tool_call
            .arguments
            .clone()
            .unwrap_or(serde_json::Value::Null);
        let symbol_name = args
            .get("symbol_name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ServerError::invalid_request("Missing 'symbol_name' parameter"))?;
        let new_code = args
            .get("new_code")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ServerError::invalid_request("Missing 'new_code' parameter"))?;
        let file_path_filter = args.get("file_path").and_then(|v| v.as_str());

        let mut target_symbol = None;

        if let Some(path) = file_path_filter {
            // Use get_document_symbols
            let request = mill_plugin_system::PluginRequest::new(
                "get_document_symbols".to_string(),
                PathBuf::from(path),
            );

            if let Ok(response) = context.plugin_manager.handle_request(request).await {
                if let Some(data) = response.data {
                    if let Some(arr) = data.as_array() {
                        for sym in arr {
                            if sym.get("name").and_then(|n| n.as_str()) == Some(symbol_name) {
                                target_symbol = Some((sym.clone(), path.to_string()));
                                break;
                            }
                        }
                    }
                }
            }
        } else {
            // Workspace search
            let all_plugins = context.plugin_manager.get_all_plugins_with_names().await;
            let mut symbols = Vec::new();

            for (_plugin_name, plugin) in all_plugins {
                let extensions = plugin.supported_extensions();
                let file_path = if let Some(ext) = extensions.first() {
                    PathBuf::from(format!("workspace.{}", ext))
                } else {
                    continue;
                };

                let mut request = mill_plugin_system::PluginRequest::new(
                    "search_workspace_symbols".to_string(),
                    file_path,
                );
                request = request.with_params(serde_json::json!({ "query": symbol_name }));

                if let Ok(response) = plugin.handle_request(request).await {
                    if let Some(data) = response.data {
                        if let Some(arr) = data.as_array() {
                            symbols.extend(arr.clone());
                        }
                    }
                }
            }

            let matches: Vec<Value> = symbols
                .into_iter()
                .filter(|s| {
                    s.get("name")
                        .and_then(|n| n.as_str())
                        .map(|n| n == symbol_name)
                        .unwrap_or(false)
                })
                .collect();

            if matches.len() == 1 {
                let s = &matches[0];
                let uri = s
                    .get("location")
                    .and_then(|l| l.get("uri"))
                    .and_then(|v| v.as_str())
                    .or_else(|| s.get("file_path").and_then(|v| v.as_str()));

                if let Some(u) = uri {
                    target_symbol = Some((s.clone(), u.to_string()));
                } else {
                    return Err(ServerError::invalid_request(
                        "Found symbol but cannot determine file path. Please specify file_path.",
                    ));
                }
            } else if matches.is_empty() {
                return Err(ServerError::invalid_request(format!(
                    "Symbol '{}' not found",
                    symbol_name
                )));
            } else {
                return Err(ServerError::invalid_request(format!(
                    "Multiple symbols found for '{}'. Please specify file_path.",
                    symbol_name
                )));
            }
        }

        let (symbol, source_file) = target_symbol.ok_or_else(|| {
            ServerError::invalid_request(format!("Symbol '{}' not found", symbol_name))
        })?;

        // Get end location
        let end_location = symbol
            .get("end_location") // Internal
            .or_else(|| {
                symbol
                    .get("location")
                    .and_then(|l| l.get("range").and_then(|r| r.get("end")))
            }) // LSP
            .ok_or_else(|| {
                ServerError::invalid_request("Symbol does not have end location information")
            })?;

        let line = end_location
            .get("line")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| ServerError::internal("Missing line"))?;
        let col = end_location
            .get("column")
            .and_then(|v| v.as_u64())
            .or_else(|| end_location.get("character").and_then(|v| v.as_u64()))
            .unwrap_or(0);

        let text_to_insert = format!("\n\n{}", new_code);

        let edit = TextEdit {
            file_path: Some(source_file.clone()),
            edit_type: EditType::Replace,
            location: EditLocation {
                start_line: line as u32,
                start_column: col as u32,
                end_line: line as u32,
                end_column: col as u32,
            },
            original_text: String::new(),
            new_text: text_to_insert,
            priority: 1,
            description: format!("Insert after symbol {}", symbol_name),
        };

        let plan = EditPlan {
            source_file: source_file.clone(),
            edits: vec![edit],
            dependency_updates: Vec::new(),
            validations: vec![ValidationRule {
                rule_type: ValidationType::SyntaxCheck,
                description: "Verify syntax after insertion".to_string(),
                parameters: HashMap::new(),
            }],
            metadata: EditPlanMetadata {
                intent_name: "insert_after_symbol".to_string(),
                intent_arguments: args.clone(),
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
