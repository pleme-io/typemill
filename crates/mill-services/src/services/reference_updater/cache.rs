//! Import cache for reference updater
//!
//! Caches file import information to avoid re-parsing files during reference updates.
//! Uses a reverse index for O(1) lookups of "which files import this path?"

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Cached information about a file's imports
#[derive(Debug, Clone)]
pub struct FileImportInfo {
    /// The files that this file imports
    pub imports: Vec<PathBuf>,
    /// Last modified time when this cache entry was created
    pub last_modified: std::time::SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CacheEntry {
    imports: Vec<PathBuf>,
    last_modified_ms: u128,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CacheSnapshot {
    version: u32,
    entries: HashMap<PathBuf, CacheEntry>,
}

const CACHE_VERSION: u32 = 1;
const FILELIST_CACHE_VERSION: u32 = 1;

/// Thread-safe import cache with reverse index for fast lookups
///
/// This cache stores both:
/// - Forward index: file_path -> list of files it imports
/// - Reverse index: file_path -> list of files that import it
///
/// The reverse index enables O(1) lookups for "which files import this path?"
/// which is the critical operation when renaming/moving files.
#[derive(Debug, Default)]
pub struct ImportCache {
    /// Forward index: file -> files it imports (with modification time for invalidation)
    forward: RwLock<HashMap<PathBuf, FileImportInfo>>,
    /// Reverse index: file -> files that import it
    reverse: RwLock<HashMap<PathBuf, HashSet<PathBuf>>>,
}

impl ImportCache {
    pub fn new() -> Self {
        Self::default()
    }

    /// Wrap in Arc for sharing across threads
    pub fn shared() -> Arc<Self> {
        Arc::new(Self::new())
    }

    /// Check if a file's imports are cached and still valid
    pub fn get_imports(&self, file: &PathBuf) -> Option<Vec<PathBuf>> {
        let forward = self.forward.read().ok()?;
        let info = forward.get(file)?;

        // Check if file has been modified since we cached it
        if let Ok(metadata) = std::fs::metadata(file) {
            if let Ok(modified) = metadata.modified() {
                if modified == info.last_modified {
                    return Some(info.imports.clone());
                }
            }
        }
        None
    }

    /// Store imports for a file and update reverse index
    pub fn set_imports(&self, file: PathBuf, imports: Vec<PathBuf>, last_modified: std::time::SystemTime) {
        // First, remove old reverse entries if this file was already cached
        if let Ok(forward) = self.forward.read() {
            if let Some(old_info) = forward.get(&file) {
                if let Ok(mut reverse) = self.reverse.write() {
                    for old_import in &old_info.imports {
                        if let Some(importers) = reverse.get_mut(old_import) {
                            importers.remove(&file);
                        }
                    }
                }
            }
        }

        // Update forward index
        if let Ok(mut forward) = self.forward.write() {
            forward.insert(file.clone(), FileImportInfo {
                imports: imports.clone(),
                last_modified,
            });
        }

        // Update reverse index
        if let Ok(mut reverse) = self.reverse.write() {
            for import in imports {
                reverse.entry(import).or_default().insert(file.clone());
            }
        }
    }

    /// Get all files that import a given path (O(1) lookup)
    ///
    /// This is the key optimization - instead of scanning all files,
    /// we can directly look up which files import a given path.
    pub fn get_importers(&self, imported_file: &PathBuf) -> HashSet<PathBuf> {
        if let Ok(reverse) = self.reverse.read() {
            reverse.get(imported_file).cloned().unwrap_or_default()
        } else {
            HashSet::new()
        }
    }

    /// Get all files that import anything under a directory prefix
    ///
    /// For directory renames, we need all files that import any file
    /// under the directory being renamed.
    pub fn get_importers_for_directory(&self, dir: &PathBuf) -> HashSet<PathBuf> {
        let mut result = HashSet::new();
        if let Ok(reverse) = self.reverse.read() {
            for (imported_path, importers) in reverse.iter() {
                if imported_path.starts_with(dir) {
                    result.extend(importers.iter().cloned());
                }
            }
        }
        result
    }

    /// Check if the cache has been populated
    pub fn is_populated(&self) -> bool {
        self.forward.read().map(|f| !f.is_empty()).unwrap_or(false)
    }

    /// Get cache statistics for debugging
    pub fn stats(&self) -> (usize, usize) {
        let forward_count = self.forward.read().map(|f| f.len()).unwrap_or(0);
        let reverse_count = self.reverse.read().map(|r| r.len()).unwrap_or(0);
        (forward_count, reverse_count)
    }

    /// Populate the reverse index from LSP-detected importers
    ///
    /// This allows caching LSP detection results for future queries.
    /// The LSP provides the reverse mapping directly (who imports this file?),
    /// so we just need to store it.
    pub fn cache_lsp_importers(&self, imported_file: PathBuf, importers: Vec<PathBuf>) {
        if let Ok(mut reverse) = self.reverse.write() {
            let entry = reverse.entry(imported_file).or_default();
            for importer in importers {
                entry.insert(importer);
            }
        }
    }

    /// Populate the reverse index for a directory from LSP-detected importers
    pub fn cache_lsp_directory_importers(&self, dir: PathBuf, importers: Vec<PathBuf>) {
        // For directory imports, we store under the directory path itself
        // This allows get_importers_for_directory to find them
        if let Ok(mut reverse) = self.reverse.write() {
            let entry = reverse.entry(dir).or_default();
            for importer in importers {
                entry.insert(importer);
            }
        }
    }

    /// Clear the cache (useful for testing or when project structure changes significantly)
    pub fn clear(&self) {
        if let Ok(mut forward) = self.forward.write() {
            forward.clear();
        }
        if let Ok(mut reverse) = self.reverse.write() {
            reverse.clear();
        }
    }

    /// Load cache from disk for a project root (best-effort)
    pub fn load_from_disk(&self, project_root: &Path) -> bool {
        let cache_path = match cache_path_for_project(project_root) {
            Some(path) => path,
            None => return false,
        };

        let data = match std::fs::read_to_string(&cache_path) {
            Ok(content) => content,
            Err(_) => return false,
        };

        let snapshot: CacheSnapshot = match serde_json::from_str(&data) {
            Ok(parsed) => parsed,
            Err(_) => return false,
        };

        if snapshot.version != CACHE_VERSION {
            return false;
        }

        let mut forward = HashMap::new();
        let mut reverse: HashMap<PathBuf, HashSet<PathBuf>> = HashMap::new();

        for (file, entry) in snapshot.entries {
            if let Ok(metadata) = std::fs::metadata(&file) {
                if let Ok(modified) = metadata.modified() {
                    if system_time_to_millis(modified) == Some(entry.last_modified_ms) {
                        forward.insert(
                            file.clone(),
                            FileImportInfo {
                                imports: entry.imports.clone(),
                                last_modified: modified,
                            },
                        );
                        for import in entry.imports {
                            reverse.entry(import).or_default().insert(file.clone());
                        }
                    }
                }
            }
        }

        if let Ok(mut forward_lock) = self.forward.write() {
            forward_lock.clear();
            forward_lock.extend(forward);
        }
        if let Ok(mut reverse_lock) = self.reverse.write() {
            reverse_lock.clear();
            reverse_lock.extend(reverse);
        }

        true
    }

    /// Persist cache to disk for a project root (best-effort)
    pub fn save_to_disk(&self, project_root: &Path) -> bool {
        let cache_path = match cache_path_for_project(project_root) {
            Some(path) => path,
            None => return false,
        };

        let forward = match self.forward.read() {
            Ok(guard) => guard.clone(),
            Err(_) => return false,
        };

        let mut entries = HashMap::new();
        for (file, info) in forward {
            if let Some(ms) = system_time_to_millis(info.last_modified) {
                entries.insert(
                    file,
                    CacheEntry {
                        imports: info.imports,
                        last_modified_ms: ms,
                    },
                );
            }
        }

        let snapshot = CacheSnapshot {
            version: CACHE_VERSION,
            entries,
        };

        if let Some(parent) = cache_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        let tmp_path = cache_path.with_extension("json.tmp");
        if let Ok(serialized) = serde_json::to_string(&snapshot) {
            if std::fs::write(&tmp_path, serialized).is_ok() {
                let _ = std::fs::rename(&tmp_path, &cache_path);
                return true;
            }
        }

        false
    }
}

fn system_time_to_millis(time: SystemTime) -> Option<u128> {
    time.duration_since(UNIX_EPOCH).ok().map(|d| d.as_millis())
}

fn cache_path_for_project(project_root: &Path) -> Option<PathBuf> {
    let base = if let Ok(dir) = std::env::var("TYPEMILL_CACHE_DIR") {
        PathBuf::from(dir)
    } else if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".typemill").join("cache")
    } else {
        return None;
    };

