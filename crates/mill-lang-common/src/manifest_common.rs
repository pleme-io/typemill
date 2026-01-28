//! Common manifest parsing and manipulation utilities
//!
//! This module provides shared functionality for working with different
//! manifest formats (TOML, JSON, YAML, XML) across language plugins.

use mill_foundation::errors::MillError;
use serde_json::Value as JsonValue;
use toml_edit::{value, Array, DocumentMut, Item};
use tracing::debug;

type PluginResult<T> = Result<T, MillError>;

/// TOML-based workspace utilities (for Rust Cargo.toml, Python pyproject.toml)
pub struct TomlWorkspace;

impl TomlWorkspace {
    fn add_cargo_member(doc: &mut DocumentMut, member: &str) -> bool {
        if let Some(workspace) = doc.get_mut("workspace") {
            if let Some(members_item) = workspace.get_mut("members") {
                if let Some(members) = members_item.as_array_mut() {
                    // Check if member already exists
                    let member_exists = members.iter().any(|v| v.as_str() == Some(member));

                    if !member_exists {
                        members.push(member);
                        debug!(member = %member, "Added member to Cargo workspace");
                    }
                }
            } else {
                // Create members array
                let mut members = Array::new();
                members.push(member);
                workspace["members"] = value(members);
            }
            return true;
        }
        false
    }

    fn add_poetry_member(doc: &mut DocumentMut, member: &str) -> bool {
        if let Some(tool) = doc.get_mut("tool") {
            if let Some(poetry) = tool.get_mut("poetry") {
                if let Some(workspace) = poetry.get_mut("workspace") {
                    if let Some(members_item) = workspace.get_mut("members") {
                        if let Some(members) = members_item.as_array_mut() {
                            let member_exists = members.iter().any(|v| v.as_str() == Some(member));

                            if !member_exists {
                                members.push(member);
                                debug!(member = %member, "Added member to Poetry workspace");
                            }
                            return true;
                        }
                    }
                }
            }
        }
        false
    }
    /// Add a member to a TOML workspace
    ///
    /// Supports both Cargo-style and Poetry/PDM-style workspace sections
    pub fn add_member(content: &str, member: &str) -> PluginResult<String> {
        let mut doc = content
            .parse::<DocumentMut>()
            .map_err(|e| MillError::parse(format!("Failed to parse TOML: {}", e)))?;

        if Self::add_cargo_member(&mut doc, member) {
            return Ok(doc.to_string());
        }

        if Self::add_poetry_member(&mut doc, member) {
            return Ok(doc.to_string());
        }

        // No workspace section found, return original
        Ok(content.to_string())
    }

    fn remove_cargo_member(doc: &mut DocumentMut, member: &str) -> bool {
        if let Some(workspace) = doc.get_mut("workspace") {
            if let Some(members_item) = workspace.get_mut("members") {
                if let Some(members) = members_item.as_array_mut() {
                    let initial_len = members.len();
                    members.retain(|v| v.as_str() != Some(member));
                    return initial_len != members.len();
                }
            }
        }
        false
    }

    fn remove_poetry_member(doc: &mut DocumentMut, member: &str) -> bool {
        if let Some(tool) = doc.get_mut("tool") {
            if let Some(poetry) = tool.get_mut("poetry") {
                if let Some(workspace) = poetry.get_mut("workspace") {
                    if let Some(members_item) = workspace.get_mut("members") {
                        if let Some(members) = members_item.as_array_mut() {
                            let initial_len = members.len();
                            members.retain(|v| v.as_str() != Some(member));
                            return initial_len != members.len();
                        }
                    }
                }
            }
        }
        false
    }
    /// Remove a member from a TOML workspace
    pub fn remove_member(content: &str, member: &str) -> PluginResult<String> {
        let mut doc = content
            .parse::<DocumentMut>()
            .map_err(|e| MillError::parse(format!("Failed to parse TOML: {}", e)))?;

        let modified = Self::remove_cargo_member(&mut doc, member)
            || Self::remove_poetry_member(&mut doc, member);

        if modified {
            debug!(member = %member, "Removed member from workspace");
        }

        Ok(doc.to_string())
    }

