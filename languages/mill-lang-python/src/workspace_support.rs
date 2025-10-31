//! Python workspace support for Poetry/PDM/Hatch monorepos
//!
//! Handles workspace operations through pyproject.toml manipulation.

use mill_plugin_api::WorkspaceSupport;
use toml_edit::{value, Array, DocumentMut, Item, Table};
use tracing::{debug, warn};

/// Python workspace support implementation
pub struct PythonWorkspaceSupport;

impl PythonWorkspaceSupport {
    pub fn new() -> Self {
        Self
    }
}

impl Default for PythonWorkspaceSupport {
    fn default() -> Self {
        Self::new()
    }
}

impl WorkspaceSupport for PythonWorkspaceSupport {
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

    fn update_package_name(&self, content: &str, new_name: &str) -> String {
        match update_package_name_impl(content, new_name) {
            Ok(result) => result,
            Err(e) => {
                warn!(error = %e, new_name = %new_name, "Failed to update package name");
                content.to_string()
            }
        }
    }
}

/// Python workspace tool detection
#[derive(Debug, PartialEq)]
#[allow(clippy::upper_case_acronyms)]
enum PythonWorkspaceTool {
    PDM,    // [tool.pdm.workspace.members]
    Poetry, // [tool.poetry.packages]
    Hatch,  // [tool.hatch.envs]
    None,
}

/// Detect which Python tool is being used
fn detect_tool(doc: &DocumentMut) -> PythonWorkspaceTool {
    // Check for PDM workspace
    if doc
        .get("tool")
        .and_then(|t| t.get("pdm"))
        .and_then(|p| p.get("workspace"))
        .is_some()
    {
        return PythonWorkspaceTool::PDM;
    }

    // Check for Poetry packages
    if doc
        .get("tool")
        .and_then(|t| t.get("poetry"))
        .and_then(|p| p.get("packages"))
        .is_some()
    {
        return PythonWorkspaceTool::Poetry;
    }

    // Check for Hatch envs (minimal support)
    if doc
        .get("tool")
        .and_then(|t| t.get("hatch"))
        .and_then(|h| h.get("envs"))
        .is_some()
    {
        return PythonWorkspaceTool::Hatch;
    }

    PythonWorkspaceTool::None
}

/// Check if pyproject.toml is a workspace manifest
fn is_workspace_manifest_impl(content: &str) -> bool {
    match content.parse::<DocumentMut>() {
        Ok(doc) => detect_tool(&doc) != PythonWorkspaceTool::None,
        Err(_) => false,
    }
}

/// Add workspace member based on detected tool
fn add_workspace_member_impl(content: &str, member: &str) -> Result<String, String> {
    let mut doc = content
        .parse::<DocumentMut>()
        .map_err(|e| format!("Failed to parse pyproject.toml: {}", e))?;

    match detect_tool(&doc) {
        PythonWorkspaceTool::PDM => add_pdm_member(&mut doc, member)?,
        PythonWorkspaceTool::Poetry => add_poetry_package(&mut doc, member)?,
        PythonWorkspaceTool::Hatch => {
            warn!("Hatch workspace member addition not fully supported");
            return Ok(content.to_string());
        }
        PythonWorkspaceTool::None => {
            // Create PDM workspace by default (simplest format)
            create_pdm_workspace(&mut doc, member)?;
        }
    }

    Ok(doc.to_string())
}

/// Remove workspace member based on detected tool
fn remove_workspace_member_impl(content: &str, member: &str) -> Result<String, String> {
    let mut doc = content
        .parse::<DocumentMut>()
        .map_err(|e| format!("Failed to parse pyproject.toml: {}", e))?;

    match detect_tool(&doc) {
        PythonWorkspaceTool::PDM => remove_pdm_member(&mut doc, member)?,
        PythonWorkspaceTool::Poetry => remove_poetry_package(&mut doc, member)?,
        PythonWorkspaceTool::Hatch => {
            warn!("Hatch workspace member removal not fully supported");
            return Ok(content.to_string());
        }
        PythonWorkspaceTool::None => {
            debug!("No workspace found, nothing to remove");
            return Ok(content.to_string());
        }
    }

    Ok(doc.to_string())
}

