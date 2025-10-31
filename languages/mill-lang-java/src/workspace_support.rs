//! Java workspace support for Maven multi-module projects
//!
//! Handles Maven workspace operations through pom.xml manipulation.

use mill_plugin_api::WorkspaceSupport;
use quick_xml::events::{BytesEnd, BytesStart, BytesText, Event};
use quick_xml::{Reader, Writer};
use std::io::Cursor;
use tracing::{debug, warn};

/// Java workspace support implementation
pub struct JavaWorkspaceSupport;

impl JavaWorkspaceSupport {
    pub fn new() -> Self {
        Self
    }
}

impl Default for JavaWorkspaceSupport {
    fn default() -> Self {
        Self::new()
    }
}

impl WorkspaceSupport for JavaWorkspaceSupport {
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
                warn!(error = %e, new_name = %new_name,
                      "Failed to update package name");
                content.to_string()
            }
        }
    }
}

/// Add a workspace member to pom.xml
fn add_workspace_member_impl(content: &str, member: &str) -> Result<String, String> {
    // First, get existing members
    let mut members = list_workspace_members_impl(content)?;

    // Check if member already exists
    if members.iter().any(|m| m == member) {
        debug!(member = %member, "Member already exists in workspace");
        return Ok(content.to_string());
    }

    // Add and sort members
    members.push(member.to_string());
    members.sort();

    // Rewrite with new members list
    rewrite_with_modules(content, &members)
}

/// Remove a workspace member from pom.xml
fn remove_workspace_member_impl(content: &str, member: &str) -> Result<String, String> {
    let mut members = list_workspace_members_impl(content)?;

    if !members.iter().any(|m| m == member) {
        debug!(member = %member, "Member not found in workspace");
        return Ok(content.to_string());
    }

    members.retain(|m| m != member);

    // Rewrite with updated members list
    rewrite_with_modules(content, &members)
}

/// Rewrite pom.xml with new modules list
fn rewrite_with_modules(content: &str, members: &[String]) -> Result<String, String> {
    let is_workspace = is_workspace_manifest_impl(content);

    if !is_workspace {
        // No existing modules section - insert after packaging
        let mut reader = Reader::from_str(content);
        reader.trim_text(true);
        let mut writer = Writer::new(Cursor::new(Vec::new()));
        let mut buf = Vec::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Eof) => break,
                Ok(Event::End(ref e)) if e.name().as_ref() == b"packaging" => {
                    writer
                        .write_event(Event::End(e.clone()))
                        .map_err(|e| e.to_string())?;
                    write_modules_section(&mut writer, members)?;
                }
                Ok(ref e) => {
                    writer.write_event(e).map_err(|e| e.to_string())?;
                }
                Err(e) => return Err(format!("XML parse error: {}", e)),
            }
            buf.clear();
        }

        return String::from_utf8(writer.into_inner().into_inner())
            .map_err(|e| format!("UTF-8 conversion error: {}", e));
    }

    // Has existing modules section - replace it
    let mut reader = Reader::from_str(content);
    reader.trim_text(true);
    let mut writer = Writer::new(Cursor::new(Vec::new()));
    let mut buf = Vec::new();
    let mut skip_depth = 0;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Eof) => break,
            Ok(Event::Start(ref e)) if e.name().as_ref() == b"modules" && skip_depth == 0 => {
                // Start skipping and write our replacement
                write_modules_section(&mut writer, members)?;
                skip_depth = 1;
            }
            Ok(Event::Start(_)) if skip_depth > 0 => {
                skip_depth += 1;
            }
            Ok(Event::End(ref e)) if e.name().as_ref() == b"modules" && skip_depth == 1 => {
                skip_depth = 0;
                // Skipped the entire old modules section
            }
            Ok(Event::End(_)) if skip_depth > 0 => {
                skip_depth -= 1;
            }
            Ok(ref e) if skip_depth == 0 => {
                writer.write_event(e).map_err(|e| e.to_string())?;
            }
            Err(e) => return Err(format!("XML parse error: {}", e)),
            _ => {} // Skip events inside modules section
        }
        buf.clear();
    }

    String::from_utf8(writer.into_inner().into_inner())
        .map_err(|e| format!("UTF-8 conversion error: {}", e))
}

