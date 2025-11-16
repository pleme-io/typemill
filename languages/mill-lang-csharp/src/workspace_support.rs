//! C# workspace support for Visual Studio Solution (.sln) files.
//!
//! Handles adding, removing, and listing project references in .sln files.

use mill_plugin_api::WorkspaceSupport;
use once_cell::sync::Lazy;
use regex::Regex;
use tracing::warn;
use uuid::Uuid;

// Regex to find project entries in a .sln file.
// Example: Project("{FAE04EC0-301F-11D3-BF4B-00C04F79EFBC}") = "MyProject", "MyProject\MyProject.csproj", "{E6B4C3A6-5A25-48F1-B244-933A354E1BFB}"
static PROJECT_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"Project\("\{(?P<type_guid>[^}]+)\}"\) = "(?P<name>[^"]+)", "(?P<path>[^"]+)", "\{(?P<proj_guid>[^}]+)\}""#)
        .expect("Invalid regex for SLN projects")
});

/// C# workspace support implementation.
#[derive(Default)]
pub struct CsharpWorkspaceSupport;

impl CsharpWorkspaceSupport {
    /// Creates a new C# workspace support instance.
    pub fn new() -> Self {
        Self
    }
}

impl WorkspaceSupport for CsharpWorkspaceSupport {
    fn add_workspace_member(&self, content: &str, member_path: &str) -> String {
        let project_name = std::path::Path::new(member_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("NewProject")
            .to_string();

        let project_type_guid = "FAE04EC0-301F-11D3-BF4B-00C04F79EFBC"; // C# Project Type GUID
        let project_guid = Uuid::new_v4().to_string().to_uppercase();

        // Check if the project already exists
        if list_workspace_members_impl(content)
            .iter()
            .any(|p| p == member_path)
        {
            warn!(member = %member_path, "Project already exists in solution.");
            return content.to_string();
        }

        let mut new_lines: Vec<String> = content.lines().map(String::from).collect();
        let mut project_inserted = false;

        if let Some(global_index) = new_lines.iter().position(|l| l.trim() == "Global") {
            let new_project_line = format!(
                r#"Project("{{{}}}") = "{}", "{}", "{{{}}}""#,
                project_type_guid, project_name, member_path, &project_guid
            );
            new_lines.insert(global_index, "EndProject".to_string());
            new_lines.insert(global_index, new_project_line);
            project_inserted = true;
        }

        if !project_inserted {
            warn!("Could not find 'Global' section in .sln file. Appending to end.");
            let new_project_line = format!(
                r#"Project("{{{}}}") = "{}", "{}", "{{{}}}""#,
                project_type_guid, project_name, member_path, &project_guid
            );
            new_lines.push(new_project_line);
            new_lines.push("EndProject".to_string());
        }

        if let Some(config_section_start) = new_lines
            .iter()
            .position(|l| l.trim() == "GlobalSection(ProjectConfigurationPlatforms) = postSolution")
        {
            let new_configs = vec![
                format!(
                    "\t\t{{{}}}.Debug|Any CPU.ActiveCfg = Debug|Any CPU",
                    project_guid
                ),
                format!(
                    "\t\t{{{}}}.Debug|Any CPU.Build.0 = Debug|Any CPU",
                    project_guid
                ),
                format!(
                    "\t\t{{{}}}.Release|Any CPU.ActiveCfg = Release|Any CPU",
                    project_guid
                ),
                format!(
                    "\t\t{{{}}}.Release|Any CPU.Build.0 = Release|Any CPU",
                    project_guid
                ),
            ];
            for (i, config) in new_configs.into_iter().enumerate() {
                new_lines.insert(config_section_start + 1 + i, config);
            }
        } else {
            warn!("Could not find 'GlobalSection(ProjectConfigurationPlatforms)' in .sln file. Project may not build correctly.");
        }

        new_lines.join("\n")
    }

    fn remove_workspace_member(&self, content: &str, member_path: &str) -> String {
        let mut project_guid_to_remove = None;

        // Find the GUID of the project to remove
        for cap in PROJECT_REGEX.captures_iter(content) {
            if &cap["path"] == member_path {
                project_guid_to_remove = Some(cap["proj_guid"].to_string());
                break;
            }
        }

        let project_guid = match project_guid_to_remove {
            Some(guid) => guid,
            None => {
                warn!(member = %member_path, "Project not found in solution.");
                return content.to_string();
            }
        };

        let mut lines: Vec<String> = content.lines().map(String::from).collect();

        // Remove the Project and EndProject lines
        let project_line_index = lines.iter().position(|line| line.contains(member_path));
        if let Some(index) = project_line_index {
            lines.remove(index); // Removes "Project(...)"
            if index < lines.len() && lines[index].trim() == "EndProject" {
                lines.remove(index); // Removes "EndProject"
            }
        }

        // Remove project configurations
        lines.retain(|line| !line.contains(&project_guid));

        lines.join("\n")
    }

    fn list_workspace_members(&self, content: &str) -> Vec<String> {
        list_workspace_members_impl(content)
    }

    fn is_workspace_manifest(&self, content: &str) -> bool {
        content
            .trim()
            .starts_with("Microsoft Visual Studio Solution File")
    }

    fn update_package_name(&self, content: &str, _new_name: &str) -> String {
        warn!("Updating the solution name is not supported for .sln files.");
        content.to_string()
    }
}

fn list_workspace_members_impl(content: &str) -> Vec<String> {
    PROJECT_REGEX
        .captures_iter(content)
        .map(|cap| cap["path"].to_string())
        .collect()
}


// Unit tests deleted - functionality is covered by workspace_harness integration tests
// See: crates/mill-test-support/src/harness/workspace_harness.rs
