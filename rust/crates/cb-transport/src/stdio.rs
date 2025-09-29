//! Stdio transport implementation for MCP

use crate::McpDispatcher;
use cb_core::model::mcp::{McpError, McpMessage, McpResponse};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use uuid::Uuid;

/// Start the stdio MCP server
pub async fn start_stdio_server(
    dispatcher: Arc<dyn McpDispatcher>,
) -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!("Starting stdio MCP server");

    let stdin = tokio::io::stdin();
    let mut stdout = tokio::io::stdout();
    let mut reader = BufReader::new(stdin);
    let mut line = String::new();

    tracing::info!("Codebuddy Server running on stdio");
    eprintln!("Codebuddy Server running on stdio");

    loop {
        line.clear();
        match reader.read_line(&mut line).await {
            Ok(0) => {
                // EOF reached
                tracing::info!("EOF reached, shutting down stdio server");
                break;
            }
            Ok(_) => {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }

                let request_id = Uuid::new_v4();
                tracing::debug!(
                    request_id = %request_id,
                    line_length = trimmed.len(),
                    "Received line"
                );

                // Parse the JSON-RPC message
                let mcp_message: McpMessage = match serde_json::from_str(trimmed) {
                    Ok(msg) => msg,
                    Err(e) => {
                        tracing::error!(
                            request_id = %request_id,
                            error = %e,
                            "Failed to parse MCP message"
                        );
                        let error_response = McpResponse {
                            jsonrpc: "2.0".to_string(),
                            id: None,
                            result: None,
                            error: Some(McpError {
                                code: -32700,
                                message: "Parse error".to_string(),
                                data: None,
                            }),
                        };

                        let response_json =
                            serde_json::to_string(&McpMessage::Response(error_response))?;
                        stdout.write_all(response_json.as_bytes()).await?;
                        stdout.write_all(b"\n").await?;
                        stdout.flush().await?;
                        continue;
                    }
                };

                // Extract the ID from the original message for error responses
                let message_id = match &mcp_message {
                    McpMessage::Request(req) => req.id.clone(),
                    _ => None,
                };

                // Handle the message
                let response = match dispatcher.dispatch(mcp_message).await {
                    Ok(response) => response,
                    Err(e) => {
                        tracing::error!(
                            request_id = %request_id,
                            error = %e,
                            "Failed to handle message"
                        );
                        McpMessage::Response(McpResponse {
                            jsonrpc: "2.0".to_string(),
                            id: message_id,
                            result: None,
                            error: Some(McpError {
                                code: -1,
                                message: e.to_string(),
                                data: None,
                            }),
                        })
                    }
                };

                // Send response
                let response_json = serde_json::to_string(&response)?;
                stdout.write_all(response_json.as_bytes()).await?;
                stdout.write_all(b"\n").await?;
                stdout.flush().await?;
            }
            Err(e) => {
                tracing::error!(
                    error = %e,
                    "Error reading from stdin"
                );
                break;
            }
        }
    }

    tracing::info!("Stdio server stopped");
    Ok(())
}
