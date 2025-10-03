//! External MCP client for subprocess communication

use super::error::{McpProxyError, McpProxyResult};
use super::protocol::{McpRequest, McpResponse};
use serde_json::Value;
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tracing::{debug, error, info};

pub struct ExternalMcpClient {
    name: String,
    process: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    message_id: AtomicU64,
}

impl ExternalMcpClient {
    pub async fn spawn(name: String, command: Vec<String>) -> McpProxyResult<Self> {
        info!(name = %name, command = ?command, "Spawning external MCP server");

        let mut child = Command::new(&command[0])
            .args(&command[1..])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| McpProxyError::spawn_failed(&name, e))?;

        let stdin = child.stdin.take().unwrap();
        let stdout = BufReader::new(child.stdout.take().unwrap());

        Ok(Self {
            name,
            process: child,
            stdin,
            stdout,
            message_id: AtomicU64::new(1),
        })
    }

    pub async fn call_tool(&mut self, tool: &str, params: Value) -> McpProxyResult<Value> {
        let id = self.message_id.fetch_add(1, Ordering::SeqCst);

        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id,
            method: format!("tools/{}", tool),
            params: Some(params),
        };

        // Send request
        let request_json = serde_json::to_string(&request)?;
        debug!(mcp_server = %self.name, tool = %tool, id = %id, "Sending MCP request");

        self.stdin.write_all(request_json.as_bytes()).await?;
        self.stdin.write_all(b"\n").await?;
        self.stdin.flush().await?;

        // Read response
        let mut response_line = String::new();
        self.stdout.read_line(&mut response_line).await?;

        let response: McpResponse = serde_json::from_str(&response_line)?;

        if let Some(error) = response.error {
            error!(mcp_server = %self.name, error = ?error, "MCP server returned error");
            return Err(McpProxyError::McpServerError(error.message));
        }

        Ok(response.result.unwrap_or(Value::Null))
    }

    pub async fn list_tools(&mut self) -> McpProxyResult<Vec<String>> {
        let id = self.message_id.fetch_add(1, Ordering::SeqCst);

        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id,
            method: "tools/list".to_string(),
            params: None,
        };

        let request_json = serde_json::to_string(&request)?;
        debug!(mcp_server = %self.name, "Requesting tool list");

        self.stdin.write_all(request_json.as_bytes()).await?;
        self.stdin.write_all(b"\n").await?;
        self.stdin.flush().await?;

        let mut response_line = String::new();
        self.stdout.read_line(&mut response_line).await?;

        let response: McpResponse = serde_json::from_str(&response_line)?;

        if let Some(error) = response.error {
            error!(mcp_server = %self.name, error = ?error, "Failed to list tools");
            return Err(McpProxyError::McpServerError(error.message));
        }

        // Parse tool names from response
        let tools: Vec<String> = if let Some(result) = response.result {
            if let Some(tools_array) = result.get("tools").and_then(|v| v.as_array()) {
                tools_array
                    .iter()
                    .filter_map(|t| t.get("name").and_then(|n| n.as_str()).map(String::from))
                    .collect()
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        debug!(mcp_server = %self.name, tools_count = tools.len(), "Discovered tools");
        Ok(tools)
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

impl Drop for ExternalMcpClient {
    fn drop(&mut self) {
        // Attempt to kill the child process when dropped
        let _ = self.process.start_kill();
    }
}
