//! LSP client implementation for communicating with a single LSP server

use crate::error::{ServerError, ServerResult};
use cb_core::config::LspServerConfig;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{mpsc, oneshot, Mutex};
use tokio::time::{timeout, Duration};
use tracing::{debug, error, info, warn};

/// Timeout for LSP requests
const LSP_REQUEST_TIMEOUT: Duration = Duration::from_secs(60);  // Increased for slow language servers
/// Timeout for LSP initialization
const LSP_INIT_TIMEOUT: Duration = Duration::from_secs(60);  // Increased significantly for slow language servers like Python
/// Buffer size for message channels
const CHANNEL_BUFFER_SIZE: usize = 1000;

/// LSP client for communicating with a single LSP server process
pub struct LspClient {
    /// Child process handle
    process: Arc<Mutex<Child>>,
    /// Channel for sending messages (requests and notifications) to the LSP server
    message_tx: mpsc::Sender<LspMessage>,
    /// Pending requests waiting for responses
    pending_requests: Arc<Mutex<HashMap<i64, oneshot::Sender<Result<Value, String>>>>>,
    /// Next request ID
    next_id: Arc<Mutex<i64>>,
    /// Whether the client has been initialized
    initialized: Arc<Mutex<bool>>,
    /// Server configuration
    config: LspServerConfig,
}

/// Internal message types for LSP communication
#[derive(Debug)]
enum LspMessage {
    Request {
        id: i64,
        method: String,
        params: Value,
        response_tx: oneshot::Sender<Result<Value, String>>,
    },
    Notification {
        method: String,
        params: Value,
    },
}

/// Internal request structure (kept for compatibility)
#[derive(Debug)]
struct LspRequest {
    id: i64,
    method: String,
    params: Value,
    response_tx: oneshot::Sender<Result<Value, String>>,
}

impl LspClient {
    /// Create a new LSP client and start the server process
    pub async fn new(config: LspServerConfig) -> ServerResult<Self> {
        if config.command.is_empty() {
            return Err(ServerError::config("LSP server command cannot be empty"));
        }

        let (command, args) = config.command.split_first().unwrap();

        // Start the LSP server process
        let mut child = Command::new(command)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .current_dir(config.root_dir.as_deref().unwrap_or(&std::env::current_dir()?))
            .spawn()
            .map_err(|e| {
                ServerError::runtime(format!(
                    "Failed to start LSP server '{}': {}",
                    config.command.join(" "),
                    e
                ))
            })?;

        // Take ownership of stdin/stdout/stderr
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| ServerError::runtime("Failed to get stdin for LSP server"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| ServerError::runtime("Failed to get stdout for LSP server"))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| ServerError::runtime("Failed to get stderr for LSP server"))?;

        let process = Arc::new(Mutex::new(child));
        let pending_requests = Arc::new(Mutex::new(HashMap::new()));
        let next_id = Arc::new(Mutex::new(1));
        let initialized = Arc::new(Mutex::new(false));

        // Create message channel for both requests and notifications
        let (message_tx, mut message_rx) = mpsc::channel::<LspMessage>(CHANNEL_BUFFER_SIZE);

        // Spawn task to handle writing to LSP server
        let stdin = Arc::new(Mutex::new(stdin));
        let write_stdin = stdin.clone();
        tokio::spawn(async move {
            let mut stdin = write_stdin.lock().await;
            while let Some(message) = message_rx.recv().await {
                let lsp_message = match &message {
                    LspMessage::Request { id, method, params, .. } => {
                        json!({
                            "jsonrpc": "2.0",
                            "id": id,
                            "method": method,
                            "params": params
                        })
                    },
                    LspMessage::Notification { method, params } => {
                        json!({
                            "jsonrpc": "2.0",
                            "method": method,
                            "params": params
                        })
                    }
                };

                let content = serde_json::to_string(&lsp_message).unwrap();
                let message_str = format!("Content-Length: {}\r\n\r\n{}", content.len(), content);

                if let Err(e) = stdin.write_all(message_str.as_bytes()).await {
                    error!("Failed to write to LSP server: {}", e);
                    if let LspMessage::Request { response_tx, .. } = message {
                        let _ = response_tx.send(Err(format!("Write error: {}", e)));
                    }
                    break;
                }

                if let Err(e) = stdin.flush().await {
                    error!("Failed to flush LSP server stdin: {}", e);
                    if let LspMessage::Request { response_tx, .. } = message {
                        let _ = response_tx.send(Err(format!("Flush error: {}", e)));
                    }
                    break;
                }

                match &message {
                    LspMessage::Request { method, .. } => debug!("Sent LSP request: {}", method),
                    LspMessage::Notification { method, .. } => debug!("Sent LSP notification: {}", method),
                }
            }
        });

