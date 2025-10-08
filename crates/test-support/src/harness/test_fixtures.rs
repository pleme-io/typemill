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
    // Go Case
    GoToDefinitionTestCase {
        language_id: "go",
        files: &[
            (
                "main.go",
                r#"package main

import "fmt"

func main() {
    result := helper()
    fmt.Println(result)
}

func helper() string {
    return "test"
}
"#,
            ),
        ],
        trigger_point: ("main.go", 5, 14), // Position on "helper" call
        expected_location: ("main.go", 9, 5), // Position of "helper" definition
    },
    // Rust Case
    GoToDefinitionTestCase {
        language_id: "rs",
        files: &[
            (
                "main.rs",
                r#"fn main() {
    let result = helper();
    println!("{}", result);
}

fn helper() -> &'static str {
    "test"
}
"#,
            ),
        ],
        trigger_point: ("main.rs", 1, 17), // Position on "helper" call
        expected_location: ("main.rs", 5, 3), // Position of "helper" definition
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
    // Python Case
    FindReferencesTestCase {
        language_id: "py",
        files: &[
            ("utils.py", "def calculate(x):\n    return x * 2"),
            ("main.py", "from utils import calculate\nresult = calculate(5)"),
        ],
        trigger_point: ("utils.py", 0, 4),
        expected_min_count: 1,
    },
    // Go Case
    FindReferencesTestCase {
        language_id: "go",
        files: &[
            (
                "main.go",
                r#"package main

func process(x int) int {
    return x * 2
}

func main() {
    val1 := process(5)
    val2 := process(10)
    _ = val1 + val2
}
"#,
            ),
        ],
        trigger_point: ("main.go", 2, 5), // Position on "process" definition
        expected_min_count: 2, // Two calls to process()
    },
    // Rust Case
    FindReferencesTestCase {
        language_id: "rs",
        files: &[
            (
                "main.rs",
                r#"fn compute(x: i32) -> i32 {
    x * 2
}

fn main() {
    let a = compute(5);
    let b = compute(10);
    println!("{} {}", a, b);
}
"#,
            ),
        ],
        trigger_point: ("main.rs", 0, 3), // Position on "compute" definition
        expected_min_count: 2, // Two calls to compute()
    },
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
    // Go Case
    HoverTestCase {
        language_id: "go",
        files: &[(
            "test.go",
            r#"package main

func multiply(a int, b int) int {
    return a * b
}

func main() {
    result := multiply(3, 4)
}
"#,
        )],
        trigger_point: ("test.go", 7, 14), // Position on "multiply" call
        should_have_contents: true,
    },
    // Rust Case
    HoverTestCase {
        language_id: "rs",
        files: &[(
            "test.rs",
            r#"fn divide(a: f64, b: f64) -> f64 {
    a / b
}

fn main() {
    let result = divide(10.0, 2.0);
}
"#,
        )],
        trigger_point: ("test.rs", 5, 17), // Position on "divide" call
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
    // Python Case
    DocumentSymbolsTestCase {
        language_id: "py",
        files: &[(
            "module.py",
            r#"
VERSION = "1.0.0"

class DataProcessor:
    def process(self, data):
        return data

def helper_function():
    pass
"#,
        )],
        document_path: "module.py",
        expected_min_count: 1,
    },
    // Go Case
    DocumentSymbolsTestCase {
        language_id: "go",
        files: &[(
            "package.go",
            r#"package mypackage

const Version = "1.0.0"

type DataService struct {
    name string
}

func (ds *DataService) Process() {
}

func HelperFunc() {
}
"#,
        )],
        document_path: "package.go",
        expected_min_count: 1,
    },
    // Rust Case
    DocumentSymbolsTestCase {
        language_id: "rs",
        files: &[(
            "lib.rs",
            r#"pub const VERSION: &str = "1.0.0";

pub struct Service {
    name: String,
}

impl Service {
    pub fn new(name: String) -> Self {
        Service { name }
    }

    pub fn process(&self) {
    }
}

pub fn helper() {
}
"#,
        )],
        document_path: "lib.rs",
        expected_min_count: 1,
    },
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
    // Python Case
    CompletionTestCase {
        language_id: "py",
        files: &[(
            "test.py",
            r#"
class MyClass:
    def method1(self):
        pass
    def method2(self):
        pass

obj = MyClass()
obj.
"#,
        )],
        trigger_point: ("test.py", 8, 4),
        should_have_items: true,
    },
    // Go Case
    CompletionTestCase {
        language_id: "go",
        files: &[(
            "test.go",
            r#"package main

type MyStruct struct {
    Field1 string
    Field2 int
}

func main() {
    obj := MyStruct{}
    obj.
}
"#,
        )],
        trigger_point: ("test.go", 9, 8),
        should_have_items: true,
    },
    // Rust Case
    CompletionTestCase {
        language_id: "rs",
        files: &[(
            "test.rs",
            r#"
struct MyStruct {
    field1: String,
    field2: i32,
}

fn main() {
    let obj = MyStruct {
        field1: String::from("test"),
        field2: 42,
    };
    obj.
}
"#,
        )],
        trigger_point: ("test.rs", 11, 8),
        should_have_items: true,
    },
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
    // Python Case
    RenameTestCase {
        language_id: "py",
        files: &[(
            "test.py",
            r#"
my_value = 100
total = my_value * 2
print(total)
"#,
        )],
        trigger_point: ("test.py", 1, 0),
        new_name: "renamed_value",
        should_have_changes: true,
    },
    // Go Case
    RenameTestCase {
        language_id: "go",
        files: &[(
            "test.go",
            r#"package main

func main() {
    myVar := 42
    result := myVar * 2
    _ = result
}
"#,
        )],
        trigger_point: ("test.go", 3, 4),
        new_name: "renamedVar",
        should_have_changes: true,
    },
    // Rust Case
    RenameTestCase {
        language_id: "rs",
        files: &[(
            "test.rs",
            r#"
fn main() {
    let my_variable = 42;
    let result = my_variable * 2;
    println!("{}", result);
}
"#,
        )],
        trigger_point: ("test.rs", 2, 8),
        new_name: "renamed_variable",
        should_have_changes: true,
    },
];

