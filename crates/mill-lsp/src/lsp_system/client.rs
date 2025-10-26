//! LSP client implementation for communicating with a single LSP server

use crate::progress::{ProgressError, ProgressManager, ProgressParams, ProgressToken};
use mill_config::LspServerConfig;
use mill_foundation::protocol::{ApiError as ServerError, ApiResult as ServerResult};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{mpsc, oneshot, Mutex};
use tokio::time::{timeout, Duration};
use tracing::{debug, info, warn};

/// Timeout for LSP requests
const LSP_REQUEST_TIMEOUT: Duration = Duration::from_secs(60); // Increased for slow language servers
/// Timeout for LSP initialization
const LSP_INIT_TIMEOUT: Duration = Duration::from_secs(60); // Increased significantly for slow language servers like Python
/// Buffer size for message channels
const CHANNEL_BUFFER_SIZE: usize = 1000;

/// Type alias for pending request responses
type PendingRequests = Arc<Mutex<HashMap<i64, oneshot::Sender<Result<Value, String>>>>>;

/// LSP client for communicating with a single LSP server process
pub struct LspClient {
    /// Child process handle
    process: Arc<Mutex<Child>>,
    /// Channel for sending messages (requests and notifications) to the LSP server
    message_tx: mpsc::Sender<LspMessage>,
    /// Pending requests waiting for responses
    pending_requests: PendingRequests,
    /// Next request ID
    next_id: Arc<Mutex<i64>>,
    /// Whether the client has been initialized
    initialized: Arc<Mutex<bool>>,
    /// Server configuration
    config: LspServerConfig,
    /// Progress notification manager
    progress_manager: ProgressManager,
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
    Response {
        id: Value,
        result: Value,
    },
    ErrorResponse {
        id: Value,
        error: Value,
    },
}

