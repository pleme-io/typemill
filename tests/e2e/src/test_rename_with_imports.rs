//! Integration tests for file rename with import updates (MIGRATED VERSION)
//!
//! BEFORE: 568 lines with duplicated setup/plan/apply logic
//! AFTER: Using shared helpers from test_helpers.rs
//!
//! Tests verify that when a file is renamed, all imports are automatically updated.

use crate::harness::{TestClient, TestWorkspace};
use crate::test_helpers::*;
use mill_test_support::harness::mcp_fixtures::{MARKDOWN_RENAME_FILE_TESTS, RENAME_FILE_TESTS};
use serde_json::json;

/// Test 1: Rename file with import updates (TypeScript) - FIXTURE LOOP
/// BEFORE: ~140 lines | AFTER: ~70 lines (~50% reduction)
#[tokio::test]
async fn test_rename_file_updates_imports_from_fixtures() {
    for case in RENAME_FILE_TESTS {
        println!("\n🧪 Running test case: {}", case.test_name);

        let workspace = TestWorkspace::new();
        setup_workspace_from_fixture(&workspace, case.initial_files);

        let mut client = TestClient::new(workspace.path());
        let params =
            build_rename_params(&workspace, case.old_file_path, case.new_file_path, "file");

        let mut params_exec = params.clone();
        if let Some(options) = params_exec.get_mut("options") {
            options["dryRun"] = json!(false);
            options["validateChecksums"] = json!(true);
        } else {
            params_exec["options"] = json!({"dryRun": false, "validateChecksums": true});
        }

        let apply_result = client.call_tool("rename_all", params_exec).await;

        if case.expect_success {
            let response = apply_result.expect("rename_all should succeed");
            let result = response
                .get("result")
                .and_then(|r| r.get("content"))
                .expect("Apply should have result.content");

            assert_eq!(
                result.get("status").and_then(|v| v.as_str()),
                Some("success"),
                "Apply should succeed for test case: {}",
                case.test_name
            );

            assert!(
                !workspace.file_exists(case.old_file_path),
                "Old file should be deleted in test case: {}",
                case.test_name
            );
            assert!(
                workspace.file_exists(case.new_file_path),
                "New file should exist in test case: {}",
                case.test_name
            );

            for (importer_path, expected_substring) in case.expected_import_updates {
                let content = workspace.read_file(importer_path);
                assert!(content.contains(expected_substring),
                    "❌ Import in '{}' was not updated correctly in '{}'.\nExpected: '{}'\nActual:\n{}",
                    importer_path, case.test_name, expected_substring, content);
                println!(
                    "  ✅ Verified import updated in '{}': contains '{}'",
                    importer_path, expected_substring
                );
            }
        } else {
            assert!(
                apply_result.is_err() || apply_result.unwrap().get("error").is_some(),
                "Operation should fail for test case: {}",
                case.test_name
            );
        }

        println!("✅ Test case '{}' passed", case.test_name);
    }
}

/// Test 2: Rename file updates parent directory importer (CLOSURE-BASED API)
/// BEFORE: ~90 lines | AFTER: ~35 lines (~61% reduction)
#[tokio::test]
async fn test_rename_file_updates_parent_directory_importer() {
    let workspace = TestWorkspace::new();
    workspace.create_directory("lib");
    workspace.create_file("lib/utils.ts", "export function helper() { return 42; }\n");
    workspace.create_file(
        "app.ts",
        "import { helper } from './lib/utils';\nconsole.log(helper());\n",
    );

    let mut client = TestClient::new(workspace.path());
    let mut params = build_rename_params(&workspace, "lib/utils.ts", "lib/helpers.ts", "file");

    if let Some(options) = params.get_mut("options") {
        options["dryRun"] = json!(false);
    } else {
        params["options"] = json!({"dryRun": false});
    }

    client
        .call_tool("rename_all", params)
        .await
        .expect("Apply should succeed");

    let content = workspace.read_file("app.ts");
    assert!(
        content.contains("from './lib/helpers'"),
        "Import should be updated to new filename"
    );
    assert!(
        !content.contains("from './lib/utils'"),
        "Old import should be gone"
    );
}

