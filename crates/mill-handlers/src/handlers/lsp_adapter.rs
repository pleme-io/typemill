//! Direct LSP adapter implementation
//!
//! This module provides a direct LSP adapter that bypasses the old LSP manager
//! and its hard-coded mappings, enabling dynamic LSP server configuration.

use async_trait::async_trait;
use mill_plugin_system::LspService;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, warn};

/// Direct LSP adapter that bypasses the old LSP manager and its hard-coded mappings
#[derive(Clone)]
pub struct DirectLspAdapter {
    /// LSP clients by extension
    lsp_clients: Arc<Mutex<HashMap<String, Arc<mill_lsp::lsp_system::LspClient>>>>,
    /// LSP configuration
    config: mill_config::config::LspConfig,
    /// Supported file extensions
    extensions: Vec<String>,
    /// Adapter name
    name: String,
}

impl DirectLspAdapter {
    pub fn new(
        config: mill_config::config::LspConfig,
        extensions: Vec<String>,
        name: String,
    ) -> Self {
        Self {
            lsp_clients: Arc::new(Mutex::new(HashMap::new())),
            config,
            extensions,
            name,
        }
    }

    /// Get or create an LSP client for the given extension
    pub async fn get_or_create_client(
        &self,
        extension: &str,
    ) -> Result<Arc<mill_lsp::lsp_system::LspClient>, String> {
        // Check if a client already exists and is alive
        let mut clients = self.lsp_clients.lock().await;
        if let Some(client) = clients.get(extension) {
            if client.is_alive().await {
                debug!(extension, "Reusing existing, live LSP client");
                return Ok(client.clone());
            } else {
                // PHASE 2: Dead client found - extract it for cleanup
                warn!(
                    extension,
                    "Found dead LSP client in cache, removing it before creating a new one."
                );
                let dead_client = clients.remove(extension);

                // Cleanup dead client immediately to prevent zombie processes
                if let Some(dead_client) = dead_client {
                    let ext = extension.to_string();
                    tokio::spawn(async move {
                        // Force shutdown (kill + wait) to prevent zombies
                        if let Err(e) = dead_client.force_shutdown().await {
                            warn!(
                                extension = %ext,
                                error = %e,
                                "Failed to force shutdown dead LSP client"
                            );
                        } else {
                            debug!(
                                extension = %ext,
                                "Force shutdown of dead LSP client completed"
                            );
                        }
                    });
                }
                // Proceed to create a new client below
            }
        }
        // Drop the lock before the potentially long operation of creating a new client
        drop(clients);

        // Find server config for this extension
        let server_config = self
            .config
            .servers
            .iter()
            .find(|server| server.extensions.contains(&extension.to_string()))
            .ok_or_else(|| format!("No LSP server configured for extension: {}", extension))?
            .clone();

        // Create new LSP client
        let client = mill_lsp::lsp_system::LspClient::new(server_config)
            .await
            .map_err(|e| format!("Failed to create LSP client: {}", e))?;

        let client = Arc::new(client);

        // Store the client
        {
            let mut clients = self.lsp_clients.lock().await;
            clients.insert(extension.to_string(), client.clone());
        }

        Ok(client)
    }