impl LspClient {
    /// Create a new LSP client and start the server process
    pub async fn new(config: LspServerConfig) -> ServerResult<Self> {
        if config.command.is_empty() {
            return Err(ServerError::config("LSP server command cannot be empty"));
        }

        let (command, args) = config
            .command
            .split_first()
            .expect("LSP server command is empty (should be caught by validation)");

        // Start the LSP server process
        // Debug logging for LSP server spawn
        let path_env = std::env::var("PATH").unwrap_or_else(|_| "NOT SET".to_string());
        let current_dir = std::env::current_dir()?;
        let root_dir = config.root_dir.as_deref().unwrap_or(&current_dir);

        tracing::debug!(
            command = %command,
            args = ?args,
            root_dir = ?root_dir,
            path_env = %path_env,
            "Attempting to spawn LSP server"
        );

        // Augment PATH to include common LSP installation locations
        let current_path = std::env::var("PATH").unwrap_or_default();

        // Build augmented PATH with common LSP locations
        let mut path_additions = vec![];

        // Add pipx bin directory (Linux/macOS)
        if let Ok(home) = std::env::var("HOME") {
            path_additions.push(format!("{}/.local/bin", home));
        }

        // Add npm global bin directory
        if let Ok(npm_bin) = std::env::var("NPM_CONFIG_PREFIX") {
            path_additions.push(format!("{}/bin", npm_bin));
        } else if let Ok(home) = std::env::var("HOME") {
            path_additions.push(format!("{}/.npm-global/bin", home));
        }

        // Add cargo bin directory
        if let Ok(cargo_home) = std::env::var("CARGO_HOME") {
            path_additions.push(format!("{}/bin", cargo_home));
        } else if let Ok(home) = std::env::var("HOME") {
            path_additions.push(format!("{}/.cargo/bin", home));
        }

        // Add NVM node bin directory (critical for typescript-language-server)
        // Instead of picking the first directory, check for the default/current version
        if let Ok(nvm_dir) = std::env::var("NVM_DIR") {
            // First try to read the default alias file to get the current version
            let default_version_path = format!("{}/alias/default", nvm_dir);
            let default_version = std::fs::read_to_string(&default_version_path)
                .ok()
                .map(|s| s.trim().to_string());

            if let Some(version_alias) = default_version {
                // Resolve the version (could be "22", "v22.20.0", etc.)
                let version_path = if version_alias.starts_with('v') {
                    format!("{}/versions/node/{}/bin", nvm_dir, version_alias)
                } else {
                    // If it's just "22", find the highest v22.x.x version
                    if let Ok(entries) = std::fs::read_dir(format!("{}/versions/node", nvm_dir)) {
                        entries
                            .filter_map(Result::ok)
                            .filter(|e| {
                                e.file_name()
                                    .to_string_lossy()
                                    .starts_with(&format!("v{}", version_alias))
                            })
                            .max_by_key(|e| e.file_name())
                            .map(|e| format!("{}/bin", e.path().display()))
                            .unwrap_or_default()
                    } else {
                        String::new()
                    }
                };

                if !version_path.is_empty() && std::path::Path::new(&version_path).exists() {
                    path_additions.push(version_path);
                }
            }
        } else if let Ok(home) = std::env::var("HOME") {
            // Fallback: try common NVM location with default version
            let nvm_default_path = format!("{}/.nvm/alias/default", home);
            if let Ok(default_version) = std::fs::read_to_string(&nvm_default_path) {
                let version = default_version.trim();
                if version.starts_with('v') {
                    let bin_path = format!("{}/.nvm/versions/node/{}/bin", home, version);
                    if std::path::Path::new(&bin_path).exists() {
                        path_additions.push(bin_path);
                    }
                }
            }
        }

        // Construct augmented PATH
        let augmented_path = if path_additions.is_empty() {
            current_path
        } else {
            format!("{}:{}", path_additions.join(":"), current_path)
        };

        tracing::debug!(
            augmented_path = %augmented_path,
            path_additions = ?path_additions,
            "Using augmented PATH for LSP server"
        );

        let mut child = Command::new(command)
            .args(args)
            .env("PATH", augmented_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .current_dir(root_dir)
            .spawn()
            .map_err(|e| {
                tracing::error!(
                    command = %command,
                    args = ?args,
                    error = %e,
                    path_env = %path_env,
                    "Failed to spawn LSP server"
                );
                ServerError::runtime(format!(
                    "Failed to start LSP server '{}': {}",
                    config.command.join(" "),
                    e
                ))
            })?;

        eprintln!(
            "✅ LSP server process spawned: {} (PID: {:?})",
            command,
            child.id()
        );
        tracing::debug!(
            command = %command,
            pid = child.id(),
            "LSP server process spawned successfully"
        );

        // Register the PID with the zombie reaper as a safety net
        if let Some(pid) = child.id() {
            use crate::lsp_system::zombie_reaper::ZOMBIE_REAPER;
            ZOMBIE_REAPER.register(pid as i32);
            tracing::debug!(pid = pid, "Registered LSP process with zombie reaper");
        }

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
        let progress_manager = ProgressManager::new();

        // Create message channel for both requests and notifications
        let (message_tx, mut message_rx) = mpsc::channel::<LspMessage>(CHANNEL_BUFFER_SIZE);

        // Spawn task to handle writing to LSP server
        let stdin = Arc::new(Mutex::new(stdin));
        let write_stdin = stdin.clone();
        tokio::spawn(async move {
            let mut stdin = write_stdin.lock().await;
            while let Some(message) = message_rx.recv().await {
                let lsp_message = match &message {
                    LspMessage::Request {
                        id, method, params, ..
                    } => {
                        json!({
                            "jsonrpc": "2.0",
                            "id": id,
                            "method": method,
                            "params": params
                        })
                    }
                    LspMessage::Notification { method, params } => {
                        json!({
                            "jsonrpc": "2.0",
                            "method": method,
                            "params": params
                        })
                    }
                    LspMessage::Response { id, result } => {
                        json!({
                            "jsonrpc": "2.0",
                            "id": id,
                            "result": result
                        })
                    }
                    LspMessage::ErrorResponse { id, error } => {
                        json!({
                            "jsonrpc": "2.0",
                            "id": id,
                            "error": error
                        })
                    }
                };

                let content = serde_json::to_string(&lsp_message)
                    .expect("LSP message serialization should never fail for valid JSON types");
                let message_str = format!("Content-Length: {}\r\n\r\n{}", content.len(), content);

                if let Err(e) = stdin.write_all(message_str.as_bytes()).await {
                    tracing::error!(
                        error_category = "lsp_communication",
                        error = %e,
                        "Failed to write to LSP server"
                    );
                    if let LspMessage::Request { response_tx, .. } = message {
                        let _ = response_tx.send(Err(format!("Write error: {}", e)));
                    }
                    break;
                }

                if let Err(e) = stdin.flush().await {
                    tracing::error!(
                        error_category = "lsp_communication",
                        error = %e,
                        "Failed to flush LSP server stdin"
                    );
                    if let LspMessage::Request { response_tx, .. } = message {
                        let _ = response_tx.send(Err(format!("Flush error: {}", e)));
                    }
                    break;
                }

                match &message {
                    LspMessage::Request { method, id, .. } => {
                        if method == "initialize" {
                            tracing::warn!(method = %method, id = %id, "Sent LSP initialize request to server");
                        } else {
                            debug!(method = %method, "Sent LSP request");
                        }
                    }
                    LspMessage::Notification { method, .. } => {
                        if method == "initialized" {
                            tracing::warn!(method = %method, "Sent LSP initialized notification");
                        } else {
                            debug!(method = %method, "Sent LSP notification");
                        }
                    }
                    LspMessage::Response { id, .. } => {
                        debug!(id = ?id, "Sent LSP response to server request");
                    }
                    LspMessage::ErrorResponse { id, .. } => {
                        debug!(id = ?id, "Sent LSP error response to server request");
                    }
                }
            }
        });

        // Spawn stderr reader task to prevent blocking
        let server_command = command.to_string();
        tokio::spawn(async move {
            eprintln!("🔍 LSP stderr reader task started for: {}", server_command);
            let mut stderr_reader = BufReader::new(stderr);
            let mut stderr_line = String::new();
            let mut line_count = 0;
            while stderr_reader.read_line(&mut stderr_line).await.is_ok() {
                if !stderr_line.is_empty() {
                    line_count += 1;
                    let trimmed = stderr_line.trim();
                    eprintln!("📢 LSP STDERR [{}]: {}", server_command, trimmed);
                    // Log stderr at ERROR level so we always see crashes/errors
                    // Regular diagnostics at debug level
                    if trimmed.contains("error")
                        || trimmed.contains("Error")
                        || trimmed.contains("ERROR")
                        || trimmed.contains("fatal")
                        || trimmed.contains("panic")
                        || trimmed.contains("crash")
                    {
                        tracing::error!(server = %server_command, stderr = %trimmed, "LSP stderr ERROR");
                    } else {
                        tracing::warn!(server = %server_command, stderr = %trimmed, "LSP stderr");
                    }
                    stderr_line.clear();
                } else {
                    break; // EOF
                }
            }
            eprintln!(
                "🛑 LSP stderr reader task ended for: {} (read {} lines)",
                server_command, line_count
            );
        });

        // Spawn task to handle reading from LSP server
        let pending_requests_clone = pending_requests.clone();
        let server_command_stdout = command.to_string();
        let message_tx_clone = message_tx.clone();
        let progress_manager_clone = progress_manager.clone();
        tokio::spawn(async move {
            eprintln!(
                "🔍 LSP stdout reader task started for: {}",
                server_command_stdout
            );
            let mut reader = BufReader::new(stdout);
            let mut buffer = String::new();
            let mut message_count = 0;

            loop {
                buffer.clear();
                match reader.read_line(&mut buffer).await {
                    Ok(0) => {
                        eprintln!(
                            "🛑 LSP stdout closed for: {} (read {} messages)",
                            server_command_stdout, message_count
                        );
                        debug!("LSP server stdout closed");
                        break;
                    }
                    Ok(_) => {
                        let line = buffer.trim();
                        // Parse Content-Length from any header line
                        let content_length_opt = Self::parse_content_length(line);

                        // If we found Content-Length, skip remaining headers and read message
                        if let Some(content_length) = content_length_opt {
                            // Read remaining header lines until we reach the empty line
                            // (LSP spec allows optional headers like Content-Type)
                            // Note: read_json_message will consume the empty line itself
                            loop {
                                buffer.clear();
                                match reader.read_line(&mut buffer).await {
                                    Ok(0) => break, // EOF
                                    Ok(_) => {
                                        if buffer.trim().is_empty() {
                                            // Found empty line - DON'T consume it, let read_json_message handle it
                                            // Actually we just consumed it, so we're at the right position
                                            break;
                                        }
                                        // Continue reading and discarding additional headers
                                    }
                                    Err(_) => break,
                                }
                            }

                            // Read the JSON message (expects to be positioned after empty line)
                            if let Ok(message) =
                                Self::read_json_message(&mut reader, content_length).await
                            {
                                message_count += 1;
                                eprintln!(
                                    "📨 LSP received message #{}: {:?}",
                                    message_count, message
                                );
                                Self::handle_message(
                                    message,
                                    &pending_requests_clone,
                                    &message_tx_clone,
                                    &progress_manager_clone,
                                )
                                .await;
                            }
                        }
                        // Otherwise, skip non-Content-Length headers and continue
                    }
                    Err(e) => {
                        tracing::error!(
                            error_category = "lsp_communication",
                            error = %e,
                            "Failed to read from LSP server"
                        );
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
            progress_manager,
        };

        // Initialize the LSP server
        client.initialize().await?;

        // Final health check after initialization attempt. If the server crashed during
        // initialization, we should fail here. This is crucial for tests like the zombie test,
        // which expect `LspClient::new` to fail if the server command is invalid.
        tokio::time::sleep(Duration::from_millis(100)).await;
        if !client.is_alive().await {
            return Err(ServerError::runtime(format!(
                "LSP server process for '{}' exited immediately after startup.",
                client.config().command.join(" ")
            )));
        }

        Ok(client)
    }

    /// Send a request to the LSP server and await the response
    pub async fn send_request(&self, method: &str, params: Value) -> ServerResult<Value> {
        // For file-specific operations, ensure the file is open in the LSP server
        if method.starts_with("textDocument/") && method != "textDocument/didOpen" {
            // Extract file path from params to open it if needed
            if let Some(uri) = params
                .get("textDocument")
                .and_then(|td| td.get("uri"))
                .and_then(|u| u.as_str())
            {
                if let Some(stripped) = uri.strip_prefix("file://") {
                    let file_path = std::path::Path::new(stripped);
                    // Notify file opened (will be no-op if already open)
                    if let Err(e) = self.notify_file_opened(file_path).await {
                        tracing::debug!(
                            file_path = %file_path.display(),
                            error = %e,
                            "Failed to open file before LSP request"
                        );
                        // Continue anyway - some operations might work without it
                    }
                }
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

        tracing::debug!(
            lsp_method = %method,
            lsp_request_id = id,
            has_params = !params.is_null(),
            "Sending LSP request"
        );

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
            return Err(ServerError::runtime(format!(
                "Failed to send request: {}",
                e
            )));
        }

        // Wait for response with timeout
        let start_time = std::time::Instant::now();
        let result = match timeout(LSP_REQUEST_TIMEOUT, response_rx).await {
            Ok(Ok(Ok(result))) => {
                tracing::debug!(
                    lsp_method = %method,
                    lsp_request_id = id,
                    duration_ms = start_time.elapsed().as_millis() as u64,
                    response_success = true,
                    "Received LSP response"
                );
                Ok(result)
            }
            Ok(Ok(Err(error))) => {
                tracing::debug!(
                    lsp_method = %method,
                    lsp_request_id = id,
                    duration_ms = start_time.elapsed().as_millis() as u64,
                    response_success = false,
                    error = %error,
                    "Received LSP error response"
                );
                Err(ServerError::runtime(format!("LSP error: {}", error)))
            }
            Ok(Err(_)) => {
                tracing::warn!(
                    lsp_method = %method,
                    lsp_request_id = id,
                    duration_ms = start_time.elapsed().as_millis() as u64,
                    "LSP response channel closed"
                );
                // Remove from pending requests
                let mut pending = self.pending_requests.lock().await;
                pending.remove(&id);
                Err(ServerError::runtime("Response channel closed"))
            }
            Err(_) => {
                tracing::warn!(
                    lsp_method = %method,
                    lsp_request_id = id,
                    duration_ms = start_time.elapsed().as_millis() as u64,
                    timeout_ms = LSP_REQUEST_TIMEOUT.as_millis() as u64,
                    "LSP request timeout"
                );
                // Remove from pending requests
                let mut pending = self.pending_requests.lock().await;
                pending.remove(&id);
                Err(ServerError::runtime("Request timeout"))
            }
        };
        result
    }

    /// Initialize the LSP server
    async fn initialize(&self) -> ServerResult<()> {
        let mut initialize_params = json!({
            "processId": std::process::id(),
            "clientInfo": {
                "name": "mill",
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
                    .unwrap_or(&std::env::current_dir()
                        .expect("Failed to get current directory for LSP workspace root"))
                    .display()),
            "workspaceFolders": [{
                "uri": format!("file://{}",
                    self.config.root_dir.as_deref()
                        .unwrap_or(&std::env::current_dir()
                            .expect("Failed to get current directory for LSP workspace folder"))
                        .display()),
                "name": "workspace"
            }]
        });

        // Add initializationOptions if provided in the config
        if let Some(ref init_options) = self.config.initialization_options {
            if let Some(obj) = initialize_params.as_object_mut() {
                obj.insert("initializationOptions".to_string(), init_options.clone());
                debug!(
                    initialization_options = ?init_options,
                    "Including custom initializationOptions in LSP initialize request"
                );
            }
        }

        debug!(params = ?initialize_params, "LSP initialize params");

        tracing::warn!(
            command = %self.config.command.join(" "),
            "Sending LSP initialize request (60s timeout)..."
        );

        // Send initialize request
        let result = timeout(
            LSP_INIT_TIMEOUT,
            self.send_request("initialize", initialize_params),
        )
        .await
        .map_err(|_| {
            tracing::error!(
                command = %self.config.command.join(" "),
                "LSP initialization TIMEOUT after 60 seconds - server never responded"
            );
            ServerError::runtime("LSP initialization timeout")
        })??;

        tracing::warn!(result = ?result, "LSP server initialization response received");

        // Send initialized notification
        self.send_notification("initialized", json!({})).await?;

        // Mark as initialized
        {
            let mut initialized = self.initialized.lock().await;
            *initialized = true;
        }

        info!(
            "LSP server initialized successfully: {}",
            self.config.command.join(" ")
        );

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
            return Err(ServerError::runtime(format!(
                "Failed to send notification: {}",
                e
            )));
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

    /// Check if the underlying LSP server process is still running.
    pub async fn is_alive(&self) -> bool {
        let mut process = self.process.lock().await;
        match process.try_wait() {
            Ok(Some(_status)) => {
                warn!("LSP process found to be exited with status: {}", _status);
                false
            }
            Ok(None) => true, // Process is still running
            Err(e) => {
                warn!("Error while checking LSP process status: {}", e);
                false // Error checking status, assume it's dead
            }
        }
    }

    /// Wait for a specific progress task to complete
    ///
    /// This method waits for a progress notification with the given token to complete.
    /// Returns `Ok(())` when the task completes successfully, or an error if the task
    /// fails or times out.
    ///
    /// ## Example
    ///
    /// ```rust,no_run
    /// # use mill_lsp::lsp_system::LspClient;
    /// # use mill_lsp::progress::ProgressToken;
    /// # use std::time::Duration;
    /// # async fn example(client: &LspClient) -> Result<(), Box<dyn std::error::Error>> {
    /// let token = ProgressToken::String("rustAnalyzer/Indexing".to_string());
    /// client.wait_for_progress(&token, Duration::from_secs(30)).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn wait_for_progress(
        &self,
        token: &ProgressToken,
        timeout: Duration,
    ) -> Result<(), ProgressError> {
        self.progress_manager
            .wait_for_completion(token, timeout)
            .await
    }

    /// Wait for rust-analyzer workspace indexing to complete
    ///
    /// This is a convenience method that waits for the `rustAnalyzer/Indexing` progress
    /// task to complete. This is particularly useful in tests or when you need to ensure
    /// rust-analyzer has finished indexing before performing workspace-level operations
    /// like `workspace/symbol`.
    ///
    /// ## Example
    ///
    /// ```rust,no_run
    /// # use mill_lsp::lsp_system::LspClient;
    /// # use std::time::Duration;
    /// # async fn example(client: &LspClient) -> Result<(), Box<dyn std::error::Error>> {
    /// // Wait up to 30 seconds for indexing to complete
    /// client.wait_for_indexing(Duration::from_secs(30)).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn wait_for_indexing(&self, timeout: Duration) -> Result<(), ProgressError> {
        let token = ProgressToken::String("rustAnalyzer/Indexing".to_string());
        self.wait_for_progress(&token, timeout).await
    }

    /// Get the current state of a progress task
    ///
    /// Returns `None` if the task doesn't exist or has been cleaned up.
    pub fn get_progress_state(
        &self,
        token: &ProgressToken,
    ) -> Option<crate::progress::ProgressState> {
        self.progress_manager.get_state(token)
    }

    /// Check if a progress task has completed
    pub fn is_progress_completed(&self, token: &ProgressToken) -> bool {
        self.progress_manager.is_completed(token)
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

        self.send_notification("textDocument/didOpen", params)
            .await?;
        debug!(
            "Sent didOpen notification for file: {}",
            file_path.display()
        );

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

    /// Force shutdown the LSP server process without requiring exclusive ownership
    ///
    /// This method can be called through an Arc reference and will:
    /// 1. Kill the process
    /// 2. Wait for it to exit (preventing zombies)
    ///
    /// Unlike `shutdown(self)` which requires ownership, this method works with `&self`.
    pub async fn force_shutdown(&self) -> ServerResult<()> {
        let pid = {
            let process = self.process.lock().await;
            process.id()
        };

        tracing::debug!(
            pid = pid,
            "Force shutdown initiated (kill + wait without ownership)"
        );

        // Step 1: Kill the process
        let mut process = self.process.lock().await;
        if let Err(e) = process.kill().await {
            tracing::warn!(
                pid = pid,
                error = %e,
                "Failed to kill LSP server process during force shutdown"
            );
            // Continue to wait anyway
        }

        // Step 2: Wait for the process to exit (prevents zombies)
        match timeout(Duration::from_secs(5), process.wait()).await {
            Ok(Ok(status)) => {
                tracing::debug!(
                    pid = pid,
                    exit_status = ?status,
                    "LSP server process force shutdown completed"
                );
                Ok(())
            }
            Ok(Err(e)) => {
                tracing::warn!(
                    pid = pid,
                    error = %e,
                    "Failed to wait for LSP server process during force shutdown"
                );
                Err(ServerError::runtime(format!(
                    "Failed to wait for LSP server process: {}",
                    e
                )))
            }
            Err(_) => {
                tracing::warn!(
                    pid = pid,
                    "Timeout waiting for LSP server process to exit during force shutdown"
                );
                Err(ServerError::runtime(
                    "Timeout waiting for LSP server process to exit",
                ))
            }
        }
    }

    /// Gracefully shutdown the LSP server process.
    ///
    /// This method performs a clean LSP shutdown sequence:
    /// 1. Sends the LSP "shutdown" request
    /// 2. Sends the LSP "exit" notification
    /// 3. Calls kill() on the child process
    /// 4. Waits up to 5 seconds for the process to exit
    ///
    /// Consumes self to prevent further use after shutdown.
    pub async fn shutdown(self) -> ServerResult<()> {
        let pid = {
            let process = self.process.lock().await;
            process.id()
        };

        // Step 1: Send LSP shutdown request
        if let Err(e) = self.send_request("shutdown", json!({})).await {
            warn!(
                pid = pid,
                error = %e,
                "Failed to send LSP shutdown request, continuing with forceful shutdown"
            );
        } else {
            debug!(pid = pid, "Sent LSP shutdown request");
        }

        // Step 2: Send LSP exit notification
        if let Err(e) = self.send_notification("exit", json!({})).await {
            warn!(
                pid = pid,
                error = %e,
                "Failed to send LSP exit notification, continuing with forceful shutdown"
            );
        } else {
            debug!(pid = pid, "Sent LSP exit notification");
        }

        // Step 3: Kill the process
        let mut process = self.process.lock().await;
        if let Err(e) = process.kill().await {
            warn!(
                pid = pid,
                error = %e,
                "Failed to kill LSP server process"
            );
        }

        // Step 4: Wait for the process to exit (with timeout)
        match timeout(Duration::from_secs(5), process.wait()).await {
            Ok(Ok(status)) => {
                debug!(
                    pid = pid,
                    exit_status = ?status,
                    "LSP server process exited gracefully"
                );
                Ok(())
            }
            Ok(Err(e)) => {
                warn!(
                    pid = pid,
                    error = %e,
                    "Failed to wait for LSP server process"
                );
                Err(ServerError::runtime(format!(
                    "Failed to wait for LSP server process: {}",
                    e
                )))
            }
            Err(_) => {
                warn!(
                    pid = pid,
                    "LSP server process did not exit within 5 seconds"
                );
                Err(ServerError::runtime(
                    "LSP server process did not exit within timeout",
                ))
            }
        }
    }

    /// Parse Content-Length header from LSP message
    fn parse_content_length(line: &str) -> Option<usize> {
        line.strip_prefix("Content-Length: ")
            .and_then(|stripped| stripped.parse().ok())
    }

    /// Read JSON message with specified content length
    /// Note: This expects to be called AFTER the empty line separator has been consumed
    async fn read_json_message(
        reader: &mut BufReader<tokio::process::ChildStdout>,
        content_length: usize,
    ) -> Result<Value, String> {
        // Read the JSON content (empty line already consumed by caller)
        let mut json_buffer = vec![0u8; content_length];
        if let Err(e) = tokio::io::AsyncReadExt::read_exact(reader, &mut json_buffer).await {
            return Err(format!("Failed to read JSON content: {}", e));
        }

        let json_str = String::from_utf8(json_buffer)
            .map_err(|e| format!("Invalid UTF-8 in JSON content: {}", e))?;

        serde_json::from_str(&json_str).map_err(|e| format!("Failed to parse JSON: {}", e))
    }

    /// Handle incoming message from LSP server
    async fn handle_message(
        message: Value,
        pending_requests: &PendingRequests,
        message_tx: &mpsc::Sender<LspMessage>,
        progress_manager: &ProgressManager,
    ) {
        tracing::warn!(message = ?message, "Received message from LSP server");

        if message.get("method").is_some() {
            if message.get("id").is_some() {
                // This is a server-initiated request that requires a response.
                Self::handle_server_request(&message, message_tx).await;
            } else {
                // This is a notification from the server
                let method = message.get("method").and_then(|m| m.as_str());

                // Handle $/progress notifications
                if method == Some("$/progress") {
                    if let Some(params) = message.get("params") {
                        // Parse progress notification
                        match serde_json::from_value::<ProgressParams>(params.clone()) {
                            Ok(progress_params) => {
                                progress_manager.handle_notification(progress_params);
                            }
                            Err(e) => {
                                debug!(
                                    error = %e,
                                    params = ?params,
                                    "Failed to parse $/progress notification"
                                );
                            }
                        }
                    }
                } else {
                    debug!(
                        method = ?method,
                        notification = ?message,
                        "Received notification from LSP server"
                    );
                }
            }
        } else if let Some(id) = message.get("id") {
            // This is a response to a client-initiated request
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
                        tracing::error!(id = id_num, error = %error_msg, "LSP request returned error");
                        let _ = sender.send(Err(error_msg));
                    } else if let Some(result) = message.get("result") {
                        tracing::warn!(id = id_num, "LSP request successful, sending result");
                        let _ = sender.send(Ok(result.clone()));
                    } else {
                        tracing::error!(id = id_num, "LSP response has no result or error field");
                        let _ = sender.send(Err("Invalid response format".to_string()));
                    }
                } else {
                    tracing::warn!(
                        id = id_num,
                        "Received response for unknown request ID (already handled or timeout)"
                    );
                }
            }
        } else {
            warn!(message = ?message, "Received unhandled message from LSP server");
        }
    }

    /// Handle server-initiated requests
    async fn handle_server_request(request: &Value, message_tx: &mpsc::Sender<LspMessage>) {
        debug!(?request, "Handling server request");
        let id = request.get("id").cloned().unwrap_or(Value::Null);
        let method = request.get("method").and_then(|m| m.as_str());

        let response = match method {
            Some("workspace/configuration") => {
                // The server requests configuration for a number of items. We must respond with
                // an array of the same length. Returning `null` for each is a valid way to say
                // "use your default".
                let items_len = request
                    .get("params")
                    .and_then(|p| p.get("items"))
                    .and_then(|i| i.as_array())
                    .map(|a| a.len())
                    .unwrap_or(0);
                LspMessage::Response {
                    id,
                    result: json!(vec![Value::Null; items_len]),
                }
            }
            Some("client/registerCapability") | Some("window/workDoneProgress/create") => {
                // Acknowledge these requests but we don't need to do anything.
                LspMessage::Response {
                    id,
                    result: Value::Null,
                }
            }
            Some("workspace/workspaceFolders") => {
                // Respond with an empty array as we manage the workspace.
                LspMessage::Response {
                    id,
                    result: json!([]),
                }
            }
            _ => {
                // For any other request, respond that we don't support it.
                warn!(method = ?method, "Received unsupported server request");
                LspMessage::ErrorResponse {
                    id,
                    error: json!({
                        "code": -32601,
                        "message": "Method not found"
                    }),
                }
            }
        };

        if let Err(e) = message_tx.send(response).await {
            tracing::error!(error = %e, "Failed to send response for server request");
        }
    }
}

impl Drop for LspClient {
    fn drop(&mut self) {
        // Get the PID before dropping for logging
        let pid = {
            // Try to get the lock without blocking - if we can't, we'll just skip the PID
            if let Ok(process) = self.process.try_lock() {
                process.id()
            } else {
                None
            }
        };

        // Log warning that shutdown() should have been called
        // The zombie reaper will handle cleanup
        if let Some(pid) = pid {
            warn!(
                pid = pid,
                "LspClient dropped without calling shutdown() - relying on zombie reaper"
            );
        } else {
            warn!("LspClient dropped without calling shutdown() - relying on zombie reaper (PID unavailable)");
        }
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
            initialization_options: None,
        }
    }

    #[tokio::test]
    async fn test_lsp_client_creation() {
        let config = create_test_config();

        // This will fail because echo is not an LSP server, but we can test the creation logic
        // Add timeout to prevent hanging
        let result =
            tokio::time::timeout(std::time::Duration::from_secs(5), LspClient::new(config)).await;

        match result {
            Ok(client_result) => {
                assert!(client_result.is_err()); // Expected to fail during initialization
            }
            Err(_) => {
                // Timeout occurred, which is also acceptable for this test
                // since we're using echo which doesn't speak LSP protocol
            }
        }
    }

    #[test]
    fn test_parse_content_length() {
        assert_eq!(
            LspClient::parse_content_length("Content-Length: 123"),
            Some(123)
        );
        assert_eq!(
            LspClient::parse_content_length("Content-Length: 0"),
            Some(0)
        );
        assert_eq!(LspClient::parse_content_length("Other header"), None);
        assert_eq!(
            LspClient::parse_content_length("Content-Length: invalid"),
            None
        );
    }
}
