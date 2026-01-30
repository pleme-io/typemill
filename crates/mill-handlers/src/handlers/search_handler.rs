//! Search tool handler
//!
//! Handles: search_code
//!
//! Implements workspace-wide symbol search with filtering and pagination.
//! Reuses the existing workspace symbol search functionality via the plugin system
//! but exposes it through a more specialized API for code search.

use super::tools::ToolHandler;
use async_trait::async_trait;
use mill_foundation::core::model::mcp::ToolCall;
use mill_foundation::errors::{MillError as ServerError, MillResult as ServerResult};
use mill_plugin_api::SymbolKind;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::PathBuf;
use tracing::{debug, warn};

// ============================================================================
// Request/Response Types
// ============================================================================

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SearchCodeRequest {
    query: String,
    #[serde(default)]
    kind: Option<String>,
    #[serde(default)]
    workspace_path: Option<String>,
    #[serde(default = "default_limit")]
    limit: usize,
    #[serde(default)]
    offset: usize,
}

fn default_limit() -> usize {
    50
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SearchCodeResponse {
    results: Vec<Value>,
    total: usize,
    processing_time_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    warnings: Option<Vec<String>>,
}

// ============================================================================
// SearchHandler - Public Interface
// ============================================================================

pub struct SearchHandler;

impl SearchHandler {
    pub fn new() -> Self {
        Self
    }

    /// Parse symbol kind from string
    fn parse_symbol_kind(kind_str: &str) -> Option<SymbolKind> {
        match kind_str.to_lowercase().as_str() {
            "function" | "func" | "fn" => Some(SymbolKind::Function),
            "class" => Some(SymbolKind::Class),
            "interface" => Some(SymbolKind::Interface),
            "struct" | "structure" => Some(SymbolKind::Struct),
            "enum" | "enumeration" => Some(SymbolKind::Enum),
            "variable" | "var" | "let" => Some(SymbolKind::Variable),
            "constant" | "const" => Some(SymbolKind::Constant),
            "module" | "mod" | "namespace" => Some(SymbolKind::Module),
            "method" => Some(SymbolKind::Method),
            "field" | "property" => Some(SymbolKind::Field),
            "type" | "typedef" | "trait" => Some(SymbolKind::Other),
            _ => None,
        }
    }

    /// Convert SymbolKind to string for comparison
    fn symbol_kind_to_string(kind: &SymbolKind) -> String {
        match kind {
            SymbolKind::Function => "function".to_string(),
            SymbolKind::Class => "class".to_string(),
            SymbolKind::Interface => "interface".to_string(),
            SymbolKind::Struct => "struct".to_string(),
            SymbolKind::Enum => "enum".to_string(),
            SymbolKind::Variable => "variable".to_string(),
            SymbolKind::Constant => "constant".to_string(),
            SymbolKind::Module => "module".to_string(),
            SymbolKind::Method => "method".to_string(),
            SymbolKind::Field => "field".to_string(),
            SymbolKind::Other => "other".to_string(),
        }
    }

    /// Filter symbols by kind if specified
    fn filter_by_kind(symbols: Vec<Value>, kind_filter: Option<SymbolKind>) -> Vec<Value> {
        if let Some(target_kind) = kind_filter {
            let target_kind_str = Self::symbol_kind_to_string(&target_kind).to_lowercase();

            symbols
                .into_iter()
                .filter(|symbol| {
                    // Try to extract kind from the symbol
                    let symbol_kind = symbol
                        .get("kind")
                        .and_then(|k| k.as_str())
                        .map(|s| s.to_lowercase());

                    // Also try the "symbolKind" field (used by some LSP responses)
                    let symbol_kind = symbol_kind.or_else(|| {
                        symbol
                            .get("symbolKind")
                            .and_then(|k| k.as_str())
                            .map(|s| s.to_lowercase())
                    });

                    // Also try numeric LSP SymbolKind values
                    let symbol_kind = symbol_kind.or_else(|| {
                        symbol
                            .get("kind")
                            .and_then(|k| k.as_u64())
                            .and_then(Self::lsp_symbol_kind_to_string)
                            .map(|s| s.to_lowercase())
                    });

                    if let Some(kind_str) = symbol_kind {
                        // Flexible matching: "function" matches "function", "func", "fn"
                        match target_kind {
                            SymbolKind::Function => {
                                kind_str.contains("function")
                                    || kind_str == "func"
                                    || kind_str == "fn"
                            }
                            SymbolKind::Variable => {
                                kind_str.contains("variable")
                                    || kind_str == "var"
                                    || kind_str == "let"
                            }
                            SymbolKind::Constant => {
                                kind_str.contains("constant") || kind_str == "const"
                            }
                            SymbolKind::Module => {
                                kind_str.contains("module")
                                    || kind_str == "mod"
                                    || kind_str.contains("namespace")
                            }
                            SymbolKind::Field => {
                                kind_str.contains("field") || kind_str.contains("property")
                            }
                            _ => kind_str.contains(&target_kind_str),
                        }
                    } else {
                        false
                    }
                })
                .collect()
        } else {
            symbols
        }
    }

    /// Convert LSP numeric SymbolKind to string
    fn lsp_symbol_kind_to_string(kind: u64) -> Option<String> {
        // LSP SymbolKind numeric values
        match kind {
            1 => Some("file".to_string()),
            2 => Some("module".to_string()),
            3 => Some("namespace".to_string()),
            4 => Some("package".to_string()),
            5 => Some("class".to_string()),
            6 => Some("method".to_string()),
            7 => Some("property".to_string()),
            8 => Some("field".to_string()),
            9 => Some("constructor".to_string()),
            10 => Some("enum".to_string()),
            11 => Some("interface".to_string()),
            12 => Some("function".to_string()),
            13 => Some("variable".to_string()),
            14 => Some("constant".to_string()),
            15 => Some("string".to_string()),
            16 => Some("number".to_string()),
            17 => Some("boolean".to_string()),
            18 => Some("array".to_string()),
            19 => Some("object".to_string()),
            20 => Some("key".to_string()),
            21 => Some("null".to_string()),
            22 => Some("enummember".to_string()),
            23 => Some("struct".to_string()),
            24 => Some("event".to_string()),
            25 => Some("operator".to_string()),
            26 => Some("typeparameter".to_string()),
            _ => None,
        }
    }

    /// Apply pagination to results
    fn paginate(symbols: Vec<Value>, limit: usize, offset: usize) -> Vec<Value> {
        symbols.into_iter().skip(offset).take(limit).collect()
    }

    /// Find a representative file in the workspace with the given extension
    async fn find_representative_file(
        workspace_path: &std::path::Path,
        extension: &str,
    ) -> Option<PathBuf> {
        use tokio::fs;

        // First, try to find a file in common source directories
        let common_dirs = ["src", "lib", "packages", "apps", "."];

        for dir in common_dirs {
            let search_path = if dir == "." {
                workspace_path.to_path_buf()
            } else {
                workspace_path.join(dir)
            };

            if search_path.is_dir() {
                // Look for files with the target extension
                if let Ok(mut entries) = fs::read_dir(&search_path).await {
                    while let Ok(Some(entry)) = entries.next_entry().await {
                        let path = entry.path();
                        if path.is_file() {
                            if let Some(ext) = path.extension() {
                                if ext == extension {
                                    return Some(path);
                                }
                            }
                        }
                    }
                }
            }
        }

        // Fall back to recursive search (limited depth)
        Box::pin(Self::find_file_recursive(workspace_path, extension, 3)).await
    }

    async fn find_file_recursive(
        dir: &std::path::Path,
        extension: &str,
        max_depth: u32,
    ) -> Option<PathBuf> {
        use tokio::fs;

        if max_depth == 0 {
            return None;
        }

        if let Ok(mut entries) = fs::read_dir(dir).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                let path = entry.path();

                // Skip hidden directories and node_modules
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.starts_with('.') || name == "node_modules" || name == "target" {
                        continue;
                    }
                }

                if path.is_file() {
                    if let Some(ext) = path.extension() {
                        if ext == extension {
                            return Some(path);
                        }
                    }
                } else if path.is_dir() {
                    if let Some(found) =
                        Box::pin(Self::find_file_recursive(&path, extension, max_depth - 1)).await
                    {
                        return Some(found);
                    }
                }
            }
        }

        None
    }

    /// Perform workspace-wide symbol search across all plugins
    async fn search_workspace_symbols(
        &self,
        context: &mill_handler_api::ToolHandlerContext,
        query: &str,
        workspace_path: PathBuf,
    ) -> ServerResult<(Vec<Value>, u64, Option<Vec<String>>)> {
        use std::time::Instant;

        debug!("search_workspace_symbols: Starting multi-plugin workspace search");

        let start_time = Instant::now();

        // Get all registered plugins
        let plugin_names = context.plugin_manager.list_plugins().await;
        debug!(
            plugin_count = plugin_names.len(),
            plugins = ?plugin_names,
            workspace = %workspace_path.display(),
            "search_workspace_symbols: Found registered plugins"
        );

        let search_args = json!({
            "query": query,
            "workspacePath": workspace_path.to_string_lossy()
        });

        // Parallelize plugin queries
        let mut futures = Vec::new();

        for plugin_name in plugin_names {
            let plugin_manager = context.plugin_manager.clone();
            let workspace_path = workspace_path.clone();
            let search_args = search_args.clone();
            let plugin_name_owned = plugin_name.clone();

            futures.push(async move {
                let mut symbols = Vec::new();
                let mut warning = None;

                if let Some(plugin) = plugin_manager.get_plugin_by_name(&plugin_name_owned).await {
                    // Get supported extensions for this plugin
                    let extensions = plugin.supported_extensions();
                    if let Some(ext) = extensions.first() {
                         // Find a real file in the workspace with this extension
                        // This is necessary to establish project context for LSP servers
                        if let Some(file_path) = Self::find_representative_file(&workspace_path, ext).await {
                             debug!(
                                plugin = %plugin_name_owned,
                                representative_file = %file_path.display(),
                                "Found representative file for plugin"
                            );

                            // Use the internal plugin method name with the real file path
                            let mut request = mill_plugin_system::PluginRequest::new(
                                "search_workspace_symbols".to_string(),
                                file_path,
                            );
                            request = request.with_params(search_args);

                            // Try to get symbols from this plugin
                            match plugin.handle_request(request).await {
                                Ok(response) => {
                                    if let Some(data) = response.data {
                                        if let Some(data_symbols) = data.as_array() {
                                            symbols.extend(data_symbols.clone());
                                        }
                                    }
                                }
                                Err(e) => {
                                    warn!(
                                        plugin = %plugin_name_owned,
                                        error = %e,
                                        "Plugin query failed"
                                    );
                                    warning = Some(format!("{}: {}", plugin_name_owned, e));
                                }
                            }
                        } else {
                            debug!(
                                plugin = %plugin_name_owned,
                                extension = %ext,
                                "No files found with extension, skipping plugin"
                            );
                        }
                    }
                }
                (symbols, warning)
            });
        }

        let results = futures::future::join_all(futures).await;

        let mut all_symbols = Vec::new();
        let mut warnings = Vec::new();

        for (symbols, warning) in results {
            all_symbols.extend(symbols);
            if let Some(w) = warning {
                warnings.push(w);
            }
        }

        let processing_time = start_time.elapsed().as_millis() as u64;

        let warnings_result = if warnings.is_empty() {
            None
        } else {
            Some(warnings)
        };

        Ok((all_symbols, processing_time, warnings_result))
    }

    async fn handle_search_code(
        &self,
        context: &mill_handler_api::ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        debug!("handle_search_code: Processing search request");

        // Parse request
        let args = tool_call.arguments.clone().unwrap_or(json!({}));
        let request: SearchCodeRequest = serde_json::from_value(args).map_err(|e| {
            ServerError::invalid_request(format!("Invalid search_code arguments: {}", e))
        })?;

        // Validate query
        if request.query.trim().is_empty() {
            return Err(ServerError::invalid_request(
                "Query parameter cannot be empty",
            ));
        }

        // Get workspace path
        let workspace_path = request
            .workspace_path
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

        // Parse kind filter if provided
        let kind_filter = if let Some(kind_str) = &request.kind {
            match Self::parse_symbol_kind(kind_str) {
                Some(kind) => Some(kind),
                None => {
                    return Err(ServerError::invalid_request(format!(
                        "Invalid symbol kind: '{}'. Valid kinds: function, class, interface, struct, enum, variable, constant, module, method, field, property, type, trait",
                        kind_str
                    )));
                }
            }
        } else {
            None
        };

        debug!(
            query = %request.query,
            kind = ?kind_filter,
            limit = request.limit,
            offset = request.offset,
            "search_code: Parsed request"
        );

        // Perform workspace search
        let (symbols, processing_time, warnings) = self
            .search_workspace_symbols(context, &request.query, workspace_path)
            .await?;

        debug!(
            total_symbols = symbols.len(),
            "search_code: Got symbols from workspace search"
        );

        // Filter by kind if specified
        let filtered_symbols = Self::filter_by_kind(symbols, kind_filter);
        let total = filtered_symbols.len();

        debug!(
            filtered_count = total,
            "search_code: Filtered symbols by kind"
        );

        // Apply pagination
        let paginated_symbols = Self::paginate(filtered_symbols, request.limit, request.offset);

        debug!(
            paginated_count = paginated_symbols.len(),
            "search_code: Applied pagination"
        );

        // Build response
        let response = SearchCodeResponse {
            results: paginated_symbols,
            total,
            processing_time_ms: processing_time,
            warnings,
        };

        serde_json::to_value(response)
            .map_err(|e| ServerError::internal(format!("Failed to serialize response: {}", e)))
    }
}

