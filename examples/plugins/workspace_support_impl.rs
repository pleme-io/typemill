// Example: WorkspaceSupport Trait Implementation
// Purpose: Enable workspace-level operations (Cargo.toml, package.json, etc.)

use cb_plugin_api::WorkspaceSupport;
use toml_edit::{Document, Item, Array};

pub struct MyLanguageWorkspaceSupport;

impl WorkspaceSupport for MyLanguageWorkspaceSupport {
    /// Check if a manifest file is a workspace manifest
    fn is_workspace_manifest(&self, content: &str) -> bool {
        content.contains("[workspace]")
    }

    /// Add a member to the workspace
    fn add_workspace_member(
        &self,
        content: &str,
        member_path: &str,
    ) -> Result<String, PluginError> {
        let mut doc = content.parse::<Document>()
            .map_err(|e| PluginError::manifest(format!("Invalid TOML: {}", e)))?;

        // Get or create workspace.members array
        let workspace = doc["workspace"]
            .or_insert(Item::Table(Default::default()));

        let members = workspace["members"]
            .or_insert(Item::Value(toml_edit::Value::Array(Array::new())));

        if let Some(arr) = members.as_array_mut() {
            arr.push(member_path);
        }

        Ok(doc.to_string())
    }

    /// Remove a member from the workspace
    fn remove_workspace_member(
        &self,
        content: &str,
        member_path: &str,
    ) -> Result<String, PluginError> {
        let mut doc = content.parse::<Document>()
            .map_err(|e| PluginError::manifest(format!("Invalid TOML: {}", e)))?;

        if let Some(arr) = doc["workspace"]["members"].as_array_mut() {
            arr.retain(|item| item.as_str() != Some(member_path));
        }

        Ok(doc.to_string())
    }

    /// Merge dependencies from source manifest into target manifest
    fn merge_dependencies(
        &self,
        source_content: &str,
        target_content: &str,
    ) -> Result<String, PluginError> {
        // Parse both manifests
        let source_doc: Document = source_content.parse()?;
        let mut target_doc: Document = target_content.parse()?;

        // Copy dependencies from source to target
        if let Some(source_deps) = source_doc["dependencies"].as_table() {
            let target_deps = target_doc["dependencies"]
                .or_insert(Item::Table(Default::default()))
                .as_table_mut()
                .unwrap();

            for (name, value) in source_deps {
                target_deps.insert(name, value.clone());
            }
        }

        Ok(target_doc.to_string())
    }
}
