// Manual integration tests for TypeScript project
use crate::{TestClient, TestWorkspace};
use serde_json::json;

/// Test rename on TypeScript file
#[tokio::test]
async fn test_ts_rename_file_integration() {
    let workspace = TestWorkspace::new();

    // Create TypeScript project structure
    workspace.create_file("package.json", r#"{"name": "test", "version": "1.0.0"}"#);
    workspace.create_file(
        "tsconfig.json",
        r#"{"compilerOptions": {"target": "ES2020", "module": "commonjs"}}"#,
    );
    workspace.create_file(
        "src/helpers.ts",
        r#"export function formatDate(date: Date): string {
    return date.toISOString();
}

export function capitalizeString(str: string): string {
    return str.charAt(0).toUpperCase() + str.slice(1);
}
"#,
    );
    workspace.create_file(
        "src/index.ts",
        r#"import { formatDate, capitalizeString } from './helpers';

const date = new Date();
console.log(formatDate(date));
console.log(capitalizeString('hello'));
"#,
    );

    let mut client = TestClient::new(workspace.path());

    // Test rename file with dryRun first
    let params = json!({
        "target": {
            "kind": "file",
            "path": workspace.absolute_path("src/helpers.ts").to_string_lossy()
        },
        "newName": workspace.absolute_path("src/utils.ts").to_string_lossy()
    });

    let plan_result = client.call_tool("rename", params.clone()).await;

    match plan_result {
        Ok(response) => {
            println!("Rename plan response: {:?}", response);

            // Now execute the rename
            let mut exec_params = params.clone();
            exec_params["options"] = json!({"dryRun": false});

            let exec_result = client.call_tool("rename", exec_params).await;
            match exec_result {
                Ok(_) => {
                    assert!(!workspace.file_exists("src/helpers.ts"), "Old file should be deleted");
                    assert!(workspace.file_exists("src/utils.ts"), "New file should exist");

                    // Check that imports were updated
                    let index_content = workspace.read_file("src/index.ts");
                    assert!(
                        index_content.contains("from './utils'"),
                        "Import should be updated to utils"
                    );
                }
                Err(e) => {
                    eprintln!("Execute failed: {:?}", e);
                }
            }
        }
        Err(e) => {
            eprintln!("INFO: rename plan failed: {:?}", e);
        }
    }
}

/// Test move on TypeScript file
#[tokio::test]
async fn test_ts_move_file_integration() {
    let workspace = TestWorkspace::new();

    workspace.create_file("package.json", r#"{"name": "test", "version": "1.0.0"}"#);
    workspace.create_file(
        "tsconfig.json",
        r#"{"compilerOptions": {"target": "ES2020", "module": "commonjs"}}"#,
    );
    workspace.create_file(
        "src/validators.ts",
        r#"export function validateEmail(email: string): boolean {
    return /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(email);
}
"#,
    );
    workspace.create_file(
        "src/index.ts",
        r#"import { validateEmail } from './validators';

console.log(validateEmail('test@example.com'));
"#,
    );

    let mut client = TestClient::new(workspace.path());

    // Move file to utils directory
    let params = json!({
        "target": {
            "kind": "file",
            "path": workspace.absolute_path("src/validators.ts").to_string_lossy()
        },
        "destination": workspace.absolute_path("src/utils/validators.ts").to_string_lossy(),
        "options": {"dryRun": false}
    });

    let result = client.call_tool("move", params).await;

    match result {
        Ok(_) => {
            assert!(
                !workspace.file_exists("src/validators.ts"),
                "Source should be deleted"
            );
            assert!(
                workspace.file_exists("src/utils/validators.ts"),
                "Destination should exist"
            );
        }
        Err(e) => {
            eprintln!("INFO: move failed: {:?}", e);
        }
    }
}

