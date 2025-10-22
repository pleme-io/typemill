use sha2::{Digest, Sha256};

/// Calculate SHA-256 checksum of file content
pub(crate) fn calculate_checksum(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Map file extension to language name
pub(crate) fn extension_to_language(extension: &str) -> String {
    match extension {
        "rs" => "rust",
        "ts" | "tsx" => "typescript",
        "js" | "jsx" => "javascript",
        "py" | "pyi" => "python",
        "go" => "go",
        "java" => "java",
        "swift" => "swift",
        "cs" => "csharp",
        "md" | "markdown" => "markdown",
        _ => "unknown",
    }
    .to_string()
}

/// Estimate impact based on number of affected files
pub(crate) fn estimate_impact(affected_files: usize) -> String {
    if affected_files <= 3 {
        "low"
    } else if affected_files <= 10 {
        "medium"
    } else {
        "high"
    }
    .to_string()
}
