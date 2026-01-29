//! Consolidation post-processing for Rust crate consolidation operations
//!
//! **LANGUAGE-SPECIFIC**: This module contains Rust-specific logic for Cargo crate consolidation.
//! It handles the post-processing tasks that must occur after moving files during a consolidation
//! operation.
//!
//! This module handles the post-processing tasks that must occur after moving
//! files during a consolidation operation:
//! 1. Flatten nested src/ directory structure
//! 2. Rename lib.rs to mod.rs for directory modules
//! 3. Add module declaration to target crate's lib.rs

use crate::cargo_helpers::merge_cargo_dependencies;
use mill_foundation::protocol::ConsolidationMetadata;
use mill_plugin_api::{PluginApiError, PluginResult};
use std::path::Path;
use tokio::fs;
use tracing::{info, warn};

/// Execute consolidation post-processing after directory move
///
/// This handles Rust-specific consolidation tasks:
/// 1. Fix directory structure (flatten nested src/)
/// 2. Rename lib.rs → mod.rs
/// 3. Add module declaration to target lib.rs
pub async fn execute_consolidation_post_processing(
    metadata: &ConsolidationMetadata,
    project_root: &Path,
) -> PluginResult<()> {
    info!(
        source_crate = %metadata.source_crate_name,
        target_crate = %metadata.target_crate_name,
        target_module = %metadata.target_module_name,
        "Executing consolidation post-processing"
    );

    // Task 0: Circular dependency validation
    // ✅ COMPLETED: Validation now happens during PLANNING phase (directory_rename.rs)
    // The validation is called before any files are moved, preventing circular dependencies.
    // See: crates/mill-handlers/src/handlers/rename_ops/directory_rename.rs:93-136

    // Task 1: Fix nested src/ structure
    flatten_nested_src_directory(&metadata.target_module_path).await?;

    // Task 2: Rename lib.rs → mod.rs
    rename_lib_rs_to_mod_rs(&metadata.target_module_path).await?;

    // Task 3: Add module declaration to target lib.rs
    add_module_declaration_to_target_lib_rs(
        &metadata.target_crate_path,
        &metadata.target_module_name,
    )
    .await?;

    // Task 4: Merge dependencies from source Cargo.toml (Bug #3 fix)
    let source_cargo = Path::new(&metadata.source_crate_path).join("Cargo.toml");
    let target_cargo = Path::new(&metadata.target_crate_path).join("Cargo.toml");

    if source_cargo.exists() && target_cargo.exists() {
        merge_cargo_dependencies(&source_cargo, &target_cargo).await?;
    }

    // Task 5: Fix self-imports in consolidated module (Bug #2 fix)
    fix_self_imports_in_consolidated_module(
        &metadata.target_crate_name,
        &metadata.target_module_path,
    )
    .await?;

    // Task 6: Update imports across workspace (Bug #1 fix)
    update_imports_for_consolidation(
        &metadata.source_crate_name,
        &metadata.target_crate_name,
        &metadata.target_module_name,
        project_root,
    )
    .await?;

    // Task 7: Clean up workspace Cargo.toml (Bug #3 fix)
    cleanup_workspace_cargo_toml(
        &metadata.source_crate_path,
        &metadata.target_crate_path,
        &metadata.source_crate_name,
        &metadata.target_crate_name,
        project_root,
    )
    .await?;

    // Task 7.5: Remove source crate dependency from target crate's Cargo.toml
    remove_source_dependency_from_target(&metadata.source_crate_name, &metadata.target_crate_path)
        .await?;

    // Task 8: Remove duplicate dependencies across workspace (Bug #5 fix)
    remove_duplicate_dependencies_in_workspace(project_root).await?;

    info!("Consolidation post-processing complete");
    Ok(())
}

