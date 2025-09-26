//! Navigation MCP tools (find_definition, find_references, etc.)

use crate::handlers::McpDispatcher;
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
    dispatcher.register_tool("find_definition".to_string(), |args| async move {
        let params: FindDefinitionArgs = serde_json::from_value(args)
            .map_err(|e| crate::error::ServerError::InvalidRequest(format!("Invalid args: {}", e)))?;

        tracing::debug!("Finding definition for {} in {}", params.symbol_name, params.file_path);

        // In a real implementation, this would:
        // 1. Connect to the appropriate LSP server
        // 2. Send textDocument/definition request
        // 3. Parse and return the results

        // Mock response for now
        let location = SymbolLocation {
            file_path: params.file_path.clone(),
            line: 42,
            column: 10,
            symbol_name: params.symbol_name,
            symbol_kind: params.symbol_kind.or(Some("function".to_string())),
        };

        Ok(json!({
            "definitions": [location],
            "status": "found"
        }))
    });

    // find_references tool
    dispatcher.register_tool("find_references".to_string(), |args| async move {
        let params: FindReferencesArgs = serde_json::from_value(args)
            .map_err(|e| crate::error::ServerError::InvalidRequest(format!("Invalid args: {}", e)))?;

        tracing::debug!("Finding references for {} in {}", params.symbol_name, params.file_path);

        // Mock response with multiple references
        let references = vec![
            SymbolLocation {
                file_path: params.file_path.clone(),
                line: 10,
                column: 5,
                symbol_name: params.symbol_name.clone(),
                symbol_kind: Some("usage".to_string()),
            },
            SymbolLocation {
                file_path: format!("{}_test", params.file_path),
                line: 25,
                column: 15,
                symbol_name: params.symbol_name.clone(),
                symbol_kind: Some("usage".to_string()),
            },
        ];

        Ok(json!({
            "references": references,
            "includeDeclaration": params.include_declaration.unwrap_or(true),
            "count": references.len()
        }))
    });

    // search_workspace_symbols tool
    dispatcher.register_tool("search_workspace_symbols".to_string(), |args| async move {
        let params: SearchWorkspaceArgs = serde_json::from_value(args)
            .map_err(|e| crate::error::ServerError::InvalidRequest(format!("Invalid args: {}", e)))?;

        tracing::debug!("Searching workspace for: {}", params.query);

        // Mock response with search results
        let symbols = vec![
            json!({
                "name": format!("{}Handler", params.query),
                "kind": "class",
                "location": {
                    "file": "src/handlers.rs",
                    "line": 10
                }
            }),
            json!({
                "name": format!("process{}", params.query),
                "kind": "function",
                "location": {
                    "file": "src/processor.rs",
                    "line": 45
                }
            }),
        ];

        Ok(json!({
            "symbols": symbols,
            "workspace": params.workspace_path.unwrap_or_else(|| ".".to_string()),
            "count": symbols.len()
        }))
    });

    // get_document_symbols tool
    dispatcher.register_tool("get_document_symbols".to_string(), |args| async move {
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