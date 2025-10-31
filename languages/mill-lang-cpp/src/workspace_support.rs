//! Workspace support for C++ projects
//!
//! Provides stub implementations for workspace operations in C++ projects.
//! C++ build systems (CMake, Bazel, etc.) don't have a standard workspace concept
//! like Rust's Cargo workspaces, so most operations are not applicable.

use async_trait::async_trait;
use mill_plugin_api::workspace_support::{MoveManifestPlan, WorkspaceSupport};
use std::path::Path;

pub struct CppWorkspaceSupport;

#[async_trait]
impl WorkspaceSupport for CppWorkspaceSupport {
    fn add_workspace_member(&self, content: &str, member: &str) -> String {
        format!("{}\nadd_subdirectory({})", content, member)
    }

    fn remove_workspace_member(&self, content: &str, member: &str) -> String {
        let re = regex::Regex::new(&format!(r"\n?add_subdirectory\({}\)", regex::escape(member))).unwrap();
        re.replace_all(content, "").to_string()
    }

    fn is_workspace_manifest(&self, _content: &str) -> bool {
        // CMakeLists.txt at root with add_subdirectory could be considered a workspace
        // But this is too simplistic for now
        false
    }

    fn list_workspace_members(&self, _content: &str) -> Vec<String> {
        // Would need to parse add_subdirectory() calls in CMakeLists.txt
        // Not implemented yet
        Vec::new()
    }

    fn update_package_name(&self, content: &str, new_name: &str) -> String {
        // Update project() declaration in CMakeLists.txt
        // Simple regex-based replacement
        let re = regex::Regex::new(r"project\s*\(\s*(\w+)").unwrap();
        re.replace(content, format!("project({})", new_name))
            .to_string()
    }

    async fn is_package(&self, dir_path: &Path) -> bool {
        // Check for CMakeLists.txt presence
        dir_path.join("CMakeLists.txt").exists()
    }

    async fn plan_directory_move(
        &self,
        _old_path: &Path,
        _new_path: &Path,
        _project_root: &Path,
    ) -> Option<MoveManifestPlan> {
        // Not implemented - would require parsing CMakeLists.txt add_subdirectory calls
        None
    }

    async fn generate_workspace_manifest(
        &self,
        _member_paths: &[&str],
        _workspace_root: &Path,
    ) -> Result<String, String> {
        Err("C++ workspace manifests not yet supported".to_string())
    }
}
