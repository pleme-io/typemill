//! SvelteKit/TypeScript Path Alias Integration Tests
//!
//! Tests that $lib and other path aliases are correctly handled during file moves
//! in TypeScript projects. Verifies:
//!
//! ## Alias Preservation
//! - Moving files within $lib preserves alias imports (test 1)
//! - Moving files out of $lib converts to relative imports (test 2)
//! - Moving files into $lib converts relative imports to alias (test 6)
//! - Directory moves within $lib update all imports correctly (test 3)
//!
//! ## Multiple Alias Patterns
//! - @/* alias pattern (Next.js style) works correctly (test 4)
//! - Custom $ aliases beyond $lib (e.g., $utils, $components) (test 8)
//! - Moving between different alias scopes updates to new alias (test 9)
//!
//! ## Safety Guarantees
//! - Dry run shows correct changes before applying (test 5)
//! - Imports not affected by move stay unchanged (test 7)
//!
//! Note: These tests use .ts files since the TypeScript plugin handles those extensions.
//! The path alias logic applies equally to any file type that uses TypeScript-style imports.

use crate::harness::{TestClient, TestWorkspace};
use serde_json::json;

/// Create a TypeScript project structure with tsconfig.json path aliases
fn setup_typescript_workspace(workspace: &TestWorkspace) {
    // Create tsconfig.json with $lib alias (SvelteKit pattern)
    workspace.create_file(
        "tsconfig.json",
        r#"{
  "compilerOptions": {
    "baseUrl": ".",
    "paths": {
      "$lib": ["src/lib"],
      "$lib/*": ["src/lib/*"]
    }
  }
}"#,
    );

    // Create package.json
    workspace.create_file(
        "package.json",
        r#"{
  "name": "test-typescript",
  "type": "module"
}"#,
    );

    // Create src/lib structure (the $lib directory)
    workspace.create_directory("src/lib");
    workspace.create_directory("src/lib/components");
    workspace.create_directory("src/lib/utils");
    workspace.create_directory("src/routes");
}

/// Test 1: Move file within $lib - imports should preserve $lib alias
#[tokio::test]
async fn test_move_within_lib_preserves_alias() {
    let workspace = TestWorkspace::new();
    setup_typescript_workspace(&workspace);

    // Create a utility file in $lib/utils
    workspace.create_file(
        "src/lib/utils/helpers.ts",
        r#"export function formatDate(date: Date): string {
    return date.toISOString();
}

export function capitalize(str: string): string {
    return str.charAt(0).toUpperCase() + str.slice(1);
}"#,
    );

    // Create a component that imports from $lib/utils
    workspace.create_file(
        "src/lib/components/DateDisplay.ts",
        r#"import { formatDate } from '$lib/utils/helpers';

export function displayDate(date: Date): string {
    return formatDate(date);
}"#,
    );

    // Create a route that imports from $lib
    workspace.create_file(
        "src/routes/page.ts",
        r#"import { capitalize } from '$lib/utils/helpers';
import { displayDate } from '$lib/components/DateDisplay';

export function render() {
    const name = capitalize('world');
    return `Hello ${name}`;
}"#,
    );

    let mut client = TestClient::new(workspace.path());

    // Move helpers.ts to a new location within $lib
    let old_path = workspace.path().join("src/lib/utils/helpers.ts");
    let new_path = workspace.path().join("src/lib/utils/string/helpers.ts");

    // Create target directory
    workspace.create_directory("src/lib/utils/string");

    let params = json!({
        "target": {
            "kind": "file",
            "filePath": old_path.to_string_lossy()
        },
        "destination": new_path.to_string_lossy(),
        "options": { "dryRun": false }
    });

    let result = client
        .call_tool("relocate", params)
        .await
        .expect("relocate should succeed");

    let content = result
        .get("result")
        .and_then(|r| r.get("content"))
        .expect("Should have result.content");

    assert_eq!(
        content.get("status").and_then(|v| v.as_str()),
        Some("success"),
        "Move should succeed"
    );

    // Verify file was moved
    assert!(!workspace.file_exists("src/lib/utils/helpers.ts"));
    assert!(workspace.file_exists("src/lib/utils/string/helpers.ts"));

    // Verify imports in component were updated (should still use $lib)
    let component_content = workspace.read_file("src/lib/components/DateDisplay.ts");
    assert!(
        component_content.contains("$lib/utils/string/helpers"),
        "Component should have updated $lib import path.\nActual content:\n{}",
        component_content
    );

    // Verify imports in route were updated
    let route_content = workspace.read_file("src/routes/page.ts");
    assert!(
        route_content.contains("$lib/utils/string/helpers"),
        "Route should have updated $lib import path.\nActual content:\n{}",
        route_content
    );

    println!("✅ Move within $lib preserves alias imports");
}

