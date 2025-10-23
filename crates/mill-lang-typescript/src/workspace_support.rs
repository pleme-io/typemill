//! TypeScript/JavaScript workspace support for npm/yarn/pnpm monorepos
//!
//! Handles workspace operations through package.json and pnpm-workspace.yaml manipulation.

use mill_plugin_api::WorkspaceSupport;
use serde_json::{json, Value};
use tracing::{debug, warn};

/// TypeScript workspace support implementation
pub struct TypeScriptWorkspaceSupport;

impl TypeScriptWorkspaceSupport {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TypeScriptWorkspaceSupport {
    fn default() -> Self {
        Self::new()
    }
}

impl WorkspaceSupport for TypeScriptWorkspaceSupport {
    fn add_workspace_member(&self, content: &str, member: &str) -> String {
        match detect_format(content) {
            WorkspaceFormat::PackageJson => match add_package_json_member(content, member) {
                Ok(result) => result,
                Err(e) => {
                    warn!(error = %e, member = %member, "Failed to add workspace member to package.json");
                    content.to_string()
                }
            },
            WorkspaceFormat::PnpmYaml => match add_pnpm_member(content, member) {
                Ok(result) => result,
                Err(e) => {
                    warn!(error = %e, member = %member, "Failed to add workspace member to pnpm-workspace.yaml");
                    content.to_string()
                }
            },
            WorkspaceFormat::Unknown => {
                warn!(format = "unknown", "Unknown workspace format");
                content.to_string()
            }
        }
    }

    fn remove_workspace_member(&self, content: &str, member: &str) -> String {
        match detect_format(content) {
            WorkspaceFormat::PackageJson => match remove_package_json_member(content, member) {
                Ok(result) => result,
                Err(e) => {
                    warn!(error = %e, member = %member, "Failed to remove workspace member from package.json");
                    content.to_string()
                }
            },
            WorkspaceFormat::PnpmYaml => match remove_pnpm_member(content, member) {
                Ok(result) => result,
                Err(e) => {
                    warn!(error = %e, member = %member, "Failed to remove workspace member from pnpm-workspace.yaml");
                    content.to_string()
                }
            },
            WorkspaceFormat::Unknown => {
                warn!(format = "unknown", "Unknown workspace format");
                content.to_string()
            }
        }
    }

    fn is_workspace_manifest(&self, content: &str) -> bool {
        match detect_format(content) {
            WorkspaceFormat::PackageJson => {
                if let Ok(parsed) = serde_json::from_str::<Value>(content) {
                    parsed.get("workspaces").is_some()
                } else {
                    false
                }
            }
            WorkspaceFormat::PnpmYaml => content.contains("packages:"),
            WorkspaceFormat::Unknown => false,
        }
    }

    fn list_workspace_members(&self, content: &str) -> Vec<String> {
        match detect_format(content) {
            WorkspaceFormat::PackageJson => list_package_json_members(content).unwrap_or_default(),
            WorkspaceFormat::PnpmYaml => list_pnpm_members(content).unwrap_or_default(),
            WorkspaceFormat::Unknown => Vec::new(),
        }
    }

