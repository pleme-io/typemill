//! Call hierarchy MCP tools (prepare_call_hierarchy, get_call_hierarchy_incoming_calls, get_call_hierarchy_outgoing_calls)

use crate::handlers::McpDispatcher;
use cb_core::model::mcp::{McpMessage, McpRequest};
use serde::{Deserialize, Serialize};
use serde_json::json;

/// Arguments for prepare_call_hierarchy tool
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
struct PrepareCallHierarchyArgs {
    file_path: String,
    line: u32,
    character: u32,
}

/// Arguments for get_call_hierarchy_incoming_calls tool
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
struct GetCallHierarchyIncomingArgs {
    item: Option<CallHierarchyItem>,
    file_path: Option<String>,
    line: Option<u32>,
    character: Option<u32>,
}

/// Arguments for get_call_hierarchy_outgoing_calls tool
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
struct GetCallHierarchyOutgoingArgs {
    item: Option<CallHierarchyItem>,
    file_path: Option<String>,
    line: Option<u32>,
    character: Option<u32>,
}

/// Call hierarchy item structure (matches LSP spec)
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct CallHierarchyItem {
    name: String,
    kind: u32,
    uri: String,
    range: LspRange,
    selection_range: LspRange,
}

/// LSP Range structure
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct LspRange {
    start: LspPosition,
    end: LspPosition,
}

/// LSP Position structure
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct LspPosition {
    line: u32,
    character: u32,
}

/// Call hierarchy incoming call
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CallHierarchyIncomingCall {
    from: CallHierarchyItem,
    from_ranges: Vec<LspRange>,
}

/// Call hierarchy outgoing call
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CallHierarchyOutgoingCall {
    to: CallHierarchyItem,
    from_ranges: Vec<LspRange>,
}

