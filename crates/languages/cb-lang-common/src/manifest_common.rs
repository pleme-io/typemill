//! Common manifest parsing and manipulation utilities
//!
//! This module provides shared functionality for working with different
//! manifest formats (TOML, JSON, YAML, XML) across language plugins.

use cb_plugin_api::{PluginError, PluginResult};
use serde_json::Value as JsonValue;
use toml_edit::{value, Array, DocumentMut, Item};
use tracing::{debug, warn};

/// TOML-based workspace utilities (for Rust Cargo.toml, Python pyproject.toml)
pub struct TomlWorkspace;

impl TomlWorkspace {
    /// Add a member to a TOML workspace
    ///
    /// Supports both Cargo-style and Poetry/PDM-style workspace sections
    pub fn add_member(content: &str, member: &str) -> PluginResult<String> {
        let mut doc = content
            .parse::<DocumentMut>()
            .map_err(|e| PluginError::parse(format!("Failed to parse TOML: {}", e)))?;

        // Try Cargo-style [workspace] first
        if let Some(workspace) = doc.get_mut("workspace") {
            if let Some(members_item) = workspace.get_mut("members") {
                if let Some(members) = members_item.as_array_mut() {
                    // Check if member already exists
                    let member_exists = members
                        .iter()
                        .any(|v| v.as_str() == Some(member));

                    if !member_exists {
                        members.push(member);
                        debug!(member = %member, "Added member to workspace");
                    }
                }
            } else {
                // Create members array
                let mut members = Array::new();
                members.push(member);
                workspace["members"] = value(members);
            }

            return Ok(doc.to_string());
        }

        // Try Poetry-style [tool.poetry.workspace] or PDM-style
        if let Some(tool) = doc.get_mut("tool") {
            if let Some(poetry) = tool.get_mut("poetry") {
                if let Some(workspace) = poetry.get_mut("workspace") {
                    if let Some(members_item) = workspace.get_mut("members") {
                        if let Some(members) = members_item.as_array_mut() {
                            let member_exists = members
                                .iter()
                                .any(|v| v.as_str() == Some(member));

                            if !member_exists {
                                members.push(member);
                            }
                            return Ok(doc.to_string());
                        }
                    }
                }
            }
        }

        // No workspace section found, return original
        Ok(content.to_string())
    }

    /// Remove a member from a TOML workspace
    pub fn remove_member(content: &str, member: &str) -> PluginResult<String> {
        let mut doc = content
            .parse::<DocumentMut>()
            .map_err(|e| PluginError::parse(format!("Failed to parse TOML: {}", e)))?;

        let mut modified = false;

        // Check Cargo-style [workspace]
        if let Some(workspace) = doc.get_mut("workspace") {
            if let Some(members_item) = workspace.get_mut("members") {
                if let Some(members) = members_item.as_array_mut() {
                    members.retain(|v| v.as_str() != Some(member));
                    modified = true;
                }
            }
        }

        // Check Poetry-style [tool.poetry.workspace]
        if let Some(tool) = doc.get_mut("tool") {
            if let Some(poetry) = tool.get_mut("poetry") {
                if let Some(workspace) = poetry.get_mut("workspace") {
                    if let Some(members_item) = workspace.get_mut("members") {
                        if let Some(members) = members_item.as_array_mut() {
                            members.retain(|v| v.as_str() != Some(member));
                            modified = true;
                        }
                    }
                }
            }
        }

        if modified {
            debug!(member = %member, "Removed member from workspace");
        }

        Ok(doc.to_string())
    }

