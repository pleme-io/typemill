//! Workspace support trait for language plugins
//!
//! Provides workspace manifest manipulation capabilities for languages
//! with multi-package project structures (Rust workspaces, TypeScript monorepos, etc.).

use async_trait::async_trait;
use std::path::Path;

/// Move/rename planning result from language plugin
///
/// Contains manifest edits and metadata needed for workspace-aware moves
#[derive(Debug, Clone)]
pub struct MoveManifestPlan {
    /// Manifest file updates (e.g., workspace members, package names, dependency paths)
    pub manifest_edits: Vec<codebuddy_foundation::protocol::TextEdit>,

    /// Rename metadata (package names, module names, etc.) for import updates
    pub rename_info: Option<serde_json::Value>,

    /// Whether this is a consolidation move (merging packages)
    pub is_consolidation: bool,
}

/// Optional trait for languages that support workspace operations
///
/// Basic operations (add/remove members) are **synchronous** (no async overhead).
/// Advanced operations (move/rename planning) are **async** to allow I/O.
#[async_trait]
pub trait WorkspaceSupport: Send + Sync {
    /// Add a new member to a workspace manifest
    ///
    /// # Arguments
    /// * `content` - Workspace manifest content (e.g., Cargo.toml)
    /// * `member` - Member path to add (e.g., "crates/new-crate")
    ///
    /// # Returns
    /// Updated manifest content with new member added
    ///
    /// # Example
    /// ```ignore
    /// // Cargo.toml before:
    /// // [workspace]
    /// // members = ["crates/foo"]
    /// //
    /// // After add_workspace_member(content, "crates/bar"):
    /// // [workspace]
    /// // members = ["crates/foo", "crates/bar"]
    /// ```
    fn add_workspace_member(&self, content: &str, member: &str) -> String;

    /// Remove a member from a workspace manifest
    ///
    /// # Arguments
    /// * `content` - Workspace manifest content
    /// * `member` - Member path to remove
    ///
    /// # Returns
    /// Updated manifest content with member removed
    fn remove_workspace_member(&self, content: &str, member: &str) -> String;

    /// Check if a manifest file represents a workspace (vs single package)
    ///
    /// # Arguments
    /// * `content` - Manifest file content
    ///
    /// # Returns
    /// true if this is a workspace manifest, false otherwise
    fn is_workspace_manifest(&self, content: &str) -> bool;

    /// List all workspace members from a workspace manifest
    ///
    /// # Arguments
    /// * `content` - Workspace manifest content
    ///
    /// # Returns
    /// List of workspace member paths
    fn list_workspace_members(&self, content: &str) -> Vec<String>;

    /// Update the name field in a package manifest
    ///
    /// # Arguments
    /// * `content` - Package manifest content
    /// * `new_name` - New package name
    ///
    /// # Returns
    /// Updated manifest content with new package name
    fn update_package_name(&self, content: &str, new_name: &str) -> String;

    // ========================================================================
    // Move/Rename Planning (Async operations for I/O)
    // ========================================================================

    /// Check if a directory is a workspace package
    ///
    /// # Arguments
    /// * `dir_path` - Directory path to check
    ///
    /// # Returns
    /// true if directory contains a package manifest (e.g., Cargo.toml, package.json)
    async fn is_package(&self, dir_path: &Path) -> bool {
        // Default: not a package
        let _ = dir_path;
        false
    }

    /// Plan manifest edits for a directory move/rename
    ///
    /// This method generates all manifest file updates needed when moving or renaming
    /// a package directory, including:
    /// - Workspace member list updates
    /// - Package name changes
    /// - Dependency path updates in dependent packages
    /// - Consolidation-specific logic (merging packages)
    ///
    /// # Arguments
    /// * `old_path` - Current package directory path
    /// * `new_path` - New package directory path
    /// * `project_root` - Workspace/project root directory
    ///
    /// # Returns
    /// MoveManifestPlan with edits and metadata, or None if not a package
    ///
    /// # Default Implementation
    /// Returns None (no manifest edits). Languages with workspace support should override.
    async fn plan_directory_move(
        &self,
        _old_path: &Path,
        _new_path: &Path,
        _project_root: &Path,
    ) -> Option<MoveManifestPlan> {
        // Default: no manifest planning
        None
    }

    /// Generate a workspace manifest file
    ///
    /// This method generates a workspace manifest file (e.g., Cargo.toml with [workspace])
    /// with the specified member paths.
    ///
    /// # Arguments
    /// * `member_paths` - List of workspace member paths
    /// * `workspace_root` - Workspace root directory
    ///
    /// # Returns
    /// Generated workspace manifest content
    ///
    /// # Default Implementation
    /// Returns NotSupported error. Languages with workspace support should override.
    async fn generate_workspace_manifest(
        &self,
        _member_paths: &[&str],
        _workspace_root: &Path,
    ) -> Result<String, String> {
        Err("generate_workspace_manifest not supported by this language".to_string())
    }

    // ========================================================================
    // Consolidation Post-Processing (Language-specific file structure fixes)
    // ========================================================================

    /// Execute language-specific post-processing after a consolidation move
    ///
    /// This method handles file structure transformations that must occur after
    /// moving files during a package consolidation, such as:
    /// - Flattening nested source directories
    /// - Renaming entry point files for module structure
    /// - Adding module declarations to target package
    /// - Validating no circular dependencies are created
    /// - Merging package manifests
    ///
    /// # Arguments
    /// * `source_crate_name` - Name of the package being consolidated
    /// * `target_crate_name` - Name of the package receiving the code
    /// * `target_module_name` - Module name in target package
    /// * `source_crate_path` - Absolute path to source package root
    /// * `target_crate_path` - Absolute path to target package root
    /// * `target_module_path` - Absolute path to target module directory
    /// * `project_root` - Workspace/project root directory
    ///
    /// # Returns
    /// Ok(()) on success, or error if post-processing fails
    ///
    /// # Default Implementation
    /// No-op. Languages with consolidation support should override.
    ///
    /// # Example (Rust)
    /// ```ignore
    /// // Rust implementation in cb-lang-rust:
    /// async fn execute_consolidation_post_processing(
    ///     &self,
    ///     source_crate_name: &str,
    ///     target_crate_name: &str,
    ///     target_module_name: &str,
    ///     source_crate_path: &Path,
    ///     target_crate_path: &Path,
    ///     target_module_path: &Path,
    ///     project_root: &Path,
    /// ) -> Result<(), String> {
    ///     // 1. Flatten nested src/ directories
    ///     // 2. Rename lib.rs → mod.rs
    ///     // 3. Add module declaration to target lib.rs
    ///     // 4. Validate no circular dependencies
    ///     // 5. Merge Cargo.toml dependencies
    ///     Ok(())
    /// }
    /// ```
    #[allow(clippy::too_many_arguments)]
    async fn execute_consolidation_post_processing(
        &self,
        _source_crate_name: &str,
        _target_crate_name: &str,
        _target_module_name: &str,
        _source_crate_path: &Path,
        _target_crate_path: &Path,
        _target_module_path: &Path,
        _project_root: &Path,
    ) -> Result<(), String> {
        // Default: no post-processing
        Ok(())
    }
}