/// Test 2: Move file from $lib to outside - should convert to relative imports
#[tokio::test]
async fn test_move_out_of_lib_converts_to_relative() {
    let workspace = TestWorkspace::new();
    setup_typescript_workspace(&workspace);

    // Create a utility in $lib
    workspace.create_file(
        "src/lib/utils/api.ts",
        r#"export async function fetchData(url: string) {
    const response = await fetch(url);
    return response.json();
}"#,
    );

    // Create a route that imports from $lib
    workspace.create_file(
        "src/routes/api/server.ts",
        r#"import { fetchData } from '$lib/utils/api';

export async function GET() {
    const data = await fetchData('/api/data');
    return new Response(JSON.stringify(data));
}"#,
    );

    let mut client = TestClient::new(workspace.path());

    // Move api.ts outside of $lib to src/server/
    let old_path = workspace.path().join("src/lib/utils/api.ts");
    let new_path = workspace.path().join("src/server/api.ts");

    workspace.create_directory("src/server");

    let params = json!({
        "target": {
            "kind": "file",
            "filePath": old_path.to_string_lossy()
        },
        "destination": new_path.to_string_lossy(),
        "options": { "dryRun": false }
    });

    let result = client
        .call_tool("relocate", params)
        .await
        .expect("relocate should succeed");

    let content = result
        .get("result")
        .and_then(|r| r.get("content"))
        .expect("Should have result.content");

    assert_eq!(
        content.get("status").and_then(|v| v.as_str()),
        Some("success"),
        "Move should succeed"
    );

    // Verify file was moved
    assert!(!workspace.file_exists("src/lib/utils/api.ts"));
    assert!(workspace.file_exists("src/server/api.ts"));

    // Verify imports were converted to relative paths
    let server_content = workspace.read_file("src/routes/api/server.ts");

    // Should no longer have $lib import
    assert!(
        !server_content.contains("$lib/utils/api"),
        "Should NOT have old $lib import.\nActual content:\n{}",
        server_content
    );

    // Should have relative import (../../server/api)
    assert!(
        server_content.contains("../") || server_content.contains("../../server/api"),
        "Should have relative import path.\nActual content:\n{}",
        server_content
    );

    println!("✅ Move out of $lib converts to relative imports");
}

