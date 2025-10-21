//! Data-driven test fixtures for MCP file operation handlers
//!
//! This module contains test data for all MCP file operation tools.
//! Each fixture struct represents a single test case with all necessary
//! setup data, operations, and expected outcomes.

/// Test fixture for create_file operations
#[derive(Debug, Clone)]
pub struct CreateFileTestCase {
    pub test_name: &'static str,
    pub file_to_create: &'static str,
    pub content: &'static str,
    pub initial_files: &'static [(&'static str, &'static str)], // (path, content)
    pub overwrite: bool,
    pub expect_success: bool,
}

/// Test fixture for read_file operations
#[derive(Debug, Clone)]
pub struct ReadFileTestCase {
    pub test_name: &'static str,
    pub file_to_read: &'static str,
    pub initial_files: &'static [(&'static str, &'static str)],
    pub expected_content: Option<&'static str>,
    pub start_line: Option<usize>,
    pub end_line: Option<usize>,
    pub expect_success: bool,
}

/// Test fixture for write_file operations
#[derive(Debug, Clone)]
pub struct WriteFileTestCase {
    pub test_name: &'static str,
    pub file_to_write: &'static str,
    pub content: &'static str,
    pub initial_files: &'static [(&'static str, &'static str)],
    pub expect_success: bool,
}

/// Test fixture for delete_file operations
#[derive(Debug, Clone)]
pub struct DeleteFileTestCase {
    pub test_name: &'static str,
    pub file_to_delete: &'static str,
    pub initial_files: &'static [(&'static str, &'static str)],
    pub expect_success: bool,
}

/// Test fixture for list_files operations
#[derive(Debug, Clone)]
pub struct ListFilesTestCase {
    pub test_name: &'static str,
    pub directory: &'static str, // Empty string means workspace root
    pub recursive: bool,
    pub pattern: Option<&'static str>,
    pub initial_files: &'static [&'static str],
    pub initial_dirs: &'static [&'static str],
    pub expected_contains: &'static [&'static str],
    pub expected_min_count: usize,
}

// =============================================================================
// CREATE FILE TEST CASES
// =============================================================================

pub const CREATE_FILE_TESTS: &[CreateFileTestCase] = &[
    CreateFileTestCase {
        test_name: "basic",
        file_to_create: "new_file.txt",
        content: "Hello, World!",
        initial_files: &[],
        overwrite: false,
        expect_success: true,
    },
    CreateFileTestCase {
        test_name: "with_directories",
        file_to_create: "nested/deep/new_file.js",
        content: "export const greeting = 'Hello from nested file!';",
        initial_files: &[],
        overwrite: false,
        expect_success: true,
    },
    CreateFileTestCase {
        test_name: "overwrite_protection",
        file_to_create: "existing.txt",
        content: "new content",
        initial_files: &[("existing.txt", "original content")],
        overwrite: false,
        expect_success: false,
    },
    CreateFileTestCase {
        test_name: "with_overwrite",
        file_to_create: "existing.txt",
        content: "new content",
        initial_files: &[("existing.txt", "original content")],
        overwrite: true,
        expect_success: true,
    },
];

// =============================================================================
// READ FILE TEST CASES
// =============================================================================

pub const READ_FILE_TESTS: &[ReadFileTestCase] = &[
    ReadFileTestCase {
        test_name: "basic",
        file_to_read: "test_file.txt",
        initial_files: &[(
            "test_file.txt",
            "This is test content\nwith multiple lines\nand unicode: 🚀",
        )],
        expected_content: Some("This is test content\nwith multiple lines\nand unicode: 🚀"),
        start_line: None,
        end_line: None,
        expect_success: true,
    },
    ReadFileTestCase {
        test_name: "nonexistent",
        file_to_read: "nonexistent.txt",
        initial_files: &[],
        expected_content: None,
        start_line: None,
        end_line: None,
        expect_success: false,
    },
];

// =============================================================================
// WRITE FILE TEST CASES
// =============================================================================

pub const WRITE_FILE_TESTS: &[WriteFileTestCase] = &[
    WriteFileTestCase {
        test_name: "basic",
        file_to_write: "write_test.txt",
        content: "Written content with special chars: @#$%^&*()",
        initial_files: &[],
        expect_success: true,
    },
    WriteFileTestCase {
        test_name: "overwrites_existing",
        file_to_write: "overwrite_test.txt",
        content: "completely new content",
        initial_files: &[("overwrite_test.txt", "original")],
        expect_success: true,
    },
];

// =============================================================================
// DELETE FILE TEST CASES
// =============================================================================

