//! Crate name extraction from Cargo.toml files
//!
//! Provides utilities to find and extract crate names from Cargo.toml manifests.

use std::path::Path;

/// Helper function to extract crate name from Cargo.toml
///
/// Walks up the directory tree from `file_path` looking for a Cargo.toml file.
/// When found, parses the file to extract the package name.
///
/// Used as fallback when path-based extraction fails (e.g., file doesn't exist yet).
///
/// # Returns
/// - `Some(String)` - The crate name if found
/// - `None` - If no Cargo.toml found or if parsing fails
pub fn find_crate_name_from_cargo_toml(file_path: &Path) -> Option<String> {
    let mut current = file_path.parent()?;
    while current.components().count() > 0 {
        let cargo_toml = current.join("Cargo.toml");
        if cargo_toml.exists() {
            if let Ok(content) = std::fs::read_to_string(&cargo_toml) {
                for line in content.lines() {
                    let trimmed = line.trim();
                    if trimmed.starts_with("name") && trimmed.contains('=') {
                        if let Some(name_part) = trimmed.split('=').nth(1) {
                            let name = name_part.trim().trim_matches('"').trim_matches('\'');
                            tracing::info!(
                                crate_name = %name,
                                cargo_toml = %cargo_toml.display(),
                                "Found crate name in Cargo.toml"
                            );
                            return Some(name.to_string());
                        }
                    }
                }
            }
            break;
        }
        current = current.parent()?;
    }
    tracing::warn!(
        file_path = %file_path.display(),
        "Could not find Cargo.toml walking up from file path"
    );
    None
}