/// Check if pom.xml is a workspace manifest (has <modules> section)
fn is_workspace_manifest_impl(content: &str) -> bool {
    let mut reader = Reader::from_str(content);
    reader.trim_text(true);
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) if e.name().as_ref() == b"modules" => {
                return true;
            }
            Ok(Event::Eof) => break,
            Err(_) => return false,
            _ => {}
        }
        buf.clear();
    }

    false
}

/// List all workspace members from pom.xml
fn list_workspace_members_impl(content: &str) -> Result<Vec<String>, String> {
    let mut reader = Reader::from_str(content);
    reader.trim_text(true);
    let mut buf = Vec::new();

    let mut in_modules = false;
    let mut members = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) if e.name().as_ref() == b"modules" => {
                in_modules = true;
            }
            Ok(Event::End(ref e)) if e.name().as_ref() == b"modules" => {
                in_modules = false;
            }
            Ok(Event::Start(ref e)) if in_modules && e.name().as_ref() == b"module" => {
                // Read module text
                buf.clear();
                if let Ok(Event::Text(t)) = reader.read_event_into(&mut buf) {
                    let module_name = t.unescape().unwrap_or_default().to_string();
                    members.push(module_name);
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(format!("XML parse error: {}", e)),
            _ => {}
        }
        buf.clear();
    }

    Ok(members)
}

/// Update package name (artifactId) in pom.xml
fn update_package_name_impl(content: &str, new_name: &str) -> Result<String, String> {
    let mut reader = Reader::from_str(content);
    reader.trim_text(true);

    let mut writer = Writer::new(Cursor::new(Vec::new()));
    let mut buf = Vec::new();
    let mut in_artifact_id = false;
    let mut depth = 0;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Eof) => break,
            Ok(Event::Start(ref e)) => {
                if e.name().as_ref() == b"artifactId" && depth == 1 {
                    // Only match top-level artifactId (direct child of project)
                    in_artifact_id = true;
                }
                writer
                    .write_event(Event::Start(e.clone()))
                    .map_err(|e| e.to_string())?;
                if e.name().as_ref() == b"project" {
                    depth += 1;
                }
            }
            Ok(Event::Text(_)) if in_artifact_id => {
                // Always replace the artifactId text with new_name
                writer
                    .write_event(Event::Text(BytesText::new(new_name)))
                    .map_err(|e| e.to_string())?;
            }
            Ok(Event::End(ref e)) => {
                writer
                    .write_event(Event::End(e.clone()))
                    .map_err(|e| e.to_string())?;
                if e.name().as_ref() == b"artifactId" {
                    in_artifact_id = false;
                }
                if e.name().as_ref() == b"project" {
                    depth -= 1;
                }
            }
            Ok(ref e) => {
                writer.write_event(e).map_err(|e| e.to_string())?;
            }
            Err(e) => return Err(format!("XML parse error: {}", e)),
        }
        buf.clear();
    }

    String::from_utf8(writer.into_inner().into_inner())
        .map_err(|e| format!("UTF-8 conversion error: {}", e))
}