pub const DELETE_FILE_TESTS: &[DeleteFileTestCase] = &[
    DeleteFileTestCase {
        test_name: "basic",
        file_to_delete: "to_delete.txt",
        initial_files: &[("to_delete.txt", "content to be deleted")],
        expect_success: true,
    },
    DeleteFileTestCase {
        test_name: "nonexistent",
        file_to_delete: "nonexistent.txt",
        initial_files: &[],
        expect_success: false,
    },
];

// =============================================================================
// LIST FILES TEST CASES
// =============================================================================

pub const LIST_FILES_TESTS: &[ListFilesTestCase] = &[
    ListFilesTestCase {
        test_name: "basic",
        directory: "",
        recursive: false,
        pattern: None,
        initial_files: &["file1.txt", "file2.js", "file3.py", "subdir/nested.txt"],
        initial_dirs: &["subdir"],
        expected_contains: &["file1.txt", "file2.js", "file3.py", "subdir"],
        expected_min_count: 4,
    },
    ListFilesTestCase {
        test_name: "with_pattern",
        directory: "",
        recursive: false,
        pattern: Some("*.js"),
        initial_files: &["test.js", "test.ts", "test.py", "test.txt", "README.md"],
        initial_dirs: &[],
        expected_contains: &["test.js"],
        expected_min_count: 1,
    },
];

// =============================================================================
// ANALYZE IMPORTS TEST CASES
// =============================================================================

/// Test fixture for analyze_imports operations
#[derive(Debug, Clone)]
pub struct AnalyzeImportsTestCase {
    pub test_name: &'static str,
    pub file_path: &'static str,
    pub initial_files: &'static [(&'static str, &'static str)],
    pub expected_import_count: usize,
    pub expect_success: bool,
}

pub const ANALYZE_IMPORTS_TESTS: &[AnalyzeImportsTestCase] = &[
    AnalyzeImportsTestCase {
        test_name: "simple_imports",
        file_path: "main.ts",
        initial_files: &[(
            "main.ts",
            r#"import { foo, bar } from './utils';
import type { MyType } from './types';
import React from 'react';

console.log(foo, bar);
"#,
        )],
        expected_import_count: 3,
        expect_success: true,
    },
    AnalyzeImportsTestCase {
        test_name: "no_imports",
        file_path: "standalone.ts",
        initial_files: &[(
            "standalone.ts",
            r#"const value = 42;
console.log(value);
"#,
        )],
        expected_import_count: 0,
        expect_success: true,
    },
    AnalyzeImportsTestCase {
        test_name: "nonexistent_file",
        file_path: "does_not_exist.ts",
        initial_files: &[],
        expected_import_count: 0,
        expect_success: false,
    },
];

// =============================================================================
// FIND DEAD CODE TEST CASES
// =============================================================================

/// Test fixture for find_dead_code operations
#[derive(Debug, Clone)]
pub struct FindDeadCodeTestCase {
    pub test_name: &'static str,
    pub initial_files: &'static [(&'static str, &'static str)],
    pub workspace_path: &'static str, // Relative to workspace root
    pub expected_dead_symbols: &'static [&'static str], // Names of dead symbols expected
    pub expect_success: bool,
}

pub const FIND_DEAD_CODE_TESTS: &[FindDeadCodeTestCase] = &[
    FindDeadCodeTestCase {
        test_name: "detect_unused_exports",
        initial_files: &[
            (
                "src/utils.ts",
                r#"export function usedFunction(x: number): number {
    return x * 2;
}

export function unusedFunction(x: number): number {
    return x * 3;
}

export class UnusedClass {
    value: number = 0;
}
"#,
            ),
            (
                "src/main.ts",
                r#"import { usedFunction } from './utils';

export function main() {
    const result = usedFunction(5);
    console.log(result);
}
"#,
            ),
        ],
        workspace_path: "",
        expected_dead_symbols: &["unusedFunction", "UnusedClass"],
        expect_success: true,
    },
    FindDeadCodeTestCase {
        test_name: "no_dead_code",
        initial_files: &[
            (
                "src/module.ts",
                r#"export function activeFunction(): void {
    console.log("active");
}
"#,
            ),
            (
                "src/app.ts",
                r#"import { activeFunction } from './module';

activeFunction();
"#,
            ),
        ],
        workspace_path: "",
        expected_dead_symbols: &[],
        expect_success: true,
    },
    FindDeadCodeTestCase {
        test_name: "empty_workspace",
        initial_files: &[],
        workspace_path: "",
        expected_dead_symbols: &[],
        expect_success: true,
    },
];

// =============================================================================
// RENAME DIRECTORY TEST CASES
// =============================================================================

/// Test fixture for rename_directory operations
#[derive(Debug, Clone)]
pub struct RenameDirectoryTestCase {
    pub test_name: &'static str,
    pub initial_files: &'static [(&'static str, &'static str)],
    pub dir_to_rename: &'static str,
    pub new_dir_name: &'static str,
    pub update_imports: bool,
    pub expect_success: bool,
}

