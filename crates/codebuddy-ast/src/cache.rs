//! AST caching system for performance optimization

use codebuddy_foundation::protocol::{CacheStats, ImportGraph};
use dashmap::DashMap;
use std::path::PathBuf;
use std::time::SystemTime;
use tracing::{debug, trace};

/// Cache key containing file path and modification time for invalidation
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CacheKey {
    /// File path
    pub path: PathBuf,
    /// Last modification time when the file was cached
    pub modified_time: SystemTime,
}

/// Cached AST data with metadata
#[derive(Debug, Clone)]
pub struct CachedEntry {
    /// Parsed import graph
    pub import_graph: ImportGraph,
    /// When this entry was cached
    pub cached_at: SystemTime,
    /// Size of the original file when cached
    pub file_size: u64,
}

/// Cache configuration settings
#[derive(Debug, Clone)]
pub struct CacheSettings {
    /// Enable caching
    pub enabled: bool,
    /// Maximum number of entries
    pub max_entries: usize,
    /// Time-to-live for cache entries in seconds
    pub ttl_seconds: u64,
    /// Maximum total size in bytes (approximate)
    pub max_size_bytes: u64,
}

impl CacheSettings {
    /// Check if cache is disabled via environment variables
    /// Returns true if cache should be disabled
    fn is_cache_disabled_by_env() -> bool {
        // Check master switch first
        if let Ok(val) = std::env::var("CODEBUDDY_DISABLE_CACHE") {
            if val == "1" || val.to_lowercase() == "true" {
                return true;
            }
        }

        // Check AST-specific switch
        if let Ok(val) = std::env::var("CODEBUDDY_DISABLE_AST_CACHE") {
            if val == "1" || val.to_lowercase() == "true" {
                return true;
            }
        }

        false
    }

    /// Create cache settings from core config
    /// This allows creating cache settings from codebuddy_config::config::CacheConfig
    pub fn from_config(enabled: bool, ttl_seconds: u64, max_size_bytes: u64) -> Self {
        // Calculate max_entries based on max_size_bytes
        // Assuming average entry size of ~10KB (includes path + import graph)
        let avg_entry_size = 10 * 1024; // 10KB
        let max_entries = (max_size_bytes / avg_entry_size as u64).max(100) as usize;

        // Check environment variables for cache control
        // Priority: CODEBUDDY_DISABLE_CACHE > CODEBUDDY_DISABLE_AST_CACHE > config
        let env_disabled = Self::is_cache_disabled_by_env();
        let final_enabled = if env_disabled { false } else { enabled };

        Self {
            enabled: final_enabled,
            max_entries,
            ttl_seconds,
            max_size_bytes,
        }
    }
}

impl Default for CacheSettings {
    fn default() -> Self {
        // Check environment variables for cache control
        let env_disabled = CacheSettings::is_cache_disabled_by_env();

        Self {
            enabled: !env_disabled,
            max_entries: 10000,
            ttl_seconds: 3600,                 // 1 hour
            max_size_bytes: 256 * 1024 * 1024, // 256 MB
        }
    }
}

/// Thread-safe AST cache using DashMap for high-performance concurrent access
#[derive(Debug)]
pub struct AstCache {
    /// Cache storage mapping file paths to cached entries
    cache: DashMap<PathBuf, CachedEntry>,
    /// Cache statistics
    stats: DashMap<String, u64>,
    /// Cache configuration
    settings: CacheSettings,
}

impl AstCache {
    /// Create a new AST cache with default settings
    pub fn new() -> Self {
        Self::with_settings(CacheSettings::default())
    }

    /// Create a new AST cache with custom settings
    pub fn with_settings(settings: CacheSettings) -> Self {
        let cache = Self {
            cache: DashMap::new(),
            stats: DashMap::new(),
            settings: settings.clone(),
        };

        // Initialize statistics counters
        cache.stats.insert("hits".to_string(), 0);
        cache.stats.insert("misses".to_string(), 0);
        cache.stats.insert("invalidations".to_string(), 0);
        cache.stats.insert("inserts".to_string(), 0);
        cache.stats.insert("evictions".to_string(), 0);

        debug!(
            enabled = settings.enabled,
            max_entries = settings.max_entries,
            ttl_seconds = settings.ttl_seconds,
            "AstCache initialized"
        );
        cache
    }

    /// Check if cache is enabled
    pub fn is_enabled(&self) -> bool {
        self.settings.enabled
    }

    /// Get cache settings
    pub fn settings(&self) -> &CacheSettings {
        &self.settings
    }

