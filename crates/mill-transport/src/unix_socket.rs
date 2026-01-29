//! Unix socket transport for daemon mode
//!
//! This module provides a Unix domain socket transport for the daemon,
//! allowing persistent LSP server reuse across CLI invocations.

use crate::McpDispatcher;
use mill_foundation::core::model::mcp::{McpError, McpMessage, McpResponse};
use mill_foundation::errors::ErrorResponse;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tracing::{debug, error, info, warn};

use crate::SessionInfo;

/// Default socket path in user's home directory
pub fn default_socket_path() -> PathBuf {
    dirs::home_dir()
        .map(|h| h.join(".typemill").join("daemon.sock"))
        .unwrap_or_else(|| PathBuf::from("/tmp/typemill-daemon.sock"))
}

/// Check if a daemon is running by trying to connect to the socket
pub async fn is_daemon_running(socket_path: &Path) -> bool {
    if !socket_path.exists() {
        return false;
    }
    // Try to connect - if successful, daemon is running
    UnixStream::connect(socket_path).await.is_ok()
}

/// Unix socket server for the daemon
pub struct UnixSocketServer {
    listener: UnixListener,
    socket_path: PathBuf,
}

impl UnixSocketServer {
    /// Create a new Unix socket server
    pub async fn bind(socket_path: &Path) -> std::io::Result<Self> {
        // Remove stale socket file if it exists
        if socket_path.exists() {
            if is_daemon_running(socket_path).await {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::AddrInUse,
                    "Daemon is already running",
                ));
            }
            std::fs::remove_file(socket_path)?;
        }

        // Ensure parent directory exists
        if let Some(parent) = socket_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let listener = UnixListener::bind(socket_path)?;

        // Set permissions to 0600 (owner only) to prevent unauthorized access
        std::fs::set_permissions(socket_path, std::fs::Permissions::from_mode(0o600))?;

        info!(socket_path = %socket_path.display(), "Unix socket server bound");

        Ok(Self {
            listener,
            socket_path: socket_path.to_path_buf(),
        })
    }

    /// Run the server loop, handling incoming connections
    pub async fn run(self, dispatcher: Arc<dyn McpDispatcher>) -> std::io::Result<()> {
        info!("Unix socket daemon started, listening for connections");

        loop {
            match self.listener.accept().await {
                Ok((stream, _addr)) => {
                    let dispatcher = dispatcher.clone();
                    tokio::spawn(async move {
                        if let Err(e) = handle_connection(stream, dispatcher).await {
                            error!(error = %e, "Connection handler error");
                        }
                    });
                }
                Err(e) => {
                    error!(error = %e, "Failed to accept connection");
                }
            }
        }
    }

    /// Get the socket path
    pub fn socket_path(&self) -> &Path {
        &self.socket_path
    }
}

impl Drop for UnixSocketServer {
    fn drop(&mut self) {
        // Clean up socket file
        if self.socket_path.exists() {
            if let Err(e) = std::fs::remove_file(&self.socket_path) {
                warn!(
                    socket_path = %self.socket_path.display(),
                    error = %e,
                    "Failed to remove socket file on shutdown"
                );
            }
        }
    }
}

/// Handle a single client connection
async fn handle_connection(
    stream: UnixStream,
    dispatcher: Arc<dyn McpDispatcher>,
) -> std::io::Result<()> {
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let session_info = SessionInfo::default();

    debug!("New daemon client connected");

    loop {
        let mut line = String::new();
        let bytes_read = reader.read_line(&mut line).await?;

        if bytes_read == 0 {
            debug!("Client disconnected");
            break;
        }

        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // Parse the MCP message
        let mcp_message: McpMessage = match serde_json::from_str(line) {
            Ok(msg) => msg,
            Err(e) => {
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
                writer.write_all(response_json.as_bytes()).await?;
                writer.write_all(b"\n").await?;
                writer.flush().await?;
                continue;
            }
        };

        // Extract message ID for error responses
        let message_id = match &mcp_message {
            McpMessage::Request(req) => req.id.clone(),
            _ => None,
        };

        // Dispatch the message
        let response = match dispatcher.dispatch(mcp_message, &session_info).await {
            Ok(response) => response,
            Err(e) => {
                let api_error: ErrorResponse = e.into();
                let error_data = serde_json::to_value(&api_error).ok();

                McpMessage::Response(McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id: message_id,
                    result: None,
                    error: Some(McpError {
                        code: -1,
                        message: api_error.message,
                        data: error_data,
                    }),
                })
            }
        };

        // Send response
        let response_json = serde_json::to_string(&response)?;
        writer.write_all(response_json.as_bytes()).await?;
        writer.write_all(b"\n").await?;
        writer.flush().await?;
    }

    Ok(())
}

/// Unix socket client for connecting to the daemon
pub struct UnixSocketClient {
    stream: UnixStream,
}

impl UnixSocketClient {
    /// Connect to a running daemon
    pub async fn connect(socket_path: &Path) -> std::io::Result<Self> {
        let stream = UnixStream::connect(socket_path).await?;
        debug!(socket_path = %socket_path.display(), "Connected to daemon");
        Ok(Self { stream })
    }

    /// Send an MCP message and receive the response
    pub async fn call(&mut self, message: McpMessage) -> std::io::Result<McpMessage> {
        let (reader, mut writer) = self.stream.split();
        let mut reader = BufReader::new(reader);

        // Send request
        let request_json = serde_json::to_string(&message)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        writer.write_all(request_json.as_bytes()).await?;
        writer.write_all(b"\n").await?;
        writer.flush().await?;

        // Read response
        let mut response_line = String::new();
        reader.read_line(&mut response_line).await?;

        let response: McpMessage = serde_json::from_str(&response_line)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_socket_path_default() {
        let path = default_socket_path();
        assert!(path.to_string_lossy().contains("typemill"));
        assert!(path.to_string_lossy().contains("daemon.sock"));
    }

    #[tokio::test]
    async fn test_is_daemon_running_no_socket() {
        let temp_dir = TempDir::new().unwrap();
        let socket_path = temp_dir.path().join("nonexistent.sock");
        assert!(!is_daemon_running(&socket_path).await);
    }
}
