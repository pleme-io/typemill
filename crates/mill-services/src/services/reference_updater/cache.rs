//! Import cache for reference updater
//!
//! Caches file import information to avoid re-parsing files during reference updates.

use std::path::PathBuf;

/// Cached information about a file's imports
#[derive(Debug, Clone)]
pub struct FileImportInfo {
    /// The files that this file imports
    pub imports: Vec<PathBuf>,
    /// Last modified time when this cache entry was created
    pub last_modified: std::time::SystemTime,
}