    fn update_package_name(&self, content: &str, new_name: &str) -> String {
        match detect_format(content) {
            WorkspaceFormat::PackageJson => match update_package_json_name(content, new_name) {
                Ok(result) => result,
                Err(e) => {
                    warn!(error = %e, new_name = %new_name, "Failed to update package name");
                    content.to_string()
                }
            },
            WorkspaceFormat::PnpmYaml => {
                // pnpm-workspace.yaml doesn't have package names
                debug!("pnpm-workspace.yaml doesn't support package name updates");
                content.to_string()
            }
            WorkspaceFormat::Unknown => content.to_string(),
        }
    }
}

/// Workspace format detection
#[derive(Debug, PartialEq)]
enum WorkspaceFormat {
    PackageJson,
    PnpmYaml,
    Unknown,
}

/// Detect workspace format from content
fn detect_format(content: &str) -> WorkspaceFormat {
    let trimmed = content.trim_start();

    if trimmed.starts_with('{') {
        WorkspaceFormat::PackageJson
    } else if trimmed.contains("packages:") {
        WorkspaceFormat::PnpmYaml
    } else {
        WorkspaceFormat::Unknown
    }
}

// ============================================================================
// package.json Operations
// ============================================================================

/// Add member to package.json workspaces
fn add_package_json_member(content: &str, member: &str) -> Result<String, String> {
    let mut parsed: Value = serde_json::from_str(content)
        .map_err(|e| format!("Failed to parse package.json: {}", e))?;

    // Get or create workspaces field
    let workspaces = parsed.get_mut("workspaces");

    match workspaces {
        Some(Value::Array(arr)) => {
            // Array format: "workspaces": ["packages/*"]
            if !arr.iter().any(|v| v.as_str() == Some(member)) {
                arr.push(json!(member));
            } else {
                debug!(member = %member, "Member already exists in workspace");
                return Ok(content.to_string());
            }
        }
        Some(Value::Object(obj)) => {
            // Object format (Yarn v1): "workspaces": { "packages": [...] }
            if let Some(Value::Array(packages)) = obj.get_mut("packages") {
                if !packages.iter().any(|v| v.as_str() == Some(member)) {
                    packages.push(json!(member));
                } else {
                    debug!(member = %member, "Member already exists in workspace");
                    return Ok(content.to_string());
                }
            } else {
                return Err("workspaces.packages is not an array".to_string());
            }
        }
        None => {
            // Create workspaces array
            parsed["workspaces"] = json!([member]);
        }
        _ => {
            return Err("Invalid workspaces format".to_string());
        }
    }

    serde_json::to_string_pretty(&parsed)
        .map_err(|e| format!("Failed to serialize package.json: {}", e))
}

/// Remove member from package.json workspaces
fn remove_package_json_member(content: &str, member: &str) -> Result<String, String> {
    let mut parsed: Value = serde_json::from_str(content)
        .map_err(|e| format!("Failed to parse package.json: {}", e))?;

    let workspaces = parsed.get_mut("workspaces");

    match workspaces {
        Some(Value::Array(arr)) => {
            arr.retain(|v| v.as_str() != Some(member));
        }
        Some(Value::Object(obj)) => {
            if let Some(Value::Array(packages)) = obj.get_mut("packages") {
                packages.retain(|v| v.as_str() != Some(member));
            }
        }
        _ => {
            debug!(member = %member, "Member not found in workspace");
            return Ok(content.to_string());
        }
    }

    serde_json::to_string_pretty(&parsed)
        .map_err(|e| format!("Failed to serialize package.json: {}", e))
}

/// List members from package.json workspaces
fn list_package_json_members(content: &str) -> Result<Vec<String>, String> {
    let parsed: Value = serde_json::from_str(content)
        .map_err(|e| format!("Failed to parse package.json: {}", e))?;

    let workspaces = parsed.get("workspaces");

    match workspaces {
        Some(Value::Array(arr)) => Ok(arr
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect()),
        Some(Value::Object(obj)) => {
            if let Some(Value::Array(packages)) = obj.get("packages") {
                Ok(packages
                    .iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect())
            } else {
                Ok(Vec::new())
            }
        }
        _ => Ok(Vec::new()),
    }
}

/// Update package name in package.json
fn update_package_json_name(content: &str, new_name: &str) -> Result<String, String> {
    let mut parsed: Value = serde_json::from_str(content)
        .map_err(|e| format!("Failed to parse package.json: {}", e))?;

    parsed["name"] = json!(new_name);

    serde_json::to_string_pretty(&parsed)
        .map_err(|e| format!("Failed to serialize package.json: {}", e))
}

// ============================================================================
// pnpm-workspace.yaml Operations
// ============================================================================

/// Add member to pnpm-workspace.yaml
fn add_pnpm_member(content: &str, member: &str) -> Result<String, String> {
    let members = list_pnpm_members(content)?;

    if members.iter().any(|m| m == member) {
        debug!(member = %member, "Member already exists in pnpm workspace");
        return Ok(content.to_string());
    }

    // Parse and add member
    let mut result = String::new();
    let mut in_packages = false;
    let mut added = false;

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("packages:") {
            in_packages = true;
            result.push_str(line);
            result.push('\n');
            continue;
        }

        // Check if we're leaving packages section
        if in_packages
            && !trimmed.is_empty()
            && !trimmed.starts_with('-')
            && !trimmed.starts_with('#')
        {
            // Add member before leaving section
            if !added {
                result.push_str(&format!("  - '{}'\n", member));
                added = true;
            }
            in_packages = false;
        }

        // Add member at end if still in packages section
        if in_packages && line.trim().is_empty() && !added {
            result.push_str(&format!("  - '{}'\n", member));
            added = true;
        }

        result.push_str(line);
        result.push('\n');
    }

    // If we never added it, add at the end
    if !added {
        if !result.ends_with('\n') {
            result.push('\n');
        }
        result.push_str(&format!("  - '{}'\n", member));
    }

    Ok(result)
}

/// Remove member from pnpm-workspace.yaml
fn remove_pnpm_member(content: &str, member: &str) -> Result<String, String> {
    let mut result = String::new();
    let member_line = format!("- '{}'", member);
    let member_line_alt = format!("- \"{}\"", member);

    for line in content.lines() {
        let trimmed = line.trim();

        // Skip lines that match the member
        if trimmed == member_line
            || trimmed == member_line_alt
            || trimmed == format!("- {}", member)
        {
            continue;
        }

        result.push_str(line);
        result.push('\n');
    }

    Ok(result)
}

/// List members from pnpm-workspace.yaml
fn list_pnpm_members(content: &str) -> Result<Vec<String>, String> {
    let mut members = Vec::new();
    let mut in_packages = false;

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("packages:") {
            in_packages = true;
            continue;
        }

        // Check if we're leaving packages section
        if in_packages
            && !trimmed.is_empty()
            && !trimmed.starts_with('-')
            && !trimmed.starts_with('#')
        {
            break;
        }

        // Parse member line
        if in_packages && trimmed.starts_with('-') {
            let member = trimmed
                .trim_start_matches('-')
                .trim()
                .trim_matches('\'')
                .trim_matches('"')
                .to_string();

            if !member.is_empty() {
                members.push(member);
            }
        }
    }

    Ok(members)
}