/// Test 3: Move directory within $lib
#[tokio::test]
async fn test_move_directory_within_lib() {
    let workspace = TestWorkspace::new();
    setup_typescript_workspace(&workspace);

    // Create multiple files in a directory
    workspace.create_file(
        "src/lib/components/Button.ts",
        r#"export function Button(label: string): string {
    return `<button>${label}</button>`;
}"#,
    );

    workspace.create_file(
        "src/lib/components/Input.ts",
        r#"export function Input(value: string): string {
    return `<input value="${value}" />`;
}"#,
    );

    // Create an index file that re-exports
    workspace.create_file(
        "src/lib/components/index.ts",
        r#"export { Button } from './Button';
export { Input } from './Input';"#,
    );

    // Create a route that imports from the components
    workspace.create_file(
        "src/routes/page.ts",
        r#"import { Button, Input } from '$lib/components';

export function render() {
    return Input('') + Button('Submit');
}"#,
    );

    let mut client = TestClient::new(workspace.path());

    // Move components directory to ui/components
    let old_path = workspace.path().join("src/lib/components");
    let new_path = workspace.path().join("src/lib/ui/components");

    workspace.create_directory("src/lib/ui");

    let params = json!({
        "target": {
            "kind": "directory",
            "filePath": old_path.to_string_lossy()
        },
        "newName": new_path.to_string_lossy(),
        "options": { "dryRun": false }
    });

    let result = client
        .call_tool("rename_all", params)
        .await
        .expect("rename_all should succeed");

    let content = result
        .get("result")
        .and_then(|r| r.get("content"))
        .expect("Should have result.content");

    assert_eq!(
        content.get("status").and_then(|v| v.as_str()),
        Some("success"),
        "Directory move should succeed"
    );

    // Verify directory was moved
    assert!(!workspace.file_exists("src/lib/components/Button.ts"));
    assert!(workspace.file_exists("src/lib/ui/components/Button.ts"));
    assert!(workspace.file_exists("src/lib/ui/components/Input.ts"));
    assert!(workspace.file_exists("src/lib/ui/components/index.ts"));

    // Verify route imports were updated
    let route_content = workspace.read_file("src/routes/page.ts");
    assert!(
        route_content.contains("$lib/ui/components"),
        "Route should have updated import path.\nActual content:\n{}",
        route_content
    );

    println!("✅ Directory move within $lib updates imports correctly");
}

/// Test 4: Verify @alias pattern (Next.js style)
#[tokio::test]
async fn test_at_alias_pattern() {
    let workspace = TestWorkspace::new();

    // Create tsconfig.json with @ alias (Next.js pattern)
    workspace.create_file(
        "tsconfig.json",
        r#"{
  "compilerOptions": {
    "baseUrl": ".",
    "paths": {
      "@/*": ["src/*"]
    }
  }
}"#,
    );

    workspace.create_file(
        "package.json",
        r#"{"name": "test-nextjs", "type": "module"}"#,
    );

    workspace.create_directory("src/components");
    workspace.create_directory("src/utils");
    workspace.create_directory("src/pages");

    // Create a utility
    workspace.create_file(
        "src/utils/format.ts",
        r#"export function formatCurrency(amount: number): string {
    return `$${amount.toFixed(2)}`;
}"#,
    );

    // Create a component that imports with @/
    workspace.create_file(
        "src/components/Price.tsx",
        r#"import { formatCurrency } from '@/utils/format';

export function Price(amount: number): string {
    return `<span>${formatCurrency(amount)}</span>`;
}"#,
    );

    // Create a page that imports with @/
    workspace.create_file(
        "src/pages/index.tsx",
        r#"import { Price } from '@/components/Price';
import { formatCurrency } from '@/utils/format';

export default function Home() {
    return Price(99.99);
}"#,
    );

    let mut client = TestClient::new(workspace.path());

    // Move format.ts to a subdirectory
    let old_path = workspace.path().join("src/utils/format.ts");
    let new_path = workspace.path().join("src/utils/currency/format.ts");

    workspace.create_directory("src/utils/currency");

    let params = json!({
        "target": {
            "kind": "file",
            "filePath": old_path.to_string_lossy()
        },
        "destination": new_path.to_string_lossy(),
        "options": { "dryRun": false }
    });

    let result = client
        .call_tool("relocate", params)
        .await
        .expect("relocate should succeed");

    let content = result
        .get("result")
        .and_then(|r| r.get("content"))
        .expect("Should have result.content");

    assert_eq!(
        content.get("status").and_then(|v| v.as_str()),
        Some("success"),
        "Move should succeed"
    );

    // Verify imports were updated with @ alias
    let component_content = workspace.read_file("src/components/Price.tsx");
    assert!(
        component_content.contains("@/utils/currency/format"),
        "Component should have updated @/ import.\nActual content:\n{}",
        component_content
    );

    let page_content = workspace.read_file("src/pages/index.tsx");
    assert!(
        page_content.contains("@/utils/currency/format"),
        "Page should have updated @/ import.\nActual content:\n{}",
        page_content
    );

    println!("✅ @/ alias pattern works correctly");
}

