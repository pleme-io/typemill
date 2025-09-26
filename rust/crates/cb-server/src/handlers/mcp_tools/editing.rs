//! Editing MCP tools (rename_symbol, format_document, etc.)

use crate::handlers::McpDispatcher;
use serde::{Deserialize, Serialize};
use serde_json::json;

/// Arguments for rename_symbol tool
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
struct RenameSymbolArgs {
    file_path: String,
    symbol_name: String,
    new_name: String,
    symbol_kind: Option<String>,
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

/// Register editing tools
pub fn register_tools(dispatcher: &mut McpDispatcher) {
    // rename_symbol tool
    dispatcher.register_tool("rename_symbol".to_string(), |args| async move {
        let params: RenameSymbolArgs = serde_json::from_value(args)
            .map_err(|e| crate::error::ServerError::InvalidRequest(format!("Invalid args: {}", e)))?;

        tracing::debug!(
            "Renaming {} to {} in {}",
            params.symbol_name,
            params.new_name,
            params.file_path
        );

        let is_dry_run = params.dry_run.unwrap_or(false);

        // Mock rename operation
        let edits = vec![
            FileEdit {
                file_path: params.file_path.clone(),
                edits: vec![
                    TextEdit {
                        range: TextRange {
                            start: Position { line: 10, character: 5 },
                            end: Position { line: 10, character: 5 + params.symbol_name.len() as u32 },
                        },
                        new_text: params.new_name.clone(),
                    },
                    TextEdit {
                        range: TextRange {
                            start: Position { line: 25, character: 10 },
                            end: Position { line: 25, character: 10 + params.symbol_name.len() as u32 },
                        },
                        new_text: params.new_name.clone(),
                    },
                ],
            },
        ];

        let result = EditResult {
            success: !is_dry_run,
            files_modified: if is_dry_run { vec![] } else { vec![params.file_path] },
            edits_count: 2,
            preview: if is_dry_run { Some(edits) } else { None },
        };

        Ok(serde_json::to_value(result)?)
    });

    // format_document tool
    dispatcher.register_tool("format_document".to_string(), |args| async move {
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

    // get_code_actions tool
    dispatcher.register_tool("get_code_actions".to_string(), |args| async move {
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
    dispatcher.register_tool("apply_workspace_edit".to_string(), |args| async move {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rename_symbol_args() {
        let args = json!({
            "file_path": "test.ts",
            "symbol_name": "oldName",
            "new_name": "newName",
            "dry_run": true
        });

        let parsed: RenameSymbolArgs = serde_json::from_value(args).unwrap();
        assert_eq!(parsed.file_path, "test.ts");
        assert_eq!(parsed.symbol_name, "oldName");
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
}