/// Test 3: Rename file updates sibling directory importer (CLOSURE-BASED API)
/// BEFORE: ~95 lines | AFTER: ~40 lines (~58% reduction)
#[tokio::test]
async fn test_rename_file_updates_sibling_directory_importer() {
    let workspace = TestWorkspace::new();
    workspace.create_directory("components");
    workspace.create_directory("services");
    workspace.create_file("services/api.ts", "export function fetchData() {}\n");
    workspace.create_file("components/DataView.tsx",
        "import { fetchData } from '../services/api';\nexport function DataView() { fetchData(); }\n");

    let mut client = TestClient::new(workspace.path());
    let mut params = build_rename_params(
        &workspace,
        "services/api.ts",
        "services/dataService.ts",
        "file",
    );

    if let Some(options) = params.get_mut("options") {
        options["dryRun"] = json!(false);
    } else {
        params["options"] = json!({"dryRun": false});
    }

    client
        .call_tool("rename_all", params)
        .await
        .expect("Apply should succeed");

    let content = workspace.read_file("components/DataView.tsx");
    assert!(
        content.contains("from '../services/dataService'"),
        "Import should be updated to new filename"
    );
}

/// Test 4: Directory rename updates all imports (CLOSURE-BASED API)
/// BEFORE: ~105 lines | AFTER: ~45 lines (~57% reduction)
#[tokio::test]
async fn test_directory_rename_updates_all_imports() {
    let workspace = TestWorkspace::new();
    workspace.create_directory("old_utils");
    workspace.create_file(
        "old_utils/math.ts",
        "export function add(a, b) { return a + b; }\n",
    );
    workspace.create_file("app.ts", "import { add } from './old_utils/math';\n");

    let mut client = TestClient::new(workspace.path());
    let mut params = build_rename_params(&workspace, "old_utils", "new_utils", "directory");

    if let Some(options) = params.get_mut("options") {
        options["dryRun"] = json!(false);
    } else {
        params["options"] = json!({"dryRun": false});
    }

    client
        .call_tool("rename_all", params)
        .await
        .expect("Apply should succeed");

    let content = workspace.read_file("app.ts");
    assert!(
        content.contains("from './new_utils/math'"),
        "Import should be updated to new directory name"
    );
}

/// Test 5: Markdown file rename updates links - FIXTURE LOOP
/// BEFORE: ~85 lines | AFTER: ~50 lines (~41% reduction)
#[tokio::test]
async fn test_markdown_file_rename_updates_links() {
    for case in MARKDOWN_RENAME_FILE_TESTS {
        println!("\n🧪 Running markdown test case: {}", case.test_name);

        let workspace = TestWorkspace::new();
        setup_workspace_from_fixture(&workspace, case.initial_files);

        let mut client = TestClient::new(workspace.path());
        let mut params =
            build_rename_params(&workspace, case.old_file_path, case.new_file_path, "file");

        if let Some(options) = params.get_mut("options") {
            options["dryRun"] = json!(false);
        } else {
            params["options"] = json!({"dryRun": false});
        }

        client
            .call_tool("rename_all", params)
            .await
            .expect("rename_all should succeed");

        for (file_path, expected_substring) in case.expected_import_updates {
            let content = workspace.read_file(file_path);
            assert!(
                content.contains(expected_substring),
                "Markdown link in '{}' not updated in '{}'.\nExpected: '{}'\nActual:\n{}",
                file_path,
                case.test_name,
                expected_substring,
                content
            );
        }

        println!("✅ Test case '{}' passed", case.test_name);
    }
}

/// Test 6: Markdown links skip external URLs (CLOSURE-BASED API)
/// BEFORE: ~53 lines | AFTER: ~35 lines (~34% reduction)
#[tokio::test]
async fn test_markdown_links_skip_external_urls() {
    let workspace = TestWorkspace::new();
    workspace.create_directory("docs");
    workspace.create_file("docs/guide.md", "# Guide\nContent here.");
    workspace.create_file(
        "README.md",
        r#"# Project

See [Guide](docs/guide.md) for details.
Visit [our website](https://example.com) for more info.
Check [GitHub](https://github.com/user/repo) repo.
"#,
    );

    let mut client = TestClient::new(workspace.path());
    let mut params = build_rename_params(&workspace, "docs/guide.md", "docs/user-guide.md", "file");

    if let Some(options) = params.get_mut("options") {
        options["dryRun"] = json!(false);
    } else {
        params["options"] = json!({"dryRun": false});
    }

    client
        .call_tool("rename_all", params)
        .await
        .expect("Apply should succeed");

    let content = workspace.read_file("README.md");
    assert!(
        content.contains("[Guide](docs/user-guide.md)"),
        "Local markdown link should be updated"
    );
    assert!(
        content.contains("[our website](https://example.com)"),
        "External URL should NOT be changed"
    );
    assert!(
        content.contains("[GitHub](https://github.com/user/repo)"),
        "GitHub URL should NOT be changed"
    );
}
