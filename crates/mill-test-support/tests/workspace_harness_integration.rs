//! Integration tests for cross-language workspace harness
//!
//! These tests verify that the workspace harness scenarios work with actual language plugins.
//! Each test runs against TypeScript, Rust, Python, Java, and Go plugins to ensure consistent behavior.

use mill_test_support::harness::{
    get_test_registry, WorkspaceExpectedBehavior, WorkspaceOperation, WorkspaceScenarios,
};

// Force linker to include plugin-bundle for inventory collection in tests
extern crate mill_plugin_bundle;

// Force linker to include each language plugin crate
// This is required for inventory system - without these declarations,
// the linker performs dead code elimination and plugins aren't discovered
// Note: Only core languages (ts/rs/py) are included in the main repo
extern crate mill_lang_python;
extern crate mill_lang_rust;
extern crate mill_lang_typescript;

#[cfg(test)]
mod workspace_harness_tests {
    use super::*;

    #[tokio::test]
    async fn test_is_workspace_manifest_positive_all_languages() {
        let registry = get_test_registry();
        let scenario = WorkspaceScenarios::is_workspace_manifest_positive();

        for fixture in scenario.fixtures {
            let plugin = registry
                .find_by_extension(fixture.language.file_extension())
                .unwrap_or_else(|| panic!("Plugin not found for {:?}", fixture.language));

            let workspace_support = plugin
                .workspace_support()
                .unwrap_or_else(|| panic!("{:?} should have workspace support", fixture.language));

            let is_workspace = workspace_support.is_workspace_manifest(fixture.manifest_content);

            match &fixture.expected {
                WorkspaceExpectedBehavior::IsWorkspace(expected) => {
                    assert_eq!(
                        is_workspace, *expected,
                        "is_workspace_manifest failed for {:?}\nManifest: {}\nExpected: {}\nGot: {}",
                        fixture.language, fixture.manifest_content, expected, is_workspace
                    );
                }
                _ => panic!("Wrong expected behavior for is_workspace_manifest test"),
            }
        }
    }

    #[tokio::test]
    async fn test_is_workspace_manifest_negative_all_languages() {
        let registry = get_test_registry();
        let scenario = WorkspaceScenarios::is_workspace_manifest_negative();

        for fixture in scenario.fixtures {
            let plugin = registry
                .find_by_extension(fixture.language.file_extension())
                .unwrap_or_else(|| panic!("Plugin not found for {:?}", fixture.language));

            let workspace_support = plugin
                .workspace_support()
                .unwrap_or_else(|| panic!("{:?} should have workspace support", fixture.language));

            let is_workspace = workspace_support.is_workspace_manifest(fixture.manifest_content);

            match &fixture.expected {
                WorkspaceExpectedBehavior::IsWorkspace(expected) => {
                    assert_eq!(
                        is_workspace, *expected,
                        "is_workspace_manifest (negative) failed for {:?}\nManifest: {}\nExpected: {}\nGot: {}",
                        fixture.language, fixture.manifest_content, expected, is_workspace
                    );
                }
                _ => panic!("Wrong expected behavior for is_workspace_manifest test"),
            }
        }
    }

    #[tokio::test]
    async fn test_list_workspace_members_all_languages() {
        let registry = get_test_registry();
        let scenario = WorkspaceScenarios::list_workspace_members();

        for fixture in scenario.fixtures {
            let plugin = registry
                .find_by_extension(fixture.language.file_extension())
                .unwrap_or_else(|| panic!("Plugin not found for {:?}", fixture.language));

            let workspace_support = plugin
                .workspace_support()
                .unwrap_or_else(|| panic!("{:?} should have workspace support", fixture.language));

            let members = workspace_support.list_workspace_members(fixture.manifest_content);

            match &fixture.expected {
                WorkspaceExpectedBehavior::MembersList(expected) => {
                    assert_eq!(
                        members, *expected,
                        "list_workspace_members failed for {:?}\nManifest: {}\nExpected: {:?}\nGot: {:?}",
                        fixture.language, fixture.manifest_content, expected, members
                    );
                }
                _ => panic!("Wrong expected behavior for list_workspace_members test"),
            }
        }
    }

