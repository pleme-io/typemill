use lazy_static::lazy_static;
use mill_plugin_api::WorkspaceSupport;
use regex::Regex;

#[derive(Default, Clone)]
pub struct SwiftWorkspaceSupport;

lazy_static! {
    static ref DEPS_ARRAY_REGEX: Regex =
        Regex::new(r"(?s)dependencies:\s*\[(.*?)\]").expect("Invalid regex for deps array");
    static ref WORKSPACE_MANIFEST_REGEX: Regex =
        Regex::new(r#"\.package\s*\(\s*path:"#).expect("Invalid regex for workspace manifest");
    static ref PKG_NAME_REGEX: Regex =
        Regex::new(r#"(name:\s*")([^"]+)""#).expect("Invalid regex for package name");
    static ref LIST_MEMBERS_REGEX: Regex =
        Regex::new(r#"\.package\s*\(\s*path:\s*"([^"]+)"\s*\)"#)
            .expect("Invalid regex for list members");
}

impl SwiftWorkspaceSupport {
    fn find_dependencies_array<'a>(&self, content: &'a str) -> Option<regex::Match<'a>> {
        DEPS_ARRAY_REGEX.find(content)
    }
}

impl WorkspaceSupport for SwiftWorkspaceSupport {
    fn is_workspace_manifest(&self, content: &str) -> bool {
        WORKSPACE_MANIFEST_REGEX.is_match(content)
    }

    fn update_package_name(&self, content: &str, new_name: &str) -> String {
        PKG_NAME_REGEX
            .replace_all(content, |caps: &regex::Captures| {
                format!(r#"{}{}""#, &caps[1], new_name)
            })
            .to_string()
    }

    fn add_workspace_member(&self, content: &str, member_path: &str) -> String {
        let mut new_content = content.to_string();
        let new_package_line = format!("\n        .package(path: \"{}\"),", member_path);

        if let Some(deps_match) = self.find_dependencies_array(&new_content) {
            let end_pos = deps_match.end() - 1; // Before the closing ']'
            new_content.insert_str(end_pos, &new_package_line);
        } else {
            // If no dependencies array, we can't add the member.
            // Returning original content as we can't signal an error here.
        }
        new_content
    }

    fn remove_workspace_member(&self, content: &str, member_path: &str) -> String {
        let pattern = format!(
            r#"(?m)^\s*\.package\s*\(\s*path:\s*"{}"\s*\),?\s*[\r\n]?"#,
            regex::escape(member_path)
        );
        if let Ok(re) = Regex::new(&pattern) {
            re.replace_all(content, "").to_string()
        } else {
            content.to_string()
        }
    }

    fn list_workspace_members(&self, content: &str) -> Vec<String> {
        LIST_MEMBERS_REGEX
            .captures_iter(content)
            .map(|cap| cap[1].to_string())
            .collect()
    }
}