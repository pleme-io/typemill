//! Rust-specific reference detection
//!
//! Handles detection of affected files for Rust cross-crate and same-crate moves.
//! Uses module path analysis to detect imports that need updating.

use cb_lang_rust::imports::{compute_module_path_from_file, find_crate_name_from_cargo_toml};
use std::path::{Path, PathBuf};

/// Find affected Rust files for a cross-crate or same-crate move
///
/// This handles Rust's crate-qualified imports (e.g., "use common::utils::foo")
/// which cannot be resolved by generic import path resolution.
///
/// Returns a list of files that import from the old module path.
pub async fn find_rust_affected_files(
    old_path: &Path,
    new_path: &Path,
    project_root: &Path,
    project_files: &[PathBuf],
) -> Vec<PathBuf> {
    let mut affected = Vec::new();

    tracing::info!(
        project_root = %project_root.display(),
        old_path = %old_path.display(),
        new_path = %new_path.display(),
        "Starting Rust cross-crate detection"
    );

    // Canonicalize paths to handle symlinks (e.g., /var vs /private/var on macOS)
    let canonical_project = project_root.canonicalize().unwrap_or_else(|e| {
        tracing::warn!(
            error = %e,
            project_root = %project_root.display(),
            "Failed to canonicalize project_root"
        );
        project_root.to_path_buf()
    });

    let canonical_old = old_path.canonicalize().unwrap_or_else(|e| {
        tracing::warn!(
            error = %e,
            old_path = %old_path.display(),
            "Failed to canonicalize old_path"
        );
        old_path.to_path_buf()
    });

    let canonical_new = new_path.canonicalize().unwrap_or_else(|e| {
        tracing::warn!(
            error = %e,
            new_path = %new_path.display(),
            "Failed to canonicalize new_path"
        );
        new_path.to_path_buf()
    });

    tracing::debug!(
        canonical_project = %canonical_project.display(),
        canonical_old = %canonical_old.display(),
        canonical_new = %canonical_new.display(),
        "Canonicalized paths"
    );

    // Extract crate names from relative paths
    let old_crate_name = canonical_old
        .strip_prefix(&canonical_project)
        .ok()
        .and_then(|rel| {
            tracing::debug!(
                relative_old = %rel.display(),
                "Stripped old_path to relative"
            );
            rel.components().next()
        })
        .and_then(|c| {
            tracing::debug!(
                first_component = ?c,
                "Extracted first component from old_path"
            );
            c.as_os_str().to_str()
        })
        .map(String::from);

    let new_crate_name = canonical_new
        .strip_prefix(&canonical_project)
        .ok()
        .and_then(|rel| {
            tracing::debug!(
                relative_new = %rel.display(),
                "Stripped new_path to relative"
            );
            rel.components().next()
        })
        .and_then(|c| {
            tracing::debug!(
                first_component = ?c,
                "Extracted first component from new_path"
            );
            c.as_os_str().to_str()
        })
        .map(String::from);

    tracing::info!(
        old_crate = ?old_crate_name,
        new_crate = ?new_crate_name,
        "Extracted crate names from paths"
    );

    // Fallback to finding crate name from Cargo.toml if path extraction failed
    let old_crate_name = old_crate_name.or_else(|| {
        tracing::info!(
            old_path = %old_path.display(),
            "Path extraction failed for old_path, trying Cargo.toml fallback"
        );
        find_crate_name_from_cargo_toml(old_path)
    });

    let new_crate_name = new_crate_name.or_else(|| {
        tracing::info!(
            new_path = %new_path.display(),
            "Path extraction failed for new_path, trying Cargo.toml fallback"
        );
        find_crate_name_from_cargo_toml(new_path)
    });

    // ALWAYS check for parent files with mod declarations
    // This is independent of crate name detection and handles simple file renames
    let old_parent = old_path.parent();
    if let Some(parent_dir) = old_parent {
        let lib_rs = parent_dir.join("lib.rs");
        let mod_rs = parent_dir.join("mod.rs");

        // Extract module name from old file (e.g., utils.rs -> utils)
        if let Some(old_module_name) = old_path.file_stem().and_then(|s| s.to_str()) {
            // Check lib.rs
            if lib_rs.exists() {
                if let Ok(content) = tokio::fs::read_to_string(&lib_rs).await {
                    // Check for mod declaration (e.g., "pub mod utils;" or "mod utils;")
                    let has_mod_declaration = content.lines().any(|line| {
                        let trimmed = line.trim();
                        (trimmed.starts_with("pub mod ") || trimmed.starts_with("mod "))
                            && trimmed.contains(&format!("{};", old_module_name))
                    });

                    if has_mod_declaration {
                        tracing::debug!(
                            file = %lib_rs.display(),
                            module_name = %old_module_name,
                            "Found parent lib.rs with mod declaration"
                        );
                        // Canonicalize to match project_files format
                        let canonical_lib_rs = lib_rs.canonicalize().unwrap_or(lib_rs);
                        if !affected.contains(&canonical_lib_rs) {
                            affected.push(canonical_lib_rs);
                        }
                    }
                }
            }

            // Check mod.rs
            if mod_rs.exists() {
                if let Ok(content) = tokio::fs::read_to_string(&mod_rs).await {
                    // Check for mod declaration
                    let has_mod_declaration = content.lines().any(|line| {
                        let trimmed = line.trim();
                        (trimmed.starts_with("pub mod ") || trimmed.starts_with("mod "))
                            && trimmed.contains(&format!("{};", old_module_name))
                    });

                    if has_mod_declaration {
                        tracing::debug!(
                            file = %mod_rs.display(),
                            module_name = %old_module_name,
                            "Found parent mod.rs with mod declaration"
                        );
                        // Canonicalize to match project_files format
                        let canonical_mod_rs = mod_rs.canonicalize().unwrap_or(mod_rs);
                        if !affected.contains(&canonical_mod_rs) {
                            affected.push(canonical_mod_rs);
                        }
                    }
                }
            }
        }
    }

    // If this is a file move (cross-crate or same-crate), compute full module paths
    if let (Some(old_crate), Some(new_crate)) = (old_crate_name, new_crate_name) {
        tracing::info!(
            old_crate = %old_crate,
            new_crate = %new_crate,
            "Both crate names extracted successfully"
        );

        // Always compute full module paths including file structure
        // This allows us to detect moves within the same crate
        let old_module_path =
            compute_module_path_from_file(old_path, &old_crate, &canonical_project);
        let new_module_path =
            compute_module_path_from_file(new_path, &new_crate, &canonical_project);

        tracing::info!(
            old_module_path = %old_module_path,
            new_module_path = %new_module_path,
            "Computed full module paths for comparison"
        );

        // Scan if module paths differ (handles both cross-crate and same-crate moves)
        if old_module_path != new_module_path {
            tracing::info!(
                old_module_path = %old_module_path,
                new_module_path = %new_module_path,
                "Detected Rust module path change, scanning for affected files"
            );

            // Scan all Rust files for imports from the old module path
            let module_pattern = format!("{}::", old_module_path);
            for file in project_files {
                if file == old_path || file == new_path {
                    continue;
                }

                // Only check Rust files
                if file.extension().and_then(|e| e.to_str()) != Some("rs") {
                    continue;
                }

                if let Ok(content) = tokio::fs::read_to_string(file).await {
                    // Check if this file has imports from the old module path
                    // Need to check both absolute paths (e.g., "mylib::core::types")
                    // and crate:: paths (e.g., "crate::core::types")
                    let has_module_import = content.lines().any(|line| {
                        let trimmed = line.trim();
                        if !trimmed.starts_with("use ") {
                            return false;
                        }

                        // Check for absolute module path
                        if trimmed.contains(&module_pattern) {
                            return true;
                        }

                        // Check for crate:: prefixed imports
                        // Extract the suffix after the crate name from old_module_path
                        // e.g., "mylib::core::types" → "core::types"
                        if let Some((_crate_name, suffix)) = old_module_path.split_once("::") {
                            let crate_pattern = format!("crate::{}::", suffix);
                            if trimmed.contains(&crate_pattern) {
                                return true;
                            }
                        }

                        false
                    });

                    if has_module_import {
                        tracing::debug!(
                            file = %file.display(),
                            old_module_path = %old_module_path,
                            "Found Rust file importing from old module path"
                        );
                        if !affected.contains(file) {
                            affected.push(file.clone());
                        }
                    }
                }
            }

            tracing::info!(
                affected_count = affected.len(),
                "Found Rust files affected by module path change"
            );
        } else {
            tracing::info!("Module paths are identical - no affected files");
        }
    }

    affected
}