    fn list_cargo_members(doc: &DocumentMut) -> Option<Vec<String>> {
        doc.get("workspace")
            .and_then(|w| w.get("members"))
            .and_then(|m| m.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
    }

    fn list_poetry_members(doc: &DocumentMut) -> Option<Vec<String>> {
        doc.get("tool")
            .and_then(|t| t.get("poetry"))
            .and_then(|p| p.get("workspace"))
            .and_then(|w| w.get("members"))
            .and_then(|m| m.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
    }
    /// List all workspace members from TOML
    pub fn list_members(content: &str) -> PluginResult<Vec<String>> {
        let doc = content
            .parse::<DocumentMut>()
            .map_err(|e| MillError::parse(format!("Failed to parse TOML: {}", e)))?;

        if let Some(members) = Self::list_cargo_members(&doc) {
            return Ok(members);
        }

        if let Some(members) = Self::list_poetry_members(&doc) {
            return Ok(members);
        }

        Ok(Vec::new())
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
            .map_err(|e| MillError::parse(format!("Failed to parse TOML: {}", e)))?;

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
            .map_err(|e| MillError::parse(format!("Failed to parse base TOML: {}", e)))?;

        let mut source_doc = source
            .parse::<DocumentMut>()
            .map_err(|e| MillError::parse(format!("Failed to parse source TOML: {}", e)))?;

        // Merge [dependencies] section (Cargo-style)
        if let Some(source_deps) = source_doc.remove("dependencies") {
            if let Ok(source_table) = source_deps.into_table() {
                let base_deps = base_doc
                    .entry("dependencies")
                    .or_insert(Item::Table(toml_edit::Table::new()));

                if let Some(base_table) = base_deps.as_table_mut() {
                    for (key, value) in source_table {
                        if !base_table.contains_key(&key) {
                            base_table.insert(&key, value);
                        }
                    }
                }
            }
        }

        // Merge dev-dependencies
        if let Some(source_deps) = source_doc.remove("dev-dependencies") {
            if let Ok(source_table) = source_deps.into_table() {
                let base_deps = base_doc
                    .entry("dev-dependencies")
                    .or_insert(Item::Table(toml_edit::Table::new()));

                if let Some(base_table) = base_deps.as_table_mut() {
                    for (key, value) in source_table {
                        if !base_table.contains_key(&key) {
                            base_table.insert(&key, value);
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
    /// Returns a mutable reference to the workspaces array if it exists
    fn get_workspaces_mut(json: &mut JsonValue) -> Option<&mut Vec<JsonValue>> {
        let workspaces = json.as_object_mut()?.get_mut("workspaces")?;
        match workspaces {
            JsonValue::Array(arr) => Some(arr),
            JsonValue::Object(obj) => obj.get_mut("packages").and_then(|v| v.as_array_mut()),
            _ => None,
        }
    }
    /// Add a member to a JSON workspace (npm/yarn/pnpm workspaces)
    pub fn add_member(content: &str, member: &str) -> PluginResult<String> {
        let mut json: JsonValue = serde_json::from_str(content)
            .map_err(|e| MillError::parse(format!("Failed to parse JSON: {}", e)))?;

        if let Some(workspaces) = Self::get_workspaces_mut(&mut json) {
            if !workspaces.iter().any(|v| v.as_str() == Some(member)) {
                workspaces.push(JsonValue::String(member.to_string()));
            }
        }

        serde_json::to_string_pretty(&json)
            .map_err(|e| MillError::parse(format!("Failed to serialize JSON: {}", e)))
    }

    /// Remove a member from a JSON workspace
    pub fn remove_member(content: &str, member: &str) -> PluginResult<String> {
        let mut json: JsonValue = serde_json::from_str(content)
            .map_err(|e| MillError::parse(format!("Failed to parse JSON: {}", e)))?;

        if let Some(workspaces) = Self::get_workspaces_mut(&mut json) {
            workspaces.retain(|v| v.as_str() != Some(member));
        }

        serde_json::to_string_pretty(&json)
            .map_err(|e| MillError::parse(format!("Failed to serialize JSON: {}", e)))
    }

    fn get_workspaces(json: &JsonValue) -> Option<&Vec<JsonValue>> {
        let workspaces = json.as_object()?.get("workspaces")?;
        match workspaces {
            JsonValue::Array(arr) => Some(arr),
            JsonValue::Object(obj) => obj.get("packages").and_then(|v| v.as_array()),
            _ => None,
        }
    }
    /// List all workspace members from JSON
    pub fn list_members(content: &str) -> PluginResult<Vec<String>> {
        let json: JsonValue = serde_json::from_str(content)
            .map_err(|e| MillError::parse(format!("Failed to parse JSON: {}", e)))?;

        if let Some(workspaces) = Self::get_workspaces(&json) {
            let members = workspaces
                .iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect();
            return Ok(members);
        }

        Ok(Vec::new())
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
            .map_err(|e| MillError::parse(format!("Failed to parse JSON: {}", e)))?;

        if let Some(obj) = json.as_object_mut() {
            obj.insert("name".to_string(), JsonValue::String(new_name.to_string()));
        }

        serde_json::to_string_pretty(&json)
            .map_err(|e| MillError::parse(format!("Failed to serialize JSON: {}", e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merge_dependencies() {
        let base = r#"
        [package]
        name = "base"
        version = "0.1.0"

        [dependencies]
        dep1 = "1.0"
        dep2 = "2.0"
        "#;

        let source = r#"
        [package]
        name = "source"
        version = "0.1.0"

        [dependencies]
        dep2 = "3.0" # Conflict, should keep base
        dep3 = "3.0" # New, should add

        [dev-dependencies]
        dev_dep1 = "1.0"
        "#;

        let result = TomlWorkspace::merge_dependencies(base, source).unwrap();
        let doc: DocumentMut = result.parse().unwrap();

        let deps = doc["dependencies"].as_table().unwrap();
        assert_eq!(deps["dep1"].as_str(), Some("1.0"));
        assert_eq!(deps["dep2"].as_str(), Some("2.0")); // Kept base
        assert_eq!(deps["dep3"].as_str(), Some("3.0")); // Added new

        let dev_deps = doc["dev-dependencies"].as_table().unwrap();
        assert_eq!(dev_deps["dev_dep1"].as_str(), Some("1.0"));
    }
}