impl Default for SearchHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ToolHandler for SearchHandler {
    fn tool_names(&self) -> &[&str] {
        &["search_code"]
    }

    async fn handle_tool_call(
        &self,
        context: &mill_handler_api::ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        debug!(tool_name = %tool_call.name, "SearchHandler::handle_tool_call called");

        match tool_call.name.as_str() {
            "search_code" => self.handle_search_code(context, tool_call).await,
            _ => Err(ServerError::not_supported(format!(
                "Unknown tool: {}",
                tool_call.name
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_symbol_kind() {
        assert_eq!(
            SearchHandler::parse_symbol_kind("function"),
            Some(SymbolKind::Function)
        );
        assert_eq!(
            SearchHandler::parse_symbol_kind("FUNCTION"),
            Some(SymbolKind::Function)
        );
        assert_eq!(
            SearchHandler::parse_symbol_kind("func"),
            Some(SymbolKind::Function)
        );
        assert_eq!(
            SearchHandler::parse_symbol_kind("class"),
            Some(SymbolKind::Class)
        );
        assert_eq!(
            SearchHandler::parse_symbol_kind("variable"),
            Some(SymbolKind::Variable)
        );
        assert_eq!(SearchHandler::parse_symbol_kind("invalid"), None);
    }

    #[test]
    fn test_symbol_kind_to_string() {
        assert_eq!(
            SearchHandler::symbol_kind_to_string(&SymbolKind::Function),
            "function"
        );
        assert_eq!(
            SearchHandler::symbol_kind_to_string(&SymbolKind::Class),
            "class"
        );
        assert_eq!(
            SearchHandler::symbol_kind_to_string(&SymbolKind::Module),
            "module"
        );
    }

    #[test]
    fn test_lsp_symbol_kind_to_string() {
        assert_eq!(
            SearchHandler::lsp_symbol_kind_to_string(12),
            Some("function".to_string())
        );
        assert_eq!(
            SearchHandler::lsp_symbol_kind_to_string(5),
            Some("class".to_string())
        );
        assert_eq!(
            SearchHandler::lsp_symbol_kind_to_string(13),
            Some("variable".to_string())
        );
        assert_eq!(SearchHandler::lsp_symbol_kind_to_string(999), None);
    }

    #[test]
    fn test_filter_by_kind() {
        let symbols = vec![
            json!({"name": "foo", "kind": "function"}),
            json!({"name": "Bar", "kind": "class"}),
            json!({"name": "baz", "kind": "variable"}),
        ];

        let filtered = SearchHandler::filter_by_kind(symbols.clone(), Some(SymbolKind::Function));
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0]["name"], "foo");

        let filtered = SearchHandler::filter_by_kind(symbols.clone(), Some(SymbolKind::Class));
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0]["name"], "Bar");

        let filtered = SearchHandler::filter_by_kind(symbols.clone(), None);
        assert_eq!(filtered.len(), 3);
    }

    #[test]
    fn test_paginate() {
        let symbols: Vec<Value> = (0..100)
            .map(|i| json!({"name": format!("symbol_{}", i)}))
            .collect();

        // First page
        let page1 = SearchHandler::paginate(symbols.clone(), 20, 0);
        assert_eq!(page1.len(), 20);
        assert_eq!(page1[0]["name"], "symbol_0");

        // Second page
        let page2 = SearchHandler::paginate(symbols.clone(), 20, 20);
        assert_eq!(page2.len(), 20);
        assert_eq!(page2[0]["name"], "symbol_20");

        // Last partial page
        let page_last = SearchHandler::paginate(symbols.clone(), 20, 90);
        assert_eq!(page_last.len(), 10);
        assert_eq!(page_last[0]["name"], "symbol_90");

        // Beyond range
        let page_empty = SearchHandler::paginate(symbols, 20, 200);
        assert_eq!(page_empty.len(), 0);
    }
}
