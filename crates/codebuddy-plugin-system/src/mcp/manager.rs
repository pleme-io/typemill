//! Manager for multiple external MCP servers

use super::client::ExternalMcpClient;
use super::error::{McpProxyError, McpProxyResult};
use crate::{PluginError, PluginRequest, PluginResponse, PluginResult};
use cb_core::config::ExternalMcpServerConfig;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

pub struct ExternalMcpManager {
    configs: Vec<ExternalMcpServerConfig>,
    clients: Arc<RwLock<HashMap<String, ExternalMcpClient>>>,
}

impl ExternalMcpManager {
    pub fn new(configs: Vec<ExternalMcpServerConfig>) -> Self {
        Self {
            configs,
            clients: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn start_server(&self, name: &str) -> McpProxyResult<()> {
        let config = self
            .configs
            .iter()
            .find(|c| c.name == name)
            .ok_or_else(|| McpProxyError::ServerNotFound(name.to_string()))?;

        info!(server = %name, "Starting external MCP server");

        let client = ExternalMcpClient::spawn(name.to_string(), config.command.clone()).await?;

        let mut clients = self.clients.write().await;
        clients.insert(name.to_string(), client);

        Ok(())
    }

    pub async fn start_all_servers(&self) -> McpProxyResult<()> {
        for config in &self.configs {
            if config.auto_start {
                if let Err(e) = self.start_server(&config.name).await {
                    warn!(server = %config.name, error = %e, "Failed to start MCP server");
                }
            }
        }
        Ok(())
    }

    pub async fn call_tool(
        &self,
        server_name: &str,
        tool_name: &str,
        params: Value,
    ) -> McpProxyResult<Value> {
        let mut clients = self.clients.write().await;

        let client = clients
            .get_mut(server_name)
            .ok_or_else(|| McpProxyError::ServerNotFound(server_name.to_string()))?;

        client.call_tool(tool_name, params).await
    }

    pub async fn handle_request(&self, request: PluginRequest) -> PluginResult<PluginResponse> {
        debug!(method = %request.method, "Handling MCP proxy request");

        // Parse the method to extract server and tool name
        // Expected format: "mcp_proxy__<server>__<tool>"
        let parts: Vec<&str> = request.method.split("__").collect();

        if parts.len() != 3 || parts[0] != "mcp_proxy" {
            return Err(PluginError::request_failed(
                "mcp-proxy",
                format!("Invalid MCP proxy method format: {}", request.method),
            ));
        }

        let server_name = parts[1];
        let tool_name = parts[2];

        let result = self
            .call_tool(server_name, tool_name, request.params)
            .await
            .map_err(|e| PluginError::request_failed("mcp-proxy", e.to_string()))?;

        Ok(PluginResponse::success(result, "mcp-proxy"))
    }

    pub fn all_tool_definitions(&self) -> Vec<Value> {
        // Return tool definitions for all configured MCP servers
        // This would need to query each server for its available tools
        vec![]
    }

    pub async fn active_servers(&self) -> Vec<String> {
        let clients = self.clients.read().await;
        clients.keys().cloned().collect()
    }
}