        // Spawn stderr reader task to prevent blocking
        tokio::spawn(async move {
            let mut stderr_reader = BufReader::new(stderr);
            let mut stderr_line = String::new();
            while stderr_reader.read_line(&mut stderr_line).await.is_ok() {
                if !stderr_line.is_empty() {
                    // Log stderr output at debug level (most LSPs write diagnostics here)
                    debug!("LSP stderr: {}", stderr_line.trim());
                    stderr_line.clear();
                }
            }
        });

        // Spawn task to handle reading from LSP server
        let pending_requests_clone = pending_requests.clone();
        tokio::spawn(async move {
            let mut reader = BufReader::new(stdout);
            let mut buffer = String::new();

            loop {
                buffer.clear();
                match reader.read_line(&mut buffer).await {
                    Ok(0) => {
                        debug!("LSP server stdout closed");
                        break;
                    }
                    Ok(_) => {
                        let line = buffer.trim();
                        if let Some(content_length) = Self::parse_content_length(line) {
                            // Read the JSON message
                            if let Ok(message) = Self::read_json_message(&mut reader, content_length).await {
                                Self::handle_message(message, &pending_requests_clone).await;
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to read from LSP server: {}", e);
                        break;
                    }
                }
            }
        });

        let client = Self {
            process,
            message_tx,
            pending_requests,
            next_id,
            initialized,
            config,
        };

        // Initialize the LSP server
        client.initialize().await?;

        Ok(client)
    }

    /// Send a request to the LSP server and await the response
    pub async fn send_request(&self, method: &str, params: Value) -> ServerResult<Value> {
        // For file-specific operations, ensure the file is open in the LSP server
        if method.starts_with("textDocument/") && method != "textDocument/didOpen" {
            info!("DEBUG: Processing {} request, checking if file needs to be opened", method);
            // Extract file path from params to open it if needed
            if let Some(uri) = params.get("textDocument").and_then(|td| td.get("uri")).and_then(|u| u.as_str()) {
                info!("DEBUG: Found URI in params: {}", uri);
                if uri.starts_with("file://") {
                    let file_path = std::path::Path::new(&uri[7..]);
                    info!("DEBUG: Opening file in LSP: {}", file_path.display());
                    // Notify file opened (will be no-op if already open)
                    if let Err(e) = self.notify_file_opened(file_path).await {
                        warn!("DEBUG: Failed to open file before request: {}", e);
                        // Continue anyway - some operations might work without it
                    } else {
                        info!("DEBUG: Successfully opened file in LSP");
                    }
                }
            } else {
                warn!("DEBUG: No textDocument.uri found in params for {}: {:?}", method, params);
            }
        }

        let id = {
            let mut next_id = self.next_id.lock().await;
            let id = *next_id;
            *next_id += 1;
            id
        };

        let (response_tx, response_rx) = oneshot::channel();

        // Store the pending request
        {
            let mut pending = self.pending_requests.lock().await;
            pending.insert(id, response_tx);
        }

        // Create a dummy tx for the message (the real one is already in pending_requests)
        let (dummy_tx, _) = oneshot::channel();

        // Create and send the request message
        let message = LspMessage::Request {
            id,
            method: method.to_string(),
            params,
            response_tx: dummy_tx, // Use dummy since real one is in pending_requests
        };

        if let Err(e) = self.message_tx.send(message).await {
            // Remove from pending requests
            let mut pending = self.pending_requests.lock().await;
            pending.remove(&id);
            return Err(ServerError::runtime(format!("Failed to send request: {}", e)));
        }

        // Wait for response with timeout
        match timeout(LSP_REQUEST_TIMEOUT, response_rx).await {
            Ok(Ok(Ok(result))) => Ok(result),
            Ok(Ok(Err(error))) => Err(ServerError::runtime(format!("LSP error: {}", error))),
            Ok(Err(_)) => {
                // Remove from pending requests
                let mut pending = self.pending_requests.lock().await;
                pending.remove(&id);
                Err(ServerError::runtime("Response channel closed"))
            }
            Err(_) => {
                // Remove from pending requests
                let mut pending = self.pending_requests.lock().await;
                pending.remove(&id);
                Err(ServerError::runtime("Request timeout"))
            }
        }
    }

    /// Initialize the LSP server
    async fn initialize(&self) -> ServerResult<()> {
        let initialize_params = json!({
            "processId": std::process::id(),
            "clientInfo": {
                "name": "codeflow-buddy",
                "version": "0.1.0"
            },
            "capabilities": {
                "textDocument": {
                    "synchronization": {
                        "didOpen": true,
                        "didChange": true,
                        "didClose": true
                    },
                    "definition": {
                        "linkSupport": false
                    },
                    "references": {
                        "includeDeclaration": true,
                        "dynamicRegistration": false
                    },
                    "rename": {
                        "prepareSupport": false
                    },
                    "completion": {
                        "completionItem": {
                            "snippetSupport": true
                        }
                    },
                    "hover": {},
                    "signatureHelp": {},
                    "diagnostic": {
                        "dynamicRegistration": false,
                        "relatedDocumentSupport": false
                    }
                },
                "workspace": {
                    "workspaceEdit": {
                        "documentChanges": true
                    },
                    "workspaceFolders": true
                }
            },
            "rootUri": format!("file://{}",
                self.config.root_dir.as_deref()
                    .unwrap_or(&std::env::current_dir().unwrap())
                    .display()),
            "workspaceFolders": [{
                "uri": format!("file://{}",
                    self.config.root_dir.as_deref()
                        .unwrap_or(&std::env::current_dir().unwrap())
                        .display()),
                "name": "workspace"
            }]
        });

        debug!("LSP initialize params: {:?}", initialize_params);

        // Send initialize request
        let result = timeout(
            LSP_INIT_TIMEOUT,
            self.send_request("initialize", initialize_params),
        )
        .await
        .map_err(|_| ServerError::runtime("LSP initialization timeout"))??;

        debug!("LSP server initialized with result: {:?}", result);

        // Send initialized notification
        self.send_notification("initialized", json!({})).await?;

        // Mark as initialized
        {
            let mut initialized = self.initialized.lock().await;
            *initialized = true;
        }

        info!("LSP server initialized successfully: {}", self.config.command.join(" "));

        Ok(())
    }

    /// Send a notification to the LSP server (no response expected)
    pub async fn send_notification(&self, method: &str, params: Value) -> ServerResult<()> {
        // Create and send the notification message
        let message = LspMessage::Notification {
            method: method.to_string(),
            params,
        };

        if let Err(e) = self.message_tx.send(message).await {
            return Err(ServerError::runtime(format!("Failed to send notification: {}", e)));
        }

        debug!("Queued LSP notification: {}", method);
        Ok(())
    }

    /// Check if the client has been initialized
    pub async fn is_initialized(&self) -> bool {
        *self.initialized.lock().await
    }

    /// Get the server configuration
    pub fn config(&self) -> &LspServerConfig {
        &self.config
    }

    /// Notify the LSP server that a file has been opened
    pub async fn notify_file_opened(&self, file_path: &std::path::Path) -> ServerResult<()> {
        if !self.is_initialized().await {
            return Err(ServerError::runtime("LSP client not initialized"));
        }

        // Read file content
        let content = match tokio::fs::read_to_string(file_path).await {
            Ok(content) => content,
            Err(e) => {
                warn!("Failed to read file for didOpen notification: {}", e);
                return Ok(()); // Don't fail the whole operation
            }
        };

        // Get file extension for language ID
        let language_id = file_path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| match ext {
                "ts" => "typescript",
                "tsx" => "typescriptreact",
                "js" => "javascript",
                "jsx" => "javascriptreact",
                "py" => "python",
                "rs" => "rust",
                "go" => "go",
                _ => ext,
            })
            .unwrap_or("plaintext");

        let params = json!({
            "textDocument": {
                "uri": format!("file://{}", file_path.display()),
                "languageId": language_id,
                "version": 1,
                "text": content
            }
        });

        self.send_notification("textDocument/didOpen", params).await?;
        debug!("Sent didOpen notification for file: {}", file_path.display());

        Ok(())
    }