    let mut hasher = Sha256::new();
    hasher.update(project_root.to_string_lossy().as_bytes());
    let hash = format!("{:x}", hasher.finalize());
    Some(base.join("imports").join(format!("{}.json", hash)))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FileListSnapshot {
    version: u32,
    created_at_ms: u128,
    files: Vec<PathBuf>,
}

pub fn load_filelist_cache(
    project_root: &Path,
    scope_key: &str,
    ttl: Duration,
) -> Option<Vec<PathBuf>> {
    if ttl.is_zero() {
        return None;
    }

    let cache_path = filelist_cache_path(project_root, scope_key)?;
    let data = std::fs::read_to_string(cache_path).ok()?;
    let snapshot: FileListSnapshot = serde_json::from_str(&data).ok()?;

    if snapshot.version != FILELIST_CACHE_VERSION {
        return None;
    }

    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()?
        .as_millis();
    if now_ms.saturating_sub(snapshot.created_at_ms) > ttl.as_millis() {
        return None;
    }

    let validate = std::env::var("TYPEMILL_FILELIST_CACHE_VALIDATE")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    if !validate {
        return Some(snapshot.files);
    }

    let mut filtered = Vec::with_capacity(snapshot.files.len());
    for path in snapshot.files {
        if path.exists() {
            filtered.push(path);
        }
    }
    Some(filtered)
}

pub fn save_filelist_cache(project_root: &Path, scope_key: &str, files: &[PathBuf]) -> bool {
    let cache_path = match filelist_cache_path(project_root, scope_key) {
        Some(path) => path,
        None => return false,
    };

    if let Some(parent) = cache_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let created_at_ms = match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => duration.as_millis(),
        Err(_) => return false,
    };

    let snapshot = FileListSnapshot {
        version: FILELIST_CACHE_VERSION,
        created_at_ms,
        files: files.to_vec(),
    };

    let tmp_path = cache_path.with_extension("json.tmp");
    if let Ok(serialized) = serde_json::to_string(&snapshot) {
        if std::fs::write(&tmp_path, serialized).is_ok() {
            let _ = std::fs::rename(&tmp_path, &cache_path);
            return true;
        }
    }

    false
}

fn filelist_cache_path(project_root: &Path, scope_key: &str) -> Option<PathBuf> {
    let base = if let Ok(dir) = std::env::var("TYPEMILL_CACHE_DIR") {
        PathBuf::from(dir)
    } else if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".typemill").join("cache")
    } else {
        return None;
    };

    let mut hasher = Sha256::new();
    hasher.update(project_root.to_string_lossy().as_bytes());
    hasher.update(scope_key.as_bytes());
    let hash = format!("{:x}", hasher.finalize());
    Some(base.join("filelist").join(format!("{}.json", hash)))
}
