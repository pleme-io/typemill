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
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
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

pub struct SearchHandler {
    /// Cache of representative files per workspace and extension
    /// Map<WorkspacePath, Map<Extension, FilePath>>
    representative_files_cache: Arc<RwLock<HashMap<PathBuf, HashMap<String, PathBuf>>>>,
}

impl SearchHandler {
    pub fn new() -> Self {
        Self {
            representative_files_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Helper to efficiently paginate results by skipping entire vectors
    fn paginate_results(
        symbol_vectors: Vec<Vec<Value>>,
        offset: usize,
        limit: usize,
    ) -> Vec<Value> {
        // Calculate safe capacity to prevent over-allocation or overflow when limit is huge
        let total_items: usize = symbol_vectors.iter().map(|v| v.len()).sum();
        let remaining_items = total_items.saturating_sub(offset);
        let capacity = std::cmp::min(limit, remaining_items);

        let mut paginated_symbols = Vec::with_capacity(capacity);
        let mut current_offset = offset;
        let mut needed = limit;

        for symbols in symbol_vectors {
            if needed == 0 {
                break;
            }

            let len = symbols.len();
            if current_offset >= len {
                current_offset -= len;
                continue;
            }

            let take_count = std::cmp::min(needed, len - current_offset);
            paginated_symbols.extend(symbols.into_iter().skip(current_offset).take(take_count));

            needed -= take_count;
            current_offset = 0;
        }
        paginated_symbols
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

            // Use tokio fs read_dir directly to avoid extra syscall
            if let Ok(mut entries) = fs::read_dir(&search_path).await {
                while let Ok(Some(entry)) = entries.next_entry().await {
                    // Check file type first
                    if let Ok(file_type) = entry.file_type().await {
                        if file_type.is_file() {
                            // Use file_name() to avoid allocating full path if not needed
                            let file_name = entry.file_name();
                            let path_from_name = std::path::Path::new(&file_name);

                            if let Some(ext) = path_from_name.extension().and_then(|e| e.to_str()) {
                                if remaining_extensions.contains(ext) {
                                    results.insert(ext.to_string(), entry.path());
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
                // Use file_name() to avoid allocating full path for exclusion check
                let file_name = entry.file_name();
                if let Some(name) = file_name.to_str() {
                    if name.starts_with('.') || name == "node_modules" || name == "target" {
                        continue;
                    }
                }

                if let Ok(file_type) = entry.file_type().await {
                    if file_type.is_file() {
                        let path_from_name = std::path::Path::new(&file_name);
                        if let Some(ext) = path_from_name.extension().and_then(|e| e.to_str()) {
                            if needed_extensions.contains(ext) {
                                results.insert(ext.to_string(), entry.path());
                                needed_extensions.remove(ext);
                            }
                        }
                    } else if file_type.is_dir() && !needed_extensions.is_empty() {
                        // Only allocate full path when recursing
                        let path = entry.path();
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
        plugin_manager: &Arc<mill_plugin_system::PluginManager>,
        query: &str,
        workspace_path: PathBuf,
        kind_filter: Option<SymbolKind>,
        limit: usize,
        offset: usize,
    ) -> ServerResult<(Vec<Value>, usize, u64, Option<Vec<String>>)> {
        use std::collections::HashSet;
        use std::time::Instant;

        debug!("search_workspace_symbols: Starting multi-plugin workspace search");

        let start_time = Instant::now();

        // Get all registered plugins
        let all_plugins = plugin_manager.get_all_plugins_with_names().await;
        debug!(
            plugin_count = all_plugins.len(),
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
        let mut plugin_extensions: Vec<(
            String,
            String,
            Arc<dyn mill_plugin_system::LanguagePlugin>,
        )> = Vec::new();
        let mut unique_extensions: HashSet<String> = HashSet::new();

        for (plugin_name, plugin) in &all_plugins {
            if let Some(ext) = plugin.supported_extensions().first() {
                plugin_extensions.push((plugin_name.clone(), ext.clone(), plugin.clone()));
                unique_extensions.insert(ext.clone());
            }
        }

        // 2. Resolve representative files using cache
        let mut representative_files = HashMap::new();
        let mut missing_extensions = unique_extensions.clone();

        // Check cache
        {
            let cache = self.representative_files_cache.read().await;
            if let Some(workspace_cache) = cache.get(&workspace_path) {
                for ext in &unique_extensions {
                    if let Some(path) = workspace_cache.get(ext) {
                        representative_files.insert(ext.clone(), path.clone());
                        missing_extensions.remove(ext);
                    }
                }
            }
        }

        // Scan for missing extensions
        if !missing_extensions.is_empty() {
            debug!(
                missing_count = missing_extensions.len(),
                "Scanning for missing representative files"
            );

            let found = Self::find_representative_files(&workspace_path, &missing_extensions).await;

            // Update cache with found files
            if !found.is_empty() {
                let mut cache = self.representative_files_cache.write().await;
                let workspace_cache = cache
                    .entry(workspace_path.clone())
                    .or_insert_with(HashMap::new);

                for (ext, path) in &found {
                    workspace_cache.insert(ext.clone(), path.clone());
                }
            }

            for (ext, path) in found {
                representative_files.insert(ext, path);
            }
        }

        debug!(
            found_count = representative_files.len(),
            requested_count = unique_extensions.len(),
            "Finished resolving representative files"
        );

        // 3. Parallelize plugin queries
        let mut futures = Vec::new();

        for (plugin_name, ext, plugin) in plugin_extensions {
            // let workspace_path = workspace_path.clone();
            let search_args = search_args.clone();

            // Look up the file path found previously
            let file_path_opt = representative_files.get(&ext).cloned();

            futures.push(async move {
                let mut symbols = Vec::new();
                let mut warning = None;

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
                                symbols.extend(data_symbols);
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
                (symbols, warning)
            });
        }

        let results = futures::future::join_all(futures).await;

        let mut warnings = Vec::new();
        let mut symbol_vectors = Vec::new();
        let mut total = 0;

        // Collect warnings and calculate total without flattening yet
        for (symbols, warning) in results {
            total += symbols.len();
            symbol_vectors.push(symbols);
            if let Some(w) = warning {
                warnings.push(w);
            }
        }

        // Stream and paginate without allocating a huge intermediate vector
        // Optimized to skip entire vectors when possible
        let paginated_symbols = Self::paginate_results(symbol_vectors, offset, limit);

        let processing_time = start_time.elapsed().as_millis() as u64;

        let warnings_result = if warnings.is_empty() {
            None
        } else {
            Some(warnings)
        };

        Ok((paginated_symbols, total, processing_time, warnings_result))
    }

    async fn handle_search_code(
        &self,
        context: &mill_handler_api::ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        debug!("handle_search_code: Processing search request");

        // Parse request
        let default_args = json!({});
        let args = tool_call.arguments.as_ref().unwrap_or(&default_args);
        let request: SearchCodeRequest = SearchCodeRequest::deserialize(args).map_err(|e| {
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

        // Perform workspace search with pagination pushed down
        let (paginated_symbols, total, processing_time, warnings) = self
            .search_workspace_symbols(
                &context.plugin_manager,
                &request.query,
                workspace_path,
                kind_filter,
                request.limit,
                request.offset,
            )
            .await?;

        debug!(
            total_symbols = total,
            paginated_count = paginated_symbols.len(),
            "search_code: Got symbols from workspace search"
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
                    let target_str = symbol_kind_to_string(&target_kind);
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
            if let Some(kind_str) = lsp_symbol_kind_to_string(kind_num) {
                if matches_target(kind_str) {
                    return true;
                }
            }
        }

        false
    }

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
        assert_eq!(symbol_kind_to_string(&SymbolKind::Function), "function");
        assert_eq!(symbol_kind_to_string(&SymbolKind::Class), "class");
        assert_eq!(symbol_kind_to_string(&SymbolKind::Module), "module");
    }

    #[test]
    fn test_lsp_symbol_kind_to_string() {
        assert_eq!(lsp_symbol_kind_to_string(12), Some("function"));
        assert_eq!(lsp_symbol_kind_to_string(5), Some("class"));
        assert_eq!(lsp_symbol_kind_to_string(13), Some("variable"));
        assert_eq!(lsp_symbol_kind_to_string(999), None);
    }

    #[test]
    fn test_pagination_optimization() {
        let v1 = vec![json!(1), json!(2)];
        let v2 = vec![json!(3), json!(4), json!(5)];
        let v3 = vec![json!(6)];
        let vectors = vec![v1.clone(), v2.clone(), v3.clone()];

        // Case 1: No skipping, full take
        let res = SearchHandler::paginate_results(vectors.clone(), 0, 10);
        assert_eq!(res.len(), 6);
        assert_eq!(res[0], json!(1));
        assert_eq!(res[5], json!(6));

        // Case 2: Skip within first vector
        let res = SearchHandler::paginate_results(vectors.clone(), 1, 10);
        assert_eq!(res.len(), 5);
        assert_eq!(res[0], json!(2));

        // Case 3: Skip entire first vector
        let res = SearchHandler::paginate_results(vectors.clone(), 2, 10);
        assert_eq!(res.len(), 4);
        assert_eq!(res[0], json!(3));

        // Case 4: Skip first and part of second
        let res = SearchHandler::paginate_results(vectors.clone(), 3, 10);
        assert_eq!(res.len(), 3);
        assert_eq!(res[0], json!(4));

        // Case 5: Limit stops early
        let res = SearchHandler::paginate_results(vectors.clone(), 0, 3);
        assert_eq!(res.len(), 3);
        assert_eq!(res, vec![json!(1), json!(2), json!(3)]);

        // Case 6: Deep skip (skip everything)
        let res = SearchHandler::paginate_results(vectors.clone(), 100, 10);
        assert!(res.is_empty());
    }

    #[test]
    fn test_check_symbol_kind() {
        let func = json!({"name": "foo", "kind": "function"});
        let class = json!({"name": "Bar", "kind": "class"});
        let var = json!({"name": "baz", "kind": "variable"});

        // Test exact matches
        assert!(check_symbol_kind(&func, SymbolKind::Function));
        assert!(!check_symbol_kind(&func, SymbolKind::Class));

        assert!(check_symbol_kind(&class, SymbolKind::Class));
        assert!(check_symbol_kind(&var, SymbolKind::Variable));

        // Test case insensitivity
        let func_upper = json!({"name": "foo", "kind": "FUNCTION"});
        assert!(check_symbol_kind(&func_upper, SymbolKind::Function));

        // Test flexible matching
        let func_short = json!({"name": "foo", "kind": "fn"});
        assert!(check_symbol_kind(&func_short, SymbolKind::Function));

        // Test LSP numeric kind
        let func_lsp = json!({"name": "foo", "kind": 12});
        assert!(check_symbol_kind(&func_lsp, SymbolKind::Function));

        // Test symbolKind field
        let func_sym_kind = json!({"name": "foo", "symbolKind": "function"});
        assert!(check_symbol_kind(&func_sym_kind, SymbolKind::Function));
    }
}

#[cfg(test)]
mod performance_tests {
    use super::*;
    use std::time::Instant;
    use tokio::fs;

    async fn create_test_workspace() -> tempfile::TempDir {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let root = temp_dir.path();

        fs::create_dir_all(root.join("src")).await.unwrap();
        fs::write(root.join("src/main.rs"), "fn main() {}")
            .await
            .unwrap();

        fs::create_dir_all(root.join("lib")).await.unwrap();
        fs::write(root.join("lib/utils.py"), "def foo(): pass")
            .await
            .unwrap();

        fs::create_dir_all(root.join("packages/pkg1"))
            .await
            .unwrap();
        fs::write(root.join("packages/pkg1/index.ts"), "const x = 1;")
            .await
            .unwrap();

        temp_dir
    }

    #[tokio::test]
    async fn test_benchmark_representative_file_scan() {
        let workspace = create_test_workspace().await;
        let workspace_path = workspace.path().to_path_buf();

        let extensions: std::collections::HashSet<String> =
            ["rs", "py", "ts"].iter().map(|s| s.to_string()).collect();

        // Measure scan time
        let start = Instant::now();
        let iterations = 50;
        for _ in 0..iterations {
            let found =
                SearchHandler::find_representative_files(&workspace_path, &extensions).await;
            assert_eq!(found.len(), 3);
        }
        let duration = start.elapsed();

        println!("BENCHMARK: Time for {} scans: {:?}", iterations, duration);
        println!("BENCHMARK: Average per scan: {:?}", duration / iterations);
    }

    #[tokio::test]
    async fn test_benchmark_caching_impact() {
        let workspace = create_test_workspace().await;
        let workspace_path = workspace.path().to_path_buf();
        let extensions: std::collections::HashSet<String> =
            ["rs", "py", "ts"].iter().map(|s| s.to_string()).collect();

        let handler = SearchHandler::new();

        // Baseline: scan (1 call)
        let start_scan = Instant::now();
        let found = SearchHandler::find_representative_files(&workspace_path, &extensions).await;
        let duration_scan = start_scan.elapsed();

        // Populate cache manually to simulate first run
        {
            let mut cache = handler.representative_files_cache.write().await;
            let workspace_cache = cache
                .entry(workspace_path.clone())
                .or_insert_with(std::collections::HashMap::new);
            for (ext, path) in &found {
                workspace_cache.insert(ext.clone(), path.clone());
            }
        }

        // Benchmark cache access (pure memory)
        let start_cache = Instant::now();
        let iterations = 1000;
        for _ in 0..iterations {
            let cache = handler.representative_files_cache.read().await;
            if let Some(workspace_cache) = cache.get(&workspace_path) {
                for ext in &extensions {
                    if let Some(path) = workspace_cache.get(ext) {
                        // Access the path to ensure it's not optimized away
                        std::hint::black_box(path);
                    }
                }
            }
        }
        let duration_cache = start_cache.elapsed();

        println!("BENCHMARK: Scan (1 call): {:?}", duration_scan);
        println!(
            "BENCHMARK: Cache ({} calls): {:?}",
            iterations, duration_cache
        );
        println!(
            "BENCHMARK: Average Cache: {:?}",
            duration_cache / iterations
        );
    }

    #[tokio::test]
    async fn test_benchmark_search_filtering_overhead() {
        use mill_plugin_system::{
            Capabilities, LanguagePlugin, PluginMetadata, PluginRequest, PluginResponse,
            PluginResult,
        };

        struct MockLargePlugin {
            symbols: Vec<Value>,
        }

        #[async_trait]
        impl LanguagePlugin for MockLargePlugin {
            fn metadata(&self) -> PluginMetadata {
                PluginMetadata::new("mock-large", "1.0.0", "test")
            }
            fn supported_extensions(&self) -> Vec<String> {
                vec!["mock".to_string()]
            }
            fn capabilities(&self) -> Capabilities {
                let mut c = Capabilities::default();
                c.navigation.workspace_symbols = true;
                c
            }
            async fn handle_request(&self, req: PluginRequest) -> PluginResult<PluginResponse> {
                // Simulate plugin-side filtering (like LspAdapterPlugin does)
                let mut symbols = self.symbols.clone();

                if let Some(kind_val) = req.get_param("kind") {
                    if let Ok(target_kind) = serde_json::from_value::<SymbolKind>(kind_val.clone())
                    {
                        symbols.retain(|s| {
                            if let Some(k_str) = s.get("kind").and_then(|k| k.as_str()) {
                                // Simple check for test purposes
                                if target_kind == SymbolKind::Function {
                                    return k_str == "function";
                                }
                            }
                            false
                        });
                    }
                }

                Ok(PluginResponse::success(Value::Array(symbols), "mock-large"))
            }
            fn configure(&self, _config: Value) -> PluginResult<()> {
                Ok(())
            }
            fn tool_definitions(&self) -> Vec<Value> {
                vec![]
            }
        }

        // Generate 50,000 symbols (25k matching "function", 25k "class")
        let mut symbols = Vec::new();
        for i in 0..50000 {
            let kind = if i % 2 == 0 { "function" } else { "class" };
            symbols.push(json!({
                "name": format!("sym_{}", i),
                "kind": kind,
                "location": { "uri": "file:///tmp/test", "range": {} }
            }));
        }

        let plugin = Arc::new(MockLargePlugin { symbols });
        let plugin_manager = Arc::new(mill_plugin_system::PluginManager::new());
        plugin_manager
            .register_plugin("mock-large", plugin)
            .await
            .unwrap();

        // Create a mock file so representative files scan works
        let workspace = create_test_workspace().await;
        let workspace_path = workspace.path().to_path_buf();
        fs::write(workspace_path.join("test.mock"), "")
            .await
            .unwrap();

        let handler = SearchHandler::new();

        let start = Instant::now();
        let (results, _, _, _) = handler
            .search_workspace_symbols(
                &plugin_manager,
                "query",
                workspace_path,
                Some(SymbolKind::Function),
                usize::MAX,
                0,
            )
            .await
            .unwrap();
        let duration = start.elapsed();

        // Should only return the 25,000 functions
        assert_eq!(results.len(), 25000);
        println!(
            "BENCHMARK: Search (50k total, 25k matching): {:?}",
            duration
        );
    }
}
