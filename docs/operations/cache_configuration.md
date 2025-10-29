# Cache Configuration

TypeMill uses multiple caching layers to improve performance. This document describes the caching system, configuration options, and troubleshooting.

## Cache Types

### 1. AST Cache
**Purpose:** Cache parsed Abstract Syntax Trees and import graphs to avoid re-parsing files
**Location:** In-memory (per-process)
**Default:** Enabled

**Features:**
- Thread-safe concurrent access via DashMap
- TTL-based expiration (default: 1 hour)
- LRU eviction when cache is full
- File modification time validation
- Statistics tracking (hits, misses, invalidations)

**Configuration:**
```json
{
  "cache": {
    "enabled": true,
    "maxSizeBytes": 268435456,
    "ttlSeconds": 3600,
    "persistent": false
  }
}
```text
**Defaults:**
- Max size: 256 MB (268435456 bytes)
- Max entries: 10,000
- TTL: 3600 seconds (1 hour)

### 2. Import Cache
**Purpose:** Cache file import lists for directory rename operations
**Location:** In-memory (per-operation)
**Default:** Enabled

**Features:**
- Stores complete import lists per file
- File modification time validation
- Used during `rename` and directory renames

### 3. LSP Method Translation Cache
**Purpose:** Cache method name translations between plugin API and LSP protocol
**Location:** In-memory (per LSP adapter instance)
**Default:** Enabled

**Features:**
- Simple HashMap for method mappings
- Example: `find_definition` → `textDocument/definition`
- Static mappings (never invalidated)
- Very low overhead (~1KB memory)

## Environment Variable Control

TypeMill supports environment variables for fine-grained cache control. This is especially useful for:
- **Debugging**: Isolate cache-related bugs
- **Testing**: Ensure fresh parses in CI/CD
- **Development**: Force fresh data during active development

### Master Switch

Disable **all** caches at once:

```bash
TYPEMILL_DISABLE_CACHE=1 mill serve
```text
or

```bash
TYPEMILL_DISABLE_CACHE=true mill serve
```text
### Individual Cache Controls

Disable specific caches while keeping others enabled:

```bash
# Disable only AST cache
TYPEMILL_DISABLE_AST_CACHE=1 mill serve

# Disable only import cache (used during directory renames)
TYPEMILL_DISABLE_IMPORT_CACHE=1 mill serve

# Disable only LSP method translation cache
TYPEMILL_DISABLE_LSP_METHOD_CACHE=1 mill serve
```text
### Priority Order

Environment variables take precedence over configuration file settings:

```text
1. TYPEMILL_DISABLE_CACHE (master switch)
2. TYPEMILL_DISABLE_*_CACHE (individual switches)
3. Configuration file (cache.enabled)
4. Default (enabled)
```text
**Example:**
```bash
# Even if config.json has "enabled": true, this will disable AST cache
TYPEMILL_DISABLE_AST_CACHE=1 mill serve
```text
## Configuration File

Configure caching in `.typemill/config.json`:

```json
{
  "cache": {
    "enabled": true,
    "maxSizeBytes": 268435456,
    "ttlSeconds": 3600,
    "persistent": false,
    "cacheDir": null
  }
}
```text
### Configuration Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `enabled` | boolean | `true` | Enable/disable AST cache |
| `maxSizeBytes` | number | `268435456` | Maximum cache size in bytes (256 MB) |
| `ttlSeconds` | number | `3600` | Time-to-live for cache entries (1 hour) |
| `persistent` | boolean | `false` | Enable persistent disk cache (⚠️ not implemented) |
| `cacheDir` | string\|null | `null` | Directory for persistent cache (⚠️ not implemented) |

### Validation Rules

- `maxSizeBytes` must be > 0 when cache is enabled
- `ttlSeconds` must be > 0
- If `persistent` is true, `cacheDir` must be specified

## Cache Statistics

The AST cache tracks detailed statistics:

- **Hits**: Number of successful cache lookups
- **Misses**: Number of cache misses (file not cached or stale)
- **Invalidations**: Number of manual invalidations
- **Inserts**: Number of new entries added
- **Evictions**: Number of LRU evictions
- **Current Entries**: Number of entries currently cached

### Viewing Statistics

Statistics are available via the health check endpoint (planned feature):

```bash
curl http://localhost:3040/health
```text
## Cache Behavior

### Cache Invalidation

Caches are automatically invalidated when:

1. **File modification**: File's modification time changes
2. **TTL expiration**: Entry exceeds configured TTL
3. **Cache full**: LRU eviction removes oldest entries

### Cache Warming

Cache warming (pre-populating caches on startup) is a planned feature.

## Troubleshooting

### Issue: Stale data after file changes

**Symptom:** TypeMill returns outdated results after file modifications

**Solutions:**
1. Check if file modification times are being updated correctly
2. Verify the cache is using modification time validation
3. Temporarily disable cache to confirm it's cache-related:
   ```bash
   TYPEMILL_DISABLE_CACHE=1 mill serve
   ```

### Issue: High memory usage

**Symptom:** TypeMill process consuming excessive memory

**Solutions:**
1. Reduce `maxSizeBytes` in configuration
2. Reduce `ttlSeconds` to expire entries sooner
3. Monitor cache statistics to identify thrashing
4. Consider disabling cache if not needed:
   ```json
   {
     "cache": {
       "enabled": false
     }
   }
   ```

### Issue: Slow first request after restart

**Symptom:** First operation is slow, then fast

**Explanation:** This is expected behavior - caches are cold after restart and warm up with use.

**Solutions:**
1. Wait for cache to warm up naturally
2. Use cache warming (planned feature)
3. Consider enabling persistent cache (when implemented)

### Issue: Debugging import updates

**Symptom:** Directory renames not updating all import statements

**Solution:** Disable import cache to force fresh scans:
```bash
TYPEMILL_DISABLE_IMPORT_CACHE=1 ./target/release/mill tool rename ...
```text
## Best Practices

### Development

During active development, consider disabling caches:

```bash
# Development mode - disable all caches
export TYPEMILL_DISABLE_CACHE=1
mill serve
```text
### Production

In production, keep caches enabled with appropriate sizing:

```json
{
  "cache": {
    "enabled": true,
    "maxSizeBytes": 536870912,
    "ttlSeconds": 7200
  }
}
```text
### CI/CD

In CI/CD pipelines, disable caches to ensure fresh results:

```bash
# CI/CD mode
TYPEMILL_DISABLE_CACHE=1 cargo test
```text
### Testing

When testing cache behavior, use individual controls:

```bash
# Test AST cache specifically
TYPEMILL_DISABLE_AST_CACHE=1 cargo test ast_cache

# Test import cache specifically
TYPEMILL_DISABLE_IMPORT_CACHE=1 cargo test import_updates
```text
## Future Enhancements

Planned cache features:

1. **Cache statistics endpoint**: View cache performance metrics
2. **Cache clear command**: `mill cache clear [--ast] [--import] [--all]`
3. **Cache warming**: Pre-populate caches on startup
4. **Persistent cache**: Disk-based cache for faster restarts
5. **Cache metrics**: Prometheus-compatible metrics export

## See Also

- [Configuration Overview](../README.md)
- [Performance Tuning](../deployment/PERFORMANCE.md) (planned)
- [Architecture: Caching](../architecture/CACHING.md) (planned)