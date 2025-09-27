//! Diagnostics MCP tool (get_diagnostics)

use crate::handlers::McpDispatcher;
use cb_core::model::mcp::{McpMessage, McpRequest};
use serde::{Deserialize, Serialize};
use serde_json::json;

/// Arguments for get_diagnostics tool
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
struct GetDiagnosticsArgs {
    file_path: String,
}

/// Diagnostic information
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Diagnostic {
    range: DiagnosticRange,
    severity: Option<u32>,
    code: Option<DiagnosticCode>,
    code_description: Option<CodeDescription>,
    source: Option<String>,
    message: String,
    tags: Option<Vec<u32>>,
    related_information: Option<Vec<DiagnosticRelatedInformation>>,
    data: Option<serde_json::Value>,
}

/// Diagnostic range
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DiagnosticRange {
    start: DiagnosticPosition,
    end: DiagnosticPosition,
}

/// Diagnostic position
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DiagnosticPosition {
    line: u32,
    character: u32,
}

/// Diagnostic code (can be string or number)
#[derive(Debug, Serialize)]
#[serde(untagged)]
enum DiagnosticCode {
    String(String),
    Number(i32),
}

/// Code description with href
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CodeDescription {
    href: String,
}

/// Related diagnostic information
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DiagnosticRelatedInformation {
    location: DiagnosticLocation,
    message: String,
}

/// Diagnostic location
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DiagnosticLocation {
    uri: String,
    range: DiagnosticRange,
}

/// Diagnostics response
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DiagnosticsResponse {
    uri: String,
    version: Option<i32>,
    diagnostics: Vec<Diagnostic>,
    diagnostic_count: DiagnosticCounts,
}

/// Diagnostic counts by severity
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DiagnosticCounts {
    error: usize,
    warning: usize,
    information: usize,
    hint: usize,
    total: usize,
}

/// Register diagnostics tools
pub fn register_tools(dispatcher: &mut McpDispatcher) {
    // get_diagnostics tool
    dispatcher.register_tool("get_diagnostics".to_string(), |app_state, args| async move {
        let params: GetDiagnosticsArgs = serde_json::from_value(args)
            .map_err(|e| crate::error::ServerError::InvalidRequest(format!("Invalid args: {}", e)))?;

        tracing::debug!("Getting diagnostics for: {}", params.file_path);

        // Create LSP request for textDocument/diagnostic
        let lsp_request = McpRequest {
            id: Some(serde_json::Value::Number(serde_json::Number::from(1))),
            method: "textDocument/diagnostic".to_string(),
            params: Some(json!({
                "textDocument": {
                    "uri": format!("file://{}", params.file_path)
                }
            })),
        };

        // Send request to LSP service
        match app_state.lsp.request(McpMessage::Request(lsp_request)).await {
            Ok(McpMessage::Response(response)) => {
                if let Some(result) = response.result {
                    // Process the diagnostic result and add metadata
                    if let Some(diagnostics_array) = result["items"].as_array() {
                        let mut error_count = 0;
                        let mut warning_count = 0;
                        let mut information_count = 0;
                        let mut hint_count = 0;

                        // Count diagnostics by severity
                        for diagnostic in diagnostics_array {
                            if let Some(severity) = diagnostic["severity"].as_u64() {
                                match severity {
                                    1 => error_count += 1,     // Error
                                    2 => warning_count += 1,   // Warning
                                    3 => information_count += 1, // Information
                                    4 => hint_count += 1,      // Hint
                                    _ => {}
                                }
                            }
                        }

                        let total_count = diagnostics_array.len();

                        // Create enhanced response with counts
                        let enhanced_result = json!({
                            "uri": format!("file://{}", params.file_path),
                            "version": result["version"],
                            "diagnostics": diagnostics_array,
                            "diagnosticCount": {
                                "error": error_count,
                                "warning": warning_count,
                                "information": information_count,
                                "hint": hint_count,
                                "total": total_count
                            }
                        });

                        Ok(enhanced_result)
                    } else {
                        // Handle the case where result is direct diagnostics array
                        let empty_vec = Vec::new();
                        let diagnostics_array = result.as_array().unwrap_or(&empty_vec);
                        let mut error_count = 0;
                        let mut warning_count = 0;
                        let mut information_count = 0;
                        let mut hint_count = 0;

                        // Count diagnostics by severity
                        for diagnostic in diagnostics_array {
                            if let Some(severity) = diagnostic["severity"].as_u64() {
                                match severity {
                                    1 => error_count += 1,     // Error
                                    2 => warning_count += 1,   // Warning
                                    3 => information_count += 1, // Information
                                    4 => hint_count += 1,      // Hint
                                    _ => {}
                                }
                            }
                        }

                        let total_count = diagnostics_array.len();

                        let enhanced_result = json!({
                            "uri": format!("file://{}", params.file_path),
                            "version": null,
                            "diagnostics": diagnostics_array,
                            "diagnosticCount": {
                                "error": error_count,
                                "warning": warning_count,
                                "information": information_count,
                                "hint": hint_count,
                                "total": total_count
                            }
                        });

                        Ok(enhanced_result)
                    }
                } else if let Some(error) = response.error {
                    Err(crate::error::ServerError::runtime(format!("LSP error: {}", error.message)))
                } else {
                    // Return empty diagnostics if no result
                    Ok(json!({
                        "uri": format!("file://{}", params.file_path),
                        "version": null,
                        "diagnostics": [],
                        "diagnosticCount": {
                            "error": 0,
                            "warning": 0,
                            "information": 0,
                            "hint": 0,
                            "total": 0
                        }
                    }))
                }
            }
            Ok(_) => Err(crate::error::ServerError::runtime("Unexpected LSP message type")),
            Err(e) => {
                tracing::warn!("LSP request failed, returning empty diagnostics: {}", e);
                // Return empty diagnostics on failure instead of erroring
                Ok(json!({
                    "uri": format!("file://{}", params.file_path),
                    "version": null,
                    "diagnostics": [],
                    "diagnosticCount": {
                        "error": 0,
                        "warning": 0,
                        "information": 0,
                        "hint": 0,
                        "total": 0
                    },
                    "error": format!("Failed to get diagnostics: {}", e)
                }))
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_diagnostics_args() {
        let args = json!({
            "file_path": "src/main.ts"
        });

        let parsed: GetDiagnosticsArgs = serde_json::from_value(args).unwrap();
        assert_eq!(parsed.file_path, "src/main.ts");
    }

    #[tokio::test]
    async fn test_diagnostic_code_variants() {
        // Test string code
        let string_code = DiagnosticCode::String("TS2345".to_string());
        let string_json = serde_json::to_value(&string_code).unwrap();
        assert_eq!(string_json, json!("TS2345"));

        // Test number code
        let number_code = DiagnosticCode::Number(2345);
        let number_json = serde_json::to_value(&number_code).unwrap();
        assert_eq!(number_json, json!(2345));
    }

    #[tokio::test]
    async fn test_diagnostic_counts() {
        let counts = DiagnosticCounts {
            error: 2,
            warning: 3,
            information: 1,
            hint: 0,
            total: 6,
        };

        let json_value = serde_json::to_value(&counts).unwrap();
        assert_eq!(json_value["error"], 2);
        assert_eq!(json_value["warning"], 3);
        assert_eq!(json_value["total"], 6);
    }
}