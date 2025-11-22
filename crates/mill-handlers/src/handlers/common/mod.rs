//! Common utilities shared across refactoring handlers
//!
//! This module provides shared functionality used by rename, move, and other
//! refactoring operations to avoid code duplication.

pub mod checksums;

pub use checksums::calculate_checksum;
pub(crate) use checksums::{
    calculate_checksums_for_directory_rename, calculate_checksums_for_edits,
};

/// Estimate impact based on number of affected files
pub fn estimate_impact(affected_files: usize) -> String {
    if affected_files <= 3 {
        "low"
    } else if affected_files <= 10 {
        "medium"
    } else {
        "high"
    }
    .to_string()
}

/// Detect language from file path extension
pub fn detect_language(file_path: &str) -> &'static str {
    use std::path::Path;
    let path = Path::new(file_path);
    match path.extension().and_then(|e| e.to_str()) {
        Some("rs") => "rust",
        Some("ts") | Some("tsx") => "typescript",
        Some("js") | Some("jsx") => "javascript",
        Some("py") | Some("pyi") => "python",
        Some("go") => "go",
        Some("java") => "java",
        Some("swift") => "swift",
        Some("cs") => "csharp",
        Some("c") | Some("h") => "c",
        Some("cpp") | Some("hpp") | Some("cc") | Some("cxx") => "cpp",
        _ => "unknown",
    }
}