/// Clean up workspace Cargo.toml after consolidation
///
/// Removes source crate from workspace members and dependencies,
/// ensures target crate is in workspace dependencies.
async fn cleanup_workspace_cargo_toml(
    source_crate_path: &str,
    target_crate_path: &str,
    source_crate_name: &str,
    target_crate_name: &str,
    project_root: &Path,
) -> PluginResult<()> {
    let workspace_toml_path = project_root.join("Cargo.toml");

    if !workspace_toml_path.exists() {
        warn!("Workspace Cargo.toml not found, skipping cleanup");
        return Ok(());
    }

    // Convert absolute paths to relative paths from project_root
    let source_path = Path::new(source_crate_path);
    let target_path = Path::new(target_crate_path);

    let source_relative = source_path
        .strip_prefix(project_root)
        .unwrap_or(source_path)
        .to_string_lossy()
        .to_string();

    let target_relative = target_path
        .strip_prefix(project_root)
        .unwrap_or(target_path)
        .to_string_lossy()
        .to_string();

    let content = fs::read_to_string(&workspace_toml_path)
        .await
        .map_err(|e| {
            PluginApiError::internal(format!("Failed to read workspace Cargo.toml: {}", e))
        })?;

    let mut doc = content.parse::<toml_edit::DocumentMut>().map_err(|e| {
        PluginApiError::internal(format!("Failed to parse workspace Cargo.toml: {}", e))
    })?;

    let mut modified = false;

    // 1. Remove source crate from workspace members
    if let Some(workspace) = doc.get_mut("workspace").and_then(|w| w.as_table_like_mut()) {
        if let Some(members) = workspace.get_mut("members").and_then(|m| m.as_array_mut()) {
            let before_len = members.len();
            members.retain(|item| item.as_str() != Some(source_relative.as_str()));

            if members.len() < before_len {
                modified = true;
                info!(
                    source_crate = %source_crate_name,
                    source_path = %source_relative,
                    "Removed from workspace members"
                );
            } else {
                info!(
                    source_path = %source_relative,
                    "Source crate not found in workspace members (may have already been removed)"
                );
            }
        }
    }

    // 2. Remove source crate from workspace dependencies
    if let Some(workspace) = doc.get_mut("workspace").and_then(|w| w.as_table_like_mut()) {
        if let Some(deps) = workspace
            .get_mut("dependencies")
            .and_then(|d| d.as_table_like_mut())
        {
            if deps.remove(source_crate_name).is_some() {
                modified = true;
                info!(
                    source_crate = %source_crate_name,
                    "Removed from workspace dependencies"
                );
            }
        }
    }

    // 3. Ensure target crate is in workspace dependencies
    if let Some(workspace) = doc.get_mut("workspace").and_then(|w| w.as_table_like_mut()) {
        if let Some(deps) = workspace
            .get_mut("dependencies")
            .and_then(|d| d.as_table_like_mut())
        {
            if !deps.contains_key(target_crate_name) {
                // Create inline table for the dependency
                let mut target_dep = toml_edit::InlineTable::new();
                target_dep.insert("path", toml_edit::Value::from(target_relative.clone()));

                deps.insert(
                    target_crate_name,
                    toml_edit::Item::Value(toml_edit::Value::InlineTable(target_dep)),
                );

                modified = true;
                info!(
                    target_crate = %target_crate_name,
                    target_path = %target_relative,
                    "Added to workspace dependencies"
                );
            }
        }
    }

    // Write back if modified
    if modified {
        fs::write(&workspace_toml_path, doc.to_string())
            .await
            .map_err(|e| {
                PluginApiError::internal(format!("Failed to write workspace Cargo.toml: {}", e))
            })?;

        info!("Workspace Cargo.toml cleanup complete");
    } else {
        info!("No workspace Cargo.toml cleanup needed");
    }

    Ok(())
}