/// List workspace members based on detected tool
fn list_workspace_members_impl(content: &str) -> Result<Vec<String>, String> {
    let doc = content
        .parse::<DocumentMut>()
        .map_err(|e| format!("Failed to parse pyproject.toml: {}", e))?;

    match detect_tool(&doc) {
        PythonWorkspaceTool::PDM => list_pdm_members(&doc),
        PythonWorkspaceTool::Poetry => list_poetry_packages(&doc),
        PythonWorkspaceTool::Hatch => list_hatch_envs(&doc),
        PythonWorkspaceTool::None => Ok(Vec::new()),
    }
}

/// Update package name (supports both PEP 621 and Poetry)
fn update_package_name_impl(content: &str, new_name: &str) -> Result<String, String> {
    let mut doc = content
        .parse::<DocumentMut>()
        .map_err(|e| format!("Failed to parse pyproject.toml: {}", e))?;

    // Try PEP 621 standard first ([project.name])
    if let Some(project) = doc.get_mut("project").and_then(|p| p.as_table_mut()) {
        project["name"] = value(new_name);
        return Ok(doc.to_string());
    }

    // Fallback to Poetry ([tool.poetry.name])
    if let Some(poetry) = doc
        .get_mut("tool")
        .and_then(|t| t.get_mut("poetry"))
        .and_then(|p| p.as_table_mut())
    {
        poetry["name"] = value(new_name);
        return Ok(doc.to_string());
    }

    Err("No package name field found ([project.name] or [tool.poetry.name])".to_string())
}

// ============================================================================
// PDM Workspace Operations (simplest - glob patterns like Rust)
// ============================================================================

/// Create PDM workspace
fn create_pdm_workspace(doc: &mut DocumentMut, member: &str) -> Result<(), String> {
    // Ensure [tool.pdm.workspace] exists
    if !doc.contains_key("tool") {
        doc["tool"] = Item::Table(Table::new());
    }

    let tool = doc["tool"].as_table_mut().ok_or("[tool] is not a table")?;

    if !tool.contains_key("pdm") {
        tool["pdm"] = Item::Table(Table::new());
    }

    let pdm = tool["pdm"]
        .as_table_mut()
        .ok_or("[tool.pdm] is not a table")?;

    if !pdm.contains_key("workspace") {
        pdm["workspace"] = Item::Table(Table::new());
    }

    let workspace = pdm["workspace"]
        .as_table_mut()
        .ok_or("[tool.pdm.workspace] is not a table")?;

    // Create members array
    let mut members_array = Array::new();
    members_array.push(member);
    workspace["members"] = value(members_array);

    Ok(())
}

/// Add PDM workspace member
fn add_pdm_member(doc: &mut DocumentMut, member: &str) -> Result<(), String> {
    let members = doc["tool"]["pdm"]["workspace"]["members"]
        .as_array_mut()
        .ok_or("[tool.pdm.workspace.members] is not an array")?;

    // Check if already exists
    if members.iter().any(|v| v.as_str() == Some(member)) {
        debug!(member = %member, "Member already exists in PDM workspace");
        return Ok(());
    }

    members.push(member);
    Ok(())
}

/// Remove PDM workspace member
fn remove_pdm_member(doc: &mut DocumentMut, member: &str) -> Result<(), String> {
    let members = doc["tool"]["pdm"]["workspace"]["members"]
        .as_array_mut()
        .ok_or("[tool.pdm.workspace.members] is not an array")?;

    let original_len = members.len();
    members.retain(|v| v.as_str() != Some(member));

    if members.len() == original_len {
        debug!(member = %member, "Member not found in PDM workspace");
    }

    Ok(())
}

/// List PDM workspace members
fn list_pdm_members(doc: &DocumentMut) -> Result<Vec<String>, String> {
    let members = doc
        .get("tool")
        .and_then(|t| t.get("pdm"))
        .and_then(|p| p.get("workspace"))
        .and_then(|w| w.get("members"))
        .and_then(|m| m.as_array())
        .ok_or("PDM workspace members not found")?;

    Ok(members
        .iter()
        .filter_map(|v| v.as_str().map(String::from))
        .collect())
}