/// Helper to write modules section
fn write_modules_section<W: std::io::Write>(
    writer: &mut Writer<W>,
    members: &[String],
) -> Result<(), String> {
    writer
        .write_event(Event::Text(BytesText::new("\n    ")))
        .map_err(|e| e.to_string())?;

    let modules_start = BytesStart::new("modules");
    writer
        .write_event(Event::Start(modules_start.borrow()))
        .map_err(|e| e.to_string())?;

    for member in members {
        writer
            .write_event(Event::Text(BytesText::new("\n        ")))
            .map_err(|e| e.to_string())?;

        writer
            .write_event(Event::Start(BytesStart::new("module")))
            .map_err(|e| e.to_string())?;

        writer
            .write_event(Event::Text(BytesText::new(member)))
            .map_err(|e| e.to_string())?;

        writer
            .write_event(Event::End(BytesEnd::new("module")))
            .map_err(|e| e.to_string())?;
    }

    writer
        .write_event(Event::Text(BytesText::new("\n    ")))
        .map_err(|e| e.to_string())?;

    writer
        .write_event(Event::End(BytesEnd::new("modules")))
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SIMPLE_WORKSPACE_POM: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<project>
    <modelVersion>4.0.0</modelVersion>
    <groupId>com.example</groupId>
    <artifactId>parent-project</artifactId>
    <version>1.0.0</version>
    <packaging>pom</packaging>

    <modules>
        <module>module-a</module>
        <module>module-b</module>
    </modules>
</project>"#;

    const SINGLE_MODULE_POM: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<project>
    <modelVersion>4.0.0</modelVersion>
    <groupId>com.example</groupId>
    <artifactId>single-module</artifactId>
    <version>1.0.0</version>
    <packaging>jar</packaging>
</project>"#;

    #[test]
    fn test_is_workspace_manifest() {
        let support = JavaWorkspaceSupport::new();

        assert!(support.is_workspace_manifest(SIMPLE_WORKSPACE_POM));
        assert!(!support.is_workspace_manifest(SINGLE_MODULE_POM));
    }

    #[test]
    fn test_list_workspace_members() {
        let support = JavaWorkspaceSupport::new();

        let members = support.list_workspace_members(SIMPLE_WORKSPACE_POM);
        assert_eq!(members.len(), 2);
        assert!(members.contains(&"module-a".to_string()));
        assert!(members.contains(&"module-b".to_string()));

        let empty = support.list_workspace_members(SINGLE_MODULE_POM);
        assert_eq!(empty.len(), 0);
    }

    #[test]
    fn test_add_workspace_member() {
        let support = JavaWorkspaceSupport::new();

        let result = support.add_workspace_member(SIMPLE_WORKSPACE_POM, "module-c");
        assert!(result.contains("<module>module-c</module>"));

        let members = support.list_workspace_members(&result);
        assert_eq!(members.len(), 3);
        assert!(members.contains(&"module-c".to_string()));
    }

    #[test]
    fn test_add_duplicate_member() {
        let support = JavaWorkspaceSupport::new();

        let result = support.add_workspace_member(SIMPLE_WORKSPACE_POM, "module-a");
        let members = support.list_workspace_members(&result);
        assert_eq!(members.len(), 2); // Should not duplicate
    }

    #[test]
    fn test_remove_workspace_member() {
        let support = JavaWorkspaceSupport::new();

        let result = support.remove_workspace_member(SIMPLE_WORKSPACE_POM, "module-b");
        assert!(!result.contains("<module>module-b</module>"));

        let members = support.list_workspace_members(&result);
        assert_eq!(members.len(), 1);
        assert!(members.contains(&"module-a".to_string()));
    }

    #[test]
    fn test_remove_nonexistent_member() {
        let support = JavaWorkspaceSupport::new();

        let result = support.remove_workspace_member(SIMPLE_WORKSPACE_POM, "module-z");
        let members = support.list_workspace_members(&result);
        assert_eq!(members.len(), 2); // Should remain unchanged
    }

    #[test]
    fn test_update_package_name() {
        let support = JavaWorkspaceSupport::new();

        let result = support.update_package_name(SINGLE_MODULE_POM, "renamed-module");

        assert!(result.contains("<artifactId>renamed-module</artifactId>"));
        assert!(!result.contains("<artifactId>single-module</artifactId>"));
    }

    #[test]
    fn test_add_member_to_nonworkspace() {
        let support = JavaWorkspaceSupport::new();

        let pom_with_packaging = r#"<?xml version="1.0" encoding="UTF-8"?>
<project>
    <modelVersion>4.0.0</modelVersion>
    <groupId>com.example</groupId>
    <artifactId>single-module</artifactId>
    <version>1.0.0</version>
    <packaging>pom</packaging>
</project>"#;

        let result = support.add_workspace_member(pom_with_packaging, "new-module");
        assert!(result.contains("<module>new-module</module>"));
        assert!(support.is_workspace_manifest(&result));
    }

    #[test]
    fn test_alphabetical_sorting() {
        let support = JavaWorkspaceSupport::new();

        let result = support.add_workspace_member(SIMPLE_WORKSPACE_POM, "module-aaa");
        let members = support.list_workspace_members(&result);

        assert_eq!(members[0], "module-a");
        assert_eq!(members[1], "module-aaa");
        assert_eq!(members[2], "module-b");
    }
}