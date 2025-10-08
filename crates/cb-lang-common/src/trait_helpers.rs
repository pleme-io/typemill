//! Helper macros and utilities for implementing plugin traits
//!
//! This module provides macros to reduce boilerplate when implementing
//! `ImportSupport` and `WorkspaceSupport` traits.

/// Generate boilerplate for `ImportSupport` trait implementation
///
/// This macro generates the trait implementation with consistent error handling
/// that logs errors and returns fallback values.
///
/// # Example
///
/// ```rust,ignore
/// use cb_lang_common::import_support_impl;
///
/// pub struct MyImportSupport;
///
/// import_support_impl! {
///     MyImportSupport,
///     parse_imports_impl,        // Function that returns PluginResult<Vec<ImportInfo>>
///     rewrite_for_rename_impl,   // Function for rename rewriting
///     rewrite_for_move_impl      // Function for move rewriting
/// }
/// ```
#[macro_export]
macro_rules! import_support_impl {
    ($struct_name:ident) => {
        impl cb_plugin_api::ImportSupport for $struct_name {
            fn parse_imports(&self, content: &str) -> Vec<String> {
                tracing::debug!("Parsing imports from content");

                match self.parse_imports_internal(content) {
                    Ok(imports) => {
                        tracing::debug!(
                            imports_count = imports.len(),
                            "Parsed imports successfully"
                        );
                        imports
                    }
                    Err(e) => {
                        tracing::debug!(error = %e, "Failed to parse imports, returning empty list");
                        Vec::new()
                    }
                }
            }

            fn rewrite_imports_for_rename(
                &self,
                content: &str,
                old_name: &str,
                new_name: &str,
            ) -> (String, usize) {
                tracing::debug!(
                    old_name = %old_name,
                    new_name = %new_name,
                    "Rewriting imports for rename"
                );

                match self.rewrite_imports_for_rename_internal(content, old_name, new_name) {
                    Ok(result) => {
                        tracing::debug!(changes_count = result.1, "Completed import rewrite");
                        result
                    }
                    Err(e) => {
                        tracing::debug!(error = %e, "Failed to rewrite imports");
                        (content.to_string(), 0)
                    }
                }
            }

            fn rewrite_imports_for_move(
                &self,
                content: &str,
                old_path: &std::path::Path,
                new_path: &std::path::Path,
            ) -> (String, usize) {
                tracing::debug!(
                    old_path = ?old_path,
                    new_path = ?new_path,
                    "Rewriting imports for move"
                );

                match self.rewrite_imports_for_move_internal(content, old_path, new_path) {
                    Ok(result) => {
                        tracing::debug!(changes_count = result.1, "Completed import rewrite for move");
                        result
                    }
                    Err(e) => {
                        tracing::debug!(error = %e, "Failed to rewrite imports for move");
                        (content.to_string(), 0)
                    }
                }
            }

            fn contains_import(&self, content: &str, module: &str) -> bool {
                self.parse_imports(content).contains(&module.to_string())
            }

            fn add_import(&self, content: &str, module: &str) -> String {
                if self.contains_import(content, module) {
                    return content.to_string();
                }

                match self.add_import_internal(content, module) {
                    Ok(result) => result,
                    Err(e) => {
                        tracing::warn!(error = %e, "Failed to add import");
                        content.to_string()
                    }
                }
            }

            fn remove_import(&self, content: &str, module: &str) -> String {
                match self.remove_import_internal(content, module) {
                    Ok(result) => result,
                    Err(e) => {
                        tracing::warn!(error = %e, "Failed to remove import");
                        content.to_string()
                    }
                }
            }
        }
    };
}

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
pub trait ImportSupportInternal {
    /// Parse imports from content (returns Result for error handling)
    fn parse_imports_internal(&self, content: &str) -> Result<Vec<String>, String>;

    /// Rewrite imports for rename operation
    fn rewrite_imports_for_rename_internal(
        &self,
        content: &str,
        old_name: &str,
        new_name: &str,
    ) -> Result<(String, usize), String>;

    /// Rewrite imports for move operation
    fn rewrite_imports_for_move_internal(
        &self,
        content: &str,
        old_path: &std::path::Path,
        new_path: &std::path::Path,
    ) -> Result<(String, usize), String>;

    /// Add an import to the content
    fn add_import_internal(&self, content: &str, import_path: &str) -> Result<String, String>;

    /// Remove an import from the content
    fn remove_import_internal(&self, content: &str, import_path: &str) -> Result<String, String>;
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

#[cfg(test)]
mod tests {
    use super::*;

    // Example implementation for testing
    struct TestImportSupport;

    impl ImportSupportInternal for TestImportSupport {
        fn parse_imports_internal(&self, _content: &str) -> Result<Vec<String>, String> {
            Ok(vec!["import1".to_string(), "import2".to_string()])
        }

        fn rewrite_imports_for_rename_internal(
            &self,
            content: &str,
            _old_name: &str,
            _new_name: &str,
        ) -> Result<(String, usize), String> {
            Ok((content.to_string(), 1))
        }

        fn rewrite_imports_for_move_internal(
            &self,
            content: &str,
            _old_path: &std::path::Path,
            _new_path: &std::path::Path,
        ) -> Result<(String, usize), String> {
            Ok((content.to_string(), 0))
        }

        fn add_import_internal(&self, content: &str, _import_path: &str) -> Result<String, String> {
            Ok(content.to_string())
        }

        fn remove_import_internal(
            &self,
            content: &str,
            _import_path: &str,
        ) -> Result<String, String> {
            Ok(content.to_string())
        }
    }

    import_support_impl!(TestImportSupport);

    #[test]
    fn test_import_support_macro() {
        use cb_plugin_api::ImportSupport;

        let support = TestImportSupport;
        let imports = support.parse_imports("test content");

        assert_eq!(imports.len(), 2);
        assert_eq!(imports[0], "import1");
        assert_eq!(imports[1], "import2");
    }
}
