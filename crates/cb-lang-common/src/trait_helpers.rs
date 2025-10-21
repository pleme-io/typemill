//! Helper macros and utilities for implementing plugin traits
//!
//! This module provides macros to reduce boilerplate when implementing
//! `WorkspaceSupport` trait.

/// Generate boilerplate for `WorkspaceSupport` trait implementation
///
/// This macro generates the trait implementation with consistent error handling
/// that logs errors and returns fallback values.
///
/// # Example
///
/// ```rust,ignore
/// use cb_lang_common::workspace_support_impl;
///
/// pub struct MyWorkspaceSupport;
///
/// workspace_support_impl!(MyWorkspaceSupport);
/// ```
#[macro_export]
macro_rules! workspace_support_impl {
    ($struct_name:ident) => {
        impl cb_plugin_api::WorkspaceSupport for $struct_name {
            fn add_workspace_member(&self, content: &str, member: &str) -> String {
                tracing::debug!(member = %member, "Adding workspace member");

                match self.add_workspace_member_internal(content, member) {
                    Ok(result) => {
                        tracing::debug!(member = %member, "Added workspace member");
                        result
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, member = %member, "Failed to add workspace member");
                        content.to_string()
                    }
                }
            }

            fn remove_workspace_member(&self, content: &str, member: &str) -> String {
                tracing::debug!(member = %member, "Removing workspace member");

                match self.remove_workspace_member_internal(content, member) {
                    Ok(result) => {
                        tracing::debug!(member = %member, "Removed workspace member");
                        result
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, member = %member, "Failed to remove workspace member");
                        content.to_string()
                    }
                }
            }

            fn is_workspace_manifest(&self, content: &str) -> bool {
                self.is_workspace_manifest_internal(content)
            }

            fn list_workspace_members(&self, content: &str) -> Vec<String> {
                tracing::debug!("Listing workspace members");

                match self.list_workspace_members_internal(content) {
                    Ok(members) => {
                        tracing::debug!(members_count = members.len(), "Found workspace members");
                        members
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, "Failed to list workspace members");
                        Vec::new()
                    }
                }
            }

            fn update_package_name(&self, content: &str, new_name: &str) -> String {
                tracing::debug!(new_name = %new_name, "Updating package name");

                match self.update_package_name_internal(content, new_name) {
                    Ok(result) => {
                        tracing::debug!(new_name = %new_name, "Updated package name");
                        result
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, new_name = %new_name, "Failed to update package name");
                        content.to_string()
                    }
                }
            }

            fn merge_dependencies(&self, base: &str, source: &str) -> String {
                tracing::debug!("Merging dependencies");

                match self.merge_dependencies_internal(base, source) {
                    Ok(result) => {
                        tracing::debug!("Merged dependencies successfully");
                        result
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, "Failed to merge dependencies");
                        base.to_string()
                    }
                }
            }
        }
    };
}

/// Helper trait that language plugins should implement to work with the macros
///
/// This trait defines the "internal" methods that do the actual work,
/// while the macros generate the boilerplate trait implementations.
pub trait WorkspaceSupportInternal {
    /// Add a workspace member
    fn add_workspace_member_internal(&self, content: &str, member: &str) -> Result<String, String>;

    /// Remove a workspace member
    fn remove_workspace_member_internal(
        &self,
        content: &str,
        member: &str,
    ) -> Result<String, String>;

    /// Check if content represents a workspace manifest
    fn is_workspace_manifest_internal(&self, content: &str) -> bool;

    /// List all workspace members
    fn list_workspace_members_internal(&self, content: &str) -> Result<Vec<String>, String>;

    /// Update the package/module name
    fn update_package_name_internal(&self, content: &str, new_name: &str)
        -> Result<String, String>;

    /// Merge dependencies from source manifest into base manifest
    fn merge_dependencies_internal(&self, base: &str, source: &str) -> Result<String, String>;
}
