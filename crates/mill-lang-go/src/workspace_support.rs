//! Go workspace support for go.work files
//!
//! Handles Go workspace operations through go.work file manipulation.
//! Go workspaces were introduced in Go 1.18 and use a simple text format.

use mill_plugin_api::WorkspaceSupport;
use tracing::{debug, warn};

/// Go workspace support implementation
pub struct GoWorkspaceSupport;

impl GoWorkspaceSupport {
    pub fn new() -> Self {
        Self
    }
}

impl Default for GoWorkspaceSupport {
    fn default() -> Self {
        Self::new()
    }
}

impl WorkspaceSupport for GoWorkspaceSupport {
    fn add_workspace_member(&self, content: &str, member: &str) -> String {
        match add_workspace_member_impl(content, member) {
            Ok(result) => result,
            Err(e) => {
                warn!(error = %e, member = %member, "Failed to add workspace member");
                content.to_string()
            }
        }
    }

    fn remove_workspace_member(&self, content: &str, member: &str) -> String {
        match remove_workspace_member_impl(content, member) {
            Ok(result) => result,
            Err(e) => {
                warn!(error = %e, member = %member, "Failed to remove workspace member");
                content.to_string()
            }
        }
    }

    fn is_workspace_manifest(&self, content: &str) -> bool {
        is_workspace_manifest_impl(content)
    }

    fn list_workspace_members(&self, content: &str) -> Vec<String> {
        match list_workspace_members_impl(content) {
            Ok(members) => members,
            Err(e) => {
                warn!(error = %e, "Failed to list workspace members");
                Vec::new()
            }
        }
    }

    fn update_package_name(&self, content: &str, _new_name: &str) -> String {
        // Go workspaces don't have a package name in go.work
        // The workspace file only contains module paths
        debug!("Go workspaces don't support package name updates in go.work");
        content.to_string()
    }
}

/// Check if content is a Go workspace manifest (has use directive)
fn is_workspace_manifest_impl(content: &str) -> bool {
    content.contains("use (") || content.contains("use(")
}

/// Parse use block and return list of module paths
fn list_workspace_members_impl(content: &str) -> Result<Vec<String>, String> {
    let mut members = Vec::new();
    let mut in_use_block = false;

    for line in content.lines() {
        let trimmed = line.trim();

        // Start of use block
        if trimmed.starts_with("use") && trimmed.contains('(') {
            in_use_block = true;
            // Handle single-line use (rare but possible)
            if trimmed.contains(')') {
                let between = trimmed
                    .trim_start_matches("use")
                    .trim_start_matches('(')
                    .trim_end_matches(')')
                    .trim();
                if !between.is_empty() {
                    members.push(normalize_path(between));
                }
                in_use_block = false;
            }
            continue;
        }

        // End of use block
        if in_use_block && trimmed == ")" {
            break;
        }

        // Inside use block - extract module path
        if in_use_block && !trimmed.is_empty() && !trimmed.starts_with("//") {
            // Remove inline comments
            let path = trimmed.split("//").next().unwrap_or(trimmed).trim();
            if !path.is_empty() {
                members.push(normalize_path(path));
            }
        }
    }

    Ok(members)
}

/// Add a workspace member to go.work
fn add_workspace_member_impl(content: &str, member: &str) -> Result<String, String> {
    let members = list_workspace_members_impl(content)?;

    // Check if already exists
    let normalized_member = normalize_path(member);
    if members.iter().any(|m| m == &normalized_member) {
        debug!(member = %member, "Member already exists in workspace");
        return Ok(content.to_string());
    }

    // Find use block and insert
    let mut result = String::new();
    let mut in_use_block = false;
    let mut added = false;

    for line in content.lines() {
        let trimmed = line.trim();

        // Start of use block
        if trimmed.starts_with("use") && trimmed.contains('(') {
            in_use_block = true;
            result.push_str(line);
            result.push('\n');
            continue;
        }

        // End of use block - add member before closing paren
        if in_use_block && trimmed == ")" && !added {
            result.push_str(&format!("\t{}\n", normalized_member));
            added = true;
        }

        result.push_str(line);
        result.push('\n');

        // End of use block
        if in_use_block && trimmed == ")" {
            in_use_block = false;
        }
    }

    // If no use block exists, create one
    if !added {
        result = create_workspace_with_member(content, &normalized_member);
    }

    Ok(result)
}

/// Remove a workspace member from go.work
fn remove_workspace_member_impl(content: &str, member: &str) -> Result<String, String> {
    let members = list_workspace_members_impl(content)?;
    let normalized_member = normalize_path(member);

    if !members.iter().any(|m| m == &normalized_member) {
        debug!(member = %member, "Member not found in workspace");
        return Ok(content.to_string());
    }

    // Reconstruct without the member
    let mut result = String::new();
    let mut in_use_block = false;

    for line in content.lines() {
        let trimmed = line.trim();

        // Start of use block
        if trimmed.starts_with("use") && trimmed.contains('(') {
            in_use_block = true;
            result.push_str(line);
            result.push('\n');
            continue;
        }

        // End of use block
        if in_use_block && trimmed == ")" {
            in_use_block = false;
            result.push_str(line);
            result.push('\n');
            continue;
        }

        // Inside use block - skip the member to remove
        if in_use_block {
            let path = trimmed.split("//").next().unwrap_or(trimmed).trim();
            if normalize_path(path) == normalized_member {
                // Skip this line (it's the member we're removing)
                continue;
            }
        }

        result.push_str(line);
        result.push('\n');
    }

    Ok(result)
}

