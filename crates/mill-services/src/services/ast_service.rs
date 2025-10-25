//! Concrete implementation of the AstService trait

use async_trait::async_trait;
use std::path::Path;
use std::sync::Arc;

use mill_plugin_api::PluginRegistry;
use mill_ast::AstCache;
use mill_foundation::protocol::{ ApiResult , CacheStats , ImportGraph };
use tracing::{debug, trace};

use mill_foundation::protocol::AstService;

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
) -> Result<mill_foundation::protocol::ImportGraph, mill_foundation::protocol::ApiError> {
    // Determine file extension
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .ok_or_else(|| {
            mill_foundation::protocol::ApiError::internal("File has no extension")
        })?;

    // For languages without plugins, fall back to mill-ast
    // Note: Only Rust and TypeScript supported after language reduction
    if !matches!(
        extension,
        "ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs" | "rs"
    ) {
        // Fallback to mill-ast parser for other languages (if any remain)
        return mill_ast::parser::build_import_graph(source, path).map_err(|e| {
            mill_foundation::protocol::ApiError::internal(format!("AST parsing failed: {}", e))
        });
    }

    // Find appropriate plugin from injected registry
    let plugin = registry.find_by_extension(extension).ok_or_else(|| {
        mill_foundation::protocol::ApiError::internal(format!(
            "No plugin found for .{} files",
            extension
        ))
    })?;

    // Use the trait method for detailed import analysis
    plugin
        .analyze_detailed_imports(source, Some(path))
        .map_err(|e| {
            mill_foundation::protocol::ApiError::internal(format!(
                "Failed to parse imports: {}",
                e
            ))
        })
}