/// Test 5: Dry run shows correct changes for alias moves
#[tokio::test]
async fn test_dry_run_shows_alias_updates() {
    let workspace = TestWorkspace::new();
    setup_typescript_workspace(&workspace);

    workspace.create_file(
        "src/lib/stores/user.ts",
        r#"export const user = { name: 'test' };"#,
    );

    workspace.create_file(
        "src/routes/layout.ts",
        r#"import { user } from '$lib/stores/user';

export function getUser() {
    return user;
}"#,
    );

    let mut client = TestClient::new(workspace.path());

    let old_path = workspace.path().join("src/lib/stores/user.ts");
    let new_path = workspace.path().join("src/lib/stores/auth/user.ts");

    workspace.create_directory("src/lib/stores/auth");

    // Dry run first
    let params = json!({
        "target": {
            "kind": "file",
            "filePath": old_path.to_string_lossy()
        },
        "destination": new_path.to_string_lossy(),
        "options": { "dryRun": true }
    });

    let result = client
        .call_tool("relocate", params)
        .await
        .expect("dry run should succeed");

    let content = result
        .get("result")
        .and_then(|r| r.get("content"))
        .expect("Should have result.content");

    // Check that the plan shows file edits (structure: changes.edits.documentChanges)
    let has_changes = content
        .get("changes")
        .and_then(|c| c.get("edits"))
        .and_then(|e| e.get("documentChanges"))
        .and_then(|dc| dc.as_array())
        .map(|arr| !arr.is_empty())
        .unwrap_or(false);
    assert!(
        has_changes,
        "Dry run should show file edits in changes.edits.documentChanges.\nPlan content:\n{:?}",
        content
    );

    // Original file should still exist (dry run)
    assert!(
        workspace.file_exists("src/lib/stores/user.ts"),
        "File should still exist after dry run"
    );

    println!("✅ Dry run correctly shows alias update plan");
}

/// Test 6: Move file INTO $lib - should convert relative imports to alias
#[tokio::test]
async fn test_move_into_lib_converts_to_alias() {
    let workspace = TestWorkspace::new();
    setup_typescript_workspace(&workspace);

    // Create a utility file OUTSIDE of $lib
    workspace.create_directory("src/server");
    workspace.create_file(
        "src/server/database.ts",
        r#"export function connect(): void {
    console.log('Connected to database');
}

export function query(sql: string): any[] {
    return [];
}"#,
    );

    // Create a route that imports with a relative path (because it's outside $lib)
    workspace.create_file(
        "src/routes/api/data.ts",
        r#"import { query } from '../../server/database';

export async function GET() {
    const results = query('SELECT * FROM users');
    return new Response(JSON.stringify(results));
}"#,
    );

    let mut client = TestClient::new(workspace.path());

    // Move database.ts INTO $lib
    let old_path = workspace.path().join("src/server/database.ts");
    let new_path = workspace.path().join("src/lib/server/database.ts");

    workspace.create_directory("src/lib/server");

    let params = json!({
        "target": {
            "kind": "file",
            "filePath": old_path.to_string_lossy()
        },
        "destination": new_path.to_string_lossy(),
        "options": { "dryRun": false }
    });

    let result = client
        .call_tool("relocate", params)
        .await
        .expect("relocate should succeed");

    let content = result
        .get("result")
        .and_then(|r| r.get("content"))
        .expect("Should have result.content");

    assert_eq!(
        content.get("status").and_then(|v| v.as_str()),
        Some("success"),
        "Move should succeed"
    );

    // Verify file was moved
    assert!(!workspace.file_exists("src/server/database.ts"));
    assert!(workspace.file_exists("src/lib/server/database.ts"));

    // Verify imports were converted to $lib alias
    let route_content = workspace.read_file("src/routes/api/data.ts");
    assert!(
        route_content.contains("$lib/server/database"),
        "Route should have $lib alias import after moving file into $lib.\nActual content:\n{}",
        route_content
    );
    assert!(
        !route_content.contains("../../server/database"),
        "Should NOT have old relative import.\nActual content:\n{}",
        route_content
    );

    println!("✅ Move into $lib converts relative imports to alias");
}