/// Normalize module path (ensure ./ prefix for relative paths)
fn normalize_path(path: &str) -> String {
    let path = path.trim();

    // Already has ./ or ../
    if path.starts_with("./") || path.starts_with("../") {
        return path.to_string();
    }

    // Absolute path
    if path.starts_with('/') {
        return path.to_string();
    }

    // Add ./ prefix for relative paths
    format!("./{}", path)
}

/// Create a new workspace with a member (when no use block exists)
fn create_workspace_with_member(content: &str, member: &str) -> String {
    // Extract go version if present
    let go_version = extract_go_version(content).unwrap_or("1.21");

    let mut result = String::new();
    let mut go_line_added = false;

    for line in content.lines() {
        if line.trim().starts_with("go ") {
            result.push_str(line);
            result.push('\n');
            go_line_added = true;
        } else {
            result.push_str(line);
            result.push('\n');
        }
    }

    // If no go version line, add it
    if !go_line_added {
        result = format!("go {}\n\n{}", go_version, result);
    }

    // Add use block
    result.push_str(&format!("\nuse (\n\t{}\n)\n", member));

    result
}

/// Extract Go version from go.work content
fn extract_go_version(content: &str) -> Option<&str> {
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("go ") {
            return Some(trimmed.trim_start_matches("go ").trim());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    const SIMPLE_WORKSPACE: &str = r#"go 1.21

use (
	./module-a
	./module-b
)
"#;

    const WORKSPACE_WITH_REPLACE: &str = r#"go 1.21

use (
	./api
)

replace example.com/old => example.com/new v1.2.3
"#;

    const WORKSPACE_WITH_COMMENTS: &str = r#"go 1.21

use (
	./api      // Core API module
	./workers  // Background workers
)
"#;

    const NON_WORKSPACE: &str = r#"go 1.21

replace example.com/old => example.com/new v1.2.3
"#;

    #[test]
    fn test_is_workspace_manifest() {
        let support = GoWorkspaceSupport::new();

        assert!(support.is_workspace_manifest(SIMPLE_WORKSPACE));
        assert!(support.is_workspace_manifest(WORKSPACE_WITH_REPLACE));
        assert!(support.is_workspace_manifest(WORKSPACE_WITH_COMMENTS));
        assert!(!support.is_workspace_manifest(NON_WORKSPACE));
    }

    #[test]
    fn test_list_workspace_members() {
        let support = GoWorkspaceSupport::new();

        let members = support.list_workspace_members(SIMPLE_WORKSPACE);
        assert_eq!(members.len(), 2);
        assert!(members.contains(&"./module-a".to_string()));
        assert!(members.contains(&"./module-b".to_string()));

        let empty = support.list_workspace_members(NON_WORKSPACE);
        assert_eq!(empty.len(), 0);
    }

    #[test]
    fn test_list_workspace_members_with_comments() {
        let support = GoWorkspaceSupport::new();

        let members = support.list_workspace_members(WORKSPACE_WITH_COMMENTS);
        assert_eq!(members.len(), 2);
        assert!(members.contains(&"./api".to_string()));
        assert!(members.contains(&"./workers".to_string()));
    }

    #[test]
    fn test_add_workspace_member() {
        let support = GoWorkspaceSupport::new();

        let result = support.add_workspace_member(SIMPLE_WORKSPACE, "./module-c");
        assert!(result.contains("./module-c"));

        let members = support.list_workspace_members(&result);
        assert_eq!(members.len(), 3);
        assert!(members.contains(&"./module-c".to_string()));
    }

    #[test]
    fn test_add_duplicate_member() {
        let support = GoWorkspaceSupport::new();

        let result = support.add_workspace_member(SIMPLE_WORKSPACE, "./module-a");
        let members = support.list_workspace_members(&result);
        assert_eq!(members.len(), 2); // Should not duplicate
    }

    #[test]
    fn test_remove_workspace_member() {
        let support = GoWorkspaceSupport::new();

        let result = support.remove_workspace_member(SIMPLE_WORKSPACE, "./module-b");
        assert!(!result.contains("./module-b"));

        let members = support.list_workspace_members(&result);
        assert_eq!(members.len(), 1);
        assert!(members.contains(&"./module-a".to_string()));
    }

    #[test]
    fn test_preserve_replace_directives() {
        let support = GoWorkspaceSupport::new();

        let result = support.add_workspace_member(WORKSPACE_WITH_REPLACE, "./workers");
        assert!(result.contains("replace example.com/old => example.com/new v1.2.3"));
        assert!(result.contains("./workers"));
    }

    #[test]
    fn test_create_workspace_from_scratch() {
        let support = GoWorkspaceSupport::new();

        let empty_file = "go 1.21\n";
        let result = support.add_workspace_member(empty_file, "./my-module");

        assert!(result.contains("use ("));
        assert!(result.contains("./my-module"));
        assert!(result.contains("go 1.21"));

        let members = support.list_workspace_members(&result);
        assert_eq!(members.len(), 1);
        assert_eq!(members[0], "./my-module");
    }

    #[test]
    fn test_normalize_path() {
        assert_eq!(normalize_path("./module"), "./module");
        assert_eq!(normalize_path("module"), "./module");
        assert_eq!(normalize_path("../external"), "../external");
        assert_eq!(normalize_path("/absolute/path"), "/absolute/path");
    }

    #[test]
    fn test_update_package_name_noop() {
        let support = GoWorkspaceSupport::new();

        // Go workspaces don't support package names
        let result = support.update_package_name(SIMPLE_WORKSPACE, "new-name");
        assert_eq!(result, SIMPLE_WORKSPACE);
    }
}
