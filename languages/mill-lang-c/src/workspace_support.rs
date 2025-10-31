use mill_plugin_api::WorkspaceSupport;
use regex::Regex;

pub struct CWorkspaceSupport;

impl WorkspaceSupport for CWorkspaceSupport {
    fn add_workspace_member(&self, manifest_content: &str, member_path: &str) -> String {
        let re = Regex::new(r"SUBDIRS\s*=\s*(.*)").unwrap();
        if let Some(caps) = re.captures(manifest_content) {
            let existing_subdirs = caps.get(1).unwrap().as_str();
            let new_subdirs = format!("{} {}", existing_subdirs, member_path);
            manifest_content.replace(existing_subdirs, &new_subdirs)
        } else {
            format!("{}\nSUBDIRS = {}", manifest_content, member_path)
        }
    }

    fn remove_workspace_member(
        &self,
        manifest_content: &str,
        member_path: &str,
    ) -> String {
        let re = Regex::new(r"SUBDIRS\s*=\s*(.*)").unwrap();
        if let Some(caps) = re.captures(manifest_content) {
            let existing_subdirs = caps.get(1).unwrap().as_str();
            let new_subdirs: String = existing_subdirs
                .split_whitespace()
                .filter(|&s| s != member_path)
                .collect::<Vec<&str>>()
                .join(" ");
            manifest_content.replace(existing_subdirs, &new_subdirs)
        } else {
            manifest_content.to_string()
        }
    }

    fn list_workspace_members(&self, manifest_content: &str) -> Vec<String> {
        let re = Regex::new(r"SUBDIRS\s*=\s*(.*)").unwrap();
        if let Some(caps) = re.captures(manifest_content) {
            let subdirs = caps.get(1).unwrap().as_str();
            subdirs.split_whitespace().map(String::from).collect()
        } else {
            vec![]
        }
    }

    fn is_workspace_manifest(&self, content: &str) -> bool {
        content.contains("SUBDIRS")
    }

    fn update_package_name(&self, content: &str, _new_name: &str) -> String {
        // Not applicable to Makefiles, so we just return the original content
        content.to_string()
    }
}