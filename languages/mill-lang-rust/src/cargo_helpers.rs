//! Minimal Cargo.toml helpers for consolidation feature
//!
//! NOTE: This module contains Rust-specific helpers for consolidation
//! post-processing workflow. Handles merging dependencies from source
//! crate into target crate during consolidation.

use mill_plugin_api::{PluginError, PluginResult};
use std::path::Path;
use tokio::fs;
use tracing::{info, warn};

/// Merge dependencies from source Cargo.toml into target Cargo.toml
///
/// This is a helper for consolidation post-processing.
pub async fn merge_cargo_dependencies(
    source_toml_path: &Path,
    target_toml_path: &Path,
) -> PluginResult<()> {
    use toml_edit::DocumentMut;

    info!(
        source = %source_toml_path.display(),
        target = %target_toml_path.display(),
        "Merging Cargo.toml dependencies (consolidation)"
    );

    // Read both TOML files
    let source_content = fs::read_to_string(source_toml_path)
        .await
        .map_err(|e| PluginError::internal(format!("Failed to read source Cargo.toml: {}", e)))?;

    let target_content = fs::read_to_string(target_toml_path)
        .await
        .map_err(|e| PluginError::internal(format!("Failed to read target Cargo.toml: {}", e)))?;

    // Parse both documents
    let source_doc = source_content
        .parse::<DocumentMut>()
        .map_err(|e| PluginError::internal(format!("Failed to parse source Cargo.toml: {}", e)))?;

    let mut target_doc = target_content
        .parse::<DocumentMut>()
        .map_err(|e| PluginError::internal(format!("Failed to parse target Cargo.toml: {}", e)))?;

    let mut merged_count = 0;
    let mut conflict_count = 0;

    // Extract target crate name for circular dependency detection
    let target_crate_name = target_doc
        .get("package")
        .and_then(|p| p.get("name"))
        .and_then(|n| n.as_str())
        .unwrap_or("unknown")
        .to_string();

    // Merge [dependencies], [dev-dependencies], and [build-dependencies]
    for section in ["dependencies", "dev-dependencies", "build-dependencies"] {
        if let Some(source_deps) = source_doc.get(section).and_then(|v| v.as_table()) {
            // Ensure target has this section
            if target_doc.get(section).is_none() {
                target_doc[section] = toml_edit::Item::Table(toml_edit::Table::new());
            }

            if let Some(target_deps) = target_doc[section].as_table_mut() {
                for (dep_name, dep_value) in source_deps.iter() {
                    // Skip self-dependency (would create circular dependency)
                    if dep_name == target_crate_name.as_str() {
                        warn!(
                            dependency = %dep_name,
                            section = %section,
                            "Skipping self-dependency during consolidation merge"
                        );
                        continue;
                    }

                    // Check if dependency already exists
                    if target_deps.contains_key(dep_name) {
                        conflict_count += 1;
                        info!(
                            dependency = %dep_name,
                            section = %section,
                            "Dependency already exists in target, skipping"
                        );
                    } else {
                        target_deps.insert(dep_name, dep_value.clone());
                        merged_count += 1;
                    }
                }
            }
        }
    }

    // Write merged content back to target
    fs::write(target_toml_path, target_doc.to_string())
        .await
        .map_err(|e| PluginError::internal(format!("Failed to write merged Cargo.toml: {}", e)))?;

    info!(
        merged = merged_count,
        conflicts = conflict_count,
        "Completed Cargo.toml dependency merge"
    );

    Ok(())
}
