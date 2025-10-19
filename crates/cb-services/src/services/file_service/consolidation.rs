//! Consolidation post-processing for Rust crate consolidation operations
//!
//! This module handles the post-processing tasks that must occur after moving
//! files during a consolidation operation:
//! 1. Flatten nested src/ directory structure
//! 2. Rename lib.rs to mod.rs for directory modules
//! 3. Add module declaration to target crate's lib.rs

use super::FileService;
use codebuddy_foundation::protocol::{ ApiError as ServerError , ApiResult as ServerResult , ConsolidationMetadata };
use std::path::Path;
use tokio::fs;
use tracing::{info, warn};

impl FileService {
    /// Execute consolidation post-processing after directory move
    ///
    /// This handles Rust-specific consolidation tasks:
    /// 1. Fix directory structure (flatten nested src/)
    /// 2. Rename lib.rs → mod.rs
    /// 3. Add module declaration to target lib.rs
    pub async fn execute_consolidation_post_processing(
        &self,
        metadata: &ConsolidationMetadata,
    ) -> ServerResult<()> {
        info!(
            source_crate = %metadata.source_crate_name,
            target_crate = %metadata.target_crate_name,
            target_module = %metadata.target_module_name,
            "Executing consolidation post-processing"
        );

        // Task 1: Fix nested src/ structure
        self.flatten_nested_src_directory(&metadata.target_module_path)
            .await?;

        // Task 2: Rename lib.rs → mod.rs
        self.rename_lib_rs_to_mod_rs(&metadata.target_module_path)
            .await?;

        // Task 3: Add module declaration to target lib.rs
        self.add_module_declaration_to_target_lib_rs(
            &metadata.target_crate_path,
            &metadata.target_module_name,
        )
        .await?;

        // Task 4: Merge dependencies from source Cargo.toml (Bug #3 fix)
        let source_cargo = Path::new(&metadata.source_crate_path).join("Cargo.toml");
        let target_cargo = Path::new(&metadata.target_crate_path).join("Cargo.toml");

        if source_cargo.exists() && target_cargo.exists() {
            self.merge_cargo_dependencies(&source_cargo, &target_cargo).await?;
        }

        // Task 5: Fix self-imports in consolidated module (Bug #2 fix)
        self.fix_self_imports_in_consolidated_module(
            &metadata.target_crate_name,
            &metadata.target_module_path,
        )
        .await?;

        // Task 6: Update imports across workspace (Bug #1 fix)
        self.update_imports_for_consolidation(
            &metadata.source_crate_name,
            &metadata.target_crate_name,
            &metadata.target_module_name,
        )
        .await?;

        info!("Consolidation post-processing complete");
        Ok(())
    }

    /// Fix Bug #1: Flatten nested protocol/src/ → protocol/
    async fn flatten_nested_src_directory(&self, module_path: &str) -> ServerResult<()> {
        let module_dir = Path::new(module_path);
        let nested_src = module_dir.join("src");

        if !nested_src.exists() {
            info!(
                module_path = %module_path,
                "No nested src/ directory, skipping flatten"
            );
            return Ok(());
        }

        info!(
            nested_src = %nested_src.display(),
            "Flattening nested src/ directory"
        );

        // Move all files from protocol/src/* to protocol/*
        let mut entries = fs::read_dir(&nested_src).await.map_err(|e| {
            ServerError::Internal(format!("Failed to read nested src/: {}", e))
        })?;

        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| ServerError::Internal(format!("Failed to iterate src/ entries: {}", e)))?
        {
            let file_name = entry.file_name();
            let source = entry.path();
            let target = module_dir.join(&file_name);

            fs::rename(&source, &target).await.map_err(|e| {
                ServerError::Internal(format!(
                    "Failed to move {} to {}: {}",
                    source.display(),
                    target.display(),
                    e
                ))
            })?;

            info!(
                file = %file_name.to_string_lossy(),
                "Moved file from nested src/"
            );
        }

        // Remove empty src/ directory
        fs::remove_dir(&nested_src).await.map_err(|e| {
            ServerError::Internal(format!("Failed to remove empty src/: {}", e))
        })?;

        // Remove Cargo.toml if it exists (should be merged already)
        let cargo_toml = module_dir.join("Cargo.toml");
        if cargo_toml.exists() {
            fs::remove_file(&cargo_toml).await.map_err(|e| {
                ServerError::Internal(format!("Failed to remove Cargo.toml: {}", e))
            })?;
            info!("Removed leftover Cargo.toml from module directory");
        }

