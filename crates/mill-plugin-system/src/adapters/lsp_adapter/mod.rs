//! LSP adapter plugin that translates between the plugin system and LSP protocol
//!
//! This adapter serves as a bridge, allowing the plugin system to work with
//! existing LSP servers without requiring changes to the LSP implementation.

pub mod constructors;
pub mod request_translator;
pub mod response_normalizer;
pub mod tool_definitions;

#[cfg(test)]
mod tests;

use crate::{
    Capabilities, DiagnosticCapabilities, EditingCapabilities, IntelligenceCapabilities,
    LanguagePlugin, NavigationCapabilities, PluginMetadata, PluginRequest, PluginResponse,
    PluginResult, PluginSystemError, RefactoringCapabilities,
};
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, error};

/// Trait for LSP service integration
/// This allows the adapter to work with any LSP implementation
#[async_trait]
pub trait LspService: Send + Sync {
    /// Send a request to the LSP server and get a response
    async fn request(&self, method: &str, params: Value) -> Result<Value, String>;

    /// Check if the service supports a specific file extension
    fn supports_extension(&self, extension: &str) -> bool;

    /// Get the service name for debugging
    fn service_name(&self) -> String;
}

/// LSP adapter plugin that bridges plugin system with LSP servers
pub struct LspAdapterPlugin {
    /// Plugin metadata
    metadata: PluginMetadata,
    /// Supported file extensions
    extensions: Vec<String>,
    /// Computed capabilities based on LSP support
    capabilities: Capabilities,
    /// LSP service for handling requests
    lsp_service: Arc<dyn LspService>,
    /// Method mapping cache for performance
    method_cache: Arc<Mutex<HashMap<String, String>>>,
}

#[async_trait]
impl LanguagePlugin for LspAdapterPlugin {
    fn metadata(&self) -> PluginMetadata {
        self.metadata.clone()
    }

    fn supported_extensions(&self) -> Vec<String> {
        self.extensions.clone()
    }

    fn tool_definitions(&self) -> Vec<Value> {
        tool_definitions::tool_definitions()
    }

    fn capabilities(&self) -> Capabilities {
        self.capabilities.clone()
    }

    async fn handle_request(&self, request: PluginRequest) -> PluginResult<PluginResponse> {
        debug!("LSP adapter handling request: {}", request.method);

        // Skip file extension check for workspace-level operations
        let is_workspace_operation = matches!(request.method.as_str(), "search_workspace_symbols");

        // Check if we support the file extension (skip for workspace operations)
        if !is_workspace_operation && !self.can_handle_file(&request.file_path) {
            return Err(PluginSystemError::plugin_not_found(
                request.file_path.to_string_lossy(),
                &request.method,
            ));
        }

        // Translate plugin request to LSP request
        let (lsp_method, mut lsp_params) = self.translate_request(&request).await?;

        // Optimization: If this is a workspace/symbol request, inject the plugin's supported extensions
        // so DirectLspAdapter knows to only query the relevant servers.
        if lsp_method == "workspace/symbol" {
            if let Value::Object(ref mut map) = lsp_params {
                map.insert(
                    "__mill_extensions".to_string(),
                    serde_json::to_value(&self.extensions).unwrap_or(Value::Null),
                );
            }
        }

        debug!(
            "Translated to LSP method: {} with params: {}",
            lsp_method, lsp_params
        );

        // Send request to LSP service
        match self.lsp_service.request(&lsp_method, lsp_params).await {
            Ok(lsp_result) => {
                debug!("LSP service returned result");
                self.translate_response(lsp_result, &request)
            }
            Err(err) => {
                error!("LSP service error: {}", err);
                Err(PluginSystemError::request_failed(&self.metadata.name, err))
            }
        }
    }

    fn configure(&self, _config: Value) -> PluginResult<()> {
        // LSP adapters generally don't need additional configuration
        // The LSP service handles its own configuration
        Ok(())
    }

    fn on_file_open(&self, path: &Path) -> PluginResult<()> {
        debug!(
            path = %path.display(),
            plugin = %self.metadata.name,
            "File opened - hook triggered"
        );

        // Note: The actual LSP textDocument/didOpen notification is sent by
        // the DirectLspAdapter in plugin_dispatcher.rs via LspClient::notify_file_opened().
        // This hook serves as a notification point for the plugin to be aware of file lifecycle.
        // Future enhancements could add plugin-specific logic here (e.g., invalidate caches,
        // update internal state, etc.)

        Ok(())
    }

    fn on_file_save(&self, path: &Path) -> PluginResult<()> {
        debug!(
            path = %path.display(),
            plugin = %self.metadata.name,
            "File saved - hook triggered"
        );

        // Note: Future implementation could send textDocument/didSave notification
        // when notify_file_saved tool is added to the MCP API

        Ok(())
    }

    fn on_file_close(&self, path: &Path) -> PluginResult<()> {
        debug!(
            path = %path.display(),
            plugin = %self.metadata.name,
            "File closed - hook triggered"
        );

        // Note: Future implementation could send textDocument/didClose notification
        // when notify_file_closed tool is added to the MCP API

        Ok(())
    }
}
