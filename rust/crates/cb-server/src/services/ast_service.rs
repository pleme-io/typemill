//! Concrete implementation of the AstService trait

use async_trait::async_trait;
use std::path::Path;
use std::sync::Arc;

use cb_api::{ApiResult, CacheStats, ImportGraph};
use cb_ast::AstCache;
use tracing::{debug, trace};

use cb_api::AstService;

/// Default implementation of the AST service with caching
pub struct DefaultAstService {
    /// Shared AST cache for performance optimization
    cache: Arc<AstCache>,
}

impl DefaultAstService {
    /// Create a new DefaultAstService with the provided cache
    pub fn new(cache: Arc<AstCache>) -> Self {
        debug!("DefaultAstService created with shared cache");
        Self { cache }
    }

    /// Create a new DefaultAstService with a new cache instance
    pub fn new_with_cache() -> Self {
        let cache = Arc::new(AstCache::new());
        debug!("DefaultAstService created with new cache instance");
        Self { cache }
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

        // Parse the file using the cb-ast crate
        let import_graph = cb_ast::parser::build_import_graph(&content, file)
            .map_err(|e| cb_api::ApiError::internal(format!("AST parsing failed: {}", e)))?;

        // Cache the result for future use
        if let Err(e) = self.cache.insert(file_path.clone(), import_graph.clone()).await {
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
