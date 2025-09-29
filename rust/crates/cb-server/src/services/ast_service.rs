//! Concrete implementation of the AstService trait

use async_trait::async_trait;
use std::path::Path;
use std::sync::Arc;

use cb_ast::{AstCache, EditPlan, ImportGraph};
use cb_core::CoreError;
use cb_core::model::IntentSpec;
use tracing::{debug, trace};

use crate::interfaces::AstService;

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
    pub fn cache_stats(&self) -> cb_ast::CacheStats {
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
    async fn build_import_graph(&self, file: &Path) -> Result<ImportGraph, CoreError> {
        let file_path = file.to_path_buf();

        trace!("Building import graph for: {}", file_path.display());

        // Check cache first
        if let Some(cached_graph) = self.cache.get(&file_path) {
            trace!("Cache hit for: {}", file_path.display());
            return Ok(cached_graph);
        }

        trace!("Cache miss for: {}, parsing file", file_path.display());

        // Read the file content
        let content = tokio::fs::read_to_string(&file_path).await?;

        // Parse the file using the cb-ast crate
        let import_graph = cb_ast::parser::build_import_graph(&content, file)
            .map_err(|e| CoreError::internal(format!("AST parsing failed: {}", e)))?;

        // Cache the result for future use
        if let Err(e) = self.cache.insert(file_path.clone(), import_graph.clone()) {
            // Cache insertion failure shouldn't fail the operation, just log it
            debug!("Failed to cache import graph for {}: {}", file_path.display(), e);
        } else {
            trace!("Cached import graph for: {}", file_path.display());
        }

        Ok(import_graph)
    }

    async fn plan_refactor(&self, intent: &IntentSpec, file: &Path) -> Result<EditPlan, CoreError> {
        match intent.name() {
            "rename_symbol_with_imports" => {
                // Extract parameters from intent
                let old_name = intent.arguments().get("oldName")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| CoreError::invalid_data("Missing 'oldName' parameter in intent"))?;

                let new_name = intent.arguments().get("newName")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| CoreError::invalid_data("Missing 'newName' parameter in intent"))?;

                // Call the plan_rename_refactor function from cb-ast
                cb_ast::refactoring::plan_rename_refactor(old_name, new_name, file)
                    .map_err(|e| CoreError::internal(format!("Refactoring planning failed: {}", e)))
            }
            _ => Err(CoreError::not_supported(format!("Intent '{}' is not supported for refactoring", intent.name())))
        }
    }
}