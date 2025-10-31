//! Go workspace support for manipulating `go.work` files.

use async_trait::async_trait;
use mill_plugin_api::WorkspaceSupport;

#[derive(Default)]
pub struct GoWorkspaceSupport;

#[async_trait]
impl WorkspaceSupport for GoWorkspaceSupport {
    fn add_workspace_member(&self, content: &str, member: &str) -> String {
        let member_line = format!("\tuse ./{}", member);
        if content.contains(&member_line) {
            return content.to_string();
        }

        let mut new_content = content.to_string();
        if let Some(pos) = new_content.find("use (") {
            new_content.insert_str(pos + "use (".len(), &format!("\n{}", member_line));
        } else {
            // No use block, add one
            new_content.push_str(&format!("\nuse (\n{}\n)\n", member_line));
        }
        new_content
    }

    fn remove_workspace_member(&self, content: &str, member: &str) -> String {
        let member_str = format!("./{}", member);
        content
            .lines()
            .filter(|line| !line.trim().contains(&member_str))
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn list_workspace_members(&self, content: &str) -> Vec<String> {
        let mut members = Vec::new();
        let mut in_use_block = false;

        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("use (") {
                in_use_block = true;
                continue;
            }
            if in_use_block {
                if trimmed == ")" {
                    break;
                }
                if let Some(path_str) = trimmed.strip_prefix("use ") {
                    // Single line use directive
                    if let Some(path) = path_str.strip_prefix("./") {
                        members.push(path.to_string());
                    }
                } else if let Some(path) = trimmed.strip_prefix("./") {
                    // Inside a use block
                    members.push(path.to_string());
                }
            } else if trimmed.starts_with("use ") {
                 // Single line use outside of a block
                if let Some(path_str) = trimmed.split_whitespace().nth(1) {
                    if let Some(path) = path_str.strip_prefix("./") {
                        members.push(path.to_string());
                    }
                }
            }
        }
        members
    }

    fn is_workspace_manifest(&self, content: &str) -> bool {
        // go.work is identified by the `go` directive.
        content.lines().any(|l| l.trim().starts_with("go "))
    }

    fn update_package_name(&self, content: &str, _new_name: &str) -> String {
        // go.work files do not have a package name
        content.to_string()
    }
}