    /// Query all active LSP servers for workspace symbols and merge results
    async fn query_all_servers_for_workspace_symbols(
        &self,
        params: Value,
    ) -> Result<Value, String> {
        const MAX_WORKSPACE_SYMBOLS: usize = 10_000;
        let mut all_symbols = Vec::new();
        let mut queried_servers = Vec::new();

        // Query each supported extension's LSP server
        for extension in &self.extensions {
            // Get or create client for this extension
            match self.get_or_create_client(extension).await {
                Ok(client) => {
                    // Check if the server supports workspace symbols
                    if !client.supports_workspace_symbols().await {
                        debug!(
                            extension = %extension,
                            "LSP server does not support workspace/symbol, skipping"
                        );
                        continue;
                    }

                    // For rust-analyzer, check if workspace indexing notifications are sent:
                    // 1. Try event-driven wait for progress notifications (500ms timeout)
                    // 2. If no progress notification arrives, assume indexing is instant or not needed
                    // This handles both cases: servers that send $/progress and those that complete instantly
                    if extension == "rs" {
                        debug!(
                            extension = %extension,
                            "Checking for rust-analyzer workspace indexing progress"
                        );

                        let token = mill_lsp::progress::ProgressToken::String(
                            "rustAnalyzer/Indexing".to_string(),
                        );

                        // Check if indexing is already completed
                        if client.is_progress_completed(&token) {
                            debug!(
                                extension = %extension,
                                "rust-analyzer indexing already complete"
                            );
                        } else {
                            // Wait briefly (500ms) to see if indexing progress notification arrives
                            // rust-analyzer doesn't send progress for small projects that index instantly
                            match client
                                .wait_for_indexing(std::time::Duration::from_millis(500))
                                .await
                            {
                                Ok(()) => {
                                    debug!(
                                        extension = %extension,
                                        "rust-analyzer indexing complete via progress notification"
                                    );
                                }
                                Err(_) => {
                                    // No progress notification - indexing either instant or not happening
                                    debug!(
                                        extension = %extension,
                                        "No progress notification in 500ms - indexing complete or not needed"
                                    );
                                }
                            }
                        }
                    }

                    // For TypeScript, warm up the server by opening a file first
                    // TypeScript LSP needs project context before workspace/symbol works
                    if extension == "ts" || extension == "tsx" || extension == "js" || extension == "jsx" {
                        // Try to warm up by ensuring the server has project context
                        // The didOpen notification will be sent automatically when we make a file-specific request
                        debug!(
                            extension = %extension,
                            "TypeScript LSP may need warmup - workspace/symbol might return 'No Project' on cold start"
                        );
                    }

                    // Send workspace/symbol request to this server
                    match client
                        .send_request("workspace/symbol", params.clone())
                        .await
                    {
                        Ok(response) => {
                            // Extract symbols from response
                            if let Some(symbols) = response.as_array() {
                                debug!(
                                    extension = %extension,
                                    symbol_count = symbols.len(),
                                    "Got workspace symbols from LSP server"
                                );
                                all_symbols.extend_from_slice(symbols);
                                queried_servers.push(extension.clone());

                                // Prevent unbounded symbol collection
                                if all_symbols.len() >= MAX_WORKSPACE_SYMBOLS {
                                    debug!(
                                        symbol_count = all_symbols.len(),
                                        "Reached maximum workspace symbol limit, stopping collection"
                                    );
                                    break;
                                }
                            }
                        }
                        Err(e) => {
                            // Log error but continue with other servers
                            warn!(
                                extension = %extension,
                                error = %e,
                                "Failed to get workspace symbols from LSP server"
                            );
                        }
                    }
                }
                Err(e) => {
                    // Log error but continue with other servers
                    warn!(
                        extension = %extension,
                        error = %e,
                        "Failed to create LSP client for workspace symbol search"
                    );
                }
            }
        }

        if all_symbols.is_empty() {
            return Ok(json!([]));
        }

        debug!(
            total_symbols = all_symbols.len(),
            servers = ?queried_servers,
            "Merged workspace symbols from multiple LSP servers"
        );

        Ok(json!(all_symbols))
    }

    /// Gracefully shutdown all LSP clients
    pub async fn shutdown(&self) -> Result<(), String> {
        let mut clients_map = self.lsp_clients.lock().await;
        let client_count = clients_map.len();

        if client_count == 0 {
            return Ok(());
        }

        debug!(
            adapter_name = %self.name,
            client_count = client_count,
            "Shutting down all LSP clients in DirectLspAdapter"
        );

        let mut errors = Vec::new();

        // Drain all clients and shutdown
        for (extension, client) in clients_map.drain() {
            let strong_count = Arc::strong_count(&client);

            // Force shutdown (kill + wait) to prevent zombies
            if let Err(e) = client.force_shutdown().await {
                warn!(
                    extension = %extension,
                    error = %e,
                    "Failed to force shutdown LSP client during adapter shutdown"
                );
                errors.push(format!(
                    "Failed to force shutdown {} client: {}",
                    extension, e
                ));
            } else {
                debug!(
                    extension = %extension,
                    arc_strong_count = strong_count,
                    "Force shutdown LSP client completed during adapter shutdown"
                );
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors.join("; "))
        }
    }

