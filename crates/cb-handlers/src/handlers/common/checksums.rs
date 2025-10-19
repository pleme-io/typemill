//! Shared checksum calculation utilities for refactoring operations
//!
//! This module provides common checksum calculation logic used by both
//! rename and move handlers to ensure file integrity during refactoring.

use crate::handlers::tools::ToolHandlerContext;
use codebuddy_foundation::protocol::{
    ApiError as ServerError, ApiResult as ServerResult, TextEdit as ProtocolTextEdit,
};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use tracing::debug;

/// Calculate SHA-256 checksum of file content
pub fn calculate_checksum(content: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Calculate checksums for all files affected by edits
///
/// Collects all unique file paths from the edit list and calculates
/// checksums for validation during apply phase.
pub async fn calculate_checksums_for_edits(
    edits: &[ProtocolTextEdit],
    additional_paths: &[PathBuf],
    context: &ToolHandlerContext,
) -> ServerResult<HashMap<String, String>> {
    let mut file_checksums = HashMap::new();
    let mut affected_files = HashSet::new();

    // Add additional paths (e.g., source file being moved)
    for path in additional_paths {
        affected_files.insert(path.clone());
    }

    // Add all files mentioned in edits
    for edit in edits {
        if let Some(ref file_path) = edit.file_path {
            affected_files.insert(Path::new(file_path).to_path_buf());
        }
    }

    // Read and checksum each file
    for file_path in affected_files {
        if file_path.exists() {
            if let Ok(content) = context.app_state.file_service.read_file(&file_path).await {
                file_checksums.insert(
                    file_path.to_string_lossy().to_string(),
                    calculate_checksum(&content),
                );
            }
        }
    }

    debug!(
        files_count = file_checksums.len(),
        "Calculated checksums for affected files"
    );

    Ok(file_checksums)
}

/// Calculate checksums for a directory and its external edits
///
/// This is specialized for directory renames where we need to checksum:
/// 1. All files inside the directory being moved
/// 2. External files being edited (e.g., for import updates)
pub async fn calculate_checksums_for_directory_rename(
    directory_path: &Path,
    edits: &[ProtocolTextEdit],
    context: &ToolHandlerContext,
) -> ServerResult<HashMap<String, String>> {
    let mut file_checksums = HashMap::new();

    // Walk directory to collect all files and calculate checksums
    let walker = ignore::WalkBuilder::new(directory_path)
        .hidden(false)
        .build();

    for entry in walker.flatten() {
        if entry.path().is_file() {
            if let Ok(content) = context.app_state.file_service.read_file(entry.path()).await {
                file_checksums.insert(
                    entry.path().to_string_lossy().to_string(),
                    calculate_checksum(&content),
                );
            }
        }
    }

    // Add checksums for files being updated (import updates outside the moved directory)
    for edit in edits {
        if let Some(ref file_path) = edit.file_path {
            let path = Path::new(file_path.as_str());

            // Skip files inside the directory being moved (already checksummed above)
            // Only checksum files OUTSIDE the moved directory that are being edited
            if path.exists() && !path.starts_with(directory_path) {
                if let Ok(content) = context.app_state.file_service.read_file(path).await {
                    file_checksums.insert(
                        file_path.clone(),
                        calculate_checksum(&content),
                    );
                }
            }
        }
    }

    debug!(
        directory = %directory_path.display(),
        files_count = file_checksums.len(),
        "Calculated checksums for directory rename"
    );

    Ok(file_checksums)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_checksum() {
        let content = "Hello, World!";
        let checksum = calculate_checksum(content);

        // SHA-256 of "Hello, World!"
        assert_eq!(
            checksum,
            "dffd6021bb2bd5b0af676290809ec3a53191dd81c7f70a4b28688a362182986f"
        );
    }

    #[test]
    fn test_calculate_checksum_empty() {
        let checksum = calculate_checksum("");

        // SHA-256 of empty string
        assert_eq!(
            checksum,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }
}