#[cfg(test)]
mod tests {
    use super::*;

    const NPM_WORKSPACE: &str = r#"{
  "name": "my-monorepo",
  "private": true,
  "workspaces": [
    "packages/*",
    "apps/*"
  ]
}"#;

    const YARN_OBJECT_WORKSPACE: &str = r#"{
  "name": "my-monorepo",
  "private": true,
  "workspaces": {
    "packages": [
      "packages/*"
    ],
    "nohoist": [
      "**/react-native"
    ]
  }
}"#;

    const PNPM_WORKSPACE: &str = r#"packages:
  - 'packages/*'
  - 'apps/*'
"#;

    const SINGLE_PACKAGE: &str = r#"{
  "name": "single-package",
  "version": "1.0.0"
}"#;

    #[test]
    fn test_detect_format() {
        assert_eq!(detect_format(NPM_WORKSPACE), WorkspaceFormat::PackageJson);
        assert_eq!(detect_format(PNPM_WORKSPACE), WorkspaceFormat::PnpmYaml);
        assert_eq!(detect_format("invalid"), WorkspaceFormat::Unknown);
    }

    #[test]
    fn test_is_workspace_manifest() {
        let support = TypeScriptWorkspaceSupport::new();

        assert!(support.is_workspace_manifest(NPM_WORKSPACE));
        assert!(support.is_workspace_manifest(YARN_OBJECT_WORKSPACE));
        assert!(support.is_workspace_manifest(PNPM_WORKSPACE));
        assert!(!support.is_workspace_manifest(SINGLE_PACKAGE));
    }

    #[test]
    fn test_list_npm_workspace_members() {
        let support = TypeScriptWorkspaceSupport::new();

        let members = support.list_workspace_members(NPM_WORKSPACE);
        assert_eq!(members.len(), 2);
        assert!(members.contains(&"packages/*".to_string()));
        assert!(members.contains(&"apps/*".to_string()));
    }

    #[test]
    fn test_list_yarn_object_workspace_members() {
        let support = TypeScriptWorkspaceSupport::new();

        let members = support.list_workspace_members(YARN_OBJECT_WORKSPACE);
        assert_eq!(members.len(), 1);
        assert!(members.contains(&"packages/*".to_string()));
    }

    #[test]
    fn test_list_pnpm_workspace_members() {
        let support = TypeScriptWorkspaceSupport::new();

        let members = support.list_workspace_members(PNPM_WORKSPACE);
        assert_eq!(members.len(), 2);
        assert!(members.contains(&"packages/*".to_string()));
        assert!(members.contains(&"apps/*".to_string()));
    }

    #[test]
    fn test_add_npm_workspace_member() {
        let support = TypeScriptWorkspaceSupport::new();

        let result = support.add_workspace_member(NPM_WORKSPACE, "tools/*");
        assert!(result.contains("\"tools/*\""));

        let members = support.list_workspace_members(&result);
        assert_eq!(members.len(), 3);
        assert!(members.contains(&"tools/*".to_string()));
    }

    #[test]
    fn test_add_pnpm_workspace_member() {
        let support = TypeScriptWorkspaceSupport::new();

        let result = support.add_workspace_member(PNPM_WORKSPACE, "tools/*");
        assert!(result.contains("'tools/*'"));

        let members = support.list_workspace_members(&result);
        assert_eq!(members.len(), 3);
        assert!(members.contains(&"tools/*".to_string()));
    }

    #[test]
    fn test_remove_npm_workspace_member() {
        let support = TypeScriptWorkspaceSupport::new();

        let result = support.remove_workspace_member(NPM_WORKSPACE, "apps/*");
        assert!(!result.contains("\"apps/*\""));

        let members = support.list_workspace_members(&result);
        assert_eq!(members.len(), 1);
        assert!(members.contains(&"packages/*".to_string()));
    }

    #[test]
    fn test_remove_pnpm_workspace_member() {
        let support = TypeScriptWorkspaceSupport::new();

        let result = support.remove_workspace_member(PNPM_WORKSPACE, "apps/*");
        assert!(!result.contains("'apps/*'"));

        let members = support.list_workspace_members(&result);
        assert_eq!(members.len(), 1);
        assert!(members.contains(&"packages/*".to_string()));
    }

    #[test]
    fn test_update_package_name() {
        let support = TypeScriptWorkspaceSupport::new();

        let result = support.update_package_name(SINGLE_PACKAGE, "new-package");
        assert!(result.contains("\"new-package\""));
        assert!(!result.contains("\"single-package\""));
    }

    #[test]
    fn test_add_duplicate_member() {
        let support = TypeScriptWorkspaceSupport::new();

        let result = support.add_workspace_member(NPM_WORKSPACE, "packages/*");
        let members = support.list_workspace_members(&result);
        assert_eq!(members.len(), 2); // Should not duplicate
    }
}