/// Test 7: Imports not affected by move should stay unchanged
#[tokio::test]
async fn test_unaffected_imports_stay_unchanged() {
    let workspace = TestWorkspace::new();
    setup_typescript_workspace(&workspace);

    // Create two separate utility files in $lib
    workspace.create_file(
        "src/lib/utils/helpers.ts",
        r#"export function formatDate(date: Date): string {
    return date.toISOString();
}"#,
    );

    workspace.create_file(
        "src/lib/utils/validators.ts",
        r#"export function isEmail(str: string): boolean {
    return str.includes('@');
}"#,
    );

    // Create a component that imports BOTH utilities
    workspace.create_file(
        "src/lib/components/Form.ts",
        r#"import { formatDate } from '$lib/utils/helpers';
import { isEmail } from '$lib/utils/validators';

export function validateForm(email: string, date: Date): boolean {
    console.log(formatDate(date));
    return isEmail(email);
}"#,
    );

    let mut client = TestClient::new(workspace.path());

    // Move ONLY helpers.ts - validators.ts import should NOT change
    let old_path = workspace.path().join("src/lib/utils/helpers.ts");
    let new_path = workspace.path().join("src/lib/utils/date/helpers.ts");

    workspace.create_directory("src/lib/utils/date");

    let params = json!({
        "target": {
            "kind": "file",
            "filePath": old_path.to_string_lossy()
        },
        "destination": new_path.to_string_lossy(),
        "options": { "dryRun": false }
    });

    let result = client
        .call_tool("relocate", params)
        .await
        .expect("relocate should succeed");

    let content = result
        .get("result")
        .and_then(|r| r.get("content"))
        .expect("Should have result.content");

    assert_eq!(
        content.get("status").and_then(|v| v.as_str()),
        Some("success"),
        "Move should succeed"
    );

    // Verify the component file
    let form_content = workspace.read_file("src/lib/components/Form.ts");

    // The helpers import SHOULD be updated
    assert!(
        form_content.contains("$lib/utils/date/helpers"),
        "helpers import should be updated.\nActual content:\n{}",
        form_content
    );

    // The validators import should NOT be changed (still exactly as before)
    assert!(
        form_content.contains("from '$lib/utils/validators'"),
        "validators import should remain UNCHANGED.\nActual content:\n{}",
        form_content
    );

    println!("✅ Unaffected imports stay unchanged");
}