/// Remove source crate dependency from target crate's Cargo.toml
///
/// After consolidation, the target crate should no longer depend on the source crate
/// since the source code is now part of the target crate.
async fn remove_source_dependency_from_target(
    source_crate_name: &str,
    target_crate_path: &str,
) -> PluginResult<()> {
    let target_cargo_toml = Path::new(target_crate_path).join("Cargo.toml");

    if !target_cargo_toml.exists() {
        warn!(
            target_path = %target_crate_path,
            "Target Cargo.toml not found, skipping dependency removal"
        );
        return Ok(());
    }

    let content = fs::read_to_string(&target_cargo_toml).await.map_err(|e| {
        PluginApiError::internal(format!("Failed to read target Cargo.toml: {}", e))
    })?;

    let mut doc = content.parse::<toml_edit::DocumentMut>().map_err(|e| {
        PluginApiError::internal(format!("Failed to parse target Cargo.toml: {}", e))
    })?;

    let mut modified = false;

    // Remove source crate from dependencies
    if let Some(deps) = doc
        .get_mut("dependencies")
        .and_then(|d| d.as_table_like_mut())
    {
        if deps.remove(source_crate_name).is_some() {
            modified = true;
            info!(
                source_crate = %source_crate_name,
                target_path = %target_crate_path,
                "Removed source crate dependency from target Cargo.toml"
            );
        }
    }

    // Write back if modified
    if modified {
        fs::write(&target_cargo_toml, doc.to_string())
            .await
            .map_err(|e| {
                PluginApiError::internal(format!("Failed to write target Cargo.toml: {}", e))
            })?;

        info!("Target Cargo.toml dependency cleanup complete");
    } else {
        info!("No target Cargo.toml dependency cleanup needed");
    }

    Ok(())
}

/// Remove duplicate dependencies from all workspace Cargo.toml files
async fn remove_duplicate_dependencies_in_workspace(project_root: &Path) -> PluginResult<()> {
    info!("Scanning workspace for duplicate dependencies");

    let mut fixed_count = 0;
    let mut checked_count = 0;

    // Use ignore crate to walk workspace
    let walker = ignore::WalkBuilder::new(project_root).hidden(false).build();

    for entry in walker {
        let entry = entry
            .map_err(|e| PluginApiError::internal(format!("Failed to walk workspace: {}", e)))?;

        // Only process Cargo.toml files
        if !entry.file_type().map(|ft| ft.is_file()).unwrap_or(false)
            || entry.file_name() != "Cargo.toml"
        {
            continue;
        }

        let path = entry.path();
        checked_count += 1;

        let content_result = fs::read_to_string(path).await;
        if content_result.is_err() {
            continue;
        }

        let content = content_result.unwrap();
        let fixed_content_result = remove_duplicate_dependencies(&content);

        if fixed_content_result.is_err() {
            continue;
        }

        let fixed_content = fixed_content_result.unwrap();

        if content != fixed_content {
            fs::write(path, &fixed_content).await.map_err(|e| {
                PluginApiError::internal(format!("Failed to write Cargo.toml: {}", e))
            })?;

            fixed_count += 1;
            info!(
                file = %path.display(),
                "Removed duplicate dependencies"
            );
        }
    }

    info!(
        checked = checked_count,
        fixed = fixed_count,
        "Duplicate dependency cleanup complete"
    );

    Ok(())
}

/// Remove duplicate dependencies from a single Cargo.toml content
fn remove_duplicate_dependencies(content: &str) -> PluginResult<String> {
    let mut doc = content
        .parse::<toml_edit::DocumentMut>()
        .map_err(|e| PluginApiError::internal(format!("Failed to parse Cargo.toml: {}", e)))?;

    // Process [dependencies]
    if let Some(deps) = doc
        .get_mut("dependencies")
        .and_then(|d| d.as_table_like_mut())
    {
        remove_duplicates_from_table(deps);
    }

    // Process [dev-dependencies]
    if let Some(deps) = doc
        .get_mut("dev-dependencies")
        .and_then(|d| d.as_table_like_mut())
    {
        remove_duplicates_from_table(deps);
    }

    // Process [build-dependencies]
    if let Some(deps) = doc
        .get_mut("build-dependencies")
        .and_then(|d| d.as_table_like_mut())
    {
        remove_duplicates_from_table(deps);
    }

    Ok(doc.to_string())
}

