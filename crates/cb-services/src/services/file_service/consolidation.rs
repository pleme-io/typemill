//! Consolidation post-processing for Rust crate consolidation operations
//!
//! This module handles the post-processing tasks that must occur after moving
//! files during a consolidation operation:
//! 1. Flatten nested src/ directory structure
//! 2. Rename lib.rs to mod.rs for directory modules
//! 3. Add module declaration to target crate's lib.rs

use super::FileService;
use cb_protocol::{ApiError as ServerError, ApiResult as ServerResult, ConsolidationMetadata};
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
}
