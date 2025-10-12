use crate::harness::{TestClient, TestWorkspace};
use serde_json::json;
use std::path::PathBuf;

use super::ProjectFixtures;

impl ProjectFixtures {
    /// Create an error-prone project for testing error handling
    pub async fn create_error_project(
        workspace: &TestWorkspace,
        client: &mut TestClient,
    ) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
        let mut created_files = Vec::new();

        // Create files with various types of errors

        // File with syntax errors
        let syntax_error_file = workspace.path().join("syntax_errors.ts");
        client
            .call_tool(
                "create_file",
                json!({
                    "file_path": syntax_error_file.to_string_lossy(),
                    "content": r#"
// This file contains intentional syntax errors for testing

interface User {
    id: number;
    name: string;
    // Missing closing brace

function brokenFunction() {
    console.log("missing closing brace"
    // Missing closing parenthesis and brace

const unclosedArray = [1, 2, 3;
// Missing closing bracket

class BrokenClass {
    constructor(public id: number {
        // Missing closing parenthesis
    }
// Missing closing brace for class

export { User, brokenFunction, BrokenClass;
// Should work despite errors above
"#
                }),
            )
            .await?;
        created_files.push(syntax_error_file);

        // File with type errors
        let type_error_file = workspace.path().join("type_errors.ts");
        client
            .call_tool(
                "create_file",
                json!({
                    "file_path": type_error_file.to_string_lossy(),
                    "content": r#"
// This file contains intentional type errors

interface User {
    id: number;
    name: string;
}

function processUser(user: User): string {
    // Type error: accessing non-existent property
    return user.nonExistentProperty;
}

function addNumbers(a: number, b: number): number {
    // Type error: returning string instead of number
    return "not a number";
}

const user: User = {
    id: "should be number", // Type error
    name: 123, // Type error
    extraProperty: "not allowed" // Type error
};

// Type error: passing wrong types
const result = addNumbers("not", "numbers");

// Valid code mixed with errors
export function validFunction(x: number): number {
    return x * 2;
}

export const validConstant = "this works";
"#
                }),
            )
            .await?;
        created_files.push(type_error_file);

        // File with import errors
        let import_error_file = workspace.path().join("import_errors.ts");
        client
            .call_tool(
                "create_file",
                json!({
                    "file_path": import_error_file.to_string_lossy(),
                    "content": r#"
// This file contains intentional import errors

import { NonExistentType } from './does-not-exist';
import { AnotherMissing } from './also-missing';
import { } from './empty-import';
import * as Missing from './missing-module';

// Circular import (if this file is imported elsewhere)
import { importErrorFile } from './import_errors';

// Using undefined imports
function useUndefinedTypes(param: NonExistentType): AnotherMissing {
    return Missing.someFunction(param);
}

// Valid imports that might work
import { validFunction } from './type_errors';

export function workingFunction(): number {
    return validFunction(42);
}
"#
                }),
            )
            .await?;
        created_files.push(import_error_file);

        Ok(created_files)
    }
}