    /// Extract file extension from LSP params
    fn extract_extension_from_params(&self, params: &Value, method: &str) -> Option<String> {
        // For workspace-level operations, no longer needed since we handle them specially
        match method {
            "workspace/symbol" => {
                // This path should not be reached anymore - handled in request() method
                warn!("extract_extension_from_params called for workspace/symbol - should be handled specially");
                None
            }
            _ => {
                // For file-specific operations, extract from textDocument.uri
                if let Some(uri) = params.get("textDocument")?.get("uri")?.as_str() {
                    if uri.starts_with("file://") {
                        let path = uri.trim_start_matches("file://");
                        return std::path::Path::new(path)
                            .extension()?
                            .to_str()
                            .map(|s| s.to_string());
                    }
                }
                None
            }
        }
    }
}

#[async_trait]
impl LspService for DirectLspAdapter {
    async fn request(&self, method: &str, params: Value) -> Result<Value, String> {
        // Special handling for workspace/symbol - query ALL active LSP servers
        if method == "workspace/symbol" {
            return self.query_all_servers_for_workspace_symbols(params).await;
        }

        // Extract extension from params for file-specific operations
        let extension = self
            .extract_extension_from_params(&params, method)
            .ok_or_else(|| {
                format!(
                    "Could not extract file extension from params for method '{}'",
                    method
                )
            })?;

        // Get appropriate LSP client
        let client = self.get_or_create_client(&extension).await?;

        // Check capabilities before sending requests that may not be supported
        if method == "textDocument/diagnostic" {
            if !client.supports_diagnostic_pull().await {
                return Err(format!(
                    "LSP server for '{}' does not support pull-model diagnostics (textDocument/diagnostic). \
                     This server uses push-model diagnostics (textDocument/publishDiagnostics) instead.",
                    extension
                ));
            }
        }

        // Send LSP method DIRECTLY to client (bypassing old manager and its hard-coded mappings!)
        client
            .send_request(method, params)
            .await
            .map_err(|e| format!("LSP request failed: {}", e))
    }

    fn supports_extension(&self, extension: &str) -> bool {
        self.extensions.contains(&extension.to_string())
    }

    fn service_name(&self) -> String {
        self.name.clone()
    }
}

impl Drop for DirectLspAdapter {
    fn drop(&mut self) {
        // Attempt to shutdown all LSP clients when the adapter is dropped
        // Use a blocking thread pool to avoid relying on tokio runtime
        // which may be shutting down during Drop

        let clients = self.lsp_clients.clone();
        let adapter_name = self.name.clone();

        // Spawn on a dedicated thread pool, not tokio runtime
        std::thread::spawn(move || {
            // Create a new tokio runtime for cleanup
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async move {
                let mut clients_map = clients.lock().await;
                let client_count = clients_map.len();

                if client_count == 0 {
                    return;
                }

                tracing::debug!(
                    adapter_name = %adapter_name,
                    client_count = client_count,
                    "DirectLspAdapter dropping - attempting to shutdown LSP clients"
                );

                // Drain all clients and attempt shutdown
                for (extension, client) in clients_map.drain() {
                    let strong_count = Arc::strong_count(&client);

                    // Force shutdown (kill + wait) to prevent zombies
                    if let Err(e) = client.force_shutdown().await {
                        tracing::warn!(
                            extension = %extension,
                            error = %e,
                            arc_strong_count = strong_count,
                            "Failed to force shutdown LSP client from DirectLspAdapter drop"
                        );
                    } else {
                        tracing::debug!(
                            extension = %extension,
                            arc_strong_count = strong_count,
                            "Force shutdown LSP client completed from DirectLspAdapter drop"
                        );
                    }
                }
            });
        });
    }
}

// Implement the LspAdapter trait for DirectLspAdapter
#[async_trait]
impl mill_handler_api::LspAdapter for DirectLspAdapter {
    async fn get_or_create_client(
        &self,
        file_extension: &str,
    ) -> Result<Arc<mill_lsp::lsp_system::LspClient>, mill_foundation::errors::MillError> {
        // Delegate to the existing implementation, converting error type
        self.get_or_create_client(file_extension)
            .await
            .map_err(mill_foundation::errors::MillError::lsp)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
