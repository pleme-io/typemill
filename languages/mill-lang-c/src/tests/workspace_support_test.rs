use crate::CPlugin;
use mill_plugin_api::LanguagePlugin;

const MAKEFILE_CONTENT: &str = "SUBDIRS = lib1 lib2";

#[test]
fn test_list_workspace_members() {
    let plugin = CPlugin::default();
    let workspace_support = plugin.workspace_support().unwrap();

    let members = workspace_support.list_workspace_members(MAKEFILE_CONTENT);
    assert_eq!(members, vec!["lib1", "lib2"]);
}

#[test]
fn test_add_workspace_member() {
    let plugin = CPlugin::default();
    let workspace_support = plugin.workspace_support().unwrap();

    let new_content = workspace_support.add_workspace_member(MAKEFILE_CONTENT, "lib3");
    assert_eq!(new_content.trim(), "SUBDIRS = lib1 lib2 lib3");
}

#[test]
fn test_remove_workspace_member() {
    let plugin = CPlugin::default();
    let workspace_support = plugin.workspace_support().unwrap();

    let new_content = workspace_support.remove_workspace_member(MAKEFILE_CONTENT, "lib2");
    assert_eq!(new_content.trim(), "SUBDIRS = lib1");
}