pub const RENAME_DIRECTORY_TESTS: &[RenameDirectoryTestCase] = &[
    RenameDirectoryTestCase {
        test_name: "simple_rename",
        initial_files: &[
            ("olddir/file1.ts", "export const value = 1;"),
            ("olddir/file2.ts", "export const value = 2;"),
        ],
        dir_to_rename: "olddir",
        new_dir_name: "newdir",
        update_imports: false,
        expect_success: true,
    },
    RenameDirectoryTestCase {
        test_name: "rename_with_import_updates",
        initial_files: &[
            (
                "components/Button.tsx",
                "export const Button = () => <button />;",
            ),
            (
                "app.tsx",
                r#"import { Button } from './components/Button';

export default function App() {
    return <Button />;
}
"#,
            ),
        ],
        dir_to_rename: "components",
        new_dir_name: "ui",
        update_imports: true,
        expect_success: true,
    },
    RenameDirectoryTestCase {
        test_name: "nonexistent_directory",
        initial_files: &[("src/file.ts", "export const value = 1;")],
        dir_to_rename: "nonexistent",
        new_dir_name: "newdir",
        update_imports: false,
        expect_success: false,
    },
];

// =============================================================================
// RENAME FILE TEST CASES
// =============================================================================

/// Test fixture for rename_file operations
#[derive(Debug, Clone)]
pub struct RenameFileTestCase {
    pub test_name: &'static str,
    pub initial_files: &'static [(&'static str, &'static str)],
    pub old_file_path: &'static str,
    pub new_file_path: &'static str,
    pub expect_success: bool,
    pub expected_import_updates: &'static [(&'static str, &'static str)], // (file_path, expected_content_substring)
}

// =============================================================================
// MOVE FILE TEST CASES
// =============================================================================

/// Test fixture for move_file operations
#[derive(Debug, Clone)]
pub struct MoveFileTestCase {
    pub test_name: &'static str,
    pub initial_files: &'static [(&'static str, &'static str)],
    pub old_file_path: &'static str,
    pub new_file_path: &'static str,
    pub expect_success: bool,
    pub expected_import_updates: &'static [(&'static str, &'static str)], // (file_path, expected_content_substring)
}

pub const MOVE_FILE_TESTS: &[MoveFileTestCase] = &[
    MoveFileTestCase {
        test_name: "basic_move_with_import_updates",
        initial_files: &[
            (
                "src/utils.ts",
                r#"export const myUtil = () => {
    return "utility function";
};

export function helperFunc(data: string): string {
    return data.toUpperCase();
}
"#,
            ),
            (
                "src/main.ts",
                r#"import { myUtil, helperFunc } from './utils';

export function main() {
    const result = myUtil();
    const processed = helperFunc(result);
    console.log(processed);
}
"#,
            ),
        ],
        old_file_path: "src/utils.ts",
        new_file_path: "src/new_dir/utils.ts",
        expect_success: true,
        expected_import_updates: &[("src/main.ts", "from './new_dir/utils'")],
    },
    MoveFileTestCase {
        test_name: "move_to_parent_directory",
        initial_files: &[
            ("src/components/Button.ts", "export class Button {}"),
            (
                "src/components/index.ts",
                "import { Button } from './Button';",
            ),
        ],
        old_file_path: "src/components/Button.ts",
        new_file_path: "src/Button.ts",
        expect_success: true,
        expected_import_updates: &[("src/components/index.ts", "from '../Button'")],
    },
    MoveFileTestCase {
        test_name: "move_between_sibling_directories",
        initial_files: &[
            ("src/components/Button.ts", "export class Button {}"),
            (
                "src/utils/helpers.ts",
                "import { Button } from '../components/Button';",
            ),
        ],
        old_file_path: "src/components/Button.ts",
        new_file_path: "src/ui/Button.ts",
        expect_success: true,
        expected_import_updates: &[("src/utils/helpers.ts", "from '../ui/Button'")],
    },
    MoveFileTestCase {
        test_name: "move_to_deeper_nesting_level",
        initial_files: &[
            ("src/Button.ts", "export class Button {}"),
            ("src/index.ts", "import { Button } from './Button';"),
        ],
        old_file_path: "src/Button.ts",
        new_file_path: "src/components/core/Button.ts",
        expect_success: true,
        expected_import_updates: &[("src/index.ts", "from './components/core/Button'")],
    },
];

