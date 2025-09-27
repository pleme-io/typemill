//! Editing MCP tools (rename_symbol, format_document, etc.)

use crate::handlers::McpDispatcher;
use super::util::forward_lsp_request;
use serde::{Deserialize, Serialize};
use serde_json::json;

/// Arguments for rename_symbol tool
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
struct RenameSymbolArgs {
    file_path: String,
    line: u32,
    character: u32,
    new_name: String,
    dry_run: Option<bool>,
}

/// Arguments for format_document tool
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
struct FormatDocumentArgs {
    file_path: String,
    options: Option<FormatOptions>,
}

/// Formatting options
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
struct FormatOptions {
    tab_size: Option<u32>,
    insert_spaces: Option<bool>,
    trim_trailing_whitespace: Option<bool>,
    insert_final_newline: Option<bool>,
    trim_final_newlines: Option<bool>,
}

/// Edit operation result
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct EditResult {
    success: bool,
    files_modified: Vec<String>,
    edits_count: u32,
    preview: Option<Vec<FileEdit>>,
}

/// File edit description
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct FileEdit {
    file_path: String,
    edits: Vec<TextEdit>,
}

/// Text edit description
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TextEdit {
    range: TextRange,
    new_text: String,
}

/// Text range
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TextRange {
    start: Position,
    end: Position,
}

/// Position in text
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Position {
    line: u32,
    character: u32,
}

/// Arguments for rename_symbol_strict tool
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
struct RenameSymbolStrictArgs {
    file_path: String,
    line: u32,
    character: u32,
    new_name: String,
    dry_run: Option<bool>,
}