    /// List all workspace members from TOML
    pub fn list_members(content: &str) -> PluginResult<Vec<String>> {
        let doc = content
            .parse::<DocumentMut>()
            .map_err(|e| PluginError::parse(format!("Failed to parse TOML: {}", e)))?;

        let mut members = Vec::new();

        // Check Cargo-style [workspace]
        if let Some(workspace) = doc.get("workspace") {
            if let Some(members_item) = workspace.get("members") {
                if let Some(arr) = members_item.as_array() {
                    for item in arr.iter() {
                        if let Some(member) = item.as_str() {
                            members.push(member.to_string());
                        }
                    }
                }
            }
        }

        // Check Poetry-style [tool.poetry.workspace]
        if members.is_empty() {
            if let Some(tool) = doc.get("tool") {
                if let Some(poetry) = tool.get("poetry") {
                    if let Some(workspace) = poetry.get("workspace") {
                        if let Some(members_item) = workspace.get("members") {
                            if let Some(arr) = members_item.as_array() {
                                for item in arr.iter() {
                                    if let Some(member) = item.as_str() {
                                        members.push(member.to_string());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(members)
    }

    /// Check if TOML content is a workspace manifest
    pub fn is_workspace(content: &str) -> bool {
        content.contains("[workspace]")
            || content.contains("[tool.poetry.workspace]")
            || content.contains("[tool.pdm]")
    }

    /// Update package name in TOML manifest
    pub fn update_package_name(content: &str, new_name: &str) -> PluginResult<String> {
        let mut doc = content
            .parse::<DocumentMut>()
            .map_err(|e| PluginError::parse(format!("Failed to parse TOML: {}", e)))?;

        // Update [package.name] (Cargo)
        if let Some(package) = doc.get_mut("package") {
            package["name"] = value(new_name);
            return Ok(doc.to_string());
        }

        // Update [project.name] (PEP 621 / PDM)
        if let Some(project) = doc.get_mut("project") {
            project["name"] = value(new_name);
            return Ok(doc.to_string());
        }

        // Update [tool.poetry.name] (Poetry)
        if let Some(tool) = doc.get_mut("tool") {
            if let Some(poetry) = tool.get_mut("poetry") {
                poetry["name"] = value(new_name);
                return Ok(doc.to_string());
            }
        }

        Ok(content.to_string())
    }

    /// Merge dependencies from source TOML into base TOML
    pub fn merge_dependencies(base: &str, source: &str) -> PluginResult<String> {
        let mut base_doc = base
            .parse::<DocumentMut>()
            .map_err(|e| PluginError::parse(format!("Failed to parse base TOML: {}", e)))?;

        let source_doc = source
            .parse::<DocumentMut>()
            .map_err(|e| PluginError::parse(format!("Failed to parse source TOML: {}", e)))?;

        // Merge [dependencies] section (Cargo-style)
        if let Some(source_deps) = source_doc.get("dependencies") {
            if let Some(source_table) = source_deps.as_table() {
                let base_deps = base_doc
                    .entry("dependencies")
                    .or_insert(Item::Table(toml_edit::Table::new()));

                if let Some(base_table) = base_deps.as_table_mut() {
                    for (key, value) in source_table.iter() {
                        if !base_table.contains_key(key) {
                            base_table.insert(key, value.clone());
                        }
                    }
                }
            }
        }

        // Merge dev-dependencies
        if let Some(source_deps) = source_doc.get("dev-dependencies") {
            if let Some(source_table) = source_deps.as_table() {
                let base_deps = base_doc
                    .entry("dev-dependencies")
                    .or_insert(Item::Table(toml_edit::Table::new()));

                if let Some(base_table) = base_deps.as_table_mut() {
                    for (key, value) in source_table.iter() {
                        if !base_table.contains_key(key) {
                            base_table.insert(key, value.clone());
                        }
                    }
                }
            }
        }

        Ok(base_doc.to_string())
    }
}

/// JSON-based workspace utilities (for TypeScript/JavaScript package.json)
pub struct JsonWorkspace;

impl JsonWorkspace {
    /// Add a member to a JSON workspace (npm/yarn/pnpm workspaces)
    pub fn add_member(content: &str, member: &str) -> PluginResult<String> {
        let mut json: JsonValue = serde_json::from_str(content)
            .map_err(|e| PluginError::parse(format!("Failed to parse JSON: {}", e)))?;

        // Get or create workspaces array
        let workspaces = json
            .as_object_mut()
            .and_then(|obj| obj.get_mut("workspaces"))
            .ok_or_else(|| PluginError::parse("No workspaces field found"))?;

        // Handle both array format and object format
        match workspaces {
            JsonValue::Array(arr) => {
                if !arr.iter().any(|v| v.as_str() == Some(member)) {
                    arr.push(JsonValue::String(member.to_string()));
                }
            }
            JsonValue::Object(obj) => {
                // pnpm-style: { "packages": [...] }
                if let Some(JsonValue::Array(packages)) = obj.get_mut("packages") {
                    if !packages.iter().any(|v| v.as_str() == Some(member)) {
                        packages.push(JsonValue::String(member.to_string()));
                    }
                }
            }
            _ => {
                return Err(PluginError::parse(
                    "workspaces field must be array or object",
                ))
            }
        }

        serde_json::to_string_pretty(&json)
            .map_err(|e| PluginError::parse(format!("Failed to serialize JSON: {}", e)))
    }

    /// Remove a member from a JSON workspace
    pub fn remove_member(content: &str, member: &str) -> PluginResult<String> {
        let mut json: JsonValue = serde_json::from_str(content)
            .map_err(|e| PluginError::parse(format!("Failed to parse JSON: {}", e)))?;

        if let Some(workspaces) = json.as_object_mut().and_then(|obj| obj.get_mut("workspaces")) {
            match workspaces {
                JsonValue::Array(arr) => {
                    arr.retain(|v| v.as_str() != Some(member));
                }
                JsonValue::Object(obj) => {
                    if let Some(JsonValue::Array(packages)) = obj.get_mut("packages") {
                        packages.retain(|v| v.as_str() != Some(member));
                    }
                }
                _ => {}
            }
        }

        serde_json::to_string_pretty(&json)
            .map_err(|e| PluginError::parse(format!("Failed to serialize JSON: {}", e)))
    }

    /// List all workspace members from JSON
    pub fn list_members(content: &str) -> PluginResult<Vec<String>> {
        let json: JsonValue = serde_json::from_str(content)
            .map_err(|e| PluginError::parse(format!("Failed to parse JSON: {}", e)))?;

        let mut members = Vec::new();

        if let Some(workspaces) = json.as_object().and_then(|obj| obj.get("workspaces")) {
            match workspaces {
                JsonValue::Array(arr) => {
                    for item in arr {
                        if let Some(member) = item.as_str() {
                            members.push(member.to_string());
                        }
                    }
                }
                JsonValue::Object(obj) => {
                    if let Some(JsonValue::Array(packages)) = obj.get("packages") {
                        for item in packages {
                            if let Some(member) = item.as_str() {
                                members.push(member.to_string());
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        Ok(members)
    }

    /// Check if JSON content is a workspace manifest
    pub fn is_workspace(content: &str) -> bool {
        if let Ok(json) = serde_json::from_str::<JsonValue>(content) {
            json.as_object()
                .and_then(|obj| obj.get("workspaces"))
                .is_some()
        } else {
            false
        }
    }

    /// Update package name in JSON manifest
    pub fn update_package_name(content: &str, new_name: &str) -> PluginResult<String> {
        let mut json: JsonValue = serde_json::from_str(content)
            .map_err(|e| PluginError::parse(format!("Failed to parse JSON: {}", e)))?;

        if let Some(obj) = json.as_object_mut() {
            obj.insert("name".to_string(), JsonValue::String(new_name.to_string()));
        }

        serde_json::to_string_pretty(&json)
            .map_err(|e| PluginError::parse(format!("Failed to serialize JSON: {}", e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_toml_workspace_add_member() {
        let content = r#"
[workspace]
members = ["crate1", "crate2"]
"#;

        let result = TomlWorkspace::add_member(content, "crate3").unwrap();
        assert!(result.contains("crate3"));
    }

    #[test]
    fn test_toml_workspace_list_members() {
        let content = r#"
[workspace]
members = ["crate1", "crate2"]
"#;

        let members = TomlWorkspace::list_members(content).unwrap();
        assert_eq!(members.len(), 2);
        assert!(members.contains(&"crate1".to_string()));
        assert!(members.contains(&"crate2".to_string()));
    }

    #[test]
    fn test_json_workspace_add_member() {
        let content = r#"
{
  "name": "my-workspace",
  "workspaces": ["package1", "package2"]
}
"#;

        let result = JsonWorkspace::add_member(content, "package3").unwrap();
        assert!(result.contains("package3"));
    }

    #[test]
    fn test_json_workspace_list_members() {
        let content = r#"
{
  "workspaces": ["package1", "package2"]
}
"#;

        let members = JsonWorkspace::list_members(content).unwrap();
        assert_eq!(members.len(), 2);
        assert!(members.contains(&"package1".to_string()));
    }
}