pub const RENAME_FILE_TESTS: &[RenameFileTestCase] = &[
    RenameFileTestCase {
        test_name: "basic_rename_with_import_updates",
        initial_files: &[
            (
                "src/utils.ts",
                r#"export const myUtil = () => {
    return "utility function";
};

export function helperFunc(data: string): string {
    return data.toUpperCase();
}
"#,
            ),
            (
                "src/main.ts",
                r#"import { myUtil, helperFunc } from './utils';

export function main() {
    const result = myUtil();
    const processed = helperFunc(result);
    console.log(processed);
}
"#,
            ),
        ],
        old_file_path: "src/utils.ts",
        new_file_path: "src/renamed_utils.ts",
        expect_success: true,
        expected_import_updates: &[("src/main.ts", "from './renamed_utils'")],
    },
    RenameFileTestCase {
        test_name: "nested_import_path_resolution",
        initial_files: &[
            (
                "src/core/types.ts",
                r#"export interface User {
    id: number;
    name: string;
}

export type Status = 'active' | 'inactive';
"#,
            ),
            (
                "src/core/models/UserModel.ts",
                r#"import { User, Status } from '../types';

export class UserModel implements User {
    constructor(
        public id: number,
        public name: string,
        public status: Status = 'active'
    ) {}
}
"#,
            ),
            (
                "src/features/users/UserService.ts",
                r#"import { UserModel } from '../../core/models/UserModel';
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
"#,
            ),
        ],
        old_file_path: "src/core/types.ts",
        new_file_path: "src/shared/types.ts",
        expect_success: true,
        expected_import_updates: &[
            ("src/core/models/UserModel.ts", "from '../../shared/types'"),
            (
                "src/features/users/UserService.ts",
                "from '../../shared/types'",
            ),
        ],
    },
    RenameFileTestCase {
        test_name: "rename_to_subdirectory",
        initial_files: &[
            (
                "config.ts",
                "export const API_URL = 'https://api.example.com';",
            ),
            (
                "app.ts",
                r#"import { API_URL } from './config';

console.log(API_URL);
"#,
            ),
        ],
        old_file_path: "config.ts",
        new_file_path: "settings/config.ts",
        expect_success: true,
        expected_import_updates: &[("app.ts", "from './settings/config'")],
    },
];

// =============================================================================
// MARKDOWN RENAME FILE TEST CASES
// =============================================================================

