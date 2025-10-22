//! Stdio transport implementation for MCP

use crate::McpDispatcher;
use codebuddy_foundation::core::model::mcp::{McpError, McpMessage, McpResponse};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use uuid::Uuid;

/// Frame delimiter used to separate JSON messages
/// Using a multi-character delimiter prevents confusion with newlines in error messages
const FRAME_DELIMITER: &[u8] = b"\n---FRAME---\n";

/// Stdio transport with message framing for reliable JSON parsing
pub struct StdioTransport<R, W> {
    reader: BufReader<R>,
    writer: W,
}

impl<R: tokio::io::AsyncRead + Unpin, W: tokio::io::AsyncWrite + Unpin> StdioTransport<R, W> {
    /// Create a new StdioTransport
    pub fn new(reader: R, writer: W) -> Self {
        Self {
            reader: BufReader::new(reader),
            writer,
        }
    }

    /// Read a single framed message from the input
    /// Returns None if EOF is reached
    pub async fn read_message(&mut self) -> Result<Option<String>, std::io::Error> {
        let mut buffer = Vec::new();
        let delimiter = FRAME_DELIMITER;

        loop {
            let bytes_read = self.reader.read_until(b'\n', &mut buffer).await?;

            if bytes_read == 0 {
                // EOF reached
                if buffer.is_empty() {
                    return Ok(None);
                }
                // Return whatever we have buffered
                return Ok(Some(String::from_utf8_lossy(&buffer).to_string()));
            }

            // Check if we've reached the delimiter
            if buffer.ends_with(delimiter) {
                // Remove the delimiter
                buffer.truncate(buffer.len() - delimiter.len());
                let message = String::from_utf8_lossy(&buffer).trim().to_string();
                return Ok(Some(message));
            }
        }
    }

    /// Write a framed message to the output
    pub async fn write_message(&mut self, message: &str) -> Result<(), std::io::Error> {
        self.writer.write_all(message.as_bytes()).await?;
        self.writer.write_all(FRAME_DELIMITER).await?;
        self.writer.flush().await?;
        Ok(())
    }
}

use crate::SessionInfo;

/// Start the stdio MCP server
pub async fn start_stdio_server(
    dispatcher: Arc<dyn McpDispatcher>,
) -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!("Starting stdio MCP server with framed transport");

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let mut transport = StdioTransport::new(stdin, stdout);

    tracing::info!("Codebuddy Server running on stdio");

    // For stdio, there is no user context, so we use a default SessionInfo.
    let session_info = SessionInfo::default();

    loop {
        let message = match transport.read_message().await {
            Ok(Some(msg)) => msg,
            Ok(None) => {
                // EOF reached
                tracing::info!("EOF reached, shutting down stdio server");
                break;
            }
            Err(e) => {
                tracing::error!(
                    error = %e,
                    "Error reading from stdin"
                );
                break;
            }
        };

        if message.trim().is_empty() {
            continue;
        }

        let request_id = Uuid::new_v4();

        // Create request span for automatic context propagation
        let span = codebuddy_config::logging::request_span(&request_id.to_string(), "stdio");
        let _enter = span.enter();

        tracing::debug!(message_length = message.len(), "Received framed message");

        // Parse the JSON-RPC message
        let mcp_message: McpMessage = match serde_json::from_str(&message) {
            Ok(msg) => msg,
            Err(e) => {
                tracing::error!(
                    request_id = %request_id,
                    error = %e,
                    message_preview = &message[..message.len().min(100)],
                    "Failed to parse MCP message"
                );
                let error_response = McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id: None,
                    result: None,
                    error: Some(McpError {
                        code: -32700,
                        message: format!("Parse error: {}", e),
                        data: None,
                    }),
                };

                let response_json = serde_json::to_string(&McpMessage::Response(error_response))?;
                transport.write_message(&response_json).await?;
                continue;
            }
        };

        // Extract the ID from the original message for error responses
        let message_id = match &mcp_message {
            McpMessage::Request(req) => req.id.clone(),
            _ => None,
        };

        // Handle the message
        let response = match dispatcher.dispatch(mcp_message, &session_info).await {
            Ok(response) => response,
            Err(e) => {
                // Convert to structured API error
                let api_error = e.to_api_response();

                tracing::error!(
                    request_id = %request_id,
                    error_code = %api_error.code,
                    error = %e,
                    "Failed to handle message"
                );

                // Serialize the structured error to JSON for the data field
                let error_data = serde_json::to_value(&api_error).ok();

                McpMessage::Response(McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id: message_id,
                    result: None,
                    error: Some(McpError {
                        code: -1,
                        message: api_error.message.clone(),
                        data: error_data,
                    }),
                })
            }
        };

        // Send response with framing
        let response_json = serde_json::to_string(&response)?;
        transport.write_message(&response_json).await?;
    }

    tracing::info!("Stdio server stopped");
    Ok(())
}
