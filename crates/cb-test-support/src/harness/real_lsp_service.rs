//! An implementation of LspService that communicates with a real LSP server process.

use async_trait::async_trait;
use codebuddy_foundation::protocol::{ApiError, LspService, Message};
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::mpsc;
use tokio::time::{timeout, Duration};

use crate::harness::lsp_setup::LspSetupHelper;

/// An LspService implementation that runs a real LSP server as a child process.
pub struct RealLspService {
    child: Arc<Mutex<Child>>,
    stdin_tx: mpsc::Sender<String>,
    responses: Arc<Mutex<HashMap<String, Message>>>,
    initialization_options: Option<Value>,
}

impl RealLspService {
    /// Create a new RealLspService for the given language extension (e.g., "ts", "py").
    pub async fn new(extension: &str, root_path: &Path) -> Result<Self, ApiError> {
        Self::new_with_options(extension, root_path, None).await
    }

    /// Create a new RealLspService with custom initialization options.
    pub async fn new_with_options(
        extension: &str,
        root_path: &Path,
        initialization_options: Option<Value>,
    ) -> Result<Self, ApiError> {
        let cmd = LspSetupHelper::get_lsp_command(extension)?;

        // Augment PATH to include cargo bin and NVM node bin
        let current_path = std::env::var("PATH").unwrap_or_default();
        let mut path_additions = Vec::new();

        // Add cargo bin directory
        if let Ok(cargo_home) = std::env::var("CARGO_HOME") {
            path_additions.push(format!("{}/bin", cargo_home));
        } else if let Ok(home) = std::env::var("HOME") {
            path_additions.push(format!("{}/.cargo/bin", home));
        }

        // Add NVM node bin directory (critical for typescript-language-server)
        if let Ok(nvm_dir) = std::env::var("NVM_DIR") {
            if let Ok(entries) = std::fs::read_dir(format!("{}/versions/node", nvm_dir)) {
                if let Some(Ok(entry)) = entries.into_iter().next() {
                    path_additions.push(format!("{}/bin", entry.path().display()));
                }
            }
        } else if let Ok(home) = std::env::var("HOME") {
            let nvm_default = format!("{}/.nvm/versions/node", home);
            if let Ok(entries) = std::fs::read_dir(&nvm_default) {
                if let Some(Ok(entry)) = entries.into_iter().next() {
                    path_additions.push(format!("{}/bin", entry.path().display()));
                }
            }
        }

        let augmented_path = if path_additions.is_empty() {
            current_path
        } else {
            format!("{}:{}", path_additions.join(":"), current_path)
        };

        let mut command = Command::new(&cmd[0]);
        command
            .args(&cmd[1..])
            .env("PATH", augmented_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .current_dir(root_path);

        let mut child = command.spawn().map_err(|e| {
            ApiError::lsp(format!(
                "Failed to spawn LSP server for {}: {}",
                extension, e
            ))
        })?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| ApiError::lsp("Failed to capture stdin of LSP server".to_string()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| ApiError::lsp("Failed to capture stdout of LSP server".to_string()))?;

        let responses = Arc::new(Mutex::new(HashMap::new()));
        let responses_clone = responses.clone();

        // Create channel for sending messages to stdin
        let (stdin_tx, mut stdin_rx) = mpsc::channel::<String>(100);

        // Spawn task to write to stdin
        tokio::spawn(async move {
            let mut stdin = stdin;
            while let Some(msg) = stdin_rx.recv().await {
                if stdin.write_all(msg.as_bytes()).await.is_err() {
                    break;
                }
                if stdin.flush().await.is_err() {
                    break;
                }
            }
        });

        // Spawn task to read from stdout
        tokio::spawn(async move {
            let mut reader = BufReader::new(stdout);
            let mut buffer = String::new();

            loop {
                buffer.clear();
                match reader.read_line(&mut buffer).await {
                    Ok(0) => break, // EOF
                    Ok(_) => {
                        let line = buffer.trim();

                        // Parse Content-Length header
                        if line.starts_with("Content-Length:") {
                            if let Some(length_str) =
                                line.strip_prefix("Content-Length:").map(|s| s.trim())
                            {
                                if let Ok(content_length) = length_str.parse::<usize>() {
                                    // Skip remaining headers until empty line
                                    loop {
                                        buffer.clear();
                                        if reader.read_line(&mut buffer).await.is_err() {
                                            break;
                                        }
                                        if buffer.trim().is_empty() {
                                            break;
                                        }
                                    }

                                    // Read JSON content
                                    let mut json_buffer = vec![0u8; content_length];
                                    if tokio::io::AsyncReadExt::read_exact(
                                        &mut reader,
                                        &mut json_buffer,
                                    )
                                    .await
                                    .is_ok()
                                    {
                                        if let Ok(json_str) = String::from_utf8(json_buffer) {
                                            if let Ok(value) =
                                                serde_json::from_str::<Value>(&json_str)
                                            {
                                                // Check if this is a response (has "id" field)
                                                if let Some(id) =
                                                    value.get("id").and_then(|v| v.as_str())
                                                {
                                                    let msg = Message {
                                                        id: Some(id.to_string()),
                                                        method: value
                                                            .get("method")
                                                            .and_then(|v| v.as_str())
                                                            .unwrap_or("response")
                                                            .to_string(),
                                                        params: value
                                                            .get("result")
                                                            .cloned()
                                                            .unwrap_or_else(|| {
                                                                value
                                                                    .get("error")
                                                                    .cloned()
                                                                    .unwrap_or(Value::Null)
                                                            }),
                                                    };
                                                    let mut resp = responses_clone.lock().unwrap();
                                                    resp.insert(id.to_string(), msg);
                                                } else if let Some(id_num) =
                                                    value.get("id").and_then(|v| v.as_i64())
                                                {
                                                    let id = id_num.to_string();
                                                    let msg = Message {
                                                        id: Some(id.clone()),
                                                        method: value
                                                            .get("method")
                                                            .and_then(|v| v.as_str())
                                                            .unwrap_or("response")
                                                            .to_string(),
                                                        params: value
                                                            .get("result")
                                                            .cloned()
                                                            .unwrap_or_else(|| {
                                                                value
                                                                    .get("error")
                                                                    .cloned()
                                                                    .unwrap_or(Value::Null)
                                                            }),
                                                    };
                                                    let mut resp = responses_clone.lock().unwrap();
                                                    resp.insert(id, msg);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        let service = Self {
            child: Arc::new(Mutex::new(child)),
            stdin_tx,
            responses,
            initialization_options,
        };

        // Initialize the LSP server
        service.initialize(root_path).await?;

        Ok(service)
    }

    /// Send LSP initialize request
    async fn initialize(&self, root_path: &Path) -> Result<(), ApiError> {
        let mut init_params = serde_json::json!({
            "processId": std::process::id(),
            "rootUri": format!("file://{}", root_path.display()),
            "capabilities": {}
        });

        // Add initializationOptions if provided
        if let Some(ref options) = self.initialization_options {
            if let Some(obj) = init_params.as_object_mut() {
                obj.insert("initializationOptions".to_string(), options.clone());
                eprintln!(
                    "🔧 TEST: Sending initializationOptions to LSP: {:?}",
                    options
                );
            }
        }

        eprintln!(
            "🔧 TEST: Full initialize params: {}",
            serde_json::to_string_pretty(&init_params).unwrap()
        );

        let init_message = Message {
            id: Some("init".to_string()),
            method: "initialize".to_string(),
            params: init_params,
        };

        // Send initialize request
        let _response = timeout(Duration::from_secs(10), self.request(init_message))
            .await
            .map_err(|_| ApiError::lsp("LSP initialization timed out".to_string()))??;

        // Send initialized notification
        let initialized_msg = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "initialized",
            "params": {}
        });

        let msg_str = serde_json::to_string(&initialized_msg).unwrap();
        let lsp_msg = format!("Content-Length: {}\r\n\r\n{}", msg_str.len(), msg_str);

        self.stdin_tx
            .send(lsp_msg)
            .await
            .map_err(|_| ApiError::lsp("Failed to send initialized notification".to_string()))?;

        Ok(())
    }
}

#[async_trait]
impl LspService for RealLspService {
    async fn request(&self, message: Message) -> Result<Message, ApiError> {
        let id = message
            .id
            .clone()
            .ok_or_else(|| ApiError::lsp("Request has no ID".to_string()))?;

        // Convert to LSP JSON-RPC format
        let lsp_request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": message.method,
            "params": message.params
        });

        let request_str = serde_json::to_string(&lsp_request)
            .map_err(|e| ApiError::lsp(format!("Failed to serialize request: {}", e)))?;

        let lsp_message = format!(
            "Content-Length: {}\r\n\r\n{}",
            request_str.len(),
            request_str
        );

        self.stdin_tx
            .send(lsp_message)
            .await
            .map_err(|e| ApiError::lsp(format!("Failed to write to LSP stdin: {}", e)))?;

        // Wait for response with timeout
        let timeout_duration = Duration::from_secs(10);
        let start = tokio::time::Instant::now();

        while start.elapsed() < timeout_duration {
            if let Some(response) = self.responses.lock().unwrap().remove(&id) {
                return Ok(response);
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        Err(ApiError::lsp("LSP request timed out".to_string()))
    }

    async fn is_available(&self, extension: &str) -> bool {
        LspSetupHelper::get_lsp_command(extension).is_ok()
    }

    async fn restart_servers(&self, _extensions: Option<Vec<String>>) -> Result<(), ApiError> {
        // This would involve killing and restarting the child process.
        Ok(())
    }

    async fn notify_file_opened(&self, _file_path: &Path) -> Result<(), ApiError> {
        // Implementation for textDocument/didOpen notification
        Ok(())
    }
}

impl Drop for RealLspService {
    fn drop(&mut self) {
        // RealLspService manages its own Child process (not LspClient)
        // We need to kill and wait for the process to avoid zombies
        if let Ok(mut child) = self.child.lock() {
            let pid = child.id();

            // First try to kill the process
            if let Err(e) = child.start_kill() {
                eprintln!(
                    "Failed to kill RealLspService process (PID {:?}): {}",
                    pid, e
                );
            }

            // Note: We can't spawn an async task here because std::sync::Mutex is not Send
            // across await points. The zombie reaper will clean up this process.
            eprintln!(
                "RealLspService process killed (PID {:?}) - zombie reaper will clean up",
                pid
            );
        }
    }
}
