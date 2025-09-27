//! Intelligence MCP tools (get_hover, get_completions, get_signature_help)

use crate::handlers::McpDispatcher;
use cb_core::model::mcp::{McpMessage, McpRequest};
use serde::{Deserialize, Serialize};
use serde_json::json;

/// Arguments for get_hover tool
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
struct GetHoverArgs {
    file_path: String,
    line: u32,
    character: u32,
}

/// Arguments for get_completions tool
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
struct GetCompletionsArgs {
    file_path: String,
    line: u32,
    character: u32,
    trigger_character: Option<String>,
}

/// Arguments for get_signature_help tool
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
struct GetSignatureHelpArgs {
    file_path: String,
    line: u32,
    character: u32,
    trigger_character: Option<String>,
}

/// Hover information result
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct HoverInfo {
    contents: String,
    range: Option<LspRange>,
}

/// LSP Range structure
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct LspRange {
    start: LspPosition,
    end: LspPosition,
}

/// LSP Position structure
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct LspPosition {
    line: u32,
    character: u32,
}

/// Completion item result
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CompletionItem {
    label: String,
    kind: Option<u32>,
    detail: Option<String>,
    documentation: Option<String>,
    insert_text: Option<String>,
}

/// Signature help result
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SignatureHelp {
    signatures: Vec<SignatureInfo>,
    active_signature: Option<u32>,
    active_parameter: Option<u32>,
}

/// Signature information
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SignatureInfo {
    label: String,
    documentation: Option<String>,
    parameters: Option<Vec<ParameterInfo>>,
}

/// Parameter information
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ParameterInfo {
    label: String,
    documentation: Option<String>,
}


/// Text edit
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TextEdit {
    range: LspRange,
    new_text: String,
}

/// Register intelligence tools
pub fn register_tools(dispatcher: &mut McpDispatcher) {
    // get_hover tool
    dispatcher.register_tool("get_hover".to_string(), |app_state, args| async move {
        let params: GetHoverArgs = serde_json::from_value(args)
            .map_err(|e| crate::error::ServerError::InvalidRequest(format!("Invalid args: {}", e)))?;

        tracing::debug!("Getting hover info for {}:{}:{}", params.file_path, params.line, params.character);

        // Create LSP request for textDocument/hover
        let lsp_request = McpRequest {
            id: Some(serde_json::Value::Number(serde_json::Number::from(1))),
            method: "textDocument/hover".to_string(),
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

    // get_completions tool
    dispatcher.register_tool("get_completions".to_string(), |app_state, args| async move {
        let params: GetCompletionsArgs = serde_json::from_value(args)
            .map_err(|e| crate::error::ServerError::InvalidRequest(format!("Invalid args: {}", e)))?;

        tracing::debug!("Getting completions for {}:{}:{}", params.file_path, params.line, params.character);

        // Create LSP request for textDocument/completion
        let mut completion_params = json!({
            "textDocument": {
                "uri": format!("file://{}", params.file_path)
            },
            "position": {
                "line": params.line,
                "character": params.character
            }
        });

        // Add trigger character if provided
        if let Some(trigger_char) = params.trigger_character {
            completion_params["context"] = json!({
                "triggerKind": 2, // TriggerCharacter
                "triggerCharacter": trigger_char
            });
        }

        let lsp_request = McpRequest {
            id: Some(serde_json::Value::Number(serde_json::Number::from(2))),
            method: "textDocument/completion".to_string(),
            params: Some(completion_params),
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

    // get_signature_help tool
    dispatcher.register_tool("get_signature_help".to_string(), |app_state, args| async move {
        let params: GetSignatureHelpArgs = serde_json::from_value(args)
            .map_err(|e| crate::error::ServerError::InvalidRequest(format!("Invalid args: {}", e)))?;

        tracing::debug!("Getting signature help for {}:{}:{}", params.file_path, params.line, params.character);

        // Create LSP request for textDocument/signatureHelp
        let mut signature_params = json!({
            "textDocument": {
                "uri": format!("file://{}", params.file_path)
            },
            "position": {
                "line": params.line,
                "character": params.character
            }
        });

        // Add trigger character if provided
        if let Some(trigger_char) = params.trigger_character {
            signature_params["context"] = json!({
                "triggerKind": 2, // TriggerCharacter
                "triggerCharacter": trigger_char,
                "isRetrigger": false
            });
        }

        let lsp_request = McpRequest {
            id: Some(serde_json::Value::Number(serde_json::Number::from(3))),
            method: "textDocument/signatureHelp".to_string(),
            params: Some(signature_params),
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
    async fn test_get_hover_args() {
        let args = json!({
            "file_path": "test.ts",
            "line": 10,
            "character": 5
        });

        let parsed: GetHoverArgs = serde_json::from_value(args).unwrap();
        assert_eq!(parsed.file_path, "test.ts");
        assert_eq!(parsed.line, 10);
        assert_eq!(parsed.character, 5);
    }

    #[tokio::test]
    async fn test_get_completions_args() {
        let args = json!({
            "file_path": "test.ts",
            "line": 15,
            "character": 10,
            "trigger_character": "."
        });

        let parsed: GetCompletionsArgs = serde_json::from_value(args).unwrap();
        assert_eq!(parsed.file_path, "test.ts");
        assert_eq!(parsed.line, 15);
        assert_eq!(parsed.character, 10);
        assert_eq!(parsed.trigger_character, Some(".".to_string()));
    }

    #[tokio::test]
    async fn test_get_signature_help_args() {
        let args = json!({
            "file_path": "test.ts",
            "line": 20,
            "character": 8,
            "trigger_character": "("
        });

        let parsed: GetSignatureHelpArgs = serde_json::from_value(args).unwrap();
        assert_eq!(parsed.file_path, "test.ts");
        assert_eq!(parsed.line, 20);
        assert_eq!(parsed.character, 8);
        assert_eq!(parsed.trigger_character, Some("(".to_string()));
    }

}