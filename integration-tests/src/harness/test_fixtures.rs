//! Data-driven test fixtures for LSP features
//!
//! This module contains language-specific test data for all LSP features.
//! Each fixture struct represents a single test case with all the necessary
//! code snippets, file names, and expected outcomes.

/// Test fixture for "go to definition" tests
#[derive(Debug, Clone)]
pub struct GoToDefinitionTestCase {
    pub language_id: &'static str,
    pub files: &'static [(&'static str, &'static str)], // (path, content)
    pub trigger_point: (&'static str, u32, u32),        // (path, line, char)
    pub expected_location: (&'static str, u32, u32),    // (path, line, char)
}

/// Test fixture for "find references" tests
#[derive(Debug, Clone)]
pub struct FindReferencesTestCase {
    pub language_id: &'static str,
    pub files: &'static [(&'static str, &'static str)],
    pub trigger_point: (&'static str, u32, u32),
    pub expected_min_count: usize, // Minimum number of references expected
}

/// Test fixture for "hover" tests
#[derive(Debug, Clone)]
pub struct HoverTestCase {
    pub language_id: &'static str,
    pub files: &'static [(&'static str, &'static str)],
    pub trigger_point: (&'static str, u32, u32),
    pub should_have_contents: bool,
}

/// Test fixture for "document symbols" tests
#[derive(Debug, Clone)]
pub struct DocumentSymbolsTestCase {
    pub language_id: &'static str,
    pub files: &'static [(&'static str, &'static str)],
    pub document_path: &'static str,
    pub expected_min_count: usize, // Minimum number of symbols expected
}

/// Test fixture for "workspace symbols" tests
#[derive(Debug, Clone)]
pub struct WorkspaceSymbolsTestCase {
    pub language_id: &'static str,
    pub files: &'static [(&'static str, &'static str)],
    pub query: &'static str,
    pub should_find_symbols: bool,
}

/// Test fixture for "completion" tests
#[derive(Debug, Clone)]
pub struct CompletionTestCase {
    pub language_id: &'static str,
    pub files: &'static [(&'static str, &'static str)],
    pub trigger_point: (&'static str, u32, u32),
    pub should_have_items: bool,
}

/// Test fixture for "rename" tests
#[derive(Debug, Clone)]
pub struct RenameTestCase {
    pub language_id: &'static str,
    pub files: &'static [(&'static str, &'static str)],
    pub trigger_point: (&'static str, u32, u32),
    pub new_name: &'static str,
    pub should_have_changes: bool,
}

// =============================================================================
// Go To Definition Test Cases
// =============================================================================

pub const GO_TO_DEFINITION_TESTS: &[GoToDefinitionTestCase] = &[
    // TypeScript Case
    GoToDefinitionTestCase {
        language_id: "ts",
        files: &[
            (
                "main.ts",
                r#"import { util } from './util';
util();"#,
            ),
            ("util.ts", "export function util() {}"),
        ],
        trigger_point: ("main.ts", 0, 9),
        expected_location: ("util.ts", 0, 17),
    },
    // Python Case
    GoToDefinitionTestCase {
        language_id: "py",
        files: &[
            ("main.py", "from helper import func\nfunc()"),
            ("helper.py", "def func():\n    return 42"),
        ],
        trigger_point: ("main.py", 0, 19),
        expected_location: ("helper.py", 0, 4),
    },
];

// =============================================================================
// Find References Test Cases
// =============================================================================

pub const FIND_REFERENCES_TESTS: &[FindReferencesTestCase] = &[
    // TypeScript Case
    FindReferencesTestCase {
        language_id: "ts",
        files: &[
            ("utils.ts", "export function helper() { return 42; }"),
            (
                "main.ts",
                "import { helper } from './utils';\nconst x = helper();",
            ),
        ],
        trigger_point: ("utils.ts", 0, 17),
        expected_min_count: 1,
    },
    // Python Case - Add when ready
];

// =============================================================================
// Hover Test Cases
// =============================================================================

pub const HOVER_TESTS: &[HoverTestCase] = &[
    // TypeScript Case
    HoverTestCase {
        language_id: "ts",
        files: &[(
            "test.ts",
            "function greet(name: string) { return 'Hello ' + name; }\nconst msg = greet('World');",
        )],
        trigger_point: ("test.ts", 1, 12),
        should_have_contents: true,
    },
    // Python Case
    HoverTestCase {
        language_id: "py",
        files: &[(
            "test.py",
            r#"
def add(x, y):
    return x + y

result = add(1, 2)
"#,
        )],
        trigger_point: ("test.py", 4, 9),
        should_have_contents: true,
    },
];

// =============================================================================
// Document Symbols Test Cases
// =============================================================================

pub const DOCUMENT_SYMBOLS_TESTS: &[DocumentSymbolsTestCase] = &[
    // TypeScript Case
    DocumentSymbolsTestCase {
        language_id: "ts",
        files: &[(
            "symbols.ts",
            r#"
export const VERSION = '1.0.0';
export class MyClass {
    method() {}
}
"#,
        )],
        document_path: "symbols.ts",
        expected_min_count: 1,
    },
    // Python Case - Add when ready
];

// =============================================================================
// Workspace Symbols Test Cases
// =============================================================================

pub const WORKSPACE_SYMBOLS_TESTS: &[WorkspaceSymbolsTestCase] = &[
    // TypeScript Case
    WorkspaceSymbolsTestCase {
        language_id: "ts",
        files: &[("models.ts", "export class DataModel {}")],
        query: "Data",
        should_find_symbols: true,
    },
    // Rust Case 1 - Empty query (documents rust-analyzer limitation)
    WorkspaceSymbolsTestCase {
        language_id: "rs",
        files: &[
            ("main.rs", "fn main() {}\nfn helper() {}"),
            ("lib.rs", "pub struct MyStruct {}\npub fn util() {}"),
        ],
        query: "",
        should_find_symbols: false, // rust-analyzer returns empty for empty query
    },
    // Rust Case 2 - Wildcard query
    WorkspaceSymbolsTestCase {
        language_id: "rs",
        files: &[
            ("main.rs", "fn main() {}\nfn helper() {}"),
            ("lib.rs", "pub struct MyStruct {}\npub fn util() {}"),
        ],
        query: "*",
        should_find_symbols: false, // rust-analyzer doesn't support wildcard
    },
    // Rust Case 3 - Specific symbol name query
    WorkspaceSymbolsTestCase {
        language_id: "rs",
        files: &[("main.rs", "fn main() {}\nfn helper_function() {}")],
        query: "helper",
        should_find_symbols: false, // Documents actual rust-analyzer behavior
    },
    // Python Case - Add when ready
];

// =============================================================================
// Completion Test Cases
// =============================================================================

pub const COMPLETION_TESTS: &[CompletionTestCase] = &[
    // TypeScript Case
    CompletionTestCase {
        language_id: "ts",
        files: &[(
            "test.ts",
            r#"
const myObj = { prop1: 'value', prop2: 42 };
myObj.
"#,
        )],
        trigger_point: ("test.ts", 2, 6),
        should_have_items: true,
    },
    // Python Case - Add when ready
];

// =============================================================================
// Rename Test Cases
// =============================================================================

pub const RENAME_TESTS: &[RenameTestCase] = &[
    // TypeScript Case
    RenameTestCase {
        language_id: "ts",
        files: &[(
            "test.ts",
            r#"
const myVariable = 42;
const result = myVariable + 10;
"#,
        )],
        trigger_point: ("test.ts", 1, 6),
        new_name: "renamedVariable",
        should_have_changes: true,
    },
    // Python Case - Add when ready
];
