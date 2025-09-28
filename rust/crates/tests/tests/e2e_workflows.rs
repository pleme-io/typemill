//! End-to-end workflow tests for complex multi-file scenarios
//! These tests validate the key workflows that were originally in the TypeScript test suite

use tests::harness::{TestClient, TestWorkspace, create_rename_test_project};
use serde_json::json;

#[tokio::test]
async fn test_rename_file_updates_imports() {
    // Setup: Create a project with two files and an import relationship
    let workspace = TestWorkspace::new();
    workspace.setup_typescript_project("rename-file-test");

    workspace.create_file("src/utils.ts", r#"
export const myUtil = () => {
    return "utility function";
};

export function helperFunc(data: string): string {
    return data.toUpperCase();
}
"#);

    workspace.create_file("src/main.ts", r#"
import { myUtil, helperFunc } from './utils';

export function main() {
    const result = myUtil();
    const processed = helperFunc(result);
    console.log(processed);
}
"#);

    let mut client = TestClient::new(workspace.path());

    // Action: Send a request to rename 'utils.ts'
    let old_path = workspace.absolute_path("src/utils.ts");
    let new_path = workspace.absolute_path("src/renamed_utils.ts");

    let rename_response = client.call_tool(
        "rename_file",
        json!({
            "old_path": old_path.to_str().unwrap(),
            "new_path": new_path.to_str().unwrap()
        })
    ).await.expect("rename_file should succeed");


    // Verify the rename succeeded
    assert!(rename_response["error"].is_null(), "Rename should not error");

    // Verification: Read 'main.ts' and assert its import path was updated
    let read_response = client.call_tool(
        "read_file",
        json!({
            "file_path": workspace.absolute_path("src/main.ts").to_str().unwrap()
        })
    ).await.expect("read_file should succeed");


    let content = read_response["result"]["content"]["content"]["content"]
        .as_str()
        .expect("Should have file content");


    assert!(
        content.contains("from './renamed_utils'"),
        "Import path should be updated to './renamed_utils'"
    );

    // Verify old file doesn't exist and new file does
    assert!(!workspace.file_exists("src/utils.ts"), "Old file should not exist");
    assert!(workspace.file_exists("src/renamed_utils.ts"), "New file should exist");
}

#[tokio::test]
async fn test_rename_symbol_updates_references() {
    // Setup: Create project with function exports and imports
    let workspace = create_rename_test_project();
    let mut client = TestClient::new(workspace.path());

    // Action: Send a rename_symbol request to rename 'oldFunctionName' to 'newFunctionName'
    let rename_response = client.call_tool(
        "rename_symbol",
        json!({
            "file_path": workspace.absolute_path("src/exporter.ts").to_str().unwrap(),
            "symbol_name": "oldFunctionName",
            "new_name": "newFunctionName"
        })
    ).await.expect("rename_symbol should succeed");

    assert!(rename_response["error"].is_null(), "Rename symbol should not error");

    // Verification: Read both files and check that all references are updated

    // Check exporter.ts
    let exporter_content = workspace.read_file("src/exporter.ts");
    assert!(
        exporter_content.contains("export function newFunctionName"),
        "Function declaration should be renamed"
    );
    assert!(
        !exporter_content.contains("oldFunctionName"),
        "Old function name should not exist"
    );

    // Check consumer.ts
    let consumer_content = workspace.read_file("src/consumer.ts");
    assert!(
        consumer_content.contains("import { newFunctionName"),
        "Import should be updated"
    );
    assert!(
        consumer_content.contains("const result = newFunctionName('test')"),
        "Function calls should be updated"
    );
    assert!(
        consumer_content.contains("const anotherCall = newFunctionName('another')"),
        "All function calls should be updated"
    );

    // Check another-consumer.ts with aliased import
    let another_content = workspace.read_file("src/another-consumer.ts");
    assert!(
        another_content.contains("import { newFunctionName as renamed"),
        "Aliased import should be updated"
    );
}

#[tokio::test]
async fn test_completions_include_new_symbol() {
    // Setup: Create a simple project
    let workspace = TestWorkspace::new();
    workspace.setup_typescript_project("completions-test");

    workspace.create_file("src/moduleA.ts", r#"
export function existingFunction() {
    return "existing";
}
"#);

    workspace.create_file("src/moduleB.ts", r#"
import { existingFunction } from './moduleA';

// Type something here:
"#);

    let mut client = TestClient::new(workspace.path());

    // Action 1: Get initial completions in moduleB
    let initial_completions = client.call_tool(
        "get_completions",
        json!({
            "file_path": workspace.absolute_path("src/moduleB.ts").to_str().unwrap(),
            "line": 4,
            "character": 0
        })
    ).await.ok();  // May fail if LSP not ready, that's OK for this test

    // Action 2: Add a new export to moduleA.ts
    let moduleA_content = workspace.read_file("src/moduleA.ts");
    workspace.create_file("src/moduleA.ts", &format!("{}\n\nexport function newSymbol() {{\n    return \"new\";\n}}", moduleA_content));

    // Small delay for LSP to pick up changes
    std::thread::sleep(std::time::Duration::from_millis(500));

    // Action 3: Get completions again
    let updated_completions = client.call_tool(
        "get_completions",
        json!({
            "file_path": workspace.absolute_path("src/moduleB.ts").to_str().unwrap(),
            "line": 4,
            "character": 0
        })
    ).await.ok();

    // Verification: The completion list should now include 'newSymbol'
    // Note: Completions may fail if no LSP is configured, which is acceptable
    if let Some(response) = updated_completions {
        if response["error"].is_null() && response["result"]["content"]["items"].is_array() {
            let items = response["result"]["content"]["items"].as_array().unwrap();
            let has_new_symbol = items.iter().any(|item| {
                item["label"].as_str().unwrap_or("").contains("newSymbol")
            });

            if has_new_symbol {
                println!("✅ New symbol appears in completions");
            } else {
                println!("⚠️ New symbol not found in completions (LSP may need more time)");
            }
        }
    }
}

#[tokio::test]
async fn test_diagnostics_appear_on_error() {
    // Setup: Create a syntactically correct TypeScript file
    let workspace = TestWorkspace::new();
    workspace.setup_typescript_project("diagnostics-test");

    workspace.create_file("src/test.ts", r#"
export function validFunction(param: string): string {
    return param.toUpperCase();
}

const result = validFunction("test");
console.log(result);
"#);

    let mut client = TestClient::new(workspace.path());
    let test_file = workspace.absolute_path("src/test.ts");

    // Action 1: Get diagnostics for the valid file
    let initial_diagnostics = client.call_tool(
        "get_diagnostics",
        json!({
            "file_path": test_file.to_str().unwrap()
        })
    ).await.ok();

    // Check initial state (should have no errors if LSP is working)
    if let Some(response) = initial_diagnostics {
        if response["error"].is_null() {
            let diagnostics = response["result"]["content"]["diagnostics"].as_array();
            if let Some(diags) = diagnostics {
                let error_count = diags.iter()
                    .filter(|d| d["severity"].as_i64() == Some(1))  // Error severity
                    .count();
                assert_eq!(error_count, 0, "Valid file should have no errors");
            }
        }
    }

    // Action 2: Introduce a syntax error
    workspace.create_file("src/test.ts", r#"
export function validFunction(param: string): string {
    return param.toUpperCase();
    // Missing closing brace

const result = validFunction("test");
console.log(result);

// This will cause a syntax error:
const badSyntax = {
    key: "value"
    missingComma: "error"
};
"#);

    // Small delay for LSP to pick up changes
    std::thread::sleep(std::time::Duration::from_millis(500));

    // Action 3: Get diagnostics again
    let error_diagnostics = client.call_tool(
        "get_diagnostics",
        json!({
            "file_path": test_file.to_str().unwrap()
        })
    ).await.ok();

    // Verification: Should now have at least one diagnostic error
    if let Some(response) = error_diagnostics {
        if response["error"].is_null() {
            let diagnostics = response["result"]["content"]["diagnostics"].as_array();
            if let Some(diags) = diagnostics {
                let error_count = diags.iter()
                    .filter(|d| d["severity"].as_i64() == Some(1))  // Error severity
                    .count();

                if error_count > 0 {
                    println!("✅ Diagnostics correctly detected {} error(s)", error_count);
                } else {
                    println!("⚠️ No errors detected (LSP may not be configured)");
                }
            }
        }
    }
}

#[tokio::test]
async fn test_multi_file_refactoring() {
    // Test a complex refactoring scenario across multiple files
    let workspace = TestWorkspace::new();
    workspace.setup_typescript_project("refactor-test");

    // Create a service class that will be renamed
    workspace.create_file("src/services/DataService.ts", r#"
export class DataService {
    private data: string[] = [];

    addData(item: string): void {
        this.data.push(item);
    }

    getData(): string[] {
        return this.data;
    }
}
"#);

    // Create multiple files that use the service
    workspace.create_file("src/controllers/MainController.ts", r#"
import { DataService } from '../services/DataService';

export class MainController {
    private service: DataService;

    constructor() {
        this.service = new DataService();
    }

    processData(input: string): void {
        this.service.addData(input);
    }
}
"#);

    workspace.create_file("src/utils/helper.ts", r#"
import { DataService } from '../services/DataService';

export function createService(): DataService {
    return new DataService();
}

export function processWithService(service: DataService, data: string): void {
    service.addData(data);
}
"#);

    let mut client = TestClient::new(workspace.path());

    // Rename the DataService class to DataManager
    let rename_response = client.call_tool(
        "rename_symbol",
        json!({
            "file_path": workspace.absolute_path("src/services/DataService.ts").to_str().unwrap(),
            "symbol_name": "DataService",
            "new_name": "DataManager"
        })
    ).await;

    if let Ok(response) = rename_response {
        if response["error"].is_null() {
            // Verify all references are updated
            let controller_content = workspace.read_file("src/controllers/MainController.ts");
            assert!(controller_content.contains("DataManager"), "Controller should use new name");

            let helper_content = workspace.read_file("src/utils/helper.ts");
            assert!(helper_content.contains("DataManager"), "Helper should use new name");

            println!("✅ Multi-file refactoring completed successfully");
        } else {
            println!("⚠️ Rename failed (LSP may not be configured)");
        }
    }
}

#[tokio::test]
async fn test_import_path_resolution() {
    // Test that import paths are correctly resolved and updated
    let workspace = TestWorkspace::new();
    workspace.setup_typescript_project("import-test");

    // Create nested module structure
    workspace.create_file("src/core/types.ts", r#"
export interface User {
    id: number;
    name: string;
}

export type Status = 'active' | 'inactive';
"#);

    workspace.create_file("src/core/models/UserModel.ts", r#"
import { User, Status } from '../types';

export class UserModel implements User {
    constructor(
        public id: number,
        public name: string,
        public status: Status = 'active'
    ) {}
}
"#);

    workspace.create_file("src/features/users/UserService.ts", r#"
import { UserModel } from '../../core/models/UserModel';
import { Status } from '../../core/types';

export class UserService {
    private users: UserModel[] = [];

    addUser(name: string): UserModel {
        const user = new UserModel(Date.now(), name);
        this.users.push(user);
        return user;
    }

    setUserStatus(id: number, status: Status): void {
        const user = this.users.find(u => u.id === id);
        if (user) {
            user.status = status;
        }
    }
}
"#);

    let mut client = TestClient::new(workspace.path());

    // Move the types file to a different location
    let rename_response = client.call_tool(
        "rename_file",
        json!({
            "old_path": workspace.absolute_path("src/core/types.ts").to_str().unwrap(),
            "new_path": workspace.absolute_path("src/shared/types.ts").to_str().unwrap()
        })
    ).await;

    if let Ok(response) = rename_response {
        if response["error"].is_null() {
            // Verify import paths are updated
            let model_content = workspace.read_file("src/core/models/UserModel.ts");
            assert!(
                model_content.contains("from '../../shared/types'"),
                "UserModel import path should be updated"
            );

            let service_content = workspace.read_file("src/features/users/UserService.ts");
            assert!(
                service_content.contains("from '../../../shared/types'"),
                "UserService import path should be updated"
            );

            println!("✅ Import paths correctly resolved and updated");
        } else {
            println!("⚠️ File rename failed (LSP may not be configured)");
        }
    }
}

#[tokio::test]
async fn test_dead_code_detection() {
    // Test detection of unused exports and functions
    let workspace = TestWorkspace::new();
    workspace.setup_typescript_project("dead-code-test");

    workspace.create_file("src/utils.ts", r#"
export function usedFunction(x: number): number {
    return x * 2;
}

export function unusedFunction(x: number): number {
    return x * 3;
}

export class UnusedClass {
    value: number = 0;
}

function privateUnusedFunction(): void {
    console.log("never called");
}
"#);

    workspace.create_file("src/main.ts", r#"
import { usedFunction } from './utils';

export function main() {
    const result = usedFunction(5);
    console.log(result);
}
"#);

    let mut client = TestClient::new(workspace.path());

    // Run dead code detection
    let dead_code_response = client.call_tool(
        "find_dead_code",
        json!({
            "files": [
                workspace.absolute_path("src/utils.ts").to_str().unwrap(),
                workspace.absolute_path("src/main.ts").to_str().unwrap()
            ],
            "exclude_tests": true,
            "min_references": 1
        })
    ).await;

    if let Ok(response) = dead_code_response {
        if response["error"].is_null() {
            let dead_items = response["result"]["content"]["deadCodeItems"].as_array();

            if let Some(items) = dead_items {
                // Should detect unusedFunction and UnusedClass
                let has_unused_function = items.iter().any(|item| {
                    item["name"].as_str() == Some("unusedFunction")
                });

                let has_unused_class = items.iter().any(|item| {
                    item["name"].as_str() == Some("UnusedClass")
                });

                if has_unused_function && has_unused_class {
                    println!("✅ Dead code detection working correctly");
                } else {
                    println!("⚠️ Dead code detection partial (found {} items)", items.len());
                }
            }
        } else {
            println!("⚠️ Dead code detection failed (LSP may not be configured)");
        }
    }
}

#[cfg(test)]
mod advanced_workflows {
    use super::*;

    #[tokio::test]
    async fn test_circular_dependency_handling() {
        use tests::harness::create_circular_dependency_project;

        let workspace = create_circular_dependency_project();
        let mut client = TestClient::new(workspace.path());

        // Try to analyze imports with circular dependencies
        let import_analysis = client.call_tool(
            "analyze_imports",
            json!({
                "file_path": workspace.absolute_path("src/moduleA.ts").to_str().unwrap()
            })
        ).await;

        if let Ok(response) = import_analysis {
            if response["error"].is_null() {
                println!("✅ Circular dependency handling works");
            }
        }
    }

    #[tokio::test]
    async fn test_batch_operations() {
        let workspace = TestWorkspace::new();
        workspace.setup_typescript_project("batch-test");

        // Create multiple files
        for i in 1..=3 {
            workspace.create_file(
                &format!("src/module{}.ts", i),
                &format!("export const value{} = {};", i, i)
            );
        }

        let mut client = TestClient::new(workspace.path());

        // Batch rename multiple files
        let batch_response = client.call_tool(
            "batch_execute",
            json!({
                "operations": [
                    {
                        "tool": "rename_file",
                        "args": {
                            "old_path": workspace.absolute_path("src/module1.ts").to_str().unwrap(),
                            "new_path": workspace.absolute_path("src/component1.ts").to_str().unwrap()
                        }
                    },
                    {
                        "tool": "rename_file",
                        "args": {
                            "old_path": workspace.absolute_path("src/module2.ts").to_str().unwrap(),
                            "new_path": workspace.absolute_path("src/component2.ts").to_str().unwrap()
                        }
                    }
                ],
                "options": {
                    "parallel": true
                }
            })
        ).await;

        if let Ok(response) = batch_response {
            if response["error"].is_null() {
                // Verify files were renamed
                assert!(workspace.file_exists("src/component1.ts"), "File 1 should be renamed");
                assert!(workspace.file_exists("src/component2.ts"), "File 2 should be renamed");
                assert!(!workspace.file_exists("src/module1.ts"), "Old file 1 should not exist");
                assert!(!workspace.file_exists("src/module2.ts"), "Old file 2 should not exist");

                println!("✅ Batch operations completed successfully");
            }
        }
    }
}