pub const MARKDOWN_RENAME_FILE_TESTS: &[RenameFileTestCase] = &[
    RenameFileTestCase {
        test_name: "markdown_basic_inline_link_update",
        initial_files: &[
            (
                "docs/guide.md",
                r#"# User Guide

This is the documentation guide.
"#,
            ),
            (
                "README.md",
                r#"# Project

See [User Guide](docs/guide.md) for details.
Also check [the guide](docs/guide.md#installation).
"#,
            ),
        ],
        old_file_path: "docs/guide.md",
        new_file_path: "docs/user-guide.md",
        expect_success: true,
        expected_import_updates: &[
            ("README.md", "[User Guide](docs/user-guide.md)"),
            ("README.md", "[the guide](docs/user-guide.md#installation)"),
        ],
    },
    RenameFileTestCase {
        test_name: "markdown_multiple_files_referencing_same_file",
        initial_files: &[
            (
                "docs/api.md",
                r#"# API Reference
"#,
            ),
            (
                "README.md",
                r#"See [API docs](docs/api.md).
"#,
            ),
            (
                "CONTRIBUTING.md",
                r#"Read the [API Reference](docs/api.md) first.
"#,
            ),
            (
                "docs/examples.md",
                r#"Check [API](docs/api.md) for details.
"#,
            ),
        ],
        old_file_path: "docs/api.md",
        new_file_path: "docs/api-reference.md",
        expect_success: true,
        expected_import_updates: &[
            ("README.md", "(docs/api-reference.md)"), // From root, use project-relative
            ("CONTRIBUTING.md", "(docs/api-reference.md)"), // From root, use project-relative
            ("docs/examples.md", "(api-reference.md)"), // Same directory, use file-relative
        ],
    },
    RenameFileTestCase {
        test_name: "markdown_links_with_anchors_preserved",
        initial_files: &[
            (
                "docs/architecture.md",
                r#"# Architecture

## Overview
## Components
"#,
            ),
            (
                "README.md",
                r#"See [Architecture Overview](docs/architecture.md#overview).
Also [Components](docs/architecture.md#components).
"#,
            ),
        ],
        old_file_path: "docs/architecture.md",
        new_file_path: "docs/system-architecture.md",
        expect_success: true,
        expected_import_updates: &[
            ("README.md", "(docs/system-architecture.md#overview)"),
            ("README.md", "(docs/system-architecture.md#components)"),
        ],
    },
    RenameFileTestCase {
        test_name: "markdown_reference_style_links",
        initial_files: &[
            (
                "docs/changelog.md",
                r#"# Changelog
"#,
            ),
            (
                "README.md",
                r#"# Project

See [the changelog][changes] for version history.

[changes]: docs/changelog.md
"#,
            ),
        ],
        old_file_path: "docs/changelog.md",
        new_file_path: "CHANGELOG.md",
        expect_success: true,
        expected_import_updates: &[("README.md", "[changes]: CHANGELOG.md")],
    },
    RenameFileTestCase {
        test_name: "markdown_move_to_different_directory",
        initial_files: &[
            (
                "docs/setup.md",
                r#"# Setup Guide
"#,
            ),
            (
                "README.md",
                r#"See [Setup](docs/setup.md) for installation.
"#,
            ),
        ],
        old_file_path: "docs/setup.md",
        new_file_path: "SETUP.md",
        expect_success: true,
        expected_import_updates: &[("README.md", "[Setup](SETUP.md)")],
    },
    RenameFileTestCase {
        test_name: "markdown_nested_directory_paths",
        initial_files: &[
            (
                "docs/development/contributing.md",
                r#"# Contributing Guide
"#,
            ),
            (
                "README.md",
                r#"See [Contributing](docs/development/contributing.md).
"#,
            ),
            (
                "docs/index.md",
                r#"Check [contributing guide](docs/development/contributing.md).
"#,
            ),
        ],
        old_file_path: "docs/development/contributing.md",
        new_file_path: "CONTRIBUTING.md",
        expect_success: true,
        expected_import_updates: &[
            ("README.md", "(CONTRIBUTING.md)"),
            ("docs/index.md", "(../CONTRIBUTING.md)"),
        ],
    },
];

// =============================================================================
// RUST MOVE FILE TEST CASES
// =============================================================================

pub const RUST_MOVE_FILE_TESTS: &[MoveFileTestCase] = &[
    MoveFileTestCase {
        test_name: "rust_move_cross_crate",
        initial_files: &[
            (
                "common/src/lib.rs",
                "pub mod utils;",
            ),
            (
                "common/src/utils.rs",
                "pub fn do_stuff() {}",
            ),
            (
                "my_crate/src/main.rs",
                "use common::utils::do_stuff; fn main() { do_stuff(); }",
            ),
            (
                "Cargo.toml",
                "[workspace]\nmembers = [\"common\", \"my_crate\", \"new_utils\"]",
            ),
            (
                "common/Cargo.toml",
                "[package]\nname = \"common\"\nversion = \"0.1.0\"\nedition = \"2021\"",
            ),
            (
                "my_crate/Cargo.toml",
                "[package]\nname = \"my_crate\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\ncommon = { path = \"../common\" }",
            ),
            (
                "new_utils/Cargo.toml",
                "[package]\nname = \"new_utils\"\nversion = \"0.1.0\"\nedition = \"2021\"",
            ),
             (
                "new_utils/src/lib.rs",
                "",
            ),
        ],
        old_file_path: "common/src/utils.rs",
        new_file_path: "new_utils/src/lib.rs",
        expect_success: true,
        expected_import_updates: &[("my_crate/src/main.rs", "use new_utils::do_stuff;")],
    },
];

// =============================================================================
// MOVE DIRECTORY TEST CASES
// =============================================================================

pub const MOVE_DIRECTORY_TESTS: &[MoveFileTestCase] = &[MoveFileTestCase {
    test_name: "move_folder_with_nested_contents_and_imports",
    initial_files: &[
        ("src/components/core/Button.ts", "export class Button {}"),
        ("src/components/core/index.ts", "export * from './Button';"),
        (
            "src/components/utils.ts",
            "import { Button } from './core/Button';",
        ),
        (
            "src/index.ts",
            "import { Button } from './components/core/Button';",
        ),
    ],
    old_file_path: "src/components", // This is a directory
    new_file_path: "src/ui",
    expect_success: true,
    expected_import_updates: &[
        ("src/index.ts", "from './ui/core/Button'"),
        ("src/ui/utils.ts", "from './core/Button'"),
    ],
}];

// =============================================================================
// RUST RENAME FILE TEST CASES
// =============================================================================

pub const RUST_RENAME_FILE_TESTS: &[RenameFileTestCase] = &[
    RenameFileTestCase {
        test_name: "rust_rename_with_mod_declaration_in_parent_mod_rs",
        initial_files: &[
            (
                "src/mod.rs",
                r#"pub mod utils;
pub mod config;

pub use utils::*;
"#,
            ),
            (
                "src/utils.rs",
                r#"pub fn calculate(x: i32) -> i32 {
    x * 2
}

pub struct Helper {
    pub value: i32,
}
"#,
            ),
            (
                "src/main.rs",
                r#"use crate::utils::{calculate, Helper};

fn main() {
    let result = calculate(5);
    let helper = Helper { value: 10 };
    println!("{} {}", result, helper.value);
}
"#,
            ),
        ],
        old_file_path: "src/utils.rs",
        new_file_path: "src/helpers.rs",
        expect_success: true,
        expected_import_updates: &[
            ("src/mod.rs", "pub mod helpers;"),
            ("src/main.rs", "use crate::helpers::{calculate, Helper}"),
        ],
    },
    RenameFileTestCase {
        test_name: "rust_rename_with_mod_declaration_in_lib_rs",
        initial_files: &[
            (
                "src/lib.rs",
                r#"pub mod database;
pub mod models;

pub use database::*;
"#,
            ),
            (
                "src/database.rs",
                r#"pub struct Connection {
    pub url: String,
}

impl Connection {
    pub fn new(url: String) -> Self {
        Self { url }
    }
}
"#,
            ),
            (
                "src/models.rs",
                r#"use crate::database::Connection;

pub struct User {
    pub conn: Connection,
}
"#,
            ),
        ],
        old_file_path: "src/database.rs",
        new_file_path: "src/db.rs",
        expect_success: true,
        expected_import_updates: &[
            ("src/lib.rs", "pub mod db;"),
            ("src/models.rs", "use crate::db::Connection"),
        ],
    },
    RenameFileTestCase {
        test_name: "rust_rename_with_sibling_mod_rs_declaration",
        initial_files: &[
            (
                "src/services/mod.rs",
                r#"pub mod auth;
pub mod storage;

pub use auth::*;
"#,
            ),
            (
                "src/services/auth.rs",
                r#"pub fn authenticate(user: &str) -> bool {
    !user.is_empty()
}

pub struct AuthToken {
    pub token: String,
}
"#,
            ),
            (
                "src/main.rs",
                r#"use myapp::services::auth::{authenticate, AuthToken};

fn main() {
    let valid = authenticate("user1");
    let token = AuthToken { token: "abc".to_string() };
    println!("{} {}", valid, token.token);
}
"#,
            ),
        ],
        old_file_path: "src/services/auth.rs",
        new_file_path: "src/services/authentication.rs",
        expect_success: true,
        expected_import_updates: &[
            ("src/services/mod.rs", "pub mod authentication;"),
            (
                "src/main.rs",
                "use myapp::services::authentication::{authenticate, AuthToken}",
            ),
        ],
    },
    RenameFileTestCase {
        test_name: "rust_rename_nested_mod_tree_multiple_levels",
        initial_files: &[
            ("src/lib.rs", "pub mod core;"),
            ("src/core/mod.rs", "pub mod engine;"),
            (
                "src/core/engine/mod.rs",
                r#"pub mod processor;

pub use processor::*;
"#,
            ),
            (
                "src/core/engine/processor.rs",
                r#"pub fn process_data(input: &str) -> String {
    input.to_uppercase()
}

pub struct Processor {
    pub buffer: Vec<u8>,
}
"#,
            ),
            (
                "src/main.rs",
                r#"use mylib::core::engine::processor::{process_data, Processor};

fn main() {
    let result = process_data("test");
    let proc = Processor { buffer: vec![1, 2, 3] };
    println!("{} {:?}", result, proc.buffer);
}
"#,
            ),
        ],
        old_file_path: "src/core/engine/processor.rs",
        new_file_path: "src/core/engine/handler.rs",
        expect_success: true,
        expected_import_updates: &[
            ("src/core/engine/mod.rs", "pub mod handler;"),
            (
                "src/main.rs",
                "use mylib::core::engine::handler::{process_data, Processor}",
            ),
        ],
    },
    RenameFileTestCase {
        test_name: "rust_rename_affecting_multiple_use_statements",
        initial_files: &[
            ("src/lib.rs", "pub mod types;"),
            (
                "src/types.rs",
                r#"pub struct Config {
    pub host: String,
    pub port: u16,
}

pub enum Status {
    Active,
    Inactive,
}

pub type Result<T> = std::result::Result<T, String>;
"#,
            ),
            (
                "src/server.rs",
                r#"use crate::types::Config;
use crate::types::Status;

pub struct Server {
    config: Config,
    status: Status,
}
"#,
            ),
            (
                "src/client.rs",
                r#"use crate::types::{Config, Result};

pub struct Client {
    config: Config,
}

impl Client {
    pub fn connect(&self) -> Result<()> {
        Ok(())
    }
}
"#,
            ),
            (
                "src/main.rs",
                r#"use myapp::types::{Config, Status};

fn main() {
    let cfg = Config { host: "localhost".to_string(), port: 8080 };
    let status = Status::Active;
    println!("{} {}", cfg.host, cfg.port);
}
"#,
            ),
        ],
        old_file_path: "src/types.rs",
        new_file_path: "src/definitions.rs",
        expect_success: true,
        expected_import_updates: &[
            ("src/lib.rs", "pub mod definitions;"),
            ("src/server.rs", "use crate::definitions::Config"),
            ("src/server.rs", "use crate::definitions::Status"),
            ("src/client.rs", "use crate::definitions::{Config, Result}"),
            ("src/main.rs", "use myapp::definitions::{Config, Status}"),
        ],
    },
    RenameFileTestCase {
        test_name: "rust_rename_affecting_both_mod_and_use_in_same_file",
        initial_files: &[
            (
                "src/lib.rs",
                r#"pub mod network;
pub mod utils;

use network::Connection;
use utils::format_url;

pub fn connect(url: &str) -> Connection {
    let formatted = format_url(url);
    Connection::new(formatted)
}
"#,
            ),
            (
                "src/network.rs",
                r#"pub struct Connection {
    pub url: String,
}

impl Connection {
    pub fn new(url: String) -> Self {
        Self { url }
    }
}
"#,
            ),
            (
                "src/utils.rs",
                r#"pub fn format_url(url: &str) -> String {
    format!("https://{}", url)
}
"#,
            ),
        ],
        old_file_path: "src/network.rs",
        new_file_path: "src/net.rs",
        expect_success: true,
        expected_import_updates: &[
            ("src/lib.rs", "pub mod net;"),
            ("src/lib.rs", "use net::Connection;"),
        ],
    },
];

