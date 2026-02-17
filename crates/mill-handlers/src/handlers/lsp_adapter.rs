//! Direct LSP adapter implementation
//!
//! This module provides a direct LSP adapter that bypasses the old LSP manager
//! and its hard-coded mappings, enabling dynamic LSP server configuration.

use async_trait::async_trait;
use mill_plugin_system::LspService;
use mill_services::services::reference_updater::LspImportFinder;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, warn};

/// Information about an LSP progress task
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LspProgressInfo {
    /// Status: "in_progress", "completed", or "failed"
    pub status: String,
    /// Title of the progress task (e.g., "Indexing")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Current message (e.g., "Processing files...")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// Progress percentage (0-100)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub percentage: Option<u32>,
}

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
        // Find server config for this extension and derive a stable cache key.
        let server_config = self
            .config
            .servers
            .iter()
            .find(|server| server.extensions.contains(&extension.to_string()))
            .ok_or_else(|| format!("No LSP server configured for extension: {}", extension))?
            .clone();

        let cache_key = server_config
            .extensions
            .first()
            .cloned()
            .unwrap_or_else(|| extension.to_string());

        // Check if a client already exists and is alive
        let mut clients = self.lsp_clients.lock().await;
        if let Some(client) = clients.get(&cache_key) {
            if client.is_alive().await {
                debug!(extension, cache_key = %cache_key, "Reusing existing, live LSP client");
                return Ok(client.clone());
            } else {
                // PHASE 2: Dead client found - extract it for cleanup
                warn!(
                    extension,
                    cache_key = %cache_key,
                    "Found dead LSP client in cache, removing it before creating a new one."
                );
                let dead_client = clients.remove(&cache_key);

                // Cleanup dead client immediately to prevent zombie processes
                if let Some(dead_client) = dead_client {
                    let ext = cache_key.clone();
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

        // Create new LSP client
        let client = mill_lsp::lsp_system::LspClient::new(server_config)
            .await
            .map_err(|e| format!("Failed to create LSP client: {}", e))?;

        let client = Arc::new(client);

        // Store the client
        {
            let mut clients = self.lsp_clients.lock().await;
            clients.insert(cache_key, client.clone());
        }

        Ok(client)
    }

    /// Get progress from all active LSP clients
    ///
    /// Returns a map of extension -> list of (token, state) pairs for all active progress tasks.
    /// Useful for monitoring LSP server warmup/indexing progress.
    pub async fn get_all_lsp_progress(
        &self,
    ) -> HashMap<String, Vec<(String, LspProgressInfo)>> {
        let clients = self.lsp_clients.lock().await;
        let mut result = HashMap::new();

        for (extension, client) in clients.iter() {
            let progress_list: Vec<(String, LspProgressInfo)> = client
                .get_active_progress()
                .into_iter()
                .map(|(token, state)| {
                    let token_str = token.to_string();
                    let info = match state {
                        mill_lsp::progress::ProgressState::InProgress {
                            title,
                            message,
                            percentage,
                        } => LspProgressInfo {
                            status: "in_progress".to_string(),
                            title: Some(title),
                            message,
                            percentage,
                        },
                        mill_lsp::progress::ProgressState::Completed { message } => {
                            LspProgressInfo {
                                status: "completed".to_string(),
                                title: None,
                                message,
                                percentage: Some(100),
                            }
                        }
                        mill_lsp::progress::ProgressState::Failed { reason } => LspProgressInfo {
                            status: "failed".to_string(),
                            title: None,
                            message: Some(reason),
                            percentage: None,
                        },
                    };
                    (token_str, info)
                })
                .collect();

            if !progress_list.is_empty() {
                result.insert(extension.clone(), progress_list);
            }
        }

        result
    }

    /// Query all active LSP servers for workspace symbols and merge results
    async fn query_all_servers_for_workspace_symbols(
        &self,
        mut params: Value,
    ) -> Result<Value, String> {
        const MAX_WORKSPACE_SYMBOLS: usize = 10_000;
        let mut all_symbols = Vec::new();
        let mut queried_servers = Vec::new();

        // Check for extension filter injected by LspAdapterPlugin
        let filter_extensions: Option<Vec<String>> = if let Value::Object(ref mut map) = params {
            map.remove("__mill_extensions")
                .and_then(|v| serde_json::from_value(v).ok())
        } else {
            None
        };

        // Check for kind filter (optimization)
        let kind_filter: Option<mill_plugin_api::SymbolKind> =
            if let Value::Object(ref mut map) = params {
                map.remove("kind")
                    .and_then(|v| serde_json::from_value(v).ok())
            } else {
                None
            };

        // 1. Identify target clients (serial phase)
        let mut target_clients = Vec::new();
        let mut seen_clients: HashSet<String> = HashSet::new();

        for extension in &self.extensions {
            // Apply filter if present - only query servers relevant to the requesting plugin
            if let Some(ref filter) = filter_extensions {
                if !filter.contains(extension) {
                    continue;
                }
            }

            // Get or create client for this extension
            match self.get_or_create_client(extension).await {
                Ok(client) => {
                    let client_key = client.config().command.join(" ");
                    if !seen_clients.insert(client_key.clone()) {
                        debug!(
                            extension = %extension,
                            client_key = %client_key,
                            "Skipping duplicate workspace/symbol query for shared LSP server"
                        );
                        continue;
                    }

                    // Check if the server supports workspace symbols
                    if client.supports_workspace_symbols().await {
                        target_clients.push((extension.clone(), client));
                    } else {
                        debug!(
                            extension = %extension,
                            "LSP server does not support workspace/symbol, skipping"
                        );
                    }
                }
                Err(e) => {
                    warn!(
                        extension = %extension,
                        error = %e,
                        "Failed to create LSP client for workspace symbol search"
                    );
                }
            }
        }

        // 2. Execute queries in parallel (parallel phase)
        let futures = target_clients.into_iter().map(|(extension, client)| {
            let params = params.clone();
            async move {
                // For rust-analyzer, check if workspace indexing notifications are sent:
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
                if extension == "ts"
                    || extension == "tsx"
                    || extension == "js"
                    || extension == "jsx"
                {
                    debug!(
                        extension = %extension,
                        "TypeScript LSP requires warmup - opening a file to establish project context"
                    );

                    // Try to find and open a representative file to establish project context
                    if let Some(root_dir) = client.config().root_dir.as_ref() {
                        let mut warmup_file = None;

                        // Prefer opening a source file to establish a TS project context.
                        let extensions_to_try = ["ts", "tsx", "js", "jsx"];
                        for ext in &extensions_to_try {
                            if let Ok(mut entries) = tokio::fs::read_dir(root_dir).await {
                                while let Ok(Some(entry)) = entries.next_entry().await {
                                    let path = entry.path();
                                    let is_file = match entry.file_type().await {
                                        Ok(ft) => ft.is_file(),
                                        Err(_) => false,
                                    };

                                    if is_file
                                        && path.extension().and_then(|e| e.to_str()) == Some(ext)
                                    {
                                        warmup_file = Some(path);
                                        break;
                                    }
                                }
                            }
                            if warmup_file.is_some() {
                                break;
                            }
                        }

                        // If still not found, try src directory
                        if warmup_file.is_none() {
                            let src_dir = root_dir.join("src");
                            let src_exists =
                                tokio::fs::try_exists(&src_dir).await.unwrap_or(false);
                            let is_dir = if src_exists {
                                tokio::fs::metadata(&src_dir)
                                    .await
                                    .map(|m| m.is_dir())
                                    .unwrap_or(false)
                            } else {
                                false
                            };

                            if is_dir {
                                if let Ok(mut entries) = tokio::fs::read_dir(&src_dir).await {
                                    while let Ok(Some(entry)) = entries.next_entry().await {
                                        let path = entry.path();
                                        let is_file = match entry.file_type().await {
                                            Ok(ft) => ft.is_file(),
                                            Err(_) => false,
                                        };

                                        if is_file {
                                            if let Some(ext) =
                                                path.extension().and_then(|e| e.to_str())
                                            {
                                                if extensions_to_try.contains(&ext) {
                                                    warmup_file = Some(path);
                                                    break;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // Final fallback: open tsconfig.json if no source file found.
                        if warmup_file.is_none() {
                            let tsconfig = root_dir.join("tsconfig.json");
                            if tsconfig.exists() && tsconfig.is_file() {
                                warmup_file = Some(tsconfig);
                            }
                        }

                        // Open the warmup file if found
                        if let Some(path) = warmup_file {
                            debug!(
                                extension = %extension,
                                warmup_file = %path.display(),
                                "Opening file to warm up TypeScript LSP"
                            );
                            if let Err(e) = client.notify_file_opened(&path).await {
                                warn!(
                                    extension = %extension,
                                    warmup_file = %path.display(),
                                    error = %e,
                                    "Failed to open warmup file for TypeScript LSP"
                                );
                            } else {
                                // Allow the server a short window to register the project context.
                                tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                            }
                        } else {
                            debug!(
                                extension = %extension,
                                "No suitable warmup file found for TypeScript LSP"
                            );
                        }
                    }
                }

                // Send workspace/symbol request to this server
                let result = client.send_request("workspace/symbol", params).await;
                (extension, result)
            }
        });

        let results = futures::future::join_all(futures).await;

        // 3. Merge results (serial phase)
        for (extension, result) in results {
            match result {
                Ok(response) => {
                    // Extract symbols from response - consume the response to avoid cloning
                    if let Value::Array(symbols) = response {
                        debug!(
                            extension = %extension,
                            symbol_count = symbols.len(),
                            "Got workspace symbols from LSP server"
                        );

                        // Filter by kind if requested (optimization)
                        if let Some(target_kind) = kind_filter {
                            for symbol in symbols {
                                if let Some(kind_num) = symbol.get("kind").and_then(|k| k.as_u64())
                                {
                                    if let Some(sym_kind) =
                                        mill_plugin_api::SymbolKind::from_lsp_kind(kind_num)
                                    {
                                        if sym_kind == target_kind {
                                            all_symbols.push(symbol);
                                        }
                                    }
                                }
                            }
                        } else {
                            all_symbols.extend(symbols);
                        }

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

        if all_symbols.is_empty() {
            return Ok(json!([]));
        }

        debug!(
            total_symbols = all_symbols.len(),
            servers = ?queried_servers,
            "Merged workspace symbols from multiple LSP servers"
        );

        Ok(Value::Array(all_symbols))
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

    /// Send workspace/willRenameFiles request to get import updates for a file rename
    ///
    /// This is the CORRECT LSP method for finding files that need import updates.
    /// Unlike textDocument/references (which returns symbol usages), this method
    /// returns a WorkspaceEdit with the actual import path changes needed.
    ///
    /// Returns the list of files that would need import updates.
    pub async fn find_files_using_will_rename(
        &self,
        old_path: &std::path::Path,
        new_path: &std::path::Path,
    ) -> Result<Vec<std::path::PathBuf>, String> {
        // Get extension from file path
        let extension = old_path
            .extension()
            .and_then(|e| e.to_str())
            .ok_or_else(|| format!("Could not get extension from path: {}", old_path.display()))?;

        // Get or create LSP client for this extension
        let client = self.get_or_create_client(extension).await?;

        // Check if the server supports willRenameFiles
        // TypeScript LSP supports this via fileOperations.willRename capability
        if !client.supports_will_rename_files().await {
            debug!(
                extension = %extension,
                "LSP server does not support workspace/willRenameFiles"
            );
            return Err(format!(
                "LSP server for '{}' does not support workspace/willRenameFiles",
                extension
            ));
        }

        // Ensure file is open in LSP for proper context
        if let Err(e) = client.notify_file_opened(old_path).await {
            debug!(
                file = %old_path.display(),
                error = %e,
                "Failed to open file in LSP for willRenameFiles"
            );
            // Continue anyway - file might already be open
        }

        // Build the request params
        let old_uri = format!("file://{}", old_path.display());
        let new_uri = format!("file://{}", new_path.display());

        let params = json!({
            "files": [
                {
                    "oldUri": &old_uri,
                    "newUri": &new_uri
                }
            ]
        });

        debug!(
            old_path = %old_path.display(),
            new_path = %new_path.display(),
            "Sending workspace/willRenameFiles request"
        );

        // Send the request
        let response = client
            .send_request("workspace/willRenameFiles", params)
            .await
            .map_err(|e| format!("workspace/willRenameFiles request failed: {}", e))?;

        // Parse the WorkspaceEdit response to extract affected files
        let affected_files = Self::extract_affected_files_from_workspace_edit(&response);

        debug!(
            old_path = %old_path.display(),
            affected_files_count = affected_files.len(),
            "workspace/willRenameFiles returned affected files"
        );

        Ok(affected_files)
    }

    /// Extract file paths from a WorkspaceEdit response
    ///
    /// WorkspaceEdit can have two formats:
    /// 1. "changes": { uri -> TextEdit[] }
    /// 2. "documentChanges": TextDocumentEdit[]
    ///
    /// This function handles both and returns unique file paths.
    fn extract_affected_files_from_workspace_edit(
        workspace_edit: &serde_json::Value,
    ) -> Vec<std::path::PathBuf> {
        let mut files = std::collections::HashSet::new();

        // Format 1: "changes" (uri -> edits[])
        if let Some(changes) = workspace_edit.get("changes").and_then(|c| c.as_object()) {
            for uri in changes.keys() {
                if let Some(path) = Self::uri_to_path(uri) {
                    files.insert(path);
                }
            }
        }

        // Format 2: "documentChanges" (array of TextDocumentEdit)
        if let Some(doc_changes) = workspace_edit
            .get("documentChanges")
            .and_then(|d| d.as_array())
        {
            for change in doc_changes {
                // TextDocumentEdit has textDocument.uri
                if let Some(uri) = change
                    .get("textDocument")
                    .and_then(|td| td.get("uri"))
                    .and_then(|u| u.as_str())
                {
                    if let Some(path) = Self::uri_to_path(uri) {
                        files.insert(path);
                    }
                }
            }
        }

        files.into_iter().collect()
    }

    /// Convert a file:// URI to a PathBuf
    fn uri_to_path(uri: &str) -> Option<std::path::PathBuf> {
        if !uri.starts_with("file://") {
            return None;
        }
        let path_str = uri.trim_start_matches("file://");
        // Handle URL-encoded paths (spaces become %20, etc.)
        match urlencoding::decode(path_str) {
            Ok(decoded) => Some(std::path::PathBuf::from(decoded.as_ref())),
            Err(_) => Some(std::path::PathBuf::from(path_str)),
        }
    }
}

#[async_trait]
impl LspImportFinder for DirectLspAdapter {
    /// Find all files that import/reference the given file path
    ///
    /// Uses LSP's workspace/willRenameFiles to find all files that would need
    /// import updates if this file were renamed. This is the CORRECT approach
    /// (unlike textDocument/references which returns symbol usages).
    ///
    /// Returns a list of file paths that import the given file.
    async fn find_files_that_import(
        &self,
        file_path: &std::path::Path,
    ) -> Result<Vec<std::path::PathBuf>, String> {
        // Generate a hypothetical new path for the willRenameFiles query.
        // We use a path that preserves the extension but changes the name,
        // which triggers the LSP to compute all import updates needed.
        let hypothetical_new_path = if let Some(parent) = file_path.parent() {
            let stem = file_path.file_stem().and_then(|s| s.to_str()).unwrap_or("file");
            let ext = file_path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if ext.is_empty() {
                parent.join(format!("{}_renamed", stem))
            } else {
                parent.join(format!("{}_renamed.{}", stem, ext))
            }
        } else {
            return Err(format!(
                "Could not determine parent directory for: {}",
                file_path.display()
            ));
        };

        // Use workspace/willRenameFiles to find files that would need import updates
        match self
            .find_files_using_will_rename(file_path, &hypothetical_new_path)
            .await
        {
            Ok(files) => {
                debug!(
                    file = %file_path.display(),
                    importing_files_count = files.len(),
                    "Found files that import this file via workspace/willRenameFiles"
                );
                Ok(files)
            }
            Err(e) => {
                debug!(
                    file = %file_path.display(),
                    error = %e,
                    "workspace/willRenameFiles failed - returning empty list"
                );
                // Return empty list instead of falling back to broken textDocument/references.
                // The plugin-based scanner will be used as the fallback in ReferenceUpdater.
                Ok(Vec::new())
            }
        }
    }

    /// Find all files that import any file within a directory
    ///
    /// This is used for directory moves to find all external importers.
    async fn find_files_that_import_directory(
        &self,
        dir_path: &std::path::Path,
    ) -> Result<Vec<std::path::PathBuf>, String> {
        use std::collections::HashSet;

        let mut all_importing_files: HashSet<std::path::PathBuf> = HashSet::new();

        // Walk the directory to find all source files
        let walker = ignore::WalkBuilder::new(dir_path)
            .hidden(false)
            .git_ignore(true)
            .build();

        let mut files_in_dir = Vec::new();
        for entry in walker.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if self.extensions.contains(&ext.to_string()) {
                        files_in_dir.push(path.to_path_buf());
                    }
                }
            }
        }

        debug!(
            dir = %dir_path.display(),
            files_count = files_in_dir.len(),
            "Found source files in directory for LSP reference search"
        );

        // Limit the number of files to check to avoid too many LSP calls
        const MAX_FILES_TO_CHECK: usize = 20;
        let files_to_check: Vec<_> = files_in_dir.into_iter().take(MAX_FILES_TO_CHECK).collect();

        // Find importers for each file using workspace/willRenameFiles
        for file_path in &files_to_check {
            match self.find_files_that_import(file_path).await {
                Ok(importers) => {
                    for importer in importers {
                        // Skip files inside the directory being moved
                        if !importer.starts_with(dir_path) {
                            all_importing_files.insert(importer);
                        }
                    }
                }
                Err(e) => {
                    debug!(
                        file = %file_path.display(),
                        error = %e,
                        "Failed to find importers for file in directory"
                    );
                    // Continue with other files
                }
            }
        }

        debug!(
            dir = %dir_path.display(),
            importing_files_count = all_importing_files.len(),
            "Found external files that import from directory via LSP"
        );

        Ok(all_importing_files.into_iter().collect())
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
        if method == "textDocument/diagnostic" && !client.supports_diagnostic_pull().await {
            // Fall back to cached diagnostics from publishDiagnostics notifications
            debug!(
                extension = %extension,
                "LSP server doesn't support pull-model diagnostics, using cached diagnostics"
            );

            // Extract URI from params
            let uri = params
                .get("textDocument")
                .and_then(|td| td.get("uri"))
                .and_then(|u| u.as_str())
                .ok_or_else(|| {
                    "Missing textDocument.uri in textDocument/diagnostic params".to_string()
                })?;

            // Parse URI string into lsp_types::Uri
            let uri_parsed = uri
                .parse::<lsp_types::Uri>()
                .map_err(|e| format!("Failed to parse URI '{}': {}", uri, e))?;

            // Get cached diagnostics for this file
            if let Some(diagnostics) = client.get_cached_diagnostics(&uri_parsed).await {
                debug!(
                    uri = %uri,
                    diagnostic_count = diagnostics.len(),
                    "Returning cached diagnostics"
                );

                // Return diagnostics in LSP pull-model format
                return Ok(json!({
                    "items": diagnostics
                }));
            } else {
                // No cached diagnostics - return empty set to avoid hard failure
                debug!(
                    uri = %uri,
                    "No cached diagnostics available; returning empty diagnostics"
                );
                return Ok(json!({
                    "items": []
                }));
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

    fn as_import_finder(&self) -> &dyn LspImportFinder {
        self
    }
}
