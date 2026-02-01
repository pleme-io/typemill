//! Import cache for reference updater
//!
//! Caches file import information to avoid re-parsing files during reference updates.
//! Uses a reverse index for O(1) lookups of "which files import this path?"

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

/// Cached information about a file's imports
#[derive(Debug, Clone)]
pub struct FileImportInfo {
    /// The files that this file imports
    pub imports: Vec<PathBuf>,
    /// Last modified time when this cache entry was created
    pub last_modified: std::time::SystemTime,
}

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
}