// =============================================================================
// LSP Compliance Test Cases - Server Behavior Documentation
// =============================================================================

/// Defines the expected behavior of an LSP server for a compliance test.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LspComplianceBehavior {
    /// Expects the response to be a JSON array with one or more elements.
    ReturnsNonEmptyArray,
    /// Expects the response to be a JSON array with zero elements.
    ReturnsEmptyArray,
    /// Expects the server to return an error for the request.
    Fails,
}

/// Represents a single test case in the LSP compliance suite.
#[derive(Debug, Clone)]
pub struct LspComplianceTestCase {
    /// The language server to test (e.g., "rust", "typescript").
    pub language_id: &'static str,
    /// A descriptive name for the feature being tested.
    pub feature_name: &'static str,
    /// The LSP method to call
    pub method: &'static str,
    /// The params as JSON value
    pub params: fn() -> serde_json::Value,
    /// Files to create in the workspace
    pub files: &'static [(&'static str, &'static str)],
    /// The expected behavior from the server.
    pub expected_behavior: LspComplianceBehavior,
}

/// Helper function to create params for workspace/symbol with minimal query
/// Using a single character to test if rust-analyzer returns symbols
fn workspace_symbol_empty_params() -> serde_json::Value {
    serde_json::json!({ "query": "m" })
}

/// Helper function for workspace/symbol with specific query
fn workspace_symbol_data_params() -> serde_json::Value {
    serde_json::json!({ "query": "Data" })
}

/// The central array of all compliance tests to be run.
pub const LSP_COMPLIANCE_TESTS: &[LspComplianceTestCase] = &[
    // Test case for rust-analyzer's handling of a minimal workspace/symbol query.
    // With the correct initializationOptions (workspace.symbol.search.kind = "all"),
    // rust-analyzer should return function symbols (not just types) for query "m" matching "main".
    LspComplianceTestCase {
        language_id: "rs",
        feature_name: "workspace_symbol_empty_query",
        method: "workspace/symbol",
        params: workspace_symbol_empty_params,
        files: &[
            (
                "Cargo.toml",
                r#"[package]
name = "test"
version = "0.1.0"
edition = "2021"
"#,
            ),
            ("src/main.rs", "fn main() {}\nfn my_func() {}"),
            (
                "src/lib.rs",
                "pub fn helper() {}\npub fn make_something() {}",
            ),
        ],
        expected_behavior: LspComplianceBehavior::ReturnsNonEmptyArray,
    },
    // Test case for TypeScript LSP - documents that it needs project initialization
    // TypeScript LSP returns error "No Project" when workspace/symbol is called too quickly
    // This documents that TS needs proper initialization time, unlike workspace-wide symbol support
    LspComplianceTestCase {
        language_id: "ts",
        feature_name: "workspace_symbol_needs_init",
        method: "workspace/symbol",
        params: workspace_symbol_data_params,
        files: &[
            (
                "tsconfig.json",
                r#"{"compilerOptions": {"target": "ES2020", "module": "commonjs"}}"#,
            ),
            ("models.ts", "export class DataModel {}"),
        ],
        expected_behavior: LspComplianceBehavior::Fails,
    },
];
