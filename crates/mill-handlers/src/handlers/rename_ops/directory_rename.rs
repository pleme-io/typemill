use super::{RenameOptions, RenameService, RenameTarget};
use crate::handlers::common::{calculate_checksums_for_directory_rename, lsp_mode};
use crate::handlers::tools::extensions::get_concrete_app_state;
use mill_foundation::errors::MillResult as ServerResult;
use mill_foundation::planning::{PlanMetadata, PlanSummary, PlanWarning, RenamePlan};
use std::path::{Path, PathBuf};
use tracing::{debug, info};

/// Detected package type for consolidation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageType {
    Cargo,
    Npm,
    Python,
}

impl RenameService {
    /// Helper to find the crate root (parent of src/) for a path
    async fn find_target_crate_root(path: &Path) -> Option<PathBuf> {
        for p in path.ancestors() {
            if p.file_name().and_then(|n| n.to_str()) == Some("src") {
                if let Some(parent) = p.parent() {
                    if tokio::fs::try_exists(parent.join("Cargo.toml"))
                        .await
                        .unwrap_or(false)
                    {
                        return Some(parent.to_path_buf());
                    }
                }
            }
        }
        None
    }

    /// Helper to find the npm package root (parent of src/) for a path
    async fn find_target_npm_root(path: &Path) -> Option<PathBuf> {
        for p in path.ancestors() {
            if p.file_name().and_then(|n| n.to_str()) == Some("src") {
                if let Some(parent) = p.parent() {
                    if tokio::fs::try_exists(parent.join("package.json"))
                        .await
                        .unwrap_or(false)
                    {
                        return Some(parent.to_path_buf());
                    }
                }
            }
        }
        None
    }

    /// Helper to find the Python package root (parent of src/) for a path
    async fn find_target_python_root(path: &Path) -> Option<PathBuf> {
        for p in path.ancestors() {
            if p.file_name().and_then(|n| n.to_str()) == Some("src") {
                if let Some(parent) = p.parent() {
                    if tokio::fs::try_exists(parent.join("pyproject.toml"))
                        .await
                        .unwrap_or(false)
                    {
                        return Some(parent.to_path_buf());
                    }
                }
            }
        }
        None
    }

    /// Auto-detect if this is a consolidation move and what package type
    ///
    /// Detects when moving a package into another package's src/ directory.
    /// Supports: Rust (Cargo.toml), TypeScript/JS (package.json), Python (pyproject.toml)
    async fn detect_consolidation_type(old_path: &Path, new_path: &Path) -> Option<PackageType> {
        // Check for Rust/Cargo consolidation
        let has_cargo = tokio::fs::try_exists(old_path.join("Cargo.toml"))
            .await
            .unwrap_or(false);
        if has_cargo && Self::find_target_crate_root(new_path).await.is_some() {
            return Some(PackageType::Cargo);
        }

        // Check for npm/TypeScript consolidation
        let has_package_json = tokio::fs::try_exists(old_path.join("package.json"))
            .await
            .unwrap_or(false);
        if has_package_json && Self::find_target_npm_root(new_path).await.is_some() {
            return Some(PackageType::Npm);
        }

        // Check for Python consolidation
        let has_pyproject = tokio::fs::try_exists(old_path.join("pyproject.toml"))
            .await
            .unwrap_or(false);
        if has_pyproject && Self::find_target_python_root(new_path).await.is_some() {
            return Some(PackageType::Python);
        }

        None
    }

