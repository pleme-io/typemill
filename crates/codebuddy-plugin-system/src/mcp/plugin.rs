//! MCP Proxy as a LanguagePlugin

use super::manager::ExternalMcpManager;
use crate::{
    Capabilities, LanguagePlugin, PluginError, PluginMetadata, PluginRequest, PluginResponse,
    PluginResult,
};
use async_trait::async_trait;
use codebuddy_config::config::ExternalMcpServerConfig;
use serde_json::Value;

pub struct McpProxyPlugin {
    manager: ExternalMcpManager,
    metadata: PluginMetadata,
}

impl McpProxyPlugin {
    pub fn new(configs: Vec<ExternalMcpServerConfig>) -> Self {
        Self {
            manager: ExternalMcpManager::new(configs),
            metadata: PluginMetadata::new("mcp-proxy", "0.1.0", "Codebuddy Team")
                .with_description("Proxy to external MCP servers"),
        }
    }

    pub async fn start_servers(&self) -> PluginResult<()> {
        self.manager
            .start_all_servers()
            .await
            .map_err(|e| PluginError::initialization_error("mcp-proxy", e.to_string()))
    }
}

#[async_trait]
impl LanguagePlugin for McpProxyPlugin {
    fn metadata(&self) -> PluginMetadata {
        self.metadata.clone()
    }

    fn supported_extensions(&self) -> Vec<String> {
        vec![] // MCP proxy doesn't handle files directly
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities::default()
    }

    async fn handle_request(&self, request: PluginRequest) -> PluginResult<PluginResponse> {
        // Route to appropriate external MCP server
        self.manager.handle_request(request).await
    }

    fn configure(&self, _config: Value) -> PluginResult<()> {
        Ok(())
    }

    fn tool_definitions(&self) -> Vec<Value> {
        self.manager.all_tool_definitions()
    }

    async fn initialize(&mut self) -> PluginResult<()> {
        self.start_servers().await
    }

    async fn shutdown(&mut self) -> PluginResult<()> {
        Ok(())
    }
}