    #[tokio::test]
    async fn test_add_workspace_member_all_languages() {
        let registry = get_test_registry();
        let scenario = WorkspaceScenarios::add_workspace_member();

        for fixture in scenario.fixtures {
            let plugin = registry
                .find_by_extension(fixture.language.file_extension())
                .unwrap_or_else(|| panic!("Plugin not found for {:?}", fixture.language));

            let workspace_support = plugin
                .workspace_support()
                .unwrap_or_else(|| panic!("{:?} should have workspace support", fixture.language));

            if let WorkspaceOperation::AddWorkspaceMember { member } = &fixture.operation {
                let result =
                    workspace_support.add_workspace_member(fixture.manifest_content, member);

                // Verify the member was added by checking the list contains it
                let members = workspace_support.list_workspace_members(&result);
                let contains = members.contains(member);

                assert!(
                    contains,
                    "add_workspace_member failed for {:?}\nManifest: {}\nMember: {}\nResult: {}\nMembers: {:?}\nShould contain the member",
                    fixture.language, fixture.manifest_content, member, result, members
                );
            } else {
                panic!("Wrong operation for add_workspace_member test");
            }
        }
    }

    #[tokio::test]
    async fn test_add_workspace_member_duplicate_all_languages() {
        let registry = get_test_registry();
        let scenario = WorkspaceScenarios::add_workspace_member_duplicate();

        for fixture in scenario.fixtures {
            let plugin = registry
                .find_by_extension(fixture.language.file_extension())
                .unwrap_or_else(|| panic!("Plugin not found for {:?}", fixture.language));

            let workspace_support = plugin
                .workspace_support()
                .unwrap_or_else(|| panic!("{:?} should have workspace support", fixture.language));

            if let WorkspaceOperation::AddWorkspaceMember { member } = &fixture.operation {
                let result =
                    workspace_support.add_workspace_member(fixture.manifest_content, member);

                // Verify it's still idempotent - member should exist exactly once
                let members = workspace_support.list_workspace_members(&result);
                let count = members.iter().filter(|m| *m == member).count();

                assert_eq!(
                    count, 1,
                    "add_workspace_member duplicate failed for {:?}\nManifest: {}\nMember: {}\nResult: {}\nMembers: {:?}\nShould have exactly 1 occurrence",
                    fixture.language, fixture.manifest_content, member, result, members
                );
            } else {
                panic!("Wrong operation for add_workspace_member test");
            }
        }
    }

    #[tokio::test]
    async fn test_remove_workspace_member_all_languages() {
        let registry = get_test_registry();
        let scenario = WorkspaceScenarios::remove_workspace_member();

        for fixture in scenario.fixtures {
            let plugin = registry
                .find_by_extension(fixture.language.file_extension())
                .unwrap_or_else(|| panic!("Plugin not found for {:?}", fixture.language));

            let workspace_support = plugin
                .workspace_support()
                .unwrap_or_else(|| panic!("{:?} should have workspace support", fixture.language));

            if let WorkspaceOperation::RemoveWorkspaceMember { member } = &fixture.operation {
                let result =
                    workspace_support.remove_workspace_member(fixture.manifest_content, member);

                // Verify the member was removed by checking the list doesn't contain it
                let members = workspace_support.list_workspace_members(&result);
                let contains = members.contains(member);

                assert!(
                    !contains,
                    "remove_workspace_member failed for {:?}\nManifest: {}\nMember: {}\nResult: {}\nMembers: {:?}\nShould NOT contain the member",
                    fixture.language, fixture.manifest_content, member, result, members
                );
            } else {
                panic!("Wrong operation for remove_workspace_member test");
            }
        }
    }