/// Register hierarchy tools
pub fn register_tools(dispatcher: &mut McpDispatcher) {
    // prepare_call_hierarchy tool
    dispatcher.register_tool("prepare_call_hierarchy".to_string(), |app_state, args| async move {
        let params: PrepareCallHierarchyArgs = serde_json::from_value(args)
            .map_err(|e| crate::error::ServerError::InvalidRequest(format!("Invalid args: {}", e)))?;

        tracing::debug!(
            "Preparing call hierarchy for {}:{}:{}",
            params.file_path,
            params.line,
            params.character
        );

        // Create LSP request for textDocument/prepareCallHierarchy
        let lsp_request = McpRequest {
            id: Some(serde_json::Value::Number(serde_json::Number::from(1))),
            method: "textDocument/prepareCallHierarchy".to_string(),
            params: Some(json!({
                "textDocument": {
                    "uri": format!("file://{}", params.file_path)
                },
                "position": {
                    "line": params.line,
                    "character": params.character
                }
            })),
        };

        // Send request to LSP service
        match app_state.lsp.request(McpMessage::Request(lsp_request)).await {
            Ok(McpMessage::Response(response)) => {
                if let Some(result) = response.result {
                    Ok(result)
                } else if let Some(error) = response.error {
                    Err(crate::error::ServerError::runtime(format!("LSP error: {}", error.message)))
                } else {
                    Err(crate::error::ServerError::runtime("Empty LSP response"))
                }
            }
            Ok(_) => Err(crate::error::ServerError::runtime("Unexpected LSP message type")),
            Err(e) => Err(crate::error::ServerError::runtime(format!("LSP request failed: {}", e))),
        }
    });

    // get_call_hierarchy_incoming_calls tool
    dispatcher.register_tool("get_call_hierarchy_incoming_calls".to_string(), |app_state, args| async move {
        let params: GetCallHierarchyIncomingArgs = serde_json::from_value(args)
            .map_err(|e| crate::error::ServerError::InvalidRequest(format!("Invalid args: {}", e)))?;

        tracing::debug!("Getting incoming calls for call hierarchy");

        // Determine if we have an item or need to prepare one
        let call_hierarchy_item = if let Some(item) = params.item {
            // Use provided item
            serde_json::to_value(item)?
        } else if let (Some(file_path), Some(line), Some(character)) = (params.file_path, params.line, params.character) {
            // First prepare call hierarchy
            let prepare_request = McpRequest {
                id: Some(serde_json::Value::Number(serde_json::Number::from(1))),
                method: "textDocument/prepareCallHierarchy".to_string(),
                params: Some(json!({
                    "textDocument": {
                        "uri": format!("file://{}", file_path)
                    },
                    "position": {
                        "line": line,
                        "character": character
                    }
                })),
            };

            match app_state.lsp.request(McpMessage::Request(prepare_request)).await {
                Ok(McpMessage::Response(response)) => {
                    if let Some(result) = response.result {
                        if let Some(items) = result.as_array() {
                            if let Some(first_item) = items.first() {
                                first_item.clone()
                            } else {
                                return Err(crate::error::ServerError::runtime("No call hierarchy items found"));
                            }
                        } else {
                            return Err(crate::error::ServerError::runtime("Invalid call hierarchy response"));
                        }
                    } else {
                        return Err(crate::error::ServerError::runtime("Failed to prepare call hierarchy"));
                    }
                }
                Ok(_) => return Err(crate::error::ServerError::runtime("Unexpected LSP message type")),
                Err(e) => return Err(crate::error::ServerError::runtime(format!("LSP request failed: {}", e))),
            }
        } else {
            return Err(crate::error::ServerError::InvalidRequest(
                "Must provide either 'item' or 'file_path', 'line', and 'character'".to_string()
            ));
        };

        // Create LSP request for callHierarchy/incomingCalls
        let lsp_request = McpRequest {
            id: Some(serde_json::Value::Number(serde_json::Number::from(2))),
            method: "callHierarchy/incomingCalls".to_string(),
            params: Some(json!({
                "item": call_hierarchy_item
            })),
        };

        // Send request to LSP service
        match app_state.lsp.request(McpMessage::Request(lsp_request)).await {
            Ok(McpMessage::Response(response)) => {
                if let Some(result) = response.result {
                    Ok(result)
                } else if let Some(error) = response.error {
                    Err(crate::error::ServerError::runtime(format!("LSP error: {}", error.message)))
                } else {
                    Err(crate::error::ServerError::runtime("Empty LSP response"))
                }
            }
            Ok(_) => Err(crate::error::ServerError::runtime("Unexpected LSP message type")),
            Err(e) => Err(crate::error::ServerError::runtime(format!("LSP request failed: {}", e))),
        }
    });

    // get_call_hierarchy_outgoing_calls tool
    dispatcher.register_tool("get_call_hierarchy_outgoing_calls".to_string(), |app_state, args| async move {
        let params: GetCallHierarchyOutgoingArgs = serde_json::from_value(args)
            .map_err(|e| crate::error::ServerError::InvalidRequest(format!("Invalid args: {}", e)))?;

        tracing::debug!("Getting outgoing calls for call hierarchy");

        // Determine if we have an item or need to prepare one
        let call_hierarchy_item = if let Some(item) = params.item {
            // Use provided item
            serde_json::to_value(item)?
        } else if let (Some(file_path), Some(line), Some(character)) = (params.file_path, params.line, params.character) {
            // First prepare call hierarchy
            let prepare_request = McpRequest {
                id: Some(serde_json::Value::Number(serde_json::Number::from(1))),
                method: "textDocument/prepareCallHierarchy".to_string(),
                params: Some(json!({
                    "textDocument": {
                        "uri": format!("file://{}", file_path)
                    },
                    "position": {
                        "line": line,
                        "character": character
                    }
                })),
            };

            match app_state.lsp.request(McpMessage::Request(prepare_request)).await {
                Ok(McpMessage::Response(response)) => {
                    if let Some(result) = response.result {
                        if let Some(items) = result.as_array() {
                            if let Some(first_item) = items.first() {
                                first_item.clone()
                            } else {
                                return Err(crate::error::ServerError::runtime("No call hierarchy items found"));
                            }
                        } else {
                            return Err(crate::error::ServerError::runtime("Invalid call hierarchy response"));
                        }
                    } else {
                        return Err(crate::error::ServerError::runtime("Failed to prepare call hierarchy"));
                    }
                }
                Ok(_) => return Err(crate::error::ServerError::runtime("Unexpected LSP message type")),
                Err(e) => return Err(crate::error::ServerError::runtime(format!("LSP request failed: {}", e))),
            }
        } else {
            return Err(crate::error::ServerError::InvalidRequest(
                "Must provide either 'item' or 'file_path', 'line', and 'character'".to_string()
            ));
        };

        // Create LSP request for callHierarchy/outgoingCalls
        let lsp_request = McpRequest {
            id: Some(serde_json::Value::Number(serde_json::Number::from(3))),
            method: "callHierarchy/outgoingCalls".to_string(),
            params: Some(json!({
                "item": call_hierarchy_item
            })),
        };

        // Send request to LSP service
        match app_state.lsp.request(McpMessage::Request(lsp_request)).await {
            Ok(McpMessage::Response(response)) => {
                if let Some(result) = response.result {
                    Ok(result)
                } else if let Some(error) = response.error {
                    Err(crate::error::ServerError::runtime(format!("LSP error: {}", error.message)))
                } else {
                    Err(crate::error::ServerError::runtime("Empty LSP response"))
                }
            }
            Ok(_) => Err(crate::error::ServerError::runtime("Unexpected LSP message type")),
            Err(e) => Err(crate::error::ServerError::runtime(format!("LSP request failed: {}", e))),
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_prepare_call_hierarchy_args() {
        let args = json!({
            "file_path": "test.ts",
            "line": 10,
            "character": 5
        });

        let parsed: PrepareCallHierarchyArgs = serde_json::from_value(args).unwrap();
        assert_eq!(parsed.file_path, "test.ts");
        assert_eq!(parsed.line, 10);
        assert_eq!(parsed.character, 5);
    }

    #[tokio::test]
    async fn test_incoming_calls_args_with_position() {
        let args = json!({
            "file_path": "test.ts",
            "line": 15,
            "character": 10
        });

        let parsed: GetCallHierarchyIncomingArgs = serde_json::from_value(args).unwrap();
        assert_eq!(parsed.file_path, Some("test.ts".to_string()));
        assert_eq!(parsed.line, Some(15));
        assert_eq!(parsed.character, Some(10));
        assert!(parsed.item.is_none());
    }

    #[tokio::test]
    async fn test_outgoing_calls_args_with_item() {
        let args = json!({
            "item": {
                "name": "testFunction",
                "kind": 12,
                "uri": "file:///test.ts",
                "range": {
                    "start": {"line": 10, "character": 0},
                    "end": {"line": 15, "character": 0}
                },
                "selectionRange": {
                    "start": {"line": 10, "character": 9},
                    "end": {"line": 10, "character": 21}
                }
            }
        });

        let parsed: GetCallHierarchyOutgoingArgs = serde_json::from_value(args).unwrap();
        assert!(parsed.item.is_some());
        assert_eq!(parsed.item.unwrap().name, "testFunction");
        assert!(parsed.file_path.is_none());
    }
}