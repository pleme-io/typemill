//! Rust-specific reference detection
//!
//! Handles detection of affected files for Rust cross-crate and same-crate moves.
//! Uses module path analysis to detect imports that need updating.

use async_trait::async_trait;
use mill_plugin_api::ReferenceDetector;
use std::path::{Path, PathBuf};
use tokio::task::JoinSet;

use crate::imports::{compute_module_path_from_file, find_crate_name_from_cargo_toml};

/// Rust reference detector implementation
#[derive(Default)]
pub struct RustReferenceDetector;

impl RustReferenceDetector {
    /// Creates a new Rust reference detector instance.
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ReferenceDetector for RustReferenceDetector {
    /// Find affected Rust files for a cross-crate or same-crate move
    ///
    /// This handles Rust's crate-qualified imports (e.g., "use common::utils::foo")
    /// which cannot be resolved by generic import path resolution.
    ///
    /// Returns a list of files that import from the old module path.
    async fn find_affected_files(
        &self,
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
            old_is_dir = old_path.is_dir(),
            "Starting Rust cross-crate detection"
        );

        // Canonicalize paths to handle symlinks (e.g., /var vs /private/var on macOS)
        // Use tokio::fs for async canonicalization
        let canonical_project = tokio::fs::canonicalize(project_root).await.unwrap_or_else(|e| {
            tracing::warn!(
                error = %e,
                project_root = %project_root.display(),
                "Failed to canonicalize project_root"
            );
            project_root.to_path_buf()
        });

        let canonical_old = tokio::fs::canonicalize(old_path).await.unwrap_or_else(|e| {
            tracing::warn!(
                error = %e,
                old_path = %old_path.display(),
                "Failed to canonicalize old_path"
            );
            old_path.to_path_buf()
        });

        let canonical_new = tokio::fs::canonicalize(new_path).await.unwrap_or_else(|e| {
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

        // ALWAYS use Cargo.toml to find crate names (more reliable than path inspection)
        // This correctly handles workspace projects where files are in subdirectories
        // For directories, check for Cargo.toml directly in the directory
        let old_crate_name = if old_path.is_dir() {
            let cargo_toml = old_path.join("Cargo.toml");
            if cargo_toml.exists() {
                if let Ok(content) = tokio::fs::read_to_string(&cargo_toml).await {
                    content
                        .lines()
                        .find(|line| {
                            let trimmed = line.trim();
                            trimmed.starts_with("name") && trimmed.contains('=')
                        })
                        .and_then(|line| line.split('=').nth(1))
                        .map(|name_part| {
                            let name = name_part.trim().trim_matches('"').trim_matches('\'');
                            // Normalize: Cargo.toml uses hyphens, but imports use underscores
                            name.replace('-', "_")
                        })
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            find_crate_name_from_cargo_toml(old_path).map(|name| name.replace('-', "_"))
        };

        // For new_path, it might not exist yet (during rename), so derive from directory name
        let new_crate_name = if new_path.exists() && new_path.is_dir() {
            let cargo_toml = new_path.join("Cargo.toml");
            if cargo_toml.exists() {
                if let Ok(content) = tokio::fs::read_to_string(&cargo_toml).await {
                    content
                        .lines()
                        .find(|line| {
                            let trimmed = line.trim();
                            trimmed.starts_with("name") && trimmed.contains('=')
                        })
                        .and_then(|line| line.split('=').nth(1))
                        .map(|name_part| {
                            let name = name_part.trim().trim_matches('"').trim_matches('\'');
                            name.replace('-', "_")
                        })
                } else {
                    // Fallback to directory name
                    new_path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .map(|s| s.replace('-', "_"))
                }
            } else {
                // No Cargo.toml, use directory name
                new_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|s| s.replace('-', "_"))
            }
        } else if new_path.exists() {
            // It's a file path
            find_crate_name_from_cargo_toml(new_path).map(|name| name.replace('-', "_"))
        } else {
            // Path doesn't exist yet - for files, try to find Cargo.toml by traversing parent directories
            // Start from the parent directory since new_path doesn't exist yet
            if let Some(parent) = new_path.parent() {
                // Try to find Cargo.toml starting from parent directory
                let mut current = parent;
                loop {
                    let cargo_toml = current.join("Cargo.toml");
                    if cargo_toml.exists() {
                        if let Ok(content) = tokio::fs::read_to_string(&cargo_toml).await {
                            let crate_name = content
                                .lines()
                                .find(|line| {
                                    let trimmed = line.trim();
                                    trimmed.starts_with("name") && trimmed.contains('=')
                                })
                                .and_then(|line| line.split('=').nth(1))
                                .map(|name_part| {
                                    let name =
                                        name_part.trim().trim_matches('"').trim_matches('\'');
                                    name.replace('-', "_")
                                });
                            if crate_name.is_some() {
                                break crate_name;
                            }
                        }
                    }
                    // Move up to parent directory
                    if let Some(parent_dir) = current.parent() {
                        current = parent_dir;
                    } else {
                        // Reached root without finding Cargo.toml
                        // Final fallback: use project root directory name as crate name
                        break project_root
                            .file_name()
                            .and_then(|n| n.to_str())
                            .map(|s| s.replace('-', "_"));
                    }
                }
            } else {
                None
            }
        };

        tracing::info!(
            old_crate = ?old_crate_name,
            new_crate = ?new_crate_name,
            old_path = %old_path.display(),
            new_path = %new_path.display(),
            "Found crate names from Cargo.toml"
        );

        // Special case: If old_path is a directory (crate rename), scan for any use of the crate name
        if old_path.is_dir() {
            if let (Some(ref old_crate), Some(ref new_crate)) = (&old_crate_name, &new_crate_name) {
                if old_crate != new_crate {
                    tracing::info!(
                        old_crate = %old_crate,
                        new_crate = %new_crate,
                        "Detected crate rename (directory rename), scanning for crate-level imports"
                    );

                    // For crate renames, scan ALL Rust files for imports from the old crate
                    // Parallelize scanning using JoinSet
                    let crate_import_pattern = format!("use {}::", old_crate);
                    let mut set = JoinSet::new();

                    for file in project_files {
                        // Skip files inside the renamed crate itself
                        if file.starts_with(old_path) {
                            continue;
                        }

                        // Only check Rust files
                        if file.extension().and_then(|e| e.to_str()) != Some("rs") {
                            continue;
                        }

                        let file_path = file.clone();
                        let pattern = crate_import_pattern.clone();
                        let old_crate_ref = old_crate.clone();

                        set.spawn(async move {
                            if let Ok(content) = tokio::fs::read_to_string(&file_path).await {
                                if content.contains(&pattern) {
                                    tracing::debug!(
                                        file = %file_path.display(),
                                        old_crate = %old_crate_ref,
                                        "Found file importing from old crate"
                                    );
                                    return Some(file_path);
                                }
                            }
                            None
                        });
                    }

                    while let Some(res) = set.join_next().await {
                        if let Ok(Some(file)) = res {
                            if !affected.contains(&file) {
                                affected.push(file);
                            }
                        }
                    }

                    tracing::info!(
                        affected_count = affected.len(),
                        affected_files = ?affected.iter().map(|p| p.display().to_string()).collect::<Vec<_>>(),
                        "Found Rust files affected by crate rename"
                    );

                    return affected;
                }
            }
        }

        // Check for mod declarations for file renames
        // For same-directory renames, files MUST be in the same crate (you can't have
        // files in the same directory belonging to different crates), so we don't need
        // to check crate names at all.
        if !old_path.is_dir() {
            let old_parent = old_path.parent();
            let new_parent = new_path.parent();

            // For same-directory renames, ALWAYS check for mod declarations
            // This works even without Cargo.toml (plain Rust projects)
            if old_parent == new_parent {
                if let Some(parent_dir) = old_parent {
                    let lib_rs = parent_dir.join("lib.rs");
                    let mod_rs = parent_dir.join("mod.rs");

                    // Extract module names from old and new files
                    if let (Some(old_module_name), Some(new_module_name)) = (
                        old_path.file_stem().and_then(|s| s.to_str()),
                        new_path.file_stem().and_then(|s| s.to_str()),
                    ) {
                        // Check lib.rs
                        if lib_rs.exists() {
                            if let Ok(content) = tokio::fs::read_to_string(&lib_rs).await {
                                // Check for mod declaration for old module name
                                let has_old_mod_declaration = content.lines().any(|line| {
                                    let trimmed = line.trim();
                                    (trimmed.starts_with("pub mod ") || trimmed.starts_with("mod "))
                                        && trimmed.contains(&format!("{};", old_module_name))
                                });

                                // Check if new module name is already declared
                                let has_new_mod_declaration = content.lines().any(|line| {
                                    let trimmed = line.trim();
                                    (trimmed.starts_with("pub mod ") || trimmed.starts_with("mod "))
                                        && trimmed.contains(&format!("{};", new_module_name))
                                });

                                // Only add as affected if old module is declared AND new module is NOT already declared
                                if has_old_mod_declaration && !has_new_mod_declaration {
                                    tracing::debug!(
                                        file = %lib_rs.display(),
                                        old_module = %old_module_name,
                                        new_module = %new_module_name,
                                        "Found parent lib.rs with mod declaration that needs updating"
                                    );
                                    let canonical_lib_rs = tokio::fs::canonicalize(&lib_rs).await.unwrap_or(lib_rs);
                                    if !affected.contains(&canonical_lib_rs) {
                                        affected.push(canonical_lib_rs);
                                    }
                                } else if has_new_mod_declaration {
                                    tracing::debug!(
                                        file = %lib_rs.display(),
                                        old_module = %old_module_name,
                                        new_module = %new_module_name,
                                        "Skipping lib.rs - new module name already declared"
                                    );
                                }
                            }
                        }

                        // Check mod.rs
                        if mod_rs.exists() {
                            if let Ok(content) = tokio::fs::read_to_string(&mod_rs).await {
                                let has_old_mod_declaration = content.lines().any(|line| {
                                    let trimmed = line.trim();
                                    (trimmed.starts_with("pub mod ") || trimmed.starts_with("mod "))
                                        && trimmed.contains(&format!("{};", old_module_name))
                                });

                                let has_new_mod_declaration = content.lines().any(|line| {
                                    let trimmed = line.trim();
                                    (trimmed.starts_with("pub mod ") || trimmed.starts_with("mod "))
                                        && trimmed.contains(&format!("{};", new_module_name))
                                });

                                if has_old_mod_declaration && !has_new_mod_declaration {
                                    tracing::debug!(
                                        file = %mod_rs.display(),
                                        old_module = %old_module_name,
                                        new_module = %new_module_name,
                                        "Found parent mod.rs with mod declaration that needs updating"
                                    );
                                    let canonical_mod_rs = tokio::fs::canonicalize(&mod_rs).await.unwrap_or(mod_rs);
                                    if !affected.contains(&canonical_mod_rs) {
                                        affected.push(canonical_mod_rs);
                                    }
                                } else if has_new_mod_declaration {
                                    tracing::debug!(
                                        file = %mod_rs.display(),
                                        old_module = %old_module_name,
                                        new_module = %new_module_name,
                                        "Skipping mod.rs - new module name already declared"
                                    );
                                }
                            }
                        }
                    }
                }
            } else {
                // old_parent != new_parent - file moved to different directory
                // Don't update mod declarations for cross-directory moves
                tracing::debug!(
                    old_parent = ?old_parent.map(|p| p.display().to_string()),
                    new_parent = ?new_parent.map(|p| p.display().to_string()),
                    "Skipping mod declaration detection - parent directories differ (file moved to different directory)"
                );
            }
        } else {
            tracing::debug!("Skipping mod declaration detection for directory rename - handled by reference_updater");
        }

        // If this is a file move with crate info, compute full module paths
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
                let mut set = JoinSet::new();

                for file in project_files {
                    if file == old_path || file == new_path {
                        continue;
                    }

                    // Only check Rust files
                    if file.extension().and_then(|e| e.to_str()) != Some("rs") {
                        continue;
                    }

                    let file_path = file.clone();
                    let module_pattern = module_pattern.clone();
                    let old_module_path = old_module_path.clone();

                    set.spawn(async move {
                        if let Ok(content) = tokio::fs::read_to_string(&file_path).await {
                            // Check if this file has imports from the old module path
                            // Need to check:
                            // 1. Absolute paths (e.g., "mylib::core::types")
                            // 2. crate:: paths (e.g., "crate::core::types")
                            // 3. Crate-relative paths (e.g., "utils::helpers" from lib.rs)
                            // 4. Relative paths (e.g., "super::common", "self::common")
                            let has_module_import = content.lines().any(|line| {
                                let trimmed = line.trim();
                                if !trimmed.starts_with("use ") {
                                    return false;
                                }

                                // Check for absolute module path (e.g., "use mylib::utils::helpers::process")
                                if trimmed.contains(&module_pattern) {
                                    return true;
                                }

                                // Extract the last component of the module path (the module name being renamed)
                                // e.g., "mylib::handlers::refactor::common" → "common"
                                let old_module_name = old_module_path.split("::").last().unwrap_or("");

                                // Check for relative imports like "use super::common::" or "use self::common::"
                                if !old_module_name.is_empty() {
                                    let super_pattern = format!("super::{}::", old_module_name);
                                    let self_pattern = format!("self::{}::", old_module_name);
                                    let super_glob = format!("super::{}::*", old_module_name);
                                    let self_glob = format!("self::{}::*", old_module_name);

                                    if trimmed.contains(&super_pattern)
                                        || trimmed.contains(&self_pattern)
                                        || trimmed.contains(&super_glob)
                                        || trimmed.contains(&self_glob)
                                    {
                                        return true;
                                    }
                                }

                                // Check for crate:: prefixed imports (e.g., "use crate::utils::helpers::process")
                                // Extract the suffix after the crate name from old_module_path
                                // e.g., "mylib::core::types" → "core::types"
                                if let Some((_crate_name, suffix)) = old_module_path.split_once("::") {
                                    let crate_pattern = format!("crate::{}::", suffix);
                                    if trimmed.contains(&crate_pattern) {
                                        return true;
                                    }

                                    // Check for crate-relative imports (e.g., "use utils::helpers::process" from lib.rs)
                                    // This matches when the use statement starts with the suffix
                                    // e.g., suffix="utils::helpers" matches "use utils::helpers::process"
                                    let relative_pattern = format!("use {}::", suffix);
                                    if trimmed.starts_with(&relative_pattern) {
                                        return true;
                                    }
                                }

                                false
                            });

                            if has_module_import {
                                tracing::debug!(
                                    file = %file_path.display(),
                                    old_module_path = %old_module_path,
                                    "Found Rust file importing from old module path"
                                );
                                return Some(file_path);
                            }
                        }
                        None
                    });
                }

                while let Some(res) = set.join_next().await {
                     if let Ok(Some(file)) = res {
                        if !affected.contains(&file) {
                            affected.push(file);
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_crate_directory_rename_detection() {
        // Setup: Create a workspace with two crates
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();

        // Create old_crate with Cargo.toml
        tokio::fs::create_dir_all(project_root.join("old_crate/src"))
            .await
            .unwrap();
        tokio::fs::write(
            project_root.join("old_crate/Cargo.toml"),
            "[package]\nname = \"old_crate\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
        )
        .await
        .unwrap();
        tokio::fs::write(
            project_root.join("old_crate/src/lib.rs"),
            "pub fn utility() {}\n",
        )
        .await
        .unwrap();

        // Create app crate that imports from old_crate
        tokio::fs::create_dir_all(project_root.join("app/src"))
            .await
            .unwrap();
        tokio::fs::write(
            project_root.join("app/Cargo.toml"),
            "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
        )
        .await
        .unwrap();
        tokio::fs::write(
            project_root.join("app/src/main.rs"),
            "use old_crate::utility;\n\nfn main() {\n    utility();\n}\n",
        )
        .await
        .unwrap();

        // Define paths
        let old_path = project_root.join("old_crate");
        let new_path = project_root.join("new_crate");

        // Project files list
        let project_files = vec![
            project_root.join("old_crate/src/lib.rs"),
            project_root.join("app/src/main.rs"),
        ];

        // Test: Run the detector
        let detector = RustReferenceDetector::new();
        let affected = detector
            .find_affected_files(&old_path, &new_path, project_root, &project_files)
            .await;

        // Verify: app/src/main.rs should be in the affected files
        assert!(
            affected.contains(&project_root.join("app/src/main.rs")),
            "app/src/main.rs should be detected as affected (imports from old_crate). Affected files: {:?}",
            affected
        );
    }

    #[tokio::test]
    async fn test_crate_relative_import_detection() {
        // Setup: Create a temporary directory with a Rust project structure
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();

        // Create Cargo.toml
        tokio::fs::write(
            project_root.join("Cargo.toml"),
            "[package]\nname = \"test_project\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
        )
        .await
        .unwrap();

        // Create src directory
        tokio::fs::create_dir_all(project_root.join("src/utils"))
            .await
            .unwrap();

        // Create lib.rs with crate-relative import
        tokio::fs::write(
            project_root.join("src/lib.rs"),
            "pub mod utils;\n\nuse utils::helpers::process;\n\npub fn lib_fn() {\n    process();\n}\n",
        )
        .await
        .unwrap();

        // Create utils/mod.rs
        tokio::fs::write(
            project_root.join("src/utils/mod.rs"),
            "pub mod helpers;\n\npub fn utils_fn() {\n    helpers::process();\n}\n",
        )
        .await
        .unwrap();

        // Create utils/helpers.rs (the file we're renaming)
        tokio::fs::write(
            project_root.join("src/utils/helpers.rs"),
            "pub fn process() {}\n",
        )
        .await
        .unwrap();

        // Define paths
        let old_path = project_root.join("src/utils/helpers.rs");
        let new_path = project_root.join("src/utils/support.rs");

        // Project files list
        let project_files = vec![
            project_root.join("src/lib.rs"),
            project_root.join("src/utils/mod.rs"),
            project_root.join("src/utils/helpers.rs"),
        ];

        // Test: Run the detector
        let detector = RustReferenceDetector::new();
        let affected = detector
            .find_affected_files(&old_path, &new_path, project_root, &project_files)
            .await;

        // Verify: lib.rs should be in the affected files
        let lib_rs = project_root.join("src/lib.rs");
        let canonical_lib_rs = tokio::fs::canonicalize(&lib_rs).await.unwrap_or(lib_rs.clone());
        assert!(
            affected.contains(&canonical_lib_rs),
            "lib.rs should be detected as affected (has crate-relative import). Affected files: {:?}",
            affected
        );

        // Verify: utils/mod.rs should also be affected (parent file)
        let mod_rs = project_root.join("src/utils/mod.rs");
        let canonical_mod_rs = tokio::fs::canonicalize(&mod_rs).await.unwrap_or(mod_rs.clone());
        assert!(
            affected.contains(&canonical_mod_rs),
            "utils/mod.rs should be detected as affected (parent file). Affected files: {:?}",
            affected
        );
    }
}