// ============================================================================
// Poetry Workspace Operations (table array format)
// ============================================================================

/// Add Poetry package
fn add_poetry_package(doc: &mut DocumentMut, member: &str) -> Result<(), String> {
    let packages = doc["tool"]["poetry"]["packages"]
        .as_array_of_tables_mut()
        .ok_or("[tool.poetry.packages] is not an array of tables")?;

    // Check if already exists
    let package_name = extract_package_name(member);
    for pkg in packages.iter() {
        if let Some(include) = pkg.get("include").and_then(|v| v.as_str()) {
            if include == package_name {
                debug!(member = %member, "Package already exists in Poetry workspace");
                return Ok(());
            }
        }
    }

    // Create new package table
    let mut pkg_table = Table::new();
    pkg_table["include"] = value(package_name);
    pkg_table["from"] = value(member);

    packages.push(pkg_table);
    Ok(())
}

/// Remove Poetry package
fn remove_poetry_package(doc: &mut DocumentMut, member: &str) -> Result<(), String> {
    let packages = doc["tool"]["poetry"]["packages"]
        .as_array_of_tables_mut()
        .ok_or("[tool.poetry.packages] is not an array of tables")?;

    let package_name = extract_package_name(member);
    packages.retain(|pkg| {
        pkg.get("include")
            .and_then(|v| v.as_str())
            .map(|inc| inc != package_name)
            .unwrap_or(true)
    });

    Ok(())
}

/// List Poetry packages
fn list_poetry_packages(doc: &DocumentMut) -> Result<Vec<String>, String> {
    let packages = doc
        .get("tool")
        .and_then(|t| t.get("poetry"))
        .and_then(|p| p.get("packages"))
        .and_then(|p| p.as_array_of_tables())
        .ok_or("Poetry packages not found")?;

    Ok(packages
        .iter()
        .filter_map(|pkg| pkg.get("from").and_then(|v| v.as_str()).map(String::from))
        .collect())
}

/// Extract package name from path (e.g., "packages/my-pkg" -> "my_pkg")
fn extract_package_name(path: &str) -> &str {
    path.split('/').next_back().unwrap_or(path)
}

// ============================================================================
// Hatch Workspace Operations (limited support)
// ============================================================================

