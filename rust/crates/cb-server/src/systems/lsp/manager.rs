//! LSP manager implementation

use crate::error::{ServerError, ServerResult};
use cb_core::CoreError;
use crate::interfaces::LspService;
use crate::systems::lsp::client::LspClient;
use async_trait::async_trait;
use cb_core::config::LspConfig;
use cb_core::model::mcp::{McpMessage, McpResponse, McpError};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, error, info};

/// LSP manager that handles multiple LSP clients by language
pub struct LspManager {
    /// LSP configuration
    config: LspConfig,
    /// Active LSP clients by language extension
    clients: Arc<Mutex<HashMap<String, Arc<LspClient>>>>,
}

impl LspManager {
    /// Create a new LSP manager
    pub fn new(config: LspConfig) -> Self {
        Self {
            config,
            clients: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Get or create an LSP client for the given file extension
    async fn get_client_for_extension(&self, extension: &str) -> ServerResult<Arc<LspClient>> {
        // Check if client already exists
        {
            let clients = self.clients.lock().await;
            if let Some(client) = clients.get(extension) {
                return Ok(client.clone());
            }
        }

        // Find server config for this extension
        let server_config = self
            .config
            .servers
            .iter()
            .find(|server| server.extensions.contains(&extension.to_string()))
            .ok_or_else(|| {
                ServerError::Unsupported(format!("No LSP server configured for extension: {}", extension))
            })?;

        // Create new client
        let client = Arc::new(LspClient::new(server_config.clone()).await?);

        // Store the client
        {
            let mut clients = self.clients.lock().await;
            clients.insert(extension.to_string(), client.clone());
        }

        info!("Created LSP client for extension: {}", extension);
        Ok(client)
    }

    /// Extract file path from MCP message parameters
    fn extract_file_path(params: &Value) -> Option<String> {
        // Try different common parameter names for file paths
        if let Some(file_path) = params.get("file_path").and_then(|v| v.as_str()) {
            return Some(file_path.to_string());
        }
        if let Some(file_path) = params.get("filePath").and_then(|v| v.as_str()) {
            return Some(file_path.to_string());
        }
        if let Some(uri) = params.get("uri").and_then(|v| v.as_str()) {
            // Convert file URI to path
            if uri.starts_with("file://") {
                return Some(uri[7..].to_string());
            }
            return Some(uri.to_string());
        }
        if let Some(text_document) = params.get("textDocument") {
            if let Some(uri) = text_document.get("uri").and_then(|v| v.as_str()) {
                if uri.starts_with("file://") {
                    return Some(uri[7..].to_string());
                }
                return Some(uri.to_string());
            }
        }
        None
    }

    /// Get file extension from file path
    fn get_extension(file_path: &str) -> Option<&str> {
        Path::new(file_path).extension()?.to_str()
    }

    // NOTE: Hard-coded MCP-to-LSP mappings have been removed!
    // All requests now flow through the plugin system which handles translation.
    // The plugin system (DirectLspAdapter) bypasses this manager entirely.

    /// Convert LSP response to MCP response
    fn lsp_to_mcp_response(lsp_result: Value, request_id: Option<Value>) -> McpResponse {
        McpResponse {
            id: request_id,
            result: Some(lsp_result),
            error: None,
        }
    }

    /// Create MCP error response
    fn create_error_response(request_id: Option<Value>, message: String) -> McpResponse {
        McpResponse {
            id: request_id,
            result: None,
            error: Some(McpError {
                code: -1,
                message,
                data: None,
            }),
        }
    }
}

#[async_trait]
impl LspService for LspManager {
    /// Send a request to the appropriate LSP server
    async fn request(&self, message: McpMessage) -> Result<McpMessage, CoreError> {
        match message {
            McpMessage::Request(request) => {
                debug!("Processing LSP request: {}", request.method);

                // Extract file path from request parameters
                let file_path = if let Some(params) = &request.params {
                    Self::extract_file_path(params)
                } else {
                    None
                };

                let file_path = file_path.ok_or_else(|| {
                    CoreError::invalid_data("No file path found in request parameters")
                })?;

                // Get file extension
                let extension = Self::get_extension(&file_path).ok_or_else(|| {
                    CoreError::invalid_data(format!("Could not determine file extension for: {}", file_path))
                })?;

                // Get LSP client for this extension
                let _client = match self.get_client_for_extension(extension).await {
                    Ok(client) => client,
                    Err(e) => {
                        error!("Failed to get LSP client for extension {}: {}", extension, e);
                        return Ok(McpMessage::Response(Self::create_error_response(
                            request.id,
                            format!("LSP server not available for {} files: {}", extension, e),
                        )));
                    }
                };

                // NOTE: This code path is DEPRECATED and bypassed by the plugin system!
                // The plugin system (DirectLspAdapter) goes directly to LspClient.
                // This is only kept for backwards compatibility but should not be used.
                return Err(CoreError::not_supported(
                    "Direct LSP manager requests are deprecated. Use the plugin system instead."
                ));

                // Old code (no longer reached):
                #[allow(unreachable_code)]
                match client.send_request("", json!({})).await {
                    Ok(result) => {
                        debug!("LSP request successful: {}", request.method);
                        Ok(McpMessage::Response(Self::lsp_to_mcp_response(result, request.id)))
                    }
                    Err(e) => {
                        error!("LSP request failed: {}", e);
                        Ok(McpMessage::Response(Self::create_error_response(
                            request.id,
                            format!("LSP request failed: {}", e),
                        )))
                    }
                }
            }
            _ => {
                // Forward other message types as-is
                Err(CoreError::not_supported("Only MCP requests are supported"))
            }
        }
    }

    /// Check if LSP server is available for the given extension
    async fn is_available(&self, extension: &str) -> bool {
        // Check if we have a server configured for this extension
        self.config
            .servers
            .iter()
            .any(|server| server.extensions.contains(&extension.to_string()))
    }

    /// Restart LSP servers for the given extensions
    async fn restart_servers(&self, extensions: Option<Vec<String>>) -> Result<(), CoreError> {
        let mut clients = self.clients.lock().await;

        if let Some(extensions) = extensions {
            // Restart specific extensions
            for extension in extensions {
                if let Some(client) = clients.remove(&extension) {
                    info!("Restarting LSP client for extension: {}", extension);
                    client.kill().await.map_err(|e| CoreError::internal(e.to_string()))?;
                }
            }
        } else {
            // Restart all clients
            info!("Restarting all LSP clients");
            for (extension, client) in clients.drain() {
                info!("Killing LSP client for extension: {}", extension);
                client.kill().await.map_err(|e| CoreError::internal(e.to_string()))?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cb_core::config::LspServerConfig;

    fn create_test_config() -> LspConfig {
        LspConfig {
            servers: vec![
                LspServerConfig {
                    extensions: vec!["ts".to_string(), "js".to_string()],
                    command: vec!["typescript-language-server".to_string(), "--stdio".to_string()],
                    root_dir: None,
                    restart_interval: None,
                },
                LspServerConfig {
                    extensions: vec!["py".to_string()],
                    command: vec!["pylsp".to_string()],
                    root_dir: None,
                    restart_interval: None,
                },
            ],
            default_timeout_ms: 5000,
            enable_preload: false,
        }
    }

    #[tokio::test]
    async fn test_is_available() {
        let config = create_test_config();
        let manager = LspManager::new(config);

        assert!(manager.is_available("ts").await);
        assert!(manager.is_available("js").await);
        assert!(manager.is_available("py").await);
        assert!(!manager.is_available("rs").await);
    }

    #[test]
    fn test_extract_file_path() {
        let params1 = json!({
            "file_path": "/path/to/file.ts"
        });
        assert_eq!(
            LspManager::extract_file_path(&params1),
            Some("/path/to/file.ts".to_string())
        );

        let params2 = json!({
            "textDocument": {
                "uri": "file:///path/to/file.py"
            }
        });
        assert_eq!(
            LspManager::extract_file_path(&params2),
            Some("/path/to/file.py".to_string())
        );

        let params3 = json!({
            "uri": "file:///path/to/file.js"
        });
        assert_eq!(
            LspManager::extract_file_path(&params3),
            Some("/path/to/file.js".to_string())
        );

        let params4 = json!({
            "other": "value"
        });
        assert_eq!(LspManager::extract_file_path(&params4), None);
    }

    #[test]
    fn test_get_extension() {
        assert_eq!(LspManager::get_extension("/path/to/file.ts"), Some("ts"));
        assert_eq!(LspManager::get_extension("/path/to/file.py"), Some("py"));
        assert_eq!(LspManager::get_extension("/path/to/file"), None);
        assert_eq!(LspManager::get_extension("file.js"), Some("js"));
    }

    // Test removed: mcp_to_lsp_request functionality is now handled by the plugin system
}