//! Manages out-of-process language plugins.
//!
//! This module is responsible for spawning, communicating with, and managing
//! the lifecycle of external language plugins that run as separate processes.
//! It uses a JSON-RPC protocol over stdio, similar to the LSP.

use cb_protocol::plugin_protocol::{PluginRequest, PluginResponse};
use dashmap::DashMap;
use serde_json::Value;
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, Command};
use tokio::sync::{oneshot, Mutex};
use tracing::{error, info, warn};

/// Timeout for plugin requests.
const PLUGIN_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// Represents a connection to a single, running external plugin process.
#[derive(Clone)]
pub struct PluginProcess {
    name: String,
    #[allow(dead_code)] // Kept for future process lifecycle management
    child: Arc<Mutex<Child>>,
    stdin: Arc<Mutex<ChildStdin>>,
    request_id_counter: Arc<AtomicU64>,
    pending_requests: Arc<DashMap<u64, oneshot::Sender<Result<PluginResponse, String>>>>,
}

impl PluginProcess {
    /// Spawns a new plugin process.
    pub async fn new(name: &str, command: &[String]) -> Result<Self, String> {
        if command.is_empty() {
            return Err("Plugin command cannot be empty".to_string());
        }

        let (program, args) = command.split_first().unwrap();

        info!(plugin_name = name, command = ?command, "Spawning external plugin process");

        let mut child = Command::new(program)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to spawn plugin '{}': {}", name, e))?;

        let stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();

        let pending_requests: Arc<DashMap<u64, oneshot::Sender<Result<PluginResponse, String>>>> =
            Arc::new(DashMap::new());
        let pending_requests_clone = pending_requests.clone();

        let plugin_name_for_stdout = name.to_string();
        tokio::spawn(async move {
            let mut reader = BufReader::new(stdout);
            loop {
                // This is a simplified reader assuming one JSON object per line.
                // A more robust implementation would handle message framing like LSP.
                let mut line = String::new();
                match reader.read_line(&mut line).await {
                    Ok(0) => {
                        info!(plugin_name = %plugin_name_for_stdout, "Plugin stdout closed");
                        break;
                    }
                    Ok(_) => match serde_json::from_str::<PluginResponse>(&line) {
                        Ok(response) => {
                            if let Some(sender) = pending_requests_clone.remove(&response.id) {
                                if let Err(_) = sender.1.send(Ok(response)) {
                                    warn!(plugin_name = %plugin_name_for_stdout, "Failed to send plugin response to receiver");
                                }
                            }
                        }
                        Err(e) => {
                            error!(plugin_name = %plugin_name_for_stdout, error = %e, "Failed to parse response from plugin");
                        }
                    },
                    Err(e) => {
                        error!(plugin_name = %plugin_name_for_stdout, error = %e, "Error reading from plugin stdout");
                        break;
                    }
                }
            }
        });

        let plugin_name_for_stderr = name.to_string();
        tokio::spawn(async move {
            let mut reader = BufReader::new(stderr);
            let mut line = String::new();
            while let Ok(_) = reader.read_line(&mut line).await {
                if !line.is_empty() {
                    warn!(plugin_name = %plugin_name_for_stderr, stderr = %line.trim(), "Plugin stderr");
                    line.clear();
                } else {
                    break;
                }
            }
        });

        Ok(Self {
            name: name.to_string(),
            child: Arc::new(Mutex::new(child)),
            stdin: Arc::new(Mutex::new(stdin)),
            request_id_counter: Arc::new(AtomicU64::new(1)),
            pending_requests,
        })
    }

    /// Calls a method on the plugin.
    pub async fn call(&self, method: &str, params: Value) -> Result<Value, String> {
        let id = self.request_id_counter.fetch_add(1, Ordering::SeqCst);
        let request = PluginRequest {
            id,
            method: method.to_string(),
            params,
        };

        let (tx, rx) = oneshot::channel();
        self.pending_requests.insert(id, tx);

        let request_json = serde_json::to_string(&request).unwrap() + "\n";

        {
            let mut stdin_guard = self.stdin.lock().await;
            stdin_guard
                .write_all(request_json.as_bytes())
                .await
                .map_err(|e| format!("Failed to write to plugin stdin: {}", e))?;
            stdin_guard
                .flush()
                .await
                .map_err(|e| format!("Failed to flush plugin stdin: {}", e))?;
        }

        match tokio::time::timeout(PLUGIN_REQUEST_TIMEOUT, rx).await {
            Ok(Ok(Ok(response))) => response
                .result
                .ok_or_else(|| "Plugin response was missing a result".to_string()),
            Ok(Ok(Err(e))) => Err(format!("Plugin returned an error: {}", e)),
            Ok(Err(_)) => Err("Response channel for plugin call was dropped".to_string()),
            Err(_) => {
                self.pending_requests.remove(&id);
                Err(format!("Request to plugin '{}' timed out", self.name))
            }
        }
    }
}

/// Manages a collection of running plugin processes.
pub struct PluginProcessManager {
    plugins: DashMap<String, PluginProcess>,
}

impl PluginProcessManager {
    pub fn new() -> Self {
        Self {
            plugins: DashMap::new(),
        }
    }

    /// Spawns and registers a new plugin.
    pub async fn register_plugin(&self, name: &str, command: &[String]) -> Result<(), String> {
        if self.plugins.contains_key(name) {
            return Ok(()); // Already registered.
        }

        let plugin_process = PluginProcess::new(name, command).await?;
        self.plugins.insert(name.to_string(), plugin_process);
        Ok(())
    }

    /// Gets a handle to a running plugin process.
    pub fn get_plugin(&self, name: &str) -> Option<PluginProcess> {
        self.plugins.get(name).map(|p| p.clone())
    }
}

impl Default for PluginProcessManager {
    fn default() -> Self {
        Self::new()
    }
}