    /// Generate plan for directory rename using FileService
    pub(crate) async fn plan_directory_rename(
        &self,
        target: &RenameTarget,
        new_name: &str,
        options: &RenameOptions,
        context: &mill_handler_api::ToolHandlerContext,
    ) -> ServerResult<RenamePlan> {
        debug!(
            old_path = %target.path,
            new_path = %new_name,
            "Planning directory rename"
        );

        // Resolve paths against workspace root, not CWD
        let workspace_root = &context.app_state.project_root;
        let old_path = if Path::new(&target.path).is_absolute() {
            Path::new(&target.path).to_path_buf()
        } else {
            workspace_root.join(&target.path)
        };
        let new_path = if Path::new(new_name).is_absolute() {
            Path::new(new_name).to_path_buf()
        } else {
            workspace_root.join(new_name)
        };

        // Determine if this is a consolidation and what package type
        let consolidation_type = if let Some(true) = options.consolidate {
            // Explicit consolidation flag, detect type
            Self::detect_consolidation_type(&old_path, &new_path).await
        } else if options.consolidate == Some(false) {
            None
        } else {
            // Auto-detect
            Self::detect_consolidation_type(&old_path, &new_path).await
        };

        let is_consolidation = consolidation_type.is_some();

        if let Some(pkg_type) = consolidation_type {
            let manifest_name = match pkg_type {
                PackageType::Cargo => "Cargo.toml",
                PackageType::Npm => "package.json",
                PackageType::Python => "pyproject.toml",
            };

            info!(
                old_path = %old_path.display(),
                new_path = %new_path.display(),
                package_type = ?pkg_type,
                "Detected consolidation move - will merge {} and update imports",
                manifest_name
            );

            // For Cargo consolidation, validate circular dependencies
            if pkg_type == PackageType::Cargo {
                // Validate that consolidation won't create circular dependencies
                // Find target crate root (the parent of src/ directory)
                let target_crate_root =
                    Self::find_target_crate_root(&new_path)
                        .await
                        .ok_or_else(|| mill_foundation::errors::MillError::InvalidRequest {
                            message: "Could not find target crate root for consolidation"
                                .to_string(),
                            parameter: Some("newName".to_string()),
                        })?;

                // Validate circular dependencies using Rust-specific analysis
                debug!(
                    source = %old_path.display(),
                    target = %target_crate_root.display(),
                    "Validating consolidation for circular dependencies"
                );

                #[cfg(feature = "lang-rust")]
                {
                    use mill_lang_rust::dependency_analysis::validate_no_circular_dependencies;

                    match validate_no_circular_dependencies(
                        &old_path,
                        &target_crate_root,
                        workspace_root,
                    )
                    .await
                    {
                        // Only reject if there are ACTUAL problematic modules that would create circular imports.
                        // It's normal for target to depend on source (e.g., app → lib) during consolidation.
                        // The key question is: are there specific modules in source that would create
                        // circular imports after being merged into target? If problematic_modules is empty,
                        // the consolidation is safe.
                        Ok(analysis)
                            if analysis.has_circular_dependency
                                && !analysis.problematic_modules.is_empty() =>
                        {
                            return Err(mill_foundation::errors::MillError::InvalidRequest {
                                message: format!(
                                "Cannot consolidate {} into {}: would create circular dependency.\n\
                                 Dependency chain: {}\n\
                                 Problematic modules: {}",
                                analysis.source_crate,
                                analysis.target_crate,
                                analysis.dependency_chain.join(" → "),
                                analysis.problematic_modules.len()
                            ),
                                parameter: Some("target".to_string()),
                            });
                        }
                        Ok(_) => {
                            info!("Circular dependency validation passed");
                        }
                        Err(e) => {
                            // Log validation error but don't fail the plan
                            // This allows consolidation to proceed if validation itself fails
                            tracing::warn!(
                                error = %e,
                                "Failed to validate circular dependencies, proceeding anyway"
                            );
                        }
                    }
                }

                #[cfg(not(feature = "lang-rust"))]
                {
                    // Rust language support not compiled in, skip validation
                    debug!(
                    "Rust language support not available, skipping circular dependency validation"
                );
                }
            } // end if pkg_type == PackageType::Cargo
        } // end if let Some(pkg_type) = consolidation_type

        // Get scope configuration from options
        let mut rename_scope = options.to_rename_scope();

        // For consolidation moves, exclude manifest files from generic path updates
        // The semantic changes (merging dependencies, updating workspace members)
        // are handled during execution, not in the plan
        if let Some(pkg_type) = consolidation_type {
            match pkg_type {
                PackageType::Cargo => {
                    rename_scope
                        .exclude_patterns
                        .push("**/Cargo.toml".to_string());
                }
                PackageType::Npm => {
                    rename_scope
                        .exclude_patterns
                        .push("**/package.json".to_string());
                }
                PackageType::Python => {
                    rename_scope
                        .exclude_patterns
                        .push("**/pyproject.toml".to_string());
                }
            }
        }

        // Get concrete AppState to access move_service()
        let concrete_state = get_concrete_app_state(&context.app_state)?;

        // Get LSP import finder from context (uses workspace/willRenameFiles for correct import detection)
        // The finder may return empty if the LSP doesn't support willRenameFiles, in which case
        // the plugin-based scanner (TypeScriptReferenceDetector) is used as a fallback.
        let lsp_adapter_guard = context.lsp_adapter.lock().await;
        let lsp_finder: Option<&dyn mill_services::services::reference_updater::LspImportFinder> =
            if lsp_mode(context) == mill_config::config::LspMode::Off {
                None
            } else {
                lsp_adapter_guard
                    .as_ref()
                    .map(|adapter| adapter.as_import_finder())
            };

        // Get the EditPlan with import updates (call MoveService directly)
        let edit_plan = concrete_state
            .move_service()
            .plan_directory_move_with_scope(&old_path, &new_path, Some(&rename_scope), lsp_finder)
            .await?;

        debug!(
            edits_count = edit_plan.edits.len(),
            "Got EditPlan with text edits for import updates"
        );

        // Calculate files_to_move by walking the directory
        let mut files_to_move = 0;
        let walker = ignore::WalkBuilder::new(&old_path).hidden(false).build();
        for entry in walker.flatten() {
            if entry.path().is_file() {
                files_to_move += 1;
            }
        }

        // Check if this is a Cargo package
        let is_cargo_package = tokio::fs::try_exists(old_path.join("Cargo.toml"))
            .await
            .unwrap_or(false);

        // For directory rename, we need to calculate checksums for all files being moved
        // Paths are already resolved against workspace root, so canonicalize directly
        let abs_old = tokio::fs::canonicalize(&old_path)
            .await
            .unwrap_or_else(|_| old_path.clone());

        // Calculate abs_new early so we can use it for checksum fallback logic
        // new_path is already resolved against workspace root or is absolute
        let abs_new = if tokio::fs::try_exists(&new_path).await.unwrap_or(false) {
            tokio::fs::canonicalize(&new_path)
                .await
                .unwrap_or_else(|_| new_path.clone())
        } else {
            // For non-existent paths, canonicalize parent and join filename
            let parent = new_path.parent().unwrap_or(workspace_root);
            let parent_abs = tokio::fs::canonicalize(parent)
                .await
                .unwrap_or_else(|_| parent.to_path_buf());
            parent_abs.join(new_path.file_name().unwrap_or(new_path.as_os_str()))
        };

        // Calculate checksums for all affected files using shared utility
        // IMPORTANT: Checksums are stored with paths at the OLD/CURRENT location.
        // Validation happens BEFORE the rename, so files exist at their old location.
        let file_checksums =
            calculate_checksums_for_directory_rename(&abs_old, &edit_plan.edits, context).await?;

        // Use shared converter to create WorkspaceEdit from EditPlan
        let workspace_edit =
            super::plan_converter::editplan_to_workspace_edit(&edit_plan, &abs_old, &abs_new)?;

        // Build summary
        let summary = PlanSummary {
            affected_files: files_to_move,
            created_files: files_to_move,
            deleted_files: files_to_move,
        };

        // Add warning if this is a package
        let mut warnings = Vec::new();
        if let Some(pkg_type) = consolidation_type {
            let (code, message) = match pkg_type {
                PackageType::Cargo => (
                    "CARGO_PACKAGE_RENAME",
                    "Renaming a Cargo package will update workspace members and dependencies",
                ),
                PackageType::Npm => (
                    "NPM_PACKAGE_RENAME",
                    "Renaming an npm package will update workspace members and dependencies",
                ),
                PackageType::Python => (
                    "PYTHON_PACKAGE_RENAME",
                    "Renaming a Python package will update workspace members and dependencies",
                ),
            };
            warnings.push(PlanWarning {
                code: code.to_string(),
                message: message.to_string(),
                candidates: None,
            });
        } else if is_cargo_package {
            warnings.push(PlanWarning {
                code: "CARGO_PACKAGE_RENAME".to_string(),
                message: "Renaming a Cargo package will update workspace members and dependencies"
                    .to_string(),
                candidates: None,
            });
        }

        // Add consolidation-specific warning
        if let Some(pkg_type) = consolidation_type {
            let module_name = new_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("module");

            let (target_root, manual_step) = match pkg_type {
                PackageType::Cargo => {
                    let root = Self::find_target_crate_root(&new_path)
                        .await
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|| "target crate".to_string());
                    let step = format!(
                        "After consolidation, manually add 'pub mod {};' to {}/src/lib.rs to expose the consolidated code",
                        module_name, root
                    );
                    (root, step)
                }
                PackageType::Npm => {
                    let root = Self::find_target_npm_root(&new_path)
                        .await
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|| "target package".to_string());
                    let step = format!(
                        "After consolidation, manually add 'export * from './{}'/' to {}/src/index.ts to expose the consolidated code",
                        module_name, root
                    );
                    (root, step)
                }
                PackageType::Python => {
                    let root = Self::find_target_python_root(&new_path)
                        .await
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|| "target package".to_string());
                    let step = format!(
                        "After consolidation, manually add 'from .{} import *' to {}/src/__init__.py to expose the consolidated code",
                        module_name, root
                    );
                    (root, step)
                }
            };

            warnings.push(PlanWarning {
                code: "CONSOLIDATION_MANUAL_STEP".to_string(),
                message: manual_step,
                candidates: None,
            });

            debug!(target_root = %target_root, "Consolidation target root");
        }

        // Determine language for metadata
        let language = match consolidation_type {
            Some(PackageType::Cargo) => "rust",
            Some(PackageType::Npm) => "typescript",
            Some(PackageType::Python) => "python",
            None if is_cargo_package => "rust",
            None => "unknown",
        };

        // Build metadata
        let metadata = PlanMetadata {
            plan_version: "1.0".to_string(),
            kind: "rename".to_string(),
            language: language.to_string(),
            estimated_impact: super::utils::estimate_impact(files_to_move),
            created_at: chrono::Utc::now().to_rfc3339(),
        };

        debug!(
            checksum_count = file_checksums.len(),
            "Generated file checksums for rename plan"
        );

        Ok(RenamePlan {
            edits: workspace_edit,
            summary,
            warnings,
            metadata,
            file_checksums,
            is_consolidation,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_cargo_consolidation_detection() {
        // Setup directory structure
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create source crate
        let src_crate = root.join("source_crate");
        fs::create_dir(&src_crate).unwrap();
        fs::write(src_crate.join("Cargo.toml"), "[package]").unwrap();

        // Create target crate
        let target_crate = root.join("target_crate");
        fs::create_dir(&target_crate).unwrap();
        fs::write(target_crate.join("Cargo.toml"), "[package]").unwrap();
        let target_src = target_crate.join("src");
        fs::create_dir(&target_src).unwrap();

        // Case 1: True consolidation
        let old_path = src_crate.clone();
        let new_path = target_src.join("module_name");
        let result = RenameService::detect_consolidation_type(&old_path, &new_path).await;
        assert_eq!(result, Some(PackageType::Cargo));

        // Case 2: Not consolidation (no cargo.toml in source)
        let other_dir = root.join("other_dir");
        fs::create_dir(&other_dir).unwrap();
        let result = RenameService::detect_consolidation_type(&other_dir, &new_path).await;
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn test_npm_consolidation_detection() {
        // Setup directory structure
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create source npm package
        let src_pkg = root.join("source_pkg");
        fs::create_dir(&src_pkg).unwrap();
        fs::write(src_pkg.join("package.json"), r#"{"name": "source"}"#).unwrap();

        // Create target npm package
        let target_pkg = root.join("target_pkg");
        fs::create_dir(&target_pkg).unwrap();
        fs::write(target_pkg.join("package.json"), r#"{"name": "target"}"#).unwrap();
        let target_src = target_pkg.join("src");
        fs::create_dir(&target_src).unwrap();

        // Case 1: True npm consolidation
        let old_path = src_pkg.clone();
        let new_path = target_src.join("module_name");
        let result = RenameService::detect_consolidation_type(&old_path, &new_path).await;
        assert_eq!(result, Some(PackageType::Npm));

        // Case 2: Not consolidation (no package.json in source)
        let other_dir = root.join("other_dir");
        fs::create_dir(&other_dir).unwrap();
        let result = RenameService::detect_consolidation_type(&other_dir, &new_path).await;
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn test_python_consolidation_detection() {
        // Setup directory structure
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create source Python package
        let src_pkg = root.join("source_pkg");
        fs::create_dir(&src_pkg).unwrap();
        fs::write(
            src_pkg.join("pyproject.toml"),
            r#"[project]
name = "source"
"#,
        )
        .unwrap();

        // Create target Python package
        let target_pkg = root.join("target_pkg");
        fs::create_dir(&target_pkg).unwrap();
        fs::write(
            target_pkg.join("pyproject.toml"),
            r#"[project]
name = "target"
"#,
        )
        .unwrap();
        let target_src = target_pkg.join("src");
        fs::create_dir(&target_src).unwrap();

        // Case 1: True Python consolidation
        let old_path = src_pkg.clone();
        let new_path = target_src.join("module_name");
        let result = RenameService::detect_consolidation_type(&old_path, &new_path).await;
        assert_eq!(result, Some(PackageType::Python));

        // Case 2: Not consolidation (no pyproject.toml in source)
        let other_dir = root.join("other_dir");
        fs::create_dir(&other_dir).unwrap();
        let result = RenameService::detect_consolidation_type(&other_dir, &new_path).await;
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn test_consolidation_priority() {
        // If a directory has multiple manifest files, Cargo takes priority
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create source with both Cargo.toml and package.json
        let src = root.join("source");
        fs::create_dir(&src).unwrap();
        fs::write(src.join("Cargo.toml"), "[package]").unwrap();
        fs::write(src.join("package.json"), r#"{"name": "source"}"#).unwrap();

        // Create Cargo target
        let target_cargo = root.join("target_cargo");
        fs::create_dir(&target_cargo).unwrap();
        fs::write(target_cargo.join("Cargo.toml"), "[package]").unwrap();
        let target_src = target_cargo.join("src");
        fs::create_dir(&target_src).unwrap();

        // Should detect as Cargo consolidation (takes priority)
        let new_path = target_src.join("module");
        let result = RenameService::detect_consolidation_type(&src, &new_path).await;
        assert_eq!(result, Some(PackageType::Cargo));
    }
}
