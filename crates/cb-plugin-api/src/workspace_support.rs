//! Workspace support trait for language plugins
//!
//! Provides workspace manifest manipulation capabilities for languages
//! with multi-package project structures (Rust workspaces, TypeScript monorepos, etc.).

/// Optional trait for languages that support workspace operations
///
/// All methods are **synchronous** (no async overhead).
/// Plugins store an implementation in their struct and return `&dyn WorkspaceSupport`
/// from the main `LanguagePlugin::workspace_support()` method.
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
}