/// List Hatch environments (minimal support)
fn list_hatch_envs(doc: &DocumentMut) -> Result<Vec<String>, String> {
    let envs = doc
        .get("tool")
        .and_then(|t| t.get("hatch"))
        .and_then(|h| h.get("envs"))
        .and_then(|e| e.as_table())
        .ok_or("Hatch envs not found")?;

    Ok(envs.iter().map(|(k, _)| k.to_string()).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    const PDM_WORKSPACE: &str = r#"[tool.pdm.workspace]
members = ["packages/*", "apps/*"]
"#;

    const POETRY_WORKSPACE: &str = r#"[tool.poetry]
name = "my-monorepo"

[[tool.poetry.packages]]
include = "my_package"
from = "packages/my-package"

[[tool.poetry.packages]]
include = "api"
from = "packages/api"
"#;

    const HATCH_WORKSPACE: &str = r#"[tool.hatch.envs.default]
dependencies = [
    "pytest"
]
"#;

    const SINGLE_PACKAGE: &str = r#"[project]
name = "single-package"
version = "1.0.0"
"#;

    const POETRY_SINGLE: &str = r#"[tool.poetry]
name = "single-package"
version = "1.0.0"
"#;

    #[test]
    fn test_detect_tool() {
        let pdm_doc: DocumentMut = PDM_WORKSPACE.parse().unwrap();
        assert_eq!(detect_tool(&pdm_doc), PythonWorkspaceTool::PDM);

        let poetry_doc: DocumentMut = POETRY_WORKSPACE.parse().unwrap();
        assert_eq!(detect_tool(&poetry_doc), PythonWorkspaceTool::Poetry);

        let hatch_doc: DocumentMut = HATCH_WORKSPACE.parse().unwrap();
        assert_eq!(detect_tool(&hatch_doc), PythonWorkspaceTool::Hatch);

        let single_doc: DocumentMut = SINGLE_PACKAGE.parse().unwrap();
        assert_eq!(detect_tool(&single_doc), PythonWorkspaceTool::None);
    }

    #[test]
    fn test_is_workspace_manifest() {
        let support = PythonWorkspaceSupport::new();

        assert!(support.is_workspace_manifest(PDM_WORKSPACE));
        assert!(support.is_workspace_manifest(POETRY_WORKSPACE));
        assert!(support.is_workspace_manifest(HATCH_WORKSPACE));
        assert!(!support.is_workspace_manifest(SINGLE_PACKAGE));
    }

    #[test]
    fn test_list_pdm_members() {
        let support = PythonWorkspaceSupport::new();

        let members = support.list_workspace_members(PDM_WORKSPACE);
        assert_eq!(members.len(), 2);
        assert!(members.contains(&"packages/*".to_string()));
        assert!(members.contains(&"apps/*".to_string()));
    }

    #[test]
    fn test_list_poetry_packages() {
        let support = PythonWorkspaceSupport::new();

        let members = support.list_workspace_members(POETRY_WORKSPACE);
        assert_eq!(members.len(), 2);
        assert!(members.contains(&"packages/my-package".to_string()));
        assert!(members.contains(&"packages/api".to_string()));
    }

    #[test]
    fn test_list_hatch_envs() {
        let support = PythonWorkspaceSupport::new();

        let members = support.list_workspace_members(HATCH_WORKSPACE);
        assert_eq!(members.len(), 1);
        assert!(members.contains(&"default".to_string()));
    }

    #[test]
    fn test_add_pdm_member() {
        let support = PythonWorkspaceSupport::new();

        let result = support.add_workspace_member(PDM_WORKSPACE, "tools/*");
        assert!(result.contains("\"tools/*\""));

        let members = support.list_workspace_members(&result);
        assert_eq!(members.len(), 3);
        assert!(members.contains(&"tools/*".to_string()));
    }

    #[test]
    fn test_add_poetry_package() {
        let support = PythonWorkspaceSupport::new();

        let result = support.add_workspace_member(POETRY_WORKSPACE, "packages/workers");
        assert!(result.contains("workers"));
        assert!(result.contains("packages/workers"));

        let members = support.list_workspace_members(&result);
        assert_eq!(members.len(), 3);
    }

    #[test]
    fn test_remove_pdm_member() {
        let support = PythonWorkspaceSupport::new();

        let result = support.remove_workspace_member(PDM_WORKSPACE, "apps/*");
        assert!(!result.contains("\"apps/*\""));

        let members = support.list_workspace_members(&result);
        assert_eq!(members.len(), 1);
        assert!(members.contains(&"packages/*".to_string()));
    }

    #[test]
    fn test_remove_poetry_package() {
        let support = PythonWorkspaceSupport::new();

        let result = support.remove_workspace_member(POETRY_WORKSPACE, "packages/api");

        let members = support.list_workspace_members(&result);
        assert_eq!(members.len(), 1);
        assert!(members.contains(&"packages/my-package".to_string()));
    }

    #[test]
    fn test_update_package_name_pep621() {
        let support = PythonWorkspaceSupport::new();

        let result = support.update_package_name(SINGLE_PACKAGE, "new-package");
        assert!(result.contains("new-package"));
        assert!(!result.contains("single-package"));
    }

    #[test]
    fn test_update_package_name_poetry() {
        let support = PythonWorkspaceSupport::new();

        let result = support.update_package_name(POETRY_SINGLE, "new-package");
        assert!(result.contains("new-package"));
        assert!(!result.contains("single-package"));
    }

    #[test]
    fn test_create_pdm_workspace_from_scratch() {
        let support = PythonWorkspaceSupport::new();

        let result = support.add_workspace_member(SINGLE_PACKAGE, "packages/*");
        assert!(result.contains("[tool.pdm.workspace]"));
        assert!(result.contains("\"packages/*\""));

        let members = support.list_workspace_members(&result);
        assert_eq!(members.len(), 1);
    }

    #[test]
    fn test_extract_package_name() {
        assert_eq!(extract_package_name("packages/my-package"), "my-package");
        assert_eq!(extract_package_name("my-package"), "my-package");
        assert_eq!(extract_package_name("packages/sub/deep"), "deep");
    }
}
