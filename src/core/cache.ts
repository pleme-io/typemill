/**
 * Advanced in-memory cache with event-driven invalidation for file operations
 * Maximizes cache hits while ensuring data consistency through file change events
 */

export interface CacheEntry<T> {
  value: T;
  timestamp: Date;
  ttl?: number; // Optional TTL - for fallback expiration only
  persistent?: boolean; // If true, only manual invalidation will remove
}

export interface CacheStats {
  size: number;
  hitRate?: number;
  totalHits: number;
  totalMisses: number;
  entries: Array<{
    key: string;
    age: number;
    ttl?: number;
    isExpired: boolean;
    isPersistent: boolean;
  }>;
}

export class AdvancedCache<T> {
  private cache = new Map<string, CacheEntry<T>>();
  private cleanupInterval: NodeJS.Timeout;
  private hitCount = 0;
  private missCount = 0;

  constructor(private defaultTTL?: number) {
    // Clean up expired entries every 60 seconds (less frequent for persistent cache)
    this.cleanupInterval = setInterval(() => {
      this.cleanup();
    }, 60000);
  }

  set(key: string, value: T, options?: { ttl?: number; persistent?: boolean }): void {
    const entry: CacheEntry<T> = {
      value,
      timestamp: new Date(),
      ttl: options?.ttl || this.defaultTTL,
      persistent: options?.persistent || false,
    };

    this.cache.set(key, entry);
  }

  setPersistent(key: string, value: T): void {
    this.set(key, value, { persistent: true });
  }

  get(key: string): T | null {
    const entry = this.cache.get(key);

    if (!entry) {
      this.missCount++;
      return null;
    }

    // For persistent entries, don't check TTL expiration
    if (entry.persistent) {
      this.hitCount++;
      return entry.value;
    }

    // Check if entry has expired (only for non-persistent entries)
    if (entry.ttl) {
      const now = Date.now();
      const entryTime = entry.timestamp.getTime();

      if (now - entryTime > entry.ttl) {
        this.cache.delete(key);
        this.missCount++;
        return null;
      }
    }

    this.hitCount++;
    return entry.value;
  }

  has(key: string): boolean {
    return this.get(key) !== null;
  }

  delete(key: string): boolean {
    return this.cache.delete(key);
  }

  clear(): void {
    this.cache.clear();
  }

  size(): number {
    return this.cache.size;
  }

  /**
   * Force invalidate a cache entry regardless of persistence
   */
  invalidate(key: string): boolean {
    return this.cache.delete(key);
  }

  /**
   * Remove expired entries from cache (only affects non-persistent entries)
   */
  private cleanup(): void {
    const now = Date.now();
    const keysToDelete: string[] = [];

    for (const [key, entry] of this.cache.entries()) {
      // Only clean up non-persistent entries with TTL
      if (!entry.persistent && entry.ttl) {
        const entryTime = entry.timestamp.getTime();
        if (now - entryTime > entry.ttl) {
          keysToDelete.push(key);
        }
      }
    }

    for (const key of keysToDelete) {
      this.cache.delete(key);
    }

    if (keysToDelete.length > 0) {
      console.debug(`Cache cleanup: removed ${keysToDelete.length} expired entries`);
    }
  }

  /**
   * Get comprehensive cache statistics
   */
  getStats(): CacheStats {
    const now = Date.now();
    const entries = Array.from(this.cache.entries()).map(([key, entry]) => {
      const age = now - entry.timestamp.getTime();
      const isExpired = !entry.persistent && entry.ttl ? age > entry.ttl : false;

      return {
        key,
        age,
        ttl: entry.ttl,
        isExpired,
        isPersistent: entry.persistent || false,
      };
    });

    const totalRequests = this.hitCount + this.missCount;
    const hitRate = totalRequests > 0 ? this.hitCount / totalRequests : undefined;

    return {
      size: this.cache.size,
      hitRate,
      totalHits: this.hitCount,
      totalMisses: this.missCount,
      entries,
    };
  }

  /**
   * Clean up resources
   */
  dispose(): void {
    if (this.cleanupInterval) {
      clearInterval(this.cleanupInterval);
    }
    this.clear();
  }
}

/**
 * Advanced file-specific cache implementation with event-driven invalidation
 */
export interface FileContent {
  content: string;
  mtime: number;
}

export class PersistentFileCache extends AdvancedCache<FileContent> {
  /**
   * Generate cache key for file read operations
   */
  static getFileKey(sessionId: string, filePath: string): string {
    return `file:${sessionId}:${filePath}`;
  }

  /**
   * Cache file content persistently (until explicitly invalidated)
   */
  setFile(sessionId: string, filePath: string, content: string, mtime: number): void {
    const key = PersistentFileCache.getFileKey(sessionId, filePath);
    this.setPersistent(key, { content, mtime });
  }

  /**
   * Get cached file content with mtime validation
   */
  getFile(sessionId: string, filePath: string, currentMtime?: number): FileContent | null {
    const key = PersistentFileCache.getFileKey(sessionId, filePath);
    const cached = this.get(key);

    if (!cached) {
      return null;
    }

    // If mtime is provided, validate it matches cached version
    if (currentMtime !== undefined && cached.mtime !== currentMtime) {
      // File was modified externally - invalidate cache
      this.invalidate(key);
      return null;
    }

    return cached;
  }

  /**
   * Event-driven invalidation for specific file
   */
  invalidateFile(sessionId: string, filePath: string): boolean {
    const key = PersistentFileCache.getFileKey(sessionId, filePath);
    return this.invalidate(key);
  }

  /**
   * Bulk invalidation for file patterns (e.g., directory changes)
   */
  invalidatePattern(sessionId: string, pattern: string): number {
    let deletedCount = 0;
    const stats = this.getStats();
    const prefix = `file:${sessionId}:`;

    for (const entry of stats.entries) {
      if (entry.key.startsWith(prefix)) {
        const filePath = entry.key.slice(prefix.length);
        if (filePath.includes(pattern)) {
          if (this.invalidate(entry.key)) {
            deletedCount++;
          }
        }
      }
    }

    return deletedCount;
  }

  /**
   * Invalidate all cache entries for a session
   */
  invalidateSession(sessionId: string): number {
    let deletedCount = 0;
    const stats = this.getStats();

    for (const entry of stats.entries) {
      if (entry.key.startsWith(`file:${sessionId}:`)) {
        if (this.invalidate(entry.key)) {
          deletedCount++;
        }
      }
    }

    return deletedCount;
  }
}

// Maintain backward compatibility
export type FileCache = PersistentFileCache;