    /// Get a cached import graph if it exists and is still valid
    pub async fn get(&self, file_path: &PathBuf) -> Option<ImportGraph> {
        // Check if cache is enabled
        if !self.settings.enabled {
            return None;
        }

        trace!("Cache get requested for: {}", file_path.display());

        // Check if we have a cached entry
        let entry = self.cache.get(file_path)?;

        // Check TTL expiration
        if let Ok(elapsed) = SystemTime::now().duration_since(entry.cached_at) {
            if elapsed.as_secs() > self.settings.ttl_seconds {
                debug!(
                    "Cache entry expired for {} (age: {}s, TTL: {}s)",
                    file_path.display(),
                    elapsed.as_secs(),
                    self.settings.ttl_seconds
                );
                self.invalidate(file_path);
                self.increment_stat("misses");
                return None;
            }
        }

        // Get current file metadata
        let current_metadata = match tokio::fs::metadata(file_path).await {
            Ok(metadata) => metadata,
            Err(e) => {
                debug!(
                    "Failed to get metadata for {}: {}, invalidating cache",
                    file_path.display(),
                    e
                );
                self.invalidate(file_path);
                return None;
            }
        };

        let current_modified = match current_metadata.modified() {
            Ok(time) => time,
            Err(e) => {
                debug!(
                    "Failed to get modification time for {}: {}, invalidating cache",
                    file_path.display(),
                    e
                );
                self.invalidate(file_path);
                return None;
            }
        };

        // Check if the cached entry is still valid (file hasn't been modified)
        // Compare modification times and file sizes
        let is_valid =
            entry.cached_at <= current_modified && entry.file_size == current_metadata.len();

        if is_valid {
            // Cache hit!
            self.increment_stat("hits");
            trace!("Cache hit for: {}", file_path.display());
            Some(entry.import_graph.clone())
        } else {
            // Cache miss due to file modification
            self.increment_stat("misses");
            debug!(
                "Cache miss for {} (file modified or size changed)",
                file_path.display()
            );
            self.invalidate(file_path);
            None
        }
    }

    /// Insert a new import graph into the cache
    pub async fn insert(
        &self,
        file_path: PathBuf,
        import_graph: ImportGraph,
    ) -> Result<(), std::io::Error> {
        // Check if cache is enabled
        if !self.settings.enabled {
            return Ok(());
        }

        trace!("Cache insert requested for: {}", file_path.display());

        // Check if we need to evict entries to stay under max_entries limit
        if self.cache.len() >= self.settings.max_entries {
            self.evict_lru();
        }

        // Get file metadata for cache validation
        let metadata = tokio::fs::metadata(&file_path).await?;
        let modified_time = metadata.modified()?;
        let file_size = metadata.len();

        let entry = CachedEntry {
            import_graph,
            cached_at: modified_time,
            file_size,
        };

        self.cache.insert(file_path.clone(), entry);
        self.increment_stat("inserts");

        debug!("Cached import graph for: {}", file_path.display());
        Ok(())
    }

    /// Evict least recently used entries when cache is full
    fn evict_lru(&self) {
        // Simple eviction strategy: remove oldest cached entries
        // In a production system, you might want to use a proper LRU implementation
        let mut entries: Vec<(PathBuf, SystemTime)> = self
            .cache
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().cached_at))
            .collect();

        // Sort by cached_at time (oldest first)
        entries.sort_by_key(|(_, cached_at)| *cached_at);

        // Remove oldest 10% of entries
        let evict_count = (self.settings.max_entries / 10).max(1);
        for (path, _) in entries.iter().take(evict_count) {
            if self.cache.remove(path).is_some() {
                self.increment_stat("evictions");
                trace!("Evicted cache entry: {}", path.display());
            }
        }

        debug!("Evicted {} cache entries due to size limit", evict_count);
    }

    /// Invalidate a cached entry
    pub fn invalidate(&self, file_path: &PathBuf) {
        if self.cache.remove(file_path).is_some() {
            self.increment_stat("invalidations");
            debug!("Invalidated cache entry for: {}", file_path.display());
        }
    }

    /// Clear all cached entries
    pub fn clear(&self) {
        let count = self.cache.len();
        self.cache.clear();
        debug!("Cleared {} cached entries", count);
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            hits: self.get_stat("hits"),
            misses: self.get_stat("misses"),
            invalidations: self.get_stat("invalidations"),
            inserts: self.get_stat("inserts"),
            current_entries: self.cache.len(),
        }
    }

    /// Get cache hit ratio as a percentage
    pub fn hit_ratio(&self) -> f64 {
        let hits = self.get_stat("hits") as f64;
        let misses = self.get_stat("misses") as f64;
        let total = hits + misses;

        if total == 0.0 {
            0.0
        } else {
            (hits / total) * 100.0
        }
    }

    /// Check if a file is cached and valid
    pub async fn is_cached(&self, file_path: &PathBuf) -> bool {
        self.get(file_path).await.is_some()
    }

    /// Get current cache size (number of entries)
    pub fn size(&self) -> usize {
        self.cache.len()
    }

    /// Get approximate memory usage of cache (rough estimation)
    pub fn estimated_memory_usage(&self) -> usize {
        // Rough estimation: each entry contains path + import graph + metadata
        // This is approximate and doesn't account for internal DashMap overhead
        self.cache.len() * 1024 // Assume ~1KB per entry on average
    }

    /// Perform cache maintenance (remove entries for files that no longer exist)
    pub fn maintenance(&self) {
        let mut removed_count = 0;
        let paths_to_remove: Vec<PathBuf> = self
            .cache
            .iter()
            .filter_map(|entry| {
                let path = entry.key();
                if !path.exists() {
                    Some(path.clone())
                } else {
                    None
                }
            })
            .collect();

        for path in paths_to_remove {
            self.invalidate(&path);
            removed_count += 1;
        }

        if removed_count > 0 {
            debug!("Cache maintenance: removed {} stale entries", removed_count);
        }
    }

    // Helper methods for statistics
    fn increment_stat(&self, key: &str) {
        self.stats
            .entry(key.to_string())
            .and_modify(|e| *e += 1)
            .or_insert(1);
    }

    fn get_stat(&self, key: &str) -> u64 {
        self.stats.get(key).map(|v| *v).unwrap_or(0)
    }
}

