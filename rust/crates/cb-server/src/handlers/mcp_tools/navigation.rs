//! Navigation MCP tools (find_definition, find_references, etc.)

use crate::handlers::McpDispatcher;
use crate::utils::{SimdJsonParser, create_paginated_response};
use super::util::forward_lsp_request;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

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
    // Pagination parameters for performance optimization
    page: Option<usize>,
    page_size: Option<usize>,
}

/// Arguments for search workspace symbols
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
struct SearchWorkspaceArgs {
    query: String,
    workspace_path: Option<String>,
    // Pagination parameters for performance optimization
    page: Option<usize>,
    page_size: Option<usize>,
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
        let params: FindReferencesArgs = SimdJsonParser::from_value(args)?;

        tracing::debug!("Finding references for {} in {} (page: {:?}, size: {:?})",
                       params.symbol_name, params.file_path, params.page, params.page_size);

        // Get all references from LSP
        let lsp_result = forward_lsp_request(
            app_state.lsp.as_ref(),
            "find_references".to_string(),
            Some(json!({
                "file_path": params.file_path,
                "symbol_name": params.symbol_name,
                "symbol_kind": params.symbol_kind,
                "include_declaration": params.include_declaration
            }))
        ).await?;

        // Apply pagination for performance optimization
        let page = params.page.unwrap_or(0);
        let page_size = params.page_size.unwrap_or(50); // Default to 50 items per page

        if let Some(references) = lsp_result.get("references") {
            if let Some(ref_array) = references.as_array() {
                let total_count = ref_array.len();

                if page_size < total_count {
                    // Use pagination for large result sets
                    let paginated_response = create_paginated_response(
                        ref_array.clone(),
                        page_size,
                        page,
                        total_count
                    );

                    tracing::debug!("Paginated {} references into page {} of {} items",
                                  total_count, page, page_size);

                    Ok(paginated_response)
                } else {
                    // Return all results if small enough
                    Ok(lsp_result)
                }
            } else {
                Ok(lsp_result)
            }
        } else {
            Ok(lsp_result)
        }
    });

    // search_workspace_symbols tool
    dispatcher.register_tool("search_workspace_symbols".to_string(), |app_state, args| async move {
        let params: SearchWorkspaceArgs = SimdJsonParser::from_value(args)?;

        tracing::debug!("Searching workspace for: {} (page: {:?}, size: {:?})",
                       params.query, params.page, params.page_size);

        // Get all symbols from LSP
        let lsp_result = forward_lsp_request(
            app_state.lsp.as_ref(),
            "search_workspace_symbols".to_string(),
            Some(json!({
                "query": params.query,
                "workspace_path": params.workspace_path
            }))
        ).await?;

        // Apply pagination for performance optimization
        let page = params.page.unwrap_or(0);
        let page_size = params.page_size.unwrap_or(100); // Default to 100 symbols per page

        if let Some(symbols) = lsp_result.get("symbols") {
            if let Some(symbol_array) = symbols.as_array() {
                let total_count = symbol_array.len();

                if page_size < total_count {
                    // Use pagination for large result sets
                    let paginated_response = create_paginated_response(
                        symbol_array.clone(),
                        page_size,
                        page,
                        total_count
                    );

                    tracing::debug!("Paginated {} workspace symbols into page {} of {} items",
                                  total_count, page, page_size);

                    Ok(paginated_response)
                } else {
                    // Return all results if small enough
                    Ok(lsp_result)
                }
            } else {
                Ok(lsp_result)
            }
        } else {
            Ok(lsp_result)
        }
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

// Include integration tests
#[cfg(test)]
#[path = "navigation_tests.rs"]
mod navigation_tests;