        Ok(())
    }

    /// Fix Bug #2: Rename lib.rs → mod.rs
    async fn rename_lib_rs_to_mod_rs(&self, module_path: &str) -> ServerResult<()> {
        let lib_rs = Path::new(module_path).join("lib.rs");
        let mod_rs = Path::new(module_path).join("mod.rs");

        if !lib_rs.exists() {
            info!(
                module_path = %module_path,
                "No lib.rs found, skipping rename"
            );
            return Ok(());
        }

        if mod_rs.exists() {
            warn!(
                module_path = %module_path,
                "mod.rs already exists, skipping rename"
            );
            return Ok(());
        }

        fs::rename(&lib_rs, &mod_rs).await.map_err(|e| {
            ServerError::Internal(format!("Failed to rename lib.rs to mod.rs: {}", e))
        })?;

        info!(
            old_path = %lib_rs.display(),
            new_path = %mod_rs.display(),
            "Renamed lib.rs to mod.rs for directory module"
        );

        Ok(())
    }

    /// Fix Bug #5: Add module declaration to target lib.rs
    async fn add_module_declaration_to_target_lib_rs(
        &self,
        target_crate_path: &str,
        module_name: &str,
    ) -> ServerResult<()> {
        let lib_rs_path = Path::new(target_crate_path).join("src/lib.rs");

        if !lib_rs_path.exists() {
            warn!(
                lib_rs = %lib_rs_path.display(),
                "Target lib.rs not found, skipping module declaration"
            );
            return Ok(());
        }

        let content = fs::read_to_string(&lib_rs_path)
            .await
            .map_err(|e| ServerError::Internal(format!("Failed to read lib.rs: {}", e)))?;

        // Check if declaration already exists
        let declaration = format!("pub mod {};", module_name);
        if content.contains(&declaration)
            || content.contains(&format!("pub mod {module_name} ;"))
        {
            info!(
                module = %module_name,
                "Module declaration already exists, skipping"
            );
            return Ok(());
        }

        // Find insertion point (after last pub mod declaration)
        let lines: Vec<&str> = content.lines().collect();
        let mut insertion_line = 0;
        let mut found_mod_declaration = false;

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with("pub mod ") || trimmed.starts_with("mod ") {
                insertion_line = i + 1;
                found_mod_declaration = true;
            } else if found_mod_declaration && !trimmed.is_empty() && !trimmed.starts_with("//") {
                // Stop at first non-comment, non-empty line after mod declarations
                break;
            }
        }

        // Insert declaration
        let mut new_lines = lines.clone();
        new_lines.insert(insertion_line, &declaration);
        let new_content = new_lines.join("\n");

        // Preserve trailing newline if original had one
        let final_content = if content.ends_with('\n') {
            format!("{}\n", new_content)
        } else {
            new_content
        };

        fs::write(&lib_rs_path, final_content)
            .await
            .map_err(|e| ServerError::Internal(format!("Failed to write lib.rs: {}", e)))?;

        info!(
            lib_rs = %lib_rs_path.display(),
            module = %module_name,
            "Added module declaration to target lib.rs"
        );

        Ok(())
    }

    /// Fix Bug #2: Fix self-imports in consolidated module
    ///
    /// After moving code INTO a crate, imports that reference that crate are now
    /// self-imports and should use `crate::` instead of the crate name.
    ///
    /// Example: When moving code into `codebuddy-foundation`:
    /// - `use codebuddy_foundation::error::CoreError;` → `use crate::error::CoreError;`
    /// - `impl From<codebuddy_foundation::model::...>` → `impl From<crate::model::...>`
    async fn fix_self_imports_in_consolidated_module(
        &self,
        target_crate_name: &str,
        target_module_path: &str,
    ) -> ServerResult<()> {
        info!(
            crate_name = %target_crate_name,
            module_path = %target_module_path,
            "Fixing self-imports in consolidated module"
        );

        // Convert crate name to Rust identifier format (e.g., "codebuddy-foundation" → "codebuddy_foundation")
        let crate_ident = target_crate_name.replace('-', "_");
        let module_dir = Path::new(target_module_path);

        if !module_dir.exists() {
            warn!("Module directory does not exist, skipping self-import fixes");
            return Ok(());
        }

        // Find all .rs files recursively in the module directory
        let mut files_fixed = 0;
        let mut replacements_made = 0;

        self.fix_self_imports_in_directory(
            module_dir,
            &crate_ident,
            &mut files_fixed,
            &mut replacements_made,
        )
        .await?;

        info!(
            files_fixed = files_fixed,
            replacements = replacements_made,
            "Fixed self-imports in consolidated module"
        );

        Ok(())
    }

    /// Recursively fix self-imports in all .rs files in a directory
    async fn fix_self_imports_in_directory(
        &self,
        dir: &Path,
        crate_ident: &str,
        files_fixed: &mut usize,
        replacements_made: &mut usize,
    ) -> ServerResult<()> {
        let mut entries = fs::read_dir(dir).await.map_err(|e| {
            ServerError::Internal(format!("Failed to read directory {}: {}", dir.display(), e))
        })?;

        while let Some(entry) = entries.next_entry().await.map_err(|e| {
            ServerError::Internal(format!("Failed to iterate directory: {}", e))
        })? {
            let path = entry.path();

            if path.is_dir() {
                // Recurse into subdirectories
                Box::pin(self.fix_self_imports_in_directory(
                    &path,
                    crate_ident,
                    files_fixed,
                    replacements_made,
                ))
                .await?;
            } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
                // Process .rs files
                self.fix_self_imports_in_file(&path, crate_ident, files_fixed, replacements_made)
                    .await?;
            }
        }

        Ok(())
    }

    /// Fix self-imports in a single Rust file
    async fn fix_self_imports_in_file(
        &self,
        file_path: &Path,
        crate_ident: &str,
        files_fixed: &mut usize,
        replacements_made: &mut usize,
    ) -> ServerResult<()> {
        let content = fs::read_to_string(file_path).await.map_err(|e| {
            ServerError::Internal(format!("Failed to read {}: {}", file_path.display(), e))
        })?;

        // Replace various forms of self-imports:
        // 1. use crate_name:: → use crate::
        // 2. crate_name:: in type paths, impl blocks, etc.

        let mut new_content = content.clone();
        let mut file_replacements = 0;

        // Pattern 1: `use crate_name::` → `use crate::`
        let use_pattern = format!("use {}::", crate_ident);
        let use_replacement = "use crate::";
        if new_content.contains(&use_pattern) {
            let count = new_content.matches(&use_pattern).count();
            new_content = new_content.replace(&use_pattern, use_replacement);
            file_replacements += count;
        }

        // Pattern 2: `crate_name::` in other contexts (type paths, impl blocks, etc.)
        // Match only when preceded by whitespace, '<', '(', or at start of line
        // to avoid matching inside identifiers
        let qualified_pattern = format!("{}::", crate_ident);
        let qualified_replacement = "crate::";

        // Split on the pattern and rebuild, being careful about context
        let parts: Vec<&str> = new_content.split(&qualified_pattern).collect();
        if parts.len() > 1 {
            let mut rebuilt = String::new();
            for (i, part) in parts.iter().enumerate() {
                rebuilt.push_str(part);
                if i < parts.len() - 1 {
                    // Check if this is a valid replacement context
                    // (after whitespace, '<', '(', or at line start)
                    let should_replace = part.is_empty()
                        || part.ends_with(|c: char| c.is_whitespace())
                        || part.ends_with('<')
                        || part.ends_with('(')
                        || part.ends_with(',');

                    if should_replace {
                        rebuilt.push_str(qualified_replacement);
                        file_replacements += 1;
                    } else {
                        rebuilt.push_str(&qualified_pattern);
                    }
                }
            }
            new_content = rebuilt;
        }

        // Only write if changes were made
        if file_replacements > 0 {
            fs::write(file_path, new_content).await.map_err(|e| {
                ServerError::Internal(format!("Failed to write {}: {}", file_path.display(), e))
            })?;

            *files_fixed += 1;
            *replacements_made += file_replacements;

            info!(
                file = %file_path.display(),
                replacements = file_replacements,
                "Fixed self-imports in file"
            );
        }

        Ok(())
    }

    /// Fix Bug #1: Update imports across workspace for consolidation
    ///
    /// When consolidating crates, all imports need to be updated:
    /// - `use old_crate::foo` → `use new_crate::module::foo`
    /// - `old_crate::bar::Baz` → `new_crate::module::bar::Baz`
    async fn update_imports_for_consolidation(
        &self,
        source_crate_name: &str,
        target_crate_name: &str,
        target_module_name: &str,
    ) -> ServerResult<()> {
        info!(
            source_crate = %source_crate_name,
            target_crate = %target_crate_name,
            target_module = %target_module_name,
            "Updating imports across workspace for consolidation"
        );

        // Convert crate names to Rust identifiers
        let source_ident = source_crate_name.replace('-', "_");
        let target_ident = target_crate_name.replace('-', "_");

        // Get workspace root (we need to scan from workspace root to update all crates)
        let workspace_root = self.project_root.clone();

        // Patterns to replace:
        // 1. `use source_crate::` → `use target_crate::module::`
        // 2. `source_crate::` (qualified paths) → `target_crate::module::`

        let mut files_updated = 0;
        let mut total_replacements = 0;

        // Scan workspace for Rust files
        self.update_imports_in_workspace_directory(
            &workspace_root,
            &source_ident,
            &target_ident,
            target_module_name,
            &mut files_updated,
            &mut total_replacements,
        )
        .await?;

        info!(
            files_updated = files_updated,
            replacements = total_replacements,
            "Updated imports across workspace for consolidation"
        );

        Ok(())
    }

    /// Recursively update imports in workspace directory
    async fn update_imports_in_workspace_directory(
        &self,
        dir: &Path,
        source_ident: &str,
        target_ident: &str,
        target_module: &str,
        files_updated: &mut usize,
        total_replacements: &mut usize,
    ) -> ServerResult<()> {
        // Skip target/node_modules/.git directories
        let dir_name = dir.file_name().and_then(|s| s.to_str()).unwrap_or("");
        if matches!(dir_name, "target" | "node_modules" | ".git" | "dist") {
            return Ok(());
        }

        let entries_result = fs::read_dir(dir).await;
        if entries_result.is_err() {
            // Skip directories we can't read (permissions, etc.)
            return Ok(());
        }

        let mut entries = entries_result.unwrap();

        while let Some(entry) = entries.next_entry().await.map_err(|e| {
            ServerError::Internal(format!("Failed to iterate directory: {}", e))
        })? {
            let path = entry.path();

            if path.is_dir() {
                // Recurse into subdirectories
                Box::pin(self.update_imports_in_workspace_directory(
                    &path,
                    source_ident,
                    target_ident,
                    target_module,
                    files_updated,
                    total_replacements,
                ))
                .await?;
            } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
                // Update Rust files
                self.update_imports_in_single_file(
                    &path,
                    source_ident,
                    target_ident,
                    target_module,
                    files_updated,
                    total_replacements,
                )
                .await?;
            }
        }

        Ok(())
    }

    /// Update imports in a single Rust file for consolidation
    async fn update_imports_in_single_file(
        &self,
        file_path: &Path,
        source_ident: &str,
        target_ident: &str,
        target_module: &str,
        files_updated: &mut usize,
        total_replacements: &mut usize,
    ) -> ServerResult<()> {
        let content_result = fs::read_to_string(file_path).await;
        if content_result.is_err() {
            // Skip files we can't read
            return Ok(());
        }

        let content = content_result.unwrap();

        // Skip if file doesn't contain the source crate identifier
        if !content.contains(source_ident) {
            return Ok(());
        }

        let mut new_content = content.clone();
        let mut file_replacements = 0;

        // Pattern 1: `use source_crate::` → `use target_crate::module::`
        let use_pattern = format!("use {}::", source_ident);
        let use_replacement = format!("use {}::{}::", target_ident, target_module);
        if new_content.contains(&use_pattern) {
            let count = new_content.matches(&use_pattern).count();
            new_content = new_content.replace(&use_pattern, &use_replacement);
            file_replacements += count;
        }

        // Pattern 2: qualified paths `source_crate::` → `target_crate::module::`
        // Be careful to only replace in valid contexts
        let qualified_pattern = format!("{}::", source_ident);
        let qualified_replacement = format!("{}::{}::", target_ident, target_module);

        let parts: Vec<&str> = new_content.split(&qualified_pattern).collect();
        if parts.len() > 1 {
            let mut rebuilt = String::new();
            for (i, part) in parts.iter().enumerate() {
                rebuilt.push_str(part);
                if i < parts.len() - 1 {
                    // Check context - should replace after whitespace, '<', '(', ',', or at start
                    let should_replace = part.is_empty()
                        || part.ends_with(|c: char| c.is_whitespace())
                        || part.ends_with('<')
                        || part.ends_with('(')
                        || part.ends_with(',')
                        || part.ends_with('{');

                    if should_replace {
                        rebuilt.push_str(&qualified_replacement);
                        file_replacements += 1;
                    } else {
                        rebuilt.push_str(&qualified_pattern);
                    }
                }
            }
            new_content = rebuilt;
        }

        // Only write if changes were made
        if file_replacements > 0 {
            fs::write(file_path, new_content).await.map_err(|e| {
                ServerError::Internal(format!("Failed to write {}: {}", file_path.display(), e))
            })?;

            *files_updated += 1;
            *total_replacements += file_replacements;

            info!(
                file = %file_path.display(),
                replacements = file_replacements,
                "Updated imports for consolidation"
            );
        }

        Ok(())
    }
}