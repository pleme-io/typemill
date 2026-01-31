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
    fn symbol_kind_to_string(kind: &SymbolKind) -> &'static str {
        match kind {
            SymbolKind::Function => "function",
            SymbolKind::Class => "class",
            SymbolKind::Interface => "interface",
            SymbolKind::Struct => "struct",
            SymbolKind::Enum => "enum",
            SymbolKind::Variable => "variable",
            SymbolKind::Constant => "constant",
            SymbolKind::Module => "module",
            SymbolKind::Method => "method",
            SymbolKind::Field => "field",
            SymbolKind::Other => "other",
        }
    }

    /// Check if a symbol matches the target kind efficiently (avoiding allocations)
    fn check_symbol_kind(symbol: &Value, target_kind: SymbolKind) -> bool {
        // Helper to check match against target kind
        let matches_target = |kind_str: &str| -> bool {
            match target_kind {
                SymbolKind::Function => {
                    kind_str.eq_ignore_ascii_case("function")
                        || kind_str.eq_ignore_ascii_case("func")
                        || kind_str.eq_ignore_ascii_case("fn")
                        || kind_str.to_ascii_lowercase().contains("function")
                }
                SymbolKind::Variable => {
                    kind_str.eq_ignore_ascii_case("variable")
                        || kind_str.eq_ignore_ascii_case("var")
                        || kind_str.eq_ignore_ascii_case("let")
                        || kind_str.to_ascii_lowercase().contains("variable")
                }
                SymbolKind::Constant => {
                    kind_str.eq_ignore_ascii_case("constant")
                        || kind_str.eq_ignore_ascii_case("const")
                        || kind_str.to_ascii_lowercase().contains("constant")
                }
                SymbolKind::Module => {
                    kind_str.eq_ignore_ascii_case("module")
                        || kind_str.eq_ignore_ascii_case("mod")
                        || kind_str.to_ascii_lowercase().contains("module")
                        || kind_str.to_ascii_lowercase().contains("namespace")
                }
                SymbolKind::Field => {
                    kind_str.eq_ignore_ascii_case("field")
                        || kind_str.to_ascii_lowercase().contains("field")
                        || kind_str.to_ascii_lowercase().contains("property")
                }
                _ => {
                    let target_str = Self::symbol_kind_to_string(&target_kind);
                    kind_str.eq_ignore_ascii_case(target_str)
                        || kind_str
                            .to_ascii_lowercase()
                            .contains(&target_str.to_ascii_lowercase())
                }
            }
        };

        // 1. Try "kind" field (string)
        if let Some(kind_str) = symbol.get("kind").and_then(|k| k.as_str()) {
            if matches_target(kind_str) {
                return true;
            }
        }

        // 2. Try "symbolKind" field (LSP string)
        if let Some(kind_str) = symbol.get("symbolKind").and_then(|k| k.as_str()) {
            if matches_target(kind_str) {
                return true;
            }
        }

        // 3. Try "kind" field (numeric LSP)
        if let Some(kind_num) = symbol.get("kind").and_then(|k| k.as_u64()) {
            if let Some(kind_str) = Self::lsp_symbol_kind_to_string(kind_num) {
                if matches_target(kind_str) {
                    return true;
                }
            }
        }

        false
    }

    /// Convert LSP numeric SymbolKind to string
    fn lsp_symbol_kind_to_string(kind: u64) -> Option<&'static str> {
        // LSP SymbolKind numeric values
        match kind {
            1 => Some("file"),
            2 => Some("module"),
            3 => Some("namespace"),
            4 => Some("package"),
            5 => Some("class"),
            6 => Some("method"),
            7 => Some("property"),
            8 => Some("field"),
            9 => Some("constructor"),
            10 => Some("enum"),
            11 => Some("interface"),
            12 => Some("function"),
            13 => Some("variable"),
            14 => Some("constant"),
            15 => Some("string"),
            16 => Some("number"),
            17 => Some("boolean"),
            18 => Some("array"),
            19 => Some("object"),
            20 => Some("key"),
            21 => Some("null"),
            22 => Some("enummember"),
            23 => Some("struct"),
            24 => Some("event"),
            25 => Some("operator"),
            26 => Some("typeparameter"),
            _ => None,
        }
    }

    /// Apply pagination to results
    fn paginate(symbols: Vec<Value>, limit: usize, offset: usize) -> Vec<Value> {
        symbols.into_iter().skip(offset).take(limit).collect()
    }

    /// Find representative files for multiple extensions in a single pass
    async fn find_representative_files(
        workspace_path: &std::path::Path,
        extensions: &std::collections::HashSet<String>,
    ) -> std::collections::HashMap<String, PathBuf> {
        use tokio::fs;

        let mut results = std::collections::HashMap::new();
        let mut remaining_extensions = extensions.clone();

        if remaining_extensions.is_empty() {
            return results;
        }

        // First, try to find a file in common source directories
        let common_dirs = ["src", "lib", "packages", "apps", "."];

        for dir in common_dirs {
            let search_path = if dir == "." {
                workspace_path.to_path_buf()
            } else {
                workspace_path.join(dir)
            };

            // Use tokio fs metadata
            if let Ok(metadata) = fs::metadata(&search_path).await {
                if metadata.is_dir() {
                    if let Ok(mut entries) = fs::read_dir(&search_path).await {
                        while let Ok(Some(entry)) = entries.next_entry().await {
                            let path = entry.path();
                            // Check file extension
                            if let Ok(file_type) = entry.file_type().await {
                                if file_type.is_file() {
                                    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                                        if remaining_extensions.contains(ext) {
                                            results.insert(ext.to_string(), path.clone());
                                            remaining_extensions.remove(ext);
                                            if remaining_extensions.is_empty() {
                                                return results;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Fall back to recursive search (limited depth)
        if !remaining_extensions.is_empty() {
            let found = Box::pin(Self::find_files_recursive(
                workspace_path,
                &remaining_extensions,
                3,
            ))
            .await;
            for (ext, path) in found {
                results.insert(ext, path);
            }
        }

        results
    }

    async fn find_files_recursive(
        dir: &std::path::Path,
        extensions: &std::collections::HashSet<String>,
        max_depth: u32,
    ) -> std::collections::HashMap<String, PathBuf> {
        use tokio::fs;
        let mut results = std::collections::HashMap::new();

        if max_depth == 0 {
            return results;
        }

        let mut needed_extensions = extensions.clone();

        if let Ok(mut entries) = fs::read_dir(dir).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                let path = entry.path();

                // Skip hidden directories and node_modules
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.starts_with('.') || name == "node_modules" || name == "target" {
                        continue;
                    }
                }

                if let Ok(file_type) = entry.file_type().await {
                    if file_type.is_file() {
                        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                            if needed_extensions.contains(ext) {
                                results.insert(ext.to_string(), path.clone());
                                needed_extensions.remove(ext);
                            }
                        }
                    } else if file_type.is_dir() {
                        if !needed_extensions.is_empty() {
                            let found_in_subdir = Box::pin(Self::find_files_recursive(
                                &path,
                                &needed_extensions,
                                max_depth - 1,
                            ))
                            .await;
                            for (ext, p) in found_in_subdir {
                                results.insert(ext.clone(), p);
                                needed_extensions.remove(&ext);
                            }
                        }
                    }
                }

                if needed_extensions.is_empty() {
                    break;
                }
            }
        }

        results
    }

    /// Perform workspace-wide symbol search across all plugins
    async fn search_workspace_symbols(
        &self,
        context: &mill_handler_api::ToolHandlerContext,
        query: &str,
        workspace_path: PathBuf,
        kind_filter: Option<SymbolKind>,
    ) -> ServerResult<(Vec<Value>, u64, Option<Vec<String>>)> {
        use std::collections::HashSet;
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

        let mut search_args = json!({
            "query": query,
            "workspacePath": workspace_path.to_string_lossy()
        });

        if let Some(kind) = kind_filter {
            if let Value::Object(map) = &mut search_args {
                map.insert(
                    "kind".to_string(),
                    serde_json::to_value(kind).unwrap_or(Value::Null),
                );
            }
        }

        // 1. Gather required extensions
        let mut plugin_extensions: Vec<(String, String)> = Vec::new();
        let mut unique_extensions: HashSet<String> = HashSet::new();

        for plugin_name in &plugin_names {
            if let Some(plugin) = context.plugin_manager.get_plugin_by_name(plugin_name).await {
                if let Some(ext) = plugin.supported_extensions().first() {
                    plugin_extensions.push((plugin_name.clone(), ext.clone()));
                    unique_extensions.insert(ext.clone());
                }
            }
        }

        // 2. Find representative files in single pass
        let representative_files =
            Self::find_representative_files(&workspace_path, &unique_extensions).await;

        debug!(
            found_count = representative_files.len(),
            requested_count = unique_extensions.len(),
            "Finished finding representative files"
        );

        // 3. Parallelize plugin queries
        let mut futures = Vec::new();

        for (plugin_name, ext) in plugin_extensions {
            let plugin_manager = context.plugin_manager.clone();
            // let workspace_path = workspace_path.clone();
            let search_args = search_args.clone();

            // Look up the file path found previously
            let file_path_opt = representative_files.get(&ext).cloned();

            futures.push(async move {
                let mut symbols = Vec::new();
                let mut warning = None;

                // We re-fetch the plugin here. It's fast (read lock).
                if let Some(plugin) = plugin_manager.get_plugin_by_name(&plugin_name).await {
                    if let Some(file_path) = file_path_opt {
                        debug!(
                            plugin = %plugin_name,
                            representative_file = %file_path.display(),
                            "Found representative file for plugin"
                        );

                        let mut request = mill_plugin_system::PluginRequest::new(
                            "search_workspace_symbols".to_string(),
                            file_path,
                        );
                        request = request.with_params(search_args);

                        // Try to get symbols from this plugin
                        match plugin.handle_request(request).await {
                            Ok(response) => {
                                if let Some(Value::Array(data_symbols)) = response.data {
                                    if let Some(kind) = kind_filter {
                                        symbols.extend(
                                            data_symbols
                                                .into_iter()
                                                .filter(|s| Self::check_symbol_kind(s, kind)),
                                        );
                                    } else {
                                        symbols.extend(data_symbols);
                                    }
                                }
                            }
                            Err(e) => {
                                warn!(plugin = %plugin_name, error = %e, "Plugin query failed");
                                warning = Some(format!("{}: {}", plugin_name, e));
                            }
                        }
                    } else {
                        debug!(
                            plugin = %plugin_name,
                            extension = %ext,
                            "No files found with extension, skipping plugin"
                        );
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
            .search_workspace_symbols(context, &request.query, workspace_path, kind_filter)
            .await?;

        debug!(
            total_symbols = symbols.len(),
            "search_code: Got symbols from workspace search"
        );

        let total = symbols.len();

        // Apply pagination
        let paginated_symbols = Self::paginate(symbols, request.limit, request.offset);

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
            Some("function")
        );
        assert_eq!(
            SearchHandler::lsp_symbol_kind_to_string(5),
            Some("class")
        );
        assert_eq!(
            SearchHandler::lsp_symbol_kind_to_string(13),
            Some("variable")
        );
        assert_eq!(SearchHandler::lsp_symbol_kind_to_string(999), None);
    }

    #[test]
    fn test_check_symbol_kind() {
        let func = json!({"name": "foo", "kind": "function"});
        let class = json!({"name": "Bar", "kind": "class"});
        let var = json!({"name": "baz", "kind": "variable"});

        // Test exact matches
        assert!(SearchHandler::check_symbol_kind(&func, SymbolKind::Function));
        assert!(!SearchHandler::check_symbol_kind(&func, SymbolKind::Class));

        assert!(SearchHandler::check_symbol_kind(&class, SymbolKind::Class));
        assert!(SearchHandler::check_symbol_kind(&var, SymbolKind::Variable));

        // Test case insensitivity
        let func_upper = json!({"name": "foo", "kind": "FUNCTION"});
        assert!(SearchHandler::check_symbol_kind(
            &func_upper,
            SymbolKind::Function
        ));

        // Test flexible matching
        let func_short = json!({"name": "foo", "kind": "fn"});
        assert!(SearchHandler::check_symbol_kind(
            &func_short,
            SymbolKind::Function
        ));

        // Test LSP numeric kind
        let func_lsp = json!({"name": "foo", "kind": 12});
        assert!(SearchHandler::check_symbol_kind(
            &func_lsp,
            SymbolKind::Function
        ));

        // Test symbolKind field
        let func_sym_kind = json!({"name": "foo", "symbolKind": "function"});
        assert!(SearchHandler::check_symbol_kind(
            &func_sym_kind,
            SymbolKind::Function
        ));
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