// =============================================================================
// RUST RENAME DIRECTORY TEST CASES
// =============================================================================

pub const RUST_RENAME_DIRECTORY_TESTS: &[RenameDirectoryTestCase] = &[
    RenameDirectoryTestCase {
        test_name: "rust_rename_workspace_member_update_cargo_toml",
        initial_files: &[
            (
                "Cargo.toml",
                r#"[workspace]
members = ["old_crate", "consumer"]

[workspace.package]
version = "0.1.0"
edition = "2021"
"#,
            ),
            (
                "old_crate/Cargo.toml",
                r#"[package]
name = "old_crate"
version = "0.1.0"
edition = "2021"
"#,
            ),
            (
                "old_crate/src/lib.rs",
                r#"pub fn greet(name: &str) -> String {
    format!("Hello, {}!", name)
}
"#,
            ),
            (
                "consumer/Cargo.toml",
                r#"[package]
name = "consumer"
version = "0.1.0"
edition = "2021"

[dependencies]
old_crate = { path = "../old_crate" }
"#,
            ),
            (
                "consumer/src/main.rs",
                r#"use old_crate::greet;

fn main() {
    println!("{}", greet("world"));
}
"#,
            ),
        ],
        dir_to_rename: "old_crate",
        new_dir_name: "new_crate",
        update_imports: true,
        expect_success: true,
    },
    RenameDirectoryTestCase {
        test_name: "rust_rename_crate_update_path_dependencies",
        initial_files: &[
            (
                "Cargo.toml",
                r#"[workspace]
members = ["my_utils", "app", "tests"]
"#,
            ),
            (
                "my_utils/Cargo.toml",
                r#"[package]
name = "my_utils"
version = "0.1.0"
edition = "2021"
"#,
            ),
            (
                "my_utils/src/lib.rs",
                r#"pub fn add(a: i32, b: i32) -> i32 {
    a + b
}
"#,
            ),
            (
                "app/Cargo.toml",
                r#"[package]
name = "app"
version = "0.1.0"
edition = "2021"

[dependencies]
my_utils = { path = "../my_utils" }
"#,
            ),
            (
                "app/src/main.rs",
                r#"use my_utils::add;

fn main() {
    println!("{}", add(2, 3));
}
"#,
            ),
            (
                "tests/Cargo.toml",
                r#"[package]
name = "tests"
version = "0.1.0"
edition = "2021"

[dependencies]
my_utils = { path = "../my_utils" }
"#,
            ),
            (
                "tests/tests/integration.rs",
                r#"use my_utils::add;

#[test]
fn test_add() {
    assert_eq!(add(1, 1), 2);
}
"#,
            ),
        ],
        dir_to_rename: "my_utils",
        new_dir_name: "common_utils",
        update_imports: true,
        expect_success: true,
    },
    RenameDirectoryTestCase {
        test_name: "rust_rename_crate_update_cross_crate_use_statements",
        initial_files: &[
            (
                "Cargo.toml",
                r#"[workspace]
members = ["core_lib", "app"]
"#,
            ),
            (
                "core_lib/Cargo.toml",
                r#"[package]
name = "core_lib"
version = "0.1.0"
edition = "2021"
"#,
            ),
            (
                "core_lib/src/lib.rs",
                r#"pub mod parser;
pub mod validator;
"#,
            ),
            (
                "core_lib/src/parser.rs",
                r#"pub fn parse(input: &str) -> Vec<String> {
    input.split_whitespace().map(|s| s.to_string()).collect()
}
"#,
            ),
            (
                "core_lib/src/validator.rs",
                r#"pub fn validate(data: &[String]) -> bool {
    !data.is_empty()
}
"#,
            ),
            (
                "app/Cargo.toml",
                r#"[package]
name = "app"
version = "0.1.0"
edition = "2021"

[dependencies]
core_lib = { path = "../core_lib" }
"#,
            ),
            (
                "app/src/main.rs",
                r#"use core_lib::parser::parse;
use core_lib::validator::validate;

fn main() {
    let data = parse("hello world");
    let valid = validate(&data);
    println!("Valid: {}", valid);
}
"#,
            ),
        ],
        dir_to_rename: "core_lib",
        new_dir_name: "foundation",
        update_imports: true,
        expect_success: true,
    },
    RenameDirectoryTestCase {
        test_name: "rust_rename_nested_module_directory_internal_use",
        initial_files: &[
            ("src/lib.rs", "pub mod services;"),
            ("src/services/mod.rs", "pub mod payment;"),
            (
                "src/services/payment/mod.rs",
                r#"pub mod processor;
pub mod gateway;

pub use processor::PaymentProcessor;
pub use gateway::Gateway;
"#,
            ),
            (
                "src/services/payment/processor.rs",
                r#"use crate::services::payment::gateway::Gateway;

pub struct PaymentProcessor {
    gateway: Gateway,
}

impl PaymentProcessor {
    pub fn new(gateway: Gateway) -> Self {
        Self { gateway }
    }
}
"#,
            ),
            (
                "src/services/payment/gateway.rs",
                r#"pub struct Gateway {
    pub url: String,
}
"#,
            ),
            (
                "src/main.rs",
                r#"use myapp::services::payment::{PaymentProcessor, Gateway};

fn main() {
    let gateway = Gateway { url: "https://api.stripe.com".to_string() };
    let processor = PaymentProcessor::new(gateway);
}
"#,
            ),
        ],
        dir_to_rename: "src/services/payment",
        new_dir_name: "src/services/billing",
        update_imports: true,
        expect_success: true,
    },
    RenameDirectoryTestCase {
        test_name: "rust_rename_complex_cargo_toml_mod_use_combined",
        initial_files: &[
            (
                "Cargo.toml",
                r#"[workspace]
members = ["db_layer", "api_server", "models"]
"#,
            ),
            (
                "db_layer/Cargo.toml",
                r#"[package]
name = "db_layer"
version = "0.1.0"
edition = "2021"
"#,
            ),
            (
                "db_layer/src/lib.rs",
                r#"pub mod connection;
pub mod query;

pub use connection::DbConnection;
"#,
            ),
            (
                "db_layer/src/connection.rs",
                r#"pub struct DbConnection {
    pub url: String,
}
"#,
            ),
            (
                "db_layer/src/query.rs",
                r#"use crate::connection::DbConnection;

pub fn execute(conn: &DbConnection, sql: &str) -> Vec<String> {
    vec![format!("Result from {}: {}", conn.url, sql)]
}
"#,
            ),
            (
                "models/Cargo.toml",
                r#"[package]
name = "models"
version = "0.1.0"
edition = "2021"

[dependencies]
db_layer = { path = "../db_layer" }
"#,
            ),
            (
                "models/src/lib.rs",
                r#"use db_layer::DbConnection;

pub struct User {
    pub id: i64,
    pub name: String,
}

pub fn fetch_user(conn: &DbConnection, id: i64) -> Option<User> {
    Some(User { id, name: "test".to_string() })
}
"#,
            ),
            (
                "api_server/Cargo.toml",
                r#"[package]
name = "api_server"
version = "0.1.0"
edition = "2021"

[dependencies]
db_layer = { path = "../db_layer" }
models = { path = "../models" }
"#,
            ),
            (
                "api_server/src/main.rs",
                r#"use db_layer::{DbConnection, query};
use models::fetch_user;

fn main() {
    let conn = DbConnection { url: "postgres://localhost".to_string() };
    let user = fetch_user(&conn, 1);
    let results = query::execute(&conn, "SELECT * FROM users");
    println!("{:?}", results);
}
"#,
            ),
        ],
        dir_to_rename: "db_layer",
        new_dir_name: "database",
        update_imports: true,
        expect_success: true,
    },
];