/// Register editing tools
pub fn register_tools(dispatcher: &mut McpDispatcher) {
    // rename_symbol tool - Returns WorkspaceEdit for transaction processing
    dispatcher.register_tool("rename_symbol".to_string(), |app_state, args| async move {
        let params: RenameSymbolArgs = serde_json::from_value(args.clone())
            .map_err(|e| crate::error::ServerError::InvalidRequest(format!("Invalid args: {}", e)))?;

        tracing::debug!(
            "Getting WorkspaceEdit for rename: {}:{}:{} to {}",
            params.file_path,
            params.line,
            params.character,
            params.new_name
        );

        // Check if this is a dry_run request
        let dry_run = params.dry_run.unwrap_or(false);

        // Request WorkspaceEdit from LSP but don't apply it
        // The dispatcher's transaction system will handle the actual application
        let workspace_edit_result = forward_lsp_request(
            app_state.lsp.as_ref(),
            "textDocument/rename".to_string(),
            Some(json!({
                "textDocument": {
                    "uri": format!("file://{}", params.file_path)
                },
                "position": {
                    "line": params.line,
                    "character": params.character
                },
                "newName": params.new_name
            }))
        ).await?;

        // Return the WorkspaceEdit with metadata for the transaction system
        // Include the original arguments so the dispatcher can create proper FileOperations
        Ok(json!({
            "workspace_edit": workspace_edit_result,
            "dry_run": dry_run,
            "operation_type": "refactor",
            "original_args": args,
            "tool": "rename_symbol"
        }))
    });

    // format_document tool
    dispatcher.register_tool("format_document".to_string(), |_app_state, args| async move {
        let params: FormatDocumentArgs = serde_json::from_value(args)
            .map_err(|e| crate::error::ServerError::InvalidRequest(format!("Invalid args: {}", e)))?;

        tracing::debug!("Formatting document: {}", params.file_path);

        let options = params.options.unwrap_or(FormatOptions {
            tab_size: Some(2),
            insert_spaces: Some(true),
            trim_trailing_whitespace: Some(true),
            insert_final_newline: Some(true),
            trim_final_newlines: Some(true),
        });

        // Mock formatting result
        let edits = vec![
            TextEdit {
                range: TextRange {
                    start: Position { line: 0, character: 0 },
                    end: Position { line: 0, character: 0 },
                },
                new_text: "// Formatted\n".to_string(),
            },
        ];

        Ok(json!({
            "formatted": true,
            "file": params.file_path,
            "edits": edits,
            "options": {
                "tabSize": options.tab_size.unwrap_or(2),
                "insertSpaces": options.insert_spaces.unwrap_or(true),
                "trimTrailingWhitespace": options.trim_trailing_whitespace.unwrap_or(true),
            }
        }))
    });

    // organize_imports tool - Returns WorkspaceEdit for transaction processing
    dispatcher.register_tool("organize_imports".to_string(), |app_state, args| async move {
        let file_path = args.get("file_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| crate::error::ServerError::InvalidRequest("Missing file_path".into()))?;

        tracing::debug!("Getting WorkspaceEdit for organize imports: {}", file_path);

        // Request code action from LSP for organizing imports
        let organize_result = forward_lsp_request(
            app_state.lsp.as_ref(),
            "textDocument/codeAction".to_string(),
            Some(json!({
                "textDocument": {
                    "uri": format!("file://{}", file_path)
                },
                "range": {
                    "start": {"line": 0, "character": 0},
                    "end": {"line": 99999, "character": 0}
                },
                "context": {
                    "only": ["source.organizeImports"]
                }
            }))
        ).await?;

        // Return the WorkspaceEdit for transaction processing
        Ok(json!({
            "workspace_edit": organize_result,
            "operation_type": "refactor",
            "original_args": args,
            "tool": "organize_imports"
        }))
    });

    // extract_function tool - Placeholder for future implementation
    dispatcher.register_tool("extract_function".to_string(), |_app_state, args| async move {
        tracing::debug!("Extract function requested (placeholder implementation)");

        // For now, return a simple workspace edit
        // In a real implementation, this would analyze the selection and create a new function
        Ok(json!({
            "workspace_edit": {
                "changes": {}
            },
            "operation_type": "refactor",
            "original_args": args,
            "tool": "extract_function",
            "message": "Extract function not yet fully implemented"
        }))
    });

    // extract_variable tool - Placeholder for future implementation
    dispatcher.register_tool("extract_variable".to_string(), |_app_state, args| async move {
        tracing::debug!("Extract variable requested (placeholder implementation)");

        // For now, return a simple workspace edit
        // In a real implementation, this would analyze the expression and create a variable
        Ok(json!({
            "workspace_edit": {
                "changes": {}
            },
            "operation_type": "refactor",
            "original_args": args,
            "tool": "extract_variable",
            "message": "Extract variable not yet fully implemented"
        }))
    });

    // inline_variable tool - Placeholder for future implementation
    dispatcher.register_tool("inline_variable".to_string(), |_app_state, args| async move {
        tracing::debug!("Inline variable requested (placeholder implementation)");

        // For now, return a simple workspace edit
        // In a real implementation, this would find variable uses and inline them
        Ok(json!({
            "workspace_edit": {
                "changes": {}
            },
            "operation_type": "refactor",
            "original_args": args,
            "tool": "inline_variable",
            "message": "Inline variable not yet fully implemented"
        }))
    });

    // get_code_actions tool
    dispatcher.register_tool("get_code_actions".to_string(), |_app_state, args| async move {
        let file_path = args["file_path"].as_str()
            .ok_or_else(|| crate::error::ServerError::InvalidRequest("Missing file_path".into()))?;

        tracing::debug!("Getting code actions for: {}", file_path);

        // Mock code actions
        let actions = vec![
            json!({
                "title": "Add missing imports",
                "kind": "quickfix",
                "isPreferred": true,
                "edit": {
                    "changes": {
                        file_path: [
                            {
                                "range": {
                                    "start": {"line": 0, "character": 0},
                                    "end": {"line": 0, "character": 0}
                                },
                                "newText": "import { Component } from 'react';\n"
                            }
                        ]
                    }
                }
            }),
            json!({
                "title": "Remove unused imports",
                "kind": "source.fixAll",
                "diagnostics": ["unused-import"],
            }),
            json!({
                "title": "Organize imports",
                "kind": "source.organizeImports",
            }),
        ];

        Ok(json!({
            "actions": actions,
            "file": file_path
        }))
    });

    // apply_workspace_edit tool
    dispatcher.register_tool("apply_workspace_edit".to_string(), |_app_state, args| async move {
        let changes = args["changes"].as_object()
            .ok_or_else(|| crate::error::ServerError::InvalidRequest("Missing changes".into()))?;

        let validate = args["validate_before_apply"].as_bool().unwrap_or(true);

        tracing::debug!("Applying workspace edit to {} files", changes.len());

        // Count total edits
        let mut total_edits = 0;
        let mut files_modified = vec![];

        for (file_path, edits) in changes {
            if let Some(edits_array) = edits.as_array() {
                total_edits += edits_array.len();
                files_modified.push(file_path.clone());
            }
        }

        Ok(json!({
            "success": true,
            "filesModified": files_modified,
            "totalEdits": total_edits,
            "validated": validate
        }))
    });

    // rename_symbol_strict tool
    dispatcher.register_tool("rename_symbol_strict".to_string(), |app_state, args| async move {
        let params: RenameSymbolStrictArgs = serde_json::from_value(args)
            .map_err(|e| crate::error::ServerError::InvalidRequest(format!("Invalid args: {}", e)))?;

        tracing::debug!(
            "Renaming symbol at exact position {}:{}:{} to {}",
            params.file_path,
            params.line,
            params.character,
            params.new_name
        );

        let is_dry_run = params.dry_run.unwrap_or(false);

        if is_dry_run {
            tracing::debug!("Dry run mode - validating rename without execution");

            // In dry run mode, return a preview of what would be renamed
            return Ok(json!({
                "dryRun": true,
                "position": {
                    "line": params.line,
                    "character": params.character
                },
                "oldName": "symbolAtPosition",
                "newName": params.new_name,
                "filesAffected": [params.file_path],
                "preview": [
                    {
                        "file": params.file_path,
                        "edits": [
                            {
                                "range": {
                                    "start": {"line": params.line, "character": params.character},
                                    "end": {"line": params.line, "character": params.character + 10}
                                },
                                "newText": params.new_name
                            }
                        ]
                    }
                ]
            }));
        }

        // Use helper function to forward request
        let result = forward_lsp_request(
            app_state.lsp.as_ref(),
            "textDocument/rename".to_string(),
            Some(json!({
                "textDocument": {
                    "uri": format!("file://{}", params.file_path)
                },
                "position": {
                    "line": params.line,
                    "character": params.character
                },
                "newName": params.new_name
            }))
        ).await?;

        // Add metadata to indicate this was a strict rename
        let mut enhanced_result = result.as_object().unwrap_or(&serde_json::Map::new()).clone();
        enhanced_result.insert("renameType".to_string(), json!("strict"));
        enhanced_result.insert("position".to_string(), json!({
            "line": params.line,
            "character": params.character
        }));

        Ok(json!(enhanced_result))
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rename_symbol_args() {
        let args = json!({
            "file_path": "test.ts",
            "line": 10,
            "character": 5,
            "new_name": "newName",
            "dry_run": true
        });

        let parsed: RenameSymbolArgs = serde_json::from_value(args).unwrap();
        assert_eq!(parsed.file_path, "test.ts");
        assert_eq!(parsed.line, 10);
        assert_eq!(parsed.character, 5);
        assert_eq!(parsed.new_name, "newName");
        assert_eq!(parsed.dry_run, Some(true));
    }

    #[tokio::test]
    async fn test_format_options() {
        let args = json!({
            "file_path": "test.ts",
            "options": {
                "tab_size": 4,
                "insert_spaces": false,
                "trim_trailing_whitespace": true
            }
        });

        let parsed: FormatDocumentArgs = serde_json::from_value(args).unwrap();
        assert_eq!(parsed.file_path, "test.ts");

        let options = parsed.options.unwrap();
        assert_eq!(options.tab_size, Some(4));
        assert_eq!(options.insert_spaces, Some(false));
        assert_eq!(options.trim_trailing_whitespace, Some(true));
    }

    #[tokio::test]
    async fn test_rename_symbol_strict_args() {
        let args = json!({
            "file_path": "test.ts",
            "line": 15,
            "character": 8,
            "new_name": "strictNewName",
            "dry_run": false
        });

        let parsed: RenameSymbolStrictArgs = serde_json::from_value(args).unwrap();
        assert_eq!(parsed.file_path, "test.ts");
        assert_eq!(parsed.line, 15);
        assert_eq!(parsed.character, 8);
        assert_eq!(parsed.new_name, "strictNewName");
        assert_eq!(parsed.dry_run, Some(false));
    }
}