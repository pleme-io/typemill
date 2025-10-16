//! Concrete implementation of the AstService trait

use async_trait::async_trait;
use std::path::Path;
use std::sync::Arc;

use cb_ast::AstCache;
use cb_plugin_api::PluginRegistry;
use cb_protocol::{ApiResult, CacheStats, ImportGraph};
use tracing::{debug, trace};

use cb_protocol::AstService;

/// Default implementation of the AST service with caching
pub struct DefaultAstService {
    /// Shared AST cache for performance optimization
    cache: Arc<AstCache>,
    /// Language plugin registry for import parsing
    plugin_registry: Arc<PluginRegistry>,
}

impl DefaultAstService {
    /// Create a new DefaultAstService with the provided cache and plugin registry
    pub fn new(cache: Arc<AstCache>, plugin_registry: Arc<PluginRegistry>) -> Self {
        debug!("DefaultAstService created with shared cache and plugin registry");
        Self {
            cache,
            plugin_registry,
        }
    }

    /// Get cache statistics for monitoring
    pub fn cache_stats(&self) -> CacheStats {
        self.cache.stats()
    }

    /// Get cache statistics for monitoring (async trait implementation)
    pub async fn cache_stats_async(&self) -> CacheStats {
        self.cache.stats()
    }

    /// Get cache hit ratio as percentage
    pub fn cache_hit_ratio(&self) -> f64 {
        self.cache.hit_ratio()
    }

    /// Clear the cache
    pub fn clear_cache(&self) {
        self.cache.clear();
    }

    /// Perform cache maintenance
    pub fn maintain_cache(&self) {
        self.cache.maintenance();
    }
}

#[async_trait]
impl AstService for DefaultAstService {
    async fn build_import_graph(&self, file: &Path) -> ApiResult<ImportGraph> {
        let file_path = file.to_path_buf();

        trace!("Building import graph for: {}", file_path.display());

        // Check cache first
        if let Some(cached_graph) = self.cache.get(&file_path).await {
            trace!("Cache hit for: {}", file_path.display());
            return Ok(cached_graph);
        }

        trace!("Cache miss for: {}, parsing file", file_path.display());

        // Read the file content
        let content = tokio::fs::read_to_string(&file_path).await?;

        // Use plugin-based parsing for languages with plugins
        let import_graph =
            build_import_graph_with_plugin(&content, file, self.plugin_registry.clone())?;

        // Cache the result for future use
        if let Err(e) = self
            .cache
            .insert(file_path.clone(), import_graph.clone())
            .await
        {
            // Cache insertion failure shouldn't fail the operation, just log it
            debug!(
                "Failed to cache import graph for {}: {}",
                file_path.display(),
                e
            );
        } else {
            trace!("Cached import graph for: {}", file_path.display());
        }

        Ok(import_graph)
    }

    async fn cache_stats(&self) -> CacheStats {
        self.cache.stats()
    }
}

/// Build import graph using language plugins
fn build_import_graph_with_plugin(
    source: &str,
    path: &Path,
    registry: Arc<PluginRegistry>,
) -> Result<cb_protocol::ImportGraph, cb_protocol::ApiError> {
    use cb_protocol::{ImportGraph, ImportGraphMetadata, ImportInfo};
    use std::collections::HashSet;

    // Determine file extension
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .ok_or_else(|| cb_protocol::ApiError::internal("File has no extension"))?;

    // For languages without plugins, fall back to cb-ast
    // Note: Only Rust and TypeScript supported after language reduction
    if !matches!(
        extension,
        "ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs" | "rs"
    ) {
        // Use legacy cb-ast for other languages (if any remain)
        return cb_ast::parser::build_import_graph(source, path)
            .map_err(|e| cb_protocol::ApiError::internal(format!("AST parsing failed: {}", e)));
    }

    // Find appropriate plugin from injected registry
    let plugin = registry.find_by_extension(extension).ok_or_else(|| {
        cb_protocol::ApiError::internal(format!("No plugin found for .{} files", extension))
    })?;

    let language = plugin.metadata().name.to_lowercase();

    // Get imports from plugin
    let imports: Vec<ImportInfo> = match language.as_str() {
        "typescript" => {
            let graph =
                cb_lang_typescript::parser::analyze_imports(source, Some(path)).map_err(|e| {
                    cb_protocol::ApiError::internal(format!("Failed to parse imports: {}", e))
                })?;
            graph.imports
        }
        "rust" => cb_lang_rust::parser::parse_imports(source).map_err(|e| {
            cb_protocol::ApiError::internal(format!("Failed to parse imports: {}", e))
        })?,
        _ => {
            return Err(cb_protocol::ApiError::internal(format!(
                "Unsupported language: {}",
                language
            )));
        }
    };

    // Detect external dependencies
    let external_dependencies = imports
        .iter()
        .filter_map(|imp| {
            if is_external_dependency(&imp.module_path) {
                Some(imp.module_path.clone())
            } else {
                None
            }
        })
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();

    Ok(ImportGraph {
        source_file: path.to_string_lossy().to_string(),
        imports,
        importers: Vec::new(),
        metadata: ImportGraphMetadata {
            language: language.clone(),
            parsed_at: chrono::Utc::now(),
            parser_version: "1.0.0-plugin".to_string(),
            circular_dependencies: Vec::new(),
            external_dependencies,
        },
    })
}

/// Check if a module path represents an external dependency
fn is_external_dependency(module_path: &str) -> bool {
    if module_path.starts_with("./") || module_path.starts_with("../") {
        return false;
    }
    if module_path.starts_with("/") || module_path.starts_with("src/") {
        return false;
    }
    if module_path.starts_with("@") {
        return true;
    }
    !module_path.contains("/")
        || module_path.contains("node_modules")
        || !module_path.starts_with(".")
}
