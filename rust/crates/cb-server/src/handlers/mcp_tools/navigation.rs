//! Navigation MCP tools (find_definition, find_references, etc.)

use crate::handlers::McpDispatcher;
use super::util::forward_lsp_request;
use serde::{Deserialize, Serialize};
use serde_json::json;

/// Arguments for find_definition tool
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
struct FindDefinitionArgs {
    file_path: String,
    symbol_name: String,
    symbol_kind: Option<String>,
}

/// Arguments for find_references tool
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
struct FindReferencesArgs {
    file_path: String,
    symbol_name: String,
    symbol_kind: Option<String>,
    include_declaration: Option<bool>,
}

/// Arguments for search workspace symbols
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
struct SearchWorkspaceArgs {
    query: String,
    workspace_path: Option<String>,
}

/// Symbol location result
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SymbolLocation {
    file_path: String,
    line: u32,
    column: u32,
    symbol_name: String,
    symbol_kind: Option<String>,
}

/// Register navigation tools
pub fn register_tools(dispatcher: &mut McpDispatcher) {
    // find_definition tool
    dispatcher.register_tool("find_definition".to_string(), |app_state, args| async move {
        let params: FindDefinitionArgs = serde_json::from_value(args)
            .map_err(|e| crate::error::ServerError::InvalidRequest(format!("Invalid args: {}", e)))?;

        tracing::debug!("Finding definition for {} in {}", params.symbol_name, params.file_path);

        // Use helper function to forward request
        forward_lsp_request(
            app_state.lsp.as_ref(),
            "find_definition".to_string(),
            Some(json!({
                "file_path": params.file_path,
                "symbol_name": params.symbol_name,
                "symbol_kind": params.symbol_kind
            }))
        ).await
    });

    // find_references tool
    dispatcher.register_tool("find_references".to_string(), |app_state, args| async move {
        let params: FindReferencesArgs = serde_json::from_value(args)
            .map_err(|e| crate::error::ServerError::InvalidRequest(format!("Invalid args: {}", e)))?;

        tracing::debug!("Finding references for {} in {}", params.symbol_name, params.file_path);

        // Use helper function to forward request
        forward_lsp_request(
            app_state.lsp.as_ref(),
            "find_references".to_string(),
            Some(json!({
                "file_path": params.file_path,
                "symbol_name": params.symbol_name,
                "symbol_kind": params.symbol_kind,
                "include_declaration": params.include_declaration
            }))
        ).await
    });

    // search_workspace_symbols tool
    dispatcher.register_tool("search_workspace_symbols".to_string(), |app_state, args| async move {
        let params: SearchWorkspaceArgs = serde_json::from_value(args)
            .map_err(|e| crate::error::ServerError::InvalidRequest(format!("Invalid args: {}", e)))?;

        tracing::debug!("Searching workspace for: {}", params.query);

        // Use helper function to forward request
        forward_lsp_request(
            app_state.lsp.as_ref(),
            "search_workspace_symbols".to_string(),
            Some(json!({
                "query": params.query,
                "workspace_path": params.workspace_path
            }))
        ).await
    });

    // get_document_symbols tool
    dispatcher.register_tool("get_document_symbols".to_string(), |_app_state, args| async move {
        let file_path = args["file_path"].as_str()
            .ok_or_else(|| crate::error::ServerError::InvalidRequest("Missing file_path".into()))?;

        tracing::debug!("Getting document symbols for: {}", file_path);

        // Mock document symbols
        let symbols = vec![
            json!({
                "name": "MyClass",
                "kind": "class",
                "range": {
                    "start": {"line": 5, "character": 0},
                    "end": {"line": 50, "character": 1}
                },
                "children": [
                    {
                        "name": "constructor",
                        "kind": "constructor",
                        "range": {
                            "start": {"line": 6, "character": 2},
                            "end": {"line": 10, "character": 3}
                        }
                    },
                    {
                        "name": "process",
                        "kind": "method",
                        "range": {
                            "start": {"line": 12, "character": 2},
                            "end": {"line": 20, "character": 3}
                        }
                    }
                ]
            }),
            json!({
                "name": "helperFunction",
                "kind": "function",
                "range": {
                    "start": {"line": 52, "character": 0},
                    "end": {"line": 60, "character": 1}
                }
            })
        ];

        Ok(json!({
            "symbols": symbols,
            "file": file_path
        }))
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_find_definition_args() {
        let args = json!({
            "file_path": "test.ts",
            "symbol_name": "myFunction",
            "symbol_kind": "function"
        });

        let parsed: FindDefinitionArgs = serde_json::from_value(args).unwrap();
        assert_eq!(parsed.file_path, "test.ts");
        assert_eq!(parsed.symbol_name, "myFunction");
        assert_eq!(parsed.symbol_kind, Some("function".to_string()));
    }

    #[tokio::test]
    async fn test_find_references_args() {
        let args = json!({
            "file_path": "test.ts",
            "symbol_name": "MyClass",
            "include_declaration": false
        });

        let parsed: FindReferencesArgs = serde_json::from_value(args).unwrap();
        assert_eq!(parsed.file_path, "test.ts");
        assert_eq!(parsed.symbol_name, "MyClass");
        assert_eq!(parsed.include_declaration, Some(false));
    }
}