/// Test 8: Custom $ aliases beyond $lib (e.g., $utils, $components)
#[tokio::test]
async fn test_custom_dollar_aliases() {
    let workspace = TestWorkspace::new();

    // Create tsconfig.json with multiple custom $ aliases
    workspace.create_file(
        "tsconfig.json",
        r#"{
  "compilerOptions": {
    "baseUrl": ".",
    "paths": {
      "$lib": ["src/lib"],
      "$lib/*": ["src/lib/*"],
      "$utils": ["src/utils"],
      "$utils/*": ["src/utils/*"],
      "$components": ["src/components"],
      "$components/*": ["src/components/*"]
    }
  }
}"#,
    );

    workspace.create_file(
        "package.json",
        r#"{"name": "test-custom-aliases", "type": "module"}"#,
    );

    // Create directory structure matching aliases
    workspace.create_directory("src/lib");
    workspace.create_directory("src/utils");
    workspace.create_directory("src/components");
    workspace.create_directory("src/routes");

    // Create files using different aliases
    workspace.create_file(
        "src/utils/format.ts",
        r#"export function formatNumber(n: number): string {
    return n.toFixed(2);
}"#,
    );

    workspace.create_file(
        "src/components/Display.ts",
        r#"import { formatNumber } from '$utils/format';

export function Display(value: number): string {
    return `<span>${formatNumber(value)}</span>`;
}"#,
    );

    workspace.create_file(
        "src/routes/page.ts",
        r#"import { Display } from '$components/Display';
import { formatNumber } from '$utils/format';

export function render() {
    return Display(42);
}"#,
    );

    let mut client = TestClient::new(workspace.path());

    // Move format.ts to a subdirectory within $utils
    let old_path = workspace.path().join("src/utils/format.ts");
    let new_path = workspace.path().join("src/utils/number/format.ts");

    workspace.create_directory("src/utils/number");

    let params = json!({
        "target": {
            "kind": "file",
            "filePath": old_path.to_string_lossy()
        },
        "destination": new_path.to_string_lossy(),
        "options": { "dryRun": false }
    });

    let result = client
        .call_tool("relocate", params)
        .await
        .expect("relocate should succeed");

    let content = result
        .get("result")
        .and_then(|r| r.get("content"))
        .expect("Should have result.content");

    assert_eq!(
        content.get("status").and_then(|v| v.as_str()),
        Some("success"),
        "Move should succeed"
    );

    // Verify $utils imports were updated
    let display_content = workspace.read_file("src/components/Display.ts");
    assert!(
        display_content.contains("$utils/number/format"),
        "Display should have updated $utils import.\nActual content:\n{}",
        display_content
    );

    let page_content = workspace.read_file("src/routes/page.ts");
    assert!(
        page_content.contains("$utils/number/format"),
        "Page should have updated $utils import.\nActual content:\n{}",
        page_content
    );

    // $components import should be unchanged
    assert!(
        page_content.contains("from '$components/Display'"),
        "$components import should be unchanged.\nActual content:\n{}",
        page_content
    );

    println!("✅ Custom $ aliases work correctly");
}

/// Test 9: Moving file between different alias scopes
#[tokio::test]
async fn test_move_between_alias_scopes() {
    let workspace = TestWorkspace::new();

    // Create tsconfig.json with multiple alias scopes
    workspace.create_file(
        "tsconfig.json",
        r#"{
  "compilerOptions": {
    "baseUrl": ".",
    "paths": {
      "$lib/*": ["src/lib/*"],
      "$server/*": ["src/server/*"]
    }
  }
}"#,
    );

    workspace.create_file(
        "package.json",
        r#"{"name": "test-multi-scope", "type": "module"}"#,
    );

    workspace.create_directory("src/lib/utils");
    workspace.create_directory("src/server");
    workspace.create_directory("src/routes");

    // Create a utility in $lib
    workspace.create_file(
        "src/lib/utils/auth.ts",
        r#"export function validateToken(token: string): boolean {
    return token.length > 0;
}"#,
    );

    // Create files that import using $lib alias
    workspace.create_file(
        "src/routes/login.ts",
        r#"import { validateToken } from '$lib/utils/auth';

export function login(token: string) {
    return validateToken(token);
}"#,
    );

    let mut client = TestClient::new(workspace.path());

    // Move auth.ts from $lib scope to $server scope
    let old_path = workspace.path().join("src/lib/utils/auth.ts");
    let new_path = workspace.path().join("src/server/auth.ts");

    let params = json!({
        "target": {
            "kind": "file",
            "filePath": old_path.to_string_lossy()
        },
        "destination": new_path.to_string_lossy(),
        "options": { "dryRun": false }
    });

    let result = client
        .call_tool("relocate", params)
        .await
        .expect("relocate should succeed");

    let content = result
        .get("result")
        .and_then(|r| r.get("content"))
        .expect("Should have result.content");

    assert_eq!(
        content.get("status").and_then(|v| v.as_str()),
        Some("success"),
        "Move should succeed"
    );

    // Verify file was moved
    assert!(!workspace.file_exists("src/lib/utils/auth.ts"));
    assert!(workspace.file_exists("src/server/auth.ts"));

    // Verify imports were updated to use $server alias (new scope)
    let login_content = workspace.read_file("src/routes/login.ts");
    assert!(
        login_content.contains("$server/auth"),
        "Import should be updated to $server alias.\nActual content:\n{}",
        login_content
    );
    assert!(
        !login_content.contains("$lib/utils/auth"),
        "Should NOT have old $lib import.\nActual content:\n{}",
        login_content
    );

    println!("✅ Move between alias scopes updates to new alias");
}