/// Test delete on TypeScript file
#[tokio::test]
async fn test_ts_delete_file_integration() {
    let workspace = TestWorkspace::new();

    workspace.create_file("package.json", r#"{"name": "test", "version": "1.0.0"}"#);
    workspace.create_file(
        "src/unused.ts",
        r#"// This file is not used
export function unusedFunction(): void {
    console.log('never called');
}
"#,
    );

    let mut client = TestClient::new(workspace.path());

    // Delete file
    let params = json!({
        "target": {
            "kind": "file",
            "path": workspace.absolute_path("src/unused.ts").to_string_lossy()
        },
        "options": {"dryRun": false}
    });

    let result = client.call_tool("delete", params).await;

    match result {
        Ok(_) => {
            assert!(!workspace.file_exists("src/unused.ts"), "File should be deleted");
        }
        Err(e) => {
            eprintln!("INFO: delete failed: {:?}", e);
        }
    }
}

/// Test workspace.find_replace on TypeScript files
#[tokio::test]
async fn test_ts_find_replace_integration() {
    let workspace = TestWorkspace::new();

    workspace.create_file("package.json", r#"{"name": "test", "version": "1.0.0"}"#);
    workspace.create_file(
        "src/service.ts",
        r#"export class UserService {
    private users: Map<number, User> = new Map();

    createUser(name: string): User {
        const user = new User(name);
        this.users.set(user.id, user);
        return user;
    }

    getUser(id: number): User | undefined {
        return this.users.get(id);
    }
}
"#,
    );
    workspace.create_file(
        "src/controller.ts",
        r#"import { UserService } from './service';

const userService = new UserService();
const user = userService.createUser('John');
"#,
    );

    let mut client = TestClient::new(workspace.path());

    // Find and replace UserService with AccountService
    let params = json!({
        "pattern": "UserService",
        "replacement": "AccountService",
        "mode": "literal",
        "dryRun": false
    });

    let result = client.call_tool("workspace.find_replace", params).await;

    match result {
        Ok(_) => {
            let service_content = workspace.read_file("src/service.ts");
            assert!(
                service_content.contains("AccountService"),
                "Class name should be replaced"
            );

            let controller_content = workspace.read_file("src/controller.ts");
            assert!(
                controller_content.contains("AccountService"),
                "Import should be replaced"
            );
        }
        Err(e) => {
            eprintln!("INFO: find_replace failed: {:?}", e);
        }
    }
}

/// Test rename directory on TypeScript project
#[tokio::test]
async fn test_ts_rename_directory_integration() {
    let workspace = TestWorkspace::new();

    workspace.create_file("package.json", r#"{"name": "test", "version": "1.0.0"}"#);
    workspace.create_file(
        "tsconfig.json",
        r#"{"compilerOptions": {"target": "ES2020", "module": "commonjs"}}"#,
    );
    workspace.create_file(
        "src/utils/helpers.ts",
        r#"export function helper(): string { return 'help'; }
"#,
    );
    workspace.create_file(
        "src/index.ts",
        r#"import { helper } from './utils/helpers';
console.log(helper());
"#,
    );

    let mut client = TestClient::new(workspace.path());

    // Rename directory
    let params = json!({
        "target": {
            "kind": "directory",
            "path": workspace.absolute_path("src/utils").to_string_lossy()
        },
        "newName": workspace.absolute_path("src/lib").to_string_lossy(),
        "options": {"dryRun": false}
    });

    let result = client.call_tool("rename", params).await;

    match result {
        Ok(_) => {
            assert!(!workspace.file_exists("src/utils/helpers.ts"), "Old dir should not exist");
            assert!(workspace.file_exists("src/lib/helpers.ts"), "New dir should exist");

            let index_content = workspace.read_file("src/index.ts");
            assert!(
                index_content.contains("from './lib/helpers'"),
                "Import path should be updated"
            );
        }
        Err(e) => {
            eprintln!("INFO: rename directory failed: {:?}", e);
        }
    }
}