/// Remove duplicates from a TOML table, keeping the first occurrence
fn remove_duplicates_from_table(table: &mut dyn toml_edit::TableLike) {
    use std::collections::HashSet;

    let mut seen = HashSet::new();
    let mut to_remove = Vec::new();

    for (key, _) in table.iter() {
        if !seen.insert(key.to_string()) {
            to_remove.push(key.to_string());
        }
    }

    // Remove duplicates (keeps first occurrence)
    for key in to_remove {
        table.remove(&key);
    }
}

/// Fix Bug #1: Flatten nested protocol/src/ → protocol/
async fn flatten_nested_src_directory(module_path: &str) -> PluginResult<()> {
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
    let mut entries = fs::read_dir(&nested_src)
        .await
        .map_err(|e| PluginApiError::internal(format!("Failed to read nested src/: {}", e)))?;

    while let Some(entry) = entries
        .next_entry()
        .await
        .map_err(|e| PluginApiError::internal(format!("Failed to iterate src/ entries: {}", e)))?
    {
        let file_name = entry.file_name();
        let source = entry.path();
        let target = module_dir.join(&file_name);

        fs::rename(&source, &target).await.map_err(|e| {
            PluginApiError::internal(format!(
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
    fs::remove_dir(&nested_src)
        .await
        .map_err(|e| PluginApiError::internal(format!("Failed to remove empty src/: {}", e)))?;

    // Remove Cargo.toml if it exists (should be merged already)
    let cargo_toml = module_dir.join("Cargo.toml");
    if cargo_toml.exists() {
        fs::remove_file(&cargo_toml)
            .await
            .map_err(|e| PluginApiError::internal(format!("Failed to remove Cargo.toml: {}", e)))?;
        info!("Removed leftover Cargo.toml from module directory");
    }

    Ok(())
}

/// Fix Bug #2: Rename lib.rs → mod.rs
async fn rename_lib_rs_to_mod_rs(module_path: &str) -> PluginResult<()> {
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
        PluginApiError::internal(format!("Failed to rename lib.rs to mod.rs: {}", e))
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
    target_crate_path: &str,
    module_name: &str,
) -> PluginResult<()> {
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
        .map_err(|e| PluginApiError::internal(format!("Failed to read lib.rs: {}", e)))?;

    // Check if declaration already exists
    let declaration = format!("pub mod {};", module_name);
    if content.contains(&declaration) || content.contains(&format!("pub mod {module_name} ;")) {
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
        .map_err(|e| PluginApiError::internal(format!("Failed to write lib.rs: {}", e)))?;

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
/// Example: When moving code into `mill-foundation`:
/// - `use mill_foundation::error::CoreError;` → `use crate::error::CoreError;`
/// - `impl From<mill_foundation::model::...>` → `impl From<crate::model::...>`
async fn fix_self_imports_in_consolidated_module(
    target_crate_name: &str,
    target_module_path: &str,
) -> PluginResult<()> {
    info!(
        crate_name = %target_crate_name,
        module_path = %target_module_path,
        "Fixing self-imports in consolidated module"
    );

    // Convert crate name to Rust identifier format (e.g., "mill-foundation" → "mill_foundation")
    let crate_ident = target_crate_name.replace('-', "_");
    let module_dir = Path::new(target_module_path);

    if !module_dir.exists() {
        warn!("Module directory does not exist, skipping self-import fixes");
        return Ok(());
    }

    // Find all .rs files recursively in the module directory
    let mut files_fixed = 0;
    let mut replacements_made = 0;

    fix_self_imports_in_directory(
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
    dir: &Path,
    crate_ident: &str,
    files_fixed: &mut usize,
    replacements_made: &mut usize,
) -> PluginResult<()> {
    let mut entries = fs::read_dir(dir).await.map_err(|e| {
        PluginApiError::internal(format!("Failed to read directory {}: {}", dir.display(), e))
    })?;

    while let Some(entry) = entries
        .next_entry()
        .await
        .map_err(|e| PluginApiError::internal(format!("Failed to iterate directory: {}", e)))?
    {
        let path = entry.path();

        if path.is_dir() {
            // Recurse into subdirectories
            Box::pin(fix_self_imports_in_directory(
                &path,
                crate_ident,
                files_fixed,
                replacements_made,
            ))
            .await?;
        } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            // Process .rs files
            fix_self_imports_in_file(&path, crate_ident, files_fixed, replacements_made).await?;
        }
    }

    Ok(())
}

/// Fix self-imports in a single Rust file
///
/// Converts external crate imports to internal crate imports:
/// 1. use statements: `use crate_name::` → `use crate::`
/// 2. pub use statements: `pub use crate_name::` → `pub use crate::`
/// 3. qualified paths: `crate_name::` → `crate::`
async fn fix_self_imports_in_file(
    file_path: &Path,
    crate_ident: &str,
    files_fixed: &mut usize,
    replacements_made: &mut usize,
) -> PluginResult<()> {
    let content = fs::read_to_string(file_path).await.map_err(|e| {
        PluginApiError::internal(format!("Failed to read {}: {}", file_path.display(), e))
    })?;

    let mut new_content = content.clone();
    let mut use_count = 0;
    let mut pub_use_count = 0;
    let mut qualified_count = 0;

    // Pattern 1: `use crate_name::` → `use crate::`
    let use_pattern = format!("use {}::", crate_ident);
    let use_replacement = "use crate::";
    if new_content.contains(&use_pattern) {
        use_count = new_content.matches(&use_pattern).count();
        new_content = new_content.replace(&use_pattern, use_replacement);
    }

    // Pattern 2: `pub use crate_name::` → `pub use crate::`
    let pub_use_pattern = format!("pub use {}::", crate_ident);
    let pub_use_replacement = "pub use crate::";
    if new_content.contains(&pub_use_pattern) {
        pub_use_count = new_content.matches(&pub_use_pattern).count();
        new_content = new_content.replace(&pub_use_pattern, pub_use_replacement);
    }

    // Pattern 3: `crate_name::` in other contexts (type paths, impl blocks, etc.)
    // Match only when preceded by valid context characters
    let qualified_pattern = format!("{}::", crate_ident);
    let qualified_replacement = "crate::";

    // Split on the pattern and rebuild with context checking
    let parts: Vec<&str> = new_content.split(&qualified_pattern).collect();
    if parts.len() > 1 {
        let mut rebuilt = String::new();
        for (i, part) in parts.iter().enumerate() {
            rebuilt.push_str(part);
            if i < parts.len() - 1 {
                // Check if this is a valid replacement context
                let should_replace = part.is_empty()
                    || part.ends_with(|c: char| c.is_whitespace())
                    || part.ends_with('<')
                    || part.ends_with('(')
                    || part.ends_with(',')
                    || part.ends_with('{')
                    || part.ends_with('[')
                    || part.ends_with('&')
                    || part.ends_with('*')
                    || part.ends_with('!');

                if should_replace {
                    rebuilt.push_str(qualified_replacement);
                    qualified_count += 1;
                } else {
                    rebuilt.push_str(&qualified_pattern);
                }
            }
        }
        new_content = rebuilt;
    }

    let file_replacements = use_count + pub_use_count + qualified_count;

    // Only write if changes were made
    if file_replacements > 0 {
        fs::write(file_path, new_content).await.map_err(|e| {
            PluginApiError::internal(format!("Failed to write {}: {}", file_path.display(), e))
        })?;

        *files_fixed += 1;
        *replacements_made += file_replacements;

        info!(
            file = %file_path.display(),
            use_statements = use_count,
            pub_use_statements = pub_use_count,
            qualified_paths = qualified_count,
            total_replacements = file_replacements,
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
    source_crate_name: &str,
    target_crate_name: &str,
    target_module_name: &str,
    project_root: &Path,
) -> PluginResult<()> {
    info!(
        source_crate = %source_crate_name,
        target_crate = %target_crate_name,
        target_module = %target_module_name,
        "Updating imports across workspace for consolidation"
    );

    // Convert crate names to Rust identifiers
    let source_ident = source_crate_name.replace('-', "_");
    let target_ident = target_crate_name.replace('-', "_");

    // Patterns to replace:
    // 1. `use source_crate::` → `use target_crate::module::`
    // 2. `source_crate::` (qualified paths) → `target_crate::module::`

    let mut files_updated = 0;
    let mut total_replacements = 0;

    // Scan workspace for Rust files
    update_imports_in_workspace_directory(
        project_root,
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
    dir: &Path,
    source_ident: &str,
    target_ident: &str,
    target_module: &str,
    files_updated: &mut usize,
    total_replacements: &mut usize,
) -> PluginResult<()> {
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

    while let Some(entry) = entries
        .next_entry()
        .await
        .map_err(|e| PluginApiError::internal(format!("Failed to iterate directory: {}", e)))?
    {
        let path = entry.path();

        if path.is_dir() {
            // Recurse into subdirectories
            Box::pin(update_imports_in_workspace_directory(
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
            update_imports_in_single_file(
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
///
/// Handles multiple import patterns:
/// 1. use statements: `use source_crate::` → `use target_crate::module::`
/// 2. pub use statements: `pub use source_crate::` → `pub use target_crate::module::`
/// 3. qualified paths: `source_crate::` → `target_crate::module::`
async fn update_imports_in_single_file(
    file_path: &Path,
    source_ident: &str,
    target_ident: &str,
    target_module: &str,
    files_updated: &mut usize,
    total_replacements: &mut usize,
) -> PluginResult<()> {
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
    let mut use_count = 0;
    let mut pub_use_count = 0;
    let mut qualified_count = 0;

    // Pattern 1: `use source_crate::` → `use target_crate::module::`
    let use_pattern = format!("use {}::", source_ident);
    let use_replacement = format!("use {}::{}::", target_ident, target_module);
    if new_content.contains(&use_pattern) {
        use_count = new_content.matches(&use_pattern).count();
        new_content = new_content.replace(&use_pattern, &use_replacement);
    }

    // Pattern 2: `pub use source_crate::` → `pub use target_crate::module::`
    let pub_use_pattern = format!("pub use {}::", source_ident);
    let pub_use_replacement = format!("pub use {}::{}::", target_ident, target_module);
    if new_content.contains(&pub_use_pattern) {
        pub_use_count = new_content.matches(&pub_use_pattern).count();
        new_content = new_content.replace(&pub_use_pattern, &pub_use_replacement);
    }

    // Pattern 3: qualified paths `source_crate::` → `target_crate::module::`
    // Only replace in valid contexts (not inside identifiers)
    let qualified_pattern = format!("{}::", source_ident);
    let qualified_replacement = format!("{}::{}::", target_ident, target_module);

    // Split and rebuild with context checking
    let parts: Vec<&str> = new_content.split(&qualified_pattern).collect();
    if parts.len() > 1 {
        let mut rebuilt = String::new();
        for (i, part) in parts.iter().enumerate() {
            rebuilt.push_str(part);
            if i < parts.len() - 1 {
                // Check context - should replace after whitespace, '<', '(', ',', '{', '[', '&', '*', or at start
                let should_replace = part.is_empty()
                    || part.ends_with(|c: char| c.is_whitespace())
                    || part.ends_with('<')
                    || part.ends_with('(')
                    || part.ends_with(',')
                    || part.ends_with('{')
                    || part.ends_with('[')
                    || part.ends_with('&')
                    || part.ends_with('*')
                    || part.ends_with('!');

                if should_replace {
                    rebuilt.push_str(&qualified_replacement);
                    qualified_count += 1;
                } else {
                    rebuilt.push_str(&qualified_pattern);
                }
            }
        }
        new_content = rebuilt;
    }

    let file_replacements = use_count + pub_use_count + qualified_count;

    // Only write if changes were made
    if file_replacements > 0 {
        fs::write(file_path, new_content).await.map_err(|e| {
            PluginApiError::internal(format!("Failed to write {}: {}", file_path.display(), e))
        })?;

        *files_updated += 1;
        *total_replacements += file_replacements;

        info!(
            file = %file_path.display(),
            use_statements = use_count,
            pub_use_statements = pub_use_count,
            qualified_paths = qualified_count,
            total_replacements = file_replacements,
            "Updated imports for consolidation"
        );
    }

    Ok(())
}