    /// Kill the LSP server process
    pub async fn kill(&self) -> ServerResult<()> {
        let mut process = self.process.lock().await;
        if let Err(e) = process.kill().await {
            warn!("Failed to kill LSP server process: {}", e);
        }
        Ok(())
    }

    /// Parse Content-Length header from LSP message
    fn parse_content_length(line: &str) -> Option<usize> {
        if line.starts_with("Content-Length: ") {
            line["Content-Length: ".len()..].parse().ok()
        } else {
            None
        }
    }

    /// Read JSON message with specified content length
    async fn read_json_message(
        reader: &mut BufReader<tokio::process::ChildStdout>,
        content_length: usize,
    ) -> Result<Value, String> {
        // Skip the empty line
        let mut buffer = String::new();
        if let Err(e) = reader.read_line(&mut buffer).await {
            return Err(format!("Failed to read separator line: {}", e));
        }

        // Read the JSON content
        let mut json_buffer = vec![0u8; content_length];
        if let Err(e) = tokio::io::AsyncReadExt::read_exact(reader, &mut json_buffer).await {
            return Err(format!("Failed to read JSON content: {}", e));
        }

        let json_str = String::from_utf8(json_buffer)
            .map_err(|e| format!("Invalid UTF-8 in JSON content: {}", e))?;

        serde_json::from_str(&json_str)
            .map_err(|e| format!("Failed to parse JSON: {}", e))
    }

