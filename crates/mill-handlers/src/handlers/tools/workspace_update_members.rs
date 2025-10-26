//! Workspace member management tool handler
//!
//! Handles: workspace.update_members
//!
//! This tool manages workspace members in Cargo.toml - adding, removing, or listing
//! workspace member packages (Proposal 50: Crate Extraction Tooling).

use super::{ToolHandler, ToolHandlerContext};
use async_trait::async_trait;
use mill_foundation::core::model::mcp::ToolCall;
use mill_foundation::protocol::{ApiError, ApiResult as ServerResult};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::path::Path;
use toml_edit::{DocumentMut, Item};
use tracing::{debug, error, warn};

/// Handler for workspace member management operations
pub struct WorkspaceUpdateMembersHandler;

impl WorkspaceUpdateMembersHandler {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ToolHandler for WorkspaceUpdateMembersHandler {
    fn tool_names(&self) -> &[&str] {
        &["workspace.update_members"]
    }

    async fn handle_tool_call(
        &self,
        context: &ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        match tool_call.name.as_str() {
            "workspace.update_members" => handle_update_members(context, tool_call).await,
            _ => Err(ApiError::InvalidRequest(format!(
                "Unknown workspace update members tool: {}",
                tool_call.name
            ))),
        }
    }
}

// Parameter types for MCP interface

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateMembersParams {
    pub workspace_manifest: String,
    pub action: MemberAction,
    #[serde(default)]
    pub members: Vec<String>,
    #[serde(default)]
    pub options: UpdateMembersOptions,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MemberAction {
    Add,
    Remove,
    List,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UpdateMembersOptions {
    #[serde(default)]
    pub dry_run: bool,
    #[serde(default)]
    pub create_if_missing: bool,
}

// Result type for MCP interface

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateMembersResult {
    pub action: String,
    pub members_before: Vec<String>,
    pub members_after: Vec<String>,
    pub changes_made: usize,
    pub workspace_updated: bool,
    pub dry_run: bool,
}

// Handler implementation

async fn handle_update_members(
    context: &ToolHandlerContext,
    tool_call: &ToolCall,
) -> ServerResult<Value> {
    debug!("Handling workspace.update_members");

    // Parse parameters
    let params: UpdateMembersParams = serde_json::from_value(
        tool_call
            .arguments
            .as_ref()
            .ok_or_else(|| ApiError::InvalidRequest("Missing arguments".to_string()))?
            .clone(),
    )
    .map_err(|e| ApiError::InvalidRequest(format!("Invalid arguments: {}", e)))?;

    debug!(
        workspace_manifest = %params.workspace_manifest,
        action = ?params.action,
        members_count = params.members.len(),
        dry_run = params.options.dry_run,
        "Parsed update_members parameters"
    );

    // Resolve path relative to workspace root
    let workspace_root = &context.app_state.project_root;
    let manifest_path = resolve_path(workspace_root, &params.workspace_manifest)?;

    // Validate file exists
    if !manifest_path.exists() {
        return Err(ApiError::InvalidRequest(format!(
            "Workspace manifest not found: {}",
            manifest_path.display()
        )));
    }

    // Read manifest
    let manifest_content = fs::read_to_string(&manifest_path).map_err(|e| {
        error!(error = %e, manifest_path = %manifest_path.display(), "Failed to read workspace manifest");
        ApiError::Internal(format!("Failed to read workspace manifest: {}", e))
    })?;

    // Execute the action
    let action_result = match params.action {
        MemberAction::List => list_members(&manifest_content)?,
        MemberAction::Add => add_members(
            &manifest_content,
            &params.members,
            &params.options,
            workspace_root,
        )?,
        MemberAction::Remove => remove_members(&manifest_content, &params.members)?,
    };

    // Write updated manifest if not dry-run and content changed
    let workspace_updated = if !params.options.dry_run && action_result.updated_content.is_some() {
        let updated = action_result.updated_content.as_ref().unwrap();
        fs::write(&manifest_path, updated).map_err(|e| {
            error!(error = %e, manifest_path = %manifest_path.display(), "Failed to write workspace manifest");
            ApiError::Internal(format!("Failed to write workspace manifest: {}", e))
        })?;
        debug!(manifest_path = %manifest_path.display(), "Wrote updated workspace manifest");
        true
    } else {
        false
    };

    // Build result
    let result = UpdateMembersResult {
        action: format!("{:?}", params.action).to_lowercase(),
        members_before: action_result.members_before,
        members_after: action_result.members_after,
        changes_made: action_result.changes_made,
        workspace_updated,
        dry_run: params.options.dry_run,
    };

    Ok(serde_json::to_value(result).unwrap())
}

// Helper types

struct ActionResult {
    members_before: Vec<String>,
    members_after: Vec<String>,
    changes_made: usize,
    updated_content: Option<String>,
}

// Core action implementations

fn list_members(content: &str) -> ServerResult<ActionResult> {
    debug!("Listing workspace members");

    let doc = content.parse::<DocumentMut>().map_err(|e| {
        error!(error = %e, "Failed to parse workspace manifest");
        ApiError::Parse {
            message: format!("Failed to parse Cargo.toml: {}", e),
        }
    })?;

    let members = extract_members(&doc);

    debug!(members_count = members.len(), "Listed workspace members");

    Ok(ActionResult {
        members_before: members.clone(),
        members_after: members,
        changes_made: 0,
        updated_content: None,
    })
}

fn add_members(
    content: &str,
    members_to_add: &[String],
    options: &UpdateMembersOptions,
    workspace_root: &Path,
) -> ServerResult<ActionResult> {
    debug!(
        members_count = members_to_add.len(),
        "Adding workspace members"
    );

    let mut doc = content.parse::<DocumentMut>().map_err(|e| {
        error!(error = %e, "Failed to parse workspace manifest");
        ApiError::Parse {
            message: format!("Failed to parse Cargo.toml: {}", e),
        }
    })?;

    let members_before = extract_members(&doc);

    // Ensure [workspace] section exists if create_if_missing is true
    if !doc.contains_key("workspace") {
        if options.create_if_missing {
            debug!("Creating [workspace] section");
            doc["workspace"] = Item::Table(toml_edit::Table::new());
        } else {
            return Err(ApiError::InvalidRequest(
                "Manifest does not contain [workspace] section".to_string(),
            ));
        }
    }

    let workspace = doc["workspace"]
        .as_table_mut()
        .ok_or_else(|| ApiError::Parse {
            message: "[workspace] is not a table".to_string(),
        })?;

    // Ensure members array exists
    if !workspace.contains_key("members") {
        if options.create_if_missing {
            debug!("Creating [workspace.members] array");
            workspace["members"] = Item::Value(toml_edit::Value::Array(toml_edit::Array::new()));
        } else {
            return Err(ApiError::InvalidRequest(
                "Manifest does not contain [workspace.members] section".to_string(),
            ));
        }
    }

    let members_array = workspace["members"]
        .as_array_mut()
        .ok_or_else(|| ApiError::Parse {
            message: "[workspace.members] is not an array".to_string(),
        })?;

    let mut changes_made = 0;

    for member in members_to_add {
        // Normalize path (forward slashes, relative to workspace root)
        let normalized = normalize_path(member);

        // Validate member path exists (relative to workspace root)
        let member_path = workspace_root.join(&normalized);
        if !member_path.exists() {
            warn!(member = %normalized, "Member path does not exist");
        }

        // Check if already exists
        let already_exists = members_array
            .iter()
            .any(|v| v.as_str() == Some(&normalized));

        if already_exists {
            debug!(member = %normalized, "Member already exists, skipping");
            continue;
        }

        // Add the member
        members_array.push(normalized.as_str());
        debug!(member = %normalized, "Added member to workspace");
        changes_made += 1;
    }

    let members_after = extract_members(&doc);
    let updated_content = if changes_made > 0 {
        Some(doc.to_string())
    } else {
        None
    };

    Ok(ActionResult {
        members_before,
        members_after,
        changes_made,
        updated_content,
    })
}

fn remove_members(content: &str, members_to_remove: &[String]) -> ServerResult<ActionResult> {
    debug!(
        members_count = members_to_remove.len(),
        "Removing workspace members"
    );

    let mut doc = content.parse::<DocumentMut>().map_err(|e| {
        error!(error = %e, "Failed to parse workspace manifest");
        ApiError::Parse {
            message: format!("Failed to parse Cargo.toml: {}", e),
        }
    })?;

    let members_before = extract_members(&doc);

    // Get workspace members array (don't error if missing)
    let members_array = match doc
        .get_mut("workspace")
        .and_then(|w| w.as_table_mut())
        .and_then(|t| t.get_mut("members"))
        .and_then(|m| m.as_array_mut())
    {
        Some(array) => array,
        None => {
            // No members to remove
            debug!("No [workspace.members] section found");
            return Ok(ActionResult {
                members_before: members_before.clone(),
                members_after: members_before,
                changes_made: 0,
                updated_content: None,
            });
        }
    };

    let mut changes_made = 0;

    for member in members_to_remove {
        // Normalize path
        let normalized = normalize_path(member);

        // Count before removal
        let count_before = members_array.len();

        // Remove the member (retain all that don't match)
        members_array.retain(|v| v.as_str() != Some(&normalized));

        let count_after = members_array.len();

        if count_before > count_after {
            debug!(member = %normalized, "Removed member from workspace");
            changes_made += count_before - count_after;
        } else {
            debug!(member = %normalized, "Member not found in workspace");
        }
    }

    let members_after = extract_members(&doc);
    let updated_content = if changes_made > 0 {
        Some(doc.to_string())
    } else {
        None
    };

    Ok(ActionResult {
        members_before,
        members_after,
        changes_made,
        updated_content,
    })
}

// Helper functions

fn resolve_path(workspace_root: &Path, path: &str) -> ServerResult<std::path::PathBuf> {
    let resolved = if Path::new(path).is_absolute() {
        Path::new(path).to_path_buf()
    } else {
        workspace_root.join(path)
    };

    Ok(resolved)
}

fn normalize_path(path: &str) -> String {
    // Normalize to forward slashes and remove trailing slashes
    path.replace('\\', "/").trim_end_matches('/').to_string()
}

fn extract_members(doc: &DocumentMut) -> Vec<String> {
    doc.get("workspace")
        .and_then(|w| w.as_table())
        .and_then(|t| t.get("members"))
        .and_then(|m| m.as_array())
        .map(|array| {
            array
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_members() {
        let content = r#"
[workspace]
members = ["crate1", "crate2", "crate3"]
"#;
        let doc = content.parse::<DocumentMut>().unwrap();
        let members = extract_members(&doc);

        assert_eq!(members.len(), 3);
        assert_eq!(members, vec!["crate1", "crate2", "crate3"]);
    }

    #[test]
    fn test_extract_members_empty() {
        let content = r#"
[workspace]
members = []
"#;
        let doc = content.parse::<DocumentMut>().unwrap();
        let members = extract_members(&doc);

        assert_eq!(members.len(), 0);
    }

    #[test]
    fn test_extract_members_no_workspace() {
        let content = r#"
[package]
name = "my-package"
"#;
        let doc = content.parse::<DocumentMut>().unwrap();
        let members = extract_members(&doc);

        assert_eq!(members.len(), 0);
    }

    #[test]
    fn test_normalize_path() {
        assert_eq!(normalize_path("crates/my-lib"), "crates/my-lib");
        assert_eq!(normalize_path("crates\\my-lib"), "crates/my-lib");
        assert_eq!(normalize_path("crates/my-lib/"), "crates/my-lib");
        assert_eq!(normalize_path("crates\\my-lib\\"), "crates/my-lib");
    }

    #[test]
    fn test_list_members() {
        let content = r#"
[workspace]
members = ["crate1", "crate2"]
"#;

        let result = list_members(content).unwrap();

        assert_eq!(result.members_before.len(), 2);
        assert_eq!(result.members_after.len(), 2);
        assert_eq!(result.changes_made, 0);
        assert!(result.updated_content.is_none());
    }

    #[test]
    fn test_add_members_basic() {
        let content = r#"
[workspace]
members = ["crate1"]
"#;

        let options = UpdateMembersOptions {
            dry_run: false,
            create_if_missing: false,
        };

        let workspace_root = Path::new("/tmp");
        let members_to_add = vec!["crate2".to_string(), "crate3".to_string()];

        let result = add_members(content, &members_to_add, &options, workspace_root).unwrap();

        assert_eq!(result.members_before.len(), 1);
        assert_eq!(result.members_after.len(), 3);
        assert_eq!(result.changes_made, 2);
        assert!(result.updated_content.is_some());

        // Verify the content
        let updated = result.updated_content.unwrap();
        assert!(updated.contains("crate1"));
        assert!(updated.contains("crate2"));
        assert!(updated.contains("crate3"));
    }

    #[test]
    fn test_add_members_duplicate() {
        let content = r#"
[workspace]
members = ["crate1"]
"#;

        let options = UpdateMembersOptions {
            dry_run: false,
            create_if_missing: false,
        };

        let workspace_root = Path::new("/tmp");
        let members_to_add = vec!["crate1".to_string()]; // Already exists

        let result = add_members(content, &members_to_add, &options, workspace_root).unwrap();

        assert_eq!(result.members_before.len(), 1);
        assert_eq!(result.members_after.len(), 1);
        assert_eq!(result.changes_made, 0); // No changes made
        assert!(result.updated_content.is_none());
    }

    #[test]
    fn test_add_members_create_if_missing() {
        let content = r#"
[package]
name = "my-package"
"#;

        let options = UpdateMembersOptions {
            dry_run: false,
            create_if_missing: true,
        };

        let workspace_root = Path::new("/tmp");
        let members_to_add = vec!["crate1".to_string()];

        let result = add_members(content, &members_to_add, &options, workspace_root).unwrap();

        assert_eq!(result.members_before.len(), 0);
        assert_eq!(result.members_after.len(), 1);
        assert_eq!(result.changes_made, 1);
        assert!(result.updated_content.is_some());

        // Verify [workspace] section was created
        let updated = result.updated_content.unwrap();
        assert!(updated.contains("[workspace]"));
        assert!(updated.contains("crate1"));
    }

    #[test]
    fn test_remove_members_basic() {
        let content = r#"
[workspace]
members = ["crate1", "crate2", "crate3"]
"#;

        let members_to_remove = vec!["crate2".to_string()];

        let result = remove_members(content, &members_to_remove).unwrap();

        assert_eq!(result.members_before.len(), 3);
        assert_eq!(result.members_after.len(), 2);
        assert_eq!(result.changes_made, 1);
        assert!(result.updated_content.is_some());

        // Verify crate2 was removed
        let updated = result.updated_content.unwrap();
        assert!(updated.contains("crate1"));
        assert!(!updated.contains("crate2"));
        assert!(updated.contains("crate3"));
    }

    #[test]
    fn test_remove_members_nonexistent() {
        let content = r#"
[workspace]
members = ["crate1"]
"#;

        let members_to_remove = vec!["nonexistent".to_string()];

        let result = remove_members(content, &members_to_remove).unwrap();

        assert_eq!(result.members_before.len(), 1);
        assert_eq!(result.members_after.len(), 1);
        assert_eq!(result.changes_made, 0); // No changes
        assert!(result.updated_content.is_none());
    }

    #[test]
    fn test_remove_members_no_workspace_section() {
        let content = r#"
[package]
name = "my-package"
"#;

        let members_to_remove = vec!["crate1".to_string()];

        let result = remove_members(content, &members_to_remove).unwrap();

        // Should not error, just return no changes
        assert_eq!(result.members_before.len(), 0);
        assert_eq!(result.members_after.len(), 0);
        assert_eq!(result.changes_made, 0);
        assert!(result.updated_content.is_none());
    }
}