    #[tokio::test]
    async fn test_update_package_name_all_languages() {
        let registry = get_test_registry();
        let scenario = WorkspaceScenarios::update_package_name();

        for fixture in scenario.fixtures {
            let plugin = registry
                .find_by_extension(fixture.language.file_extension())
                .unwrap_or_else(|| panic!("Plugin not found for {:?}", fixture.language));

            let workspace_support = plugin
                .workspace_support()
                .unwrap_or_else(|| panic!("{:?} should have workspace support", fixture.language));

            if let WorkspaceOperation::UpdatePackageName { new_name } = &fixture.operation {
                let result =
                    workspace_support.update_package_name(fixture.manifest_content, new_name);

                match &fixture.expected {
                    WorkspaceExpectedBehavior::NameUpdated(expected_name) => {
                        // Verify the new name appears in the result
                        // Different languages format differently, so just check the string contains the new name
                        assert!(
                            result.contains(expected_name),
                            "update_package_name failed for {:?}\nManifest: {}\nNew name: {}\nResult: {}\nShould contain the new name",
                            fixture.language, fixture.manifest_content, new_name, result
                        );
                    }
                    _ => panic!("Wrong expected behavior for update_package_name test"),
                }
            } else {
                panic!("Wrong operation for update_package_name test");
            }
        }
    }

    #[tokio::test]
    async fn test_list_workspace_members_empty_all_languages() {
        let registry = get_test_registry();
        let scenario = WorkspaceScenarios::list_workspace_members_empty();

        for fixture in scenario.fixtures {
            let plugin = registry
                .find_by_extension(fixture.language.file_extension())
                .unwrap_or_else(|| panic!("Plugin not found for {:?}", fixture.language));

            let workspace_support = plugin
                .workspace_support()
                .unwrap_or_else(|| panic!("{:?} should have workspace support", fixture.language));

            let members = workspace_support.list_workspace_members(fixture.manifest_content);

            match &fixture.expected {
                WorkspaceExpectedBehavior::MembersList(expected) => {
                    assert_eq!(
                        members, *expected,
                        "list_workspace_members_empty failed for {:?}\nManifest: {}\nExpected: {:?}\nGot: {:?}",
                        fixture.language, fixture.manifest_content, expected, members
                    );
                    assert!(
                        members.is_empty(),
                        "Empty workspace should return empty list for {:?}",
                        fixture.language
                    );
                }
                _ => panic!("Wrong expected behavior for list_workspace_members_empty test"),
            }
        }
    }

    #[tokio::test]
    async fn test_remove_nonexistent_member_all_languages() {
        let registry = get_test_registry();
        let scenario = WorkspaceScenarios::remove_nonexistent_member();

        for fixture in scenario.fixtures {
            let plugin = registry
                .find_by_extension(fixture.language.file_extension())
                .unwrap_or_else(|| panic!("Plugin not found for {:?}", fixture.language));

            let workspace_support = plugin
                .workspace_support()
                .unwrap_or_else(|| panic!("{:?} should have workspace support", fixture.language));

            if let WorkspaceOperation::RemoveWorkspaceMember { member } = &fixture.operation {
                let result =
                    workspace_support.remove_workspace_member(fixture.manifest_content, member);

                // Verify the member was not in the list (operation should be no-op)
                let members_before =
                    workspace_support.list_workspace_members(fixture.manifest_content);
                let members_after = workspace_support.list_workspace_members(&result);

                assert_eq!(
                    members_before, members_after,
                    "remove_nonexistent_member should be no-op for {:?}\nManifest: {}\nMember: {}\nBefore: {:?}\nAfter: {:?}",
                    fixture.language, fixture.manifest_content, member, members_before, members_after
                );

                assert!(
                    !members_after.contains(member),
                    "Nonexistent member should not be in list for {:?}",
                    fixture.language
                );
            } else {
                panic!("Wrong operation for remove_nonexistent_member test");
            }
        }
    }
}