    /// Handle incoming message from LSP server
    async fn handle_message(
        message: Value,
        pending_requests: &Arc<Mutex<HashMap<i64, oneshot::Sender<Result<Value, String>>>>>,
    ) {
        if let Some(id) = message.get("id") {
            if let Some(id_num) = id.as_i64() {
                let sender = {
                    let mut pending = pending_requests.lock().await;
                    pending.remove(&id_num)
                };

                if let Some(sender) = sender {
                    if message.get("error").is_some() {
                        let error_msg = message["error"]["message"]
                            .as_str()
                            .unwrap_or("Unknown error")
                            .to_string();
                        let _ = sender.send(Err(error_msg));
                    } else if let Some(result) = message.get("result") {
                        let _ = sender.send(Ok(result.clone()));
                    } else {
                        let _ = sender.send(Err("Invalid response format".to_string()));
                    }
                }
            }
        } else if message.get("method").is_some() {
            // Handle notifications from server
            debug!("Received notification from LSP server: {:?}", message);
        }
    }
}

impl Drop for LspClient {
    fn drop(&mut self) {
        // Kill the process when the client is dropped
        let process = self.process.clone();
        tokio::spawn(async move {
            let mut process = process.lock().await;
            if let Err(e) = process.kill().await {
                warn!("Failed to kill LSP server process on drop: {}", e);
            }
            // IMPORTANT: Wait for the process to actually exit to avoid zombies
            let _ = process.wait().await;
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config() -> LspServerConfig {
        LspServerConfig {
            extensions: vec!["py".to_string()],
            command: vec!["echo".to_string(), "test".to_string()], // Use echo for testing
            root_dir: None,
            restart_interval: None,
        }
    }

    #[tokio::test]
    async fn test_lsp_client_creation() {
        let config = create_test_config();

        // This will fail because echo is not an LSP server, but we can test the creation logic
        let result = LspClient::new(config).await;
        assert!(result.is_err()); // Expected to fail during initialization
    }

    #[test]
    fn test_parse_content_length() {
        assert_eq!(LspClient::parse_content_length("Content-Length: 123"), Some(123));
        assert_eq!(LspClient::parse_content_length("Content-Length: 0"), Some(0));
        assert_eq!(LspClient::parse_content_length("Other header"), None);
        assert_eq!(LspClient::parse_content_length("Content-Length: invalid"), None);
    }
}