/// Test 10: Exact SvelteKit structure with jsconfig.json extending .svelte-kit/tsconfig.json
/// This mimics the exact structure of a real SvelteKit project
#[tokio::test]
async fn test_sveltekit_exact_structure() {
    let workspace = TestWorkspace::new();

    // Create .svelte-kit directory with tsconfig.json (generated by SvelteKit build)
    workspace.create_directory(".svelte-kit");
    workspace.create_file(
        ".svelte-kit/tsconfig.json",
        r#"{
  "compilerOptions": {
    "paths": {
      "$lib": ["../src/lib"],
      "$lib/*": ["../src/lib/*"]
    }
  }
}"#,
    );

    // Create jsconfig.json that extends .svelte-kit/tsconfig.json (typical SvelteKit setup)
    workspace.create_file(
        "jsconfig.json",
        r#"{ "extends": "./.svelte-kit/tsconfig.json" }"#,
    );

    workspace.create_file(
        "package.json",
        r#"{"name": "test-sveltekit-exact", "type": "module"}"#,
    );

    // Create src/lib/api.js (the file we'll move)
    workspace.create_directory("src/lib");
    workspace.create_file(
        "src/lib/api.js",
        r#"export function get(path) { return fetch(path); }
export function post(path, data) { return fetch(path, { method: 'POST', body: JSON.stringify(data) }); }"#,
    );

    // Create src/routes/+page.server.js that imports from $lib/api
    workspace.create_directory("src/routes");
    workspace.create_file(
        "src/routes/+page.server.js",
        r#"import * as api from '$lib/api';

export async function load() {
    const data = await api.get('/data');
    return { data };
}"#,
    );

    // Also create a file that imports with .js extension
    workspace.create_file(
        "src/routes/other.server.js",
        r#"import * as api from '$lib/api.js';

export async function load() {
    return api.post('/submit', {});
}"#,
    );

    let mut client = TestClient::new(workspace.path());

    // Move api.js to a subdirectory within $lib
    let old_path = workspace.path().join("src/lib/api.js");
    let new_path = workspace.path().join("src/lib/services/api.js");

    workspace.create_directory("src/lib/services");

    let params = json!({
        "target": {
            "kind": "file",
            "filePath": old_path.to_string_lossy()
        },
        "destination": new_path.to_string_lossy(),
        "options": { "dryRun": false }
    });

    let result = client
        .call_tool("relocate", params)
        .await
        .expect("relocate should succeed");

    let content = result
        .get("result")
        .and_then(|r| r.get("content"))
        .expect("Should have result.content");

    assert_eq!(
        content.get("status").and_then(|v| v.as_str()),
        Some("success"),
        "Move should succeed"
    );

    // Verify file was moved
    assert!(!workspace.file_exists("src/lib/api.js"));
    assert!(workspace.file_exists("src/lib/services/api.js"));

    // Verify imports were updated in BOTH files
    let page_content = workspace.read_file("src/routes/+page.server.js");
    assert!(
        page_content.contains("$lib/services/api"),
        "+page.server.js should have updated $lib import.\nActual content:\n{}",
        page_content
    );

    let other_content = workspace.read_file("src/routes/other.server.js");
    assert!(
        other_content.contains("$lib/services/api"),
        "other.server.js should have updated $lib import (with .js extension).\nActual content:\n{}",
        other_content
    );

    println!("✅ SvelteKit exact structure (jsconfig extending .svelte-kit/tsconfig.json) works correctly");
}