impl Default for AstCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Cache statistics for monitoring and debugging
// CacheStats now comes from cb-api

// CacheStats impl methods now in cb-api

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_cache_basic_operations() {
        let cache = AstCache::new();

        // Create a temporary file
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_path_buf();

        // Write some content
        fs::write(&path, "export const test = 42;").unwrap();

        // Create a mock import graph
        let import_graph = ImportGraph {
            source_file: path.to_string_lossy().to_string(),
            imports: vec![],
            importers: vec![],
            metadata: codebuddy_foundation::protocol::ImportGraphMetadata {
                language: "javascript".to_string(),
                parsed_at: chrono::Utc::now(),
                parser_version: "0.3.0-test".to_string(),
                circular_dependencies: vec![],
                external_dependencies: vec![],
            },
        };

        // Test insert and get
        assert!(cache
            .insert(path.clone(), import_graph.clone())
            .await
            .is_ok());
        assert_eq!(cache.size(), 1);

        let cached = cache.get(&path).await;
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().source_file, import_graph.source_file);

        // Test stats
        let stats = cache.stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.inserts, 1);
        assert_eq!(stats.current_entries, 1);
    }

    #[tokio::test]
    async fn test_cache_invalidation() {
        let cache = AstCache::new();

        // Create a temporary file
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_path_buf();

        // Write initial content
        fs::write(&path, "export const test = 42;").unwrap();

        let import_graph = ImportGraph {
            source_file: path.to_string_lossy().to_string(),
            imports: vec![],
            importers: vec![],
            metadata: codebuddy_foundation::protocol::ImportGraphMetadata {
                language: "javascript".to_string(),
                parsed_at: chrono::Utc::now(),
                parser_version: "0.3.0-test".to_string(),
                circular_dependencies: vec![],
                external_dependencies: vec![],
            },
        };

        // Cache the file
        cache.insert(path.clone(), import_graph).await.unwrap();
        assert!(cache.is_cached(&path).await);

        // Invalidate manually
        cache.invalidate(&path);
        assert!(!cache.is_cached(&path).await);
        assert_eq!(cache.size(), 0);

        let stats = cache.stats();
        assert_eq!(stats.invalidations, 1);
    }

    #[tokio::test]
    async fn test_cache_stats_and_hit_ratio() {
        let cache = AstCache::new();

        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_path_buf();
        fs::write(&path, "test").unwrap();

        let import_graph = ImportGraph {
            source_file: path.to_string_lossy().to_string(),
            imports: vec![],
            importers: vec![],
            metadata: codebuddy_foundation::protocol::ImportGraphMetadata {
                language: "javascript".to_string(),
                parsed_at: chrono::Utc::now(),
                parser_version: "0.3.0-test".to_string(),
                circular_dependencies: vec![],
                external_dependencies: vec![],
            },
        };

        cache.insert(path.clone(), import_graph).await.unwrap();

        // Multiple gets should increase hit count
        cache.get(&path).await;
        cache.get(&path).await;
        cache.get(&path).await;

        let stats = cache.stats();
        assert_eq!(stats.hits, 3);
        assert_eq!(stats.misses, 0);
        assert_eq!(stats.hit_ratio(), 100.0);
    }
}
