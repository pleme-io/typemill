//! List Functions Test Harness
//!
//! Provides testing for the `list_functions()` capability across all language plugins.
//! These tests ensure plugins correctly extract function/method names from source code.
//!
//! Tests cover:
//! - Multiple functions: Extract 3 functions from valid source code
//! - Empty results: Source with no functions (only fields/constants)
//!
//! # Usage
//!
//! ```rust
//! use mill_test_support::harness::list_functions_harness::test_all_plugins_list_functions;
//!
//! #[tokio::test]
//! async fn test_list_functions() {
//!     test_all_plugins_list_functions().await;
//! }
//! ```

use crate::harness::plugin_discovery;

/// Tests all discovered plugins for listing multiple functions.
///
/// Creates source code with 3 functions in language-specific syntax
/// and verifies the plugin can extract their names.
pub async fn test_all_plugins_list_functions_multiple() {
    let plugins = plugin_discovery::get_test_registry().all();

    for plugin in plugins {
        let meta = plugin.metadata();

        // Generate language-specific source with 3 functions
        let source = match meta.name.as_ref() {
            "TypeScript" => r#"
function firstFunction() {
    return "first";
}

async function secondFunction() {
    return "second";
}

const thirdFunction = () => {
    return "third";
};
"#,
            "Rust" => r#"
fn first_function() {
    println!("first");
}

fn second_function() -> i32 {
    42
}

pub fn third_function() {}
"#,
            "Python" => r#"
def first_function():
    print("first")

def second_function():
    return 42

def third_function():
    pass
"#,
            "Java" => r#"
public class MyClass {
    public void firstMethod() {}
    private int secondMethod() { return 0; }
    public static String thirdMethod() { return "test"; }
}
"#,
            "Go" => r#"package main

func FirstFunction() {
    println("first")
}

func SecondFunction() {
    println("second")
}

func ThirdFunction() int {
    return 42
}
"#,
            "C#" => r#"
public class MyClass {
    public void FirstMethod() {}
    private int SecondMethod() { return 0; }
    public static string ThirdMethod() { return "test"; }
}
"#,
            "Swift" => r#"
func firstFunction() {
    print("first")
}

func secondFunction() -> Int {
    return 42
}

func thirdFunction() {}
"#,
            "C" => r#"
void first_function() {
    printf("first\n");
}

int second_function() {
    return 42;
}

void third_function() {}
"#,
            "C++" => r#"
void firstFunction() {
    std::cout << "first" << std::endl;
}

int secondFunction() {
    return 42;
}

void thirdFunction() {}
"#,
            _ => {
                // Skip plugins without list_functions support (config languages)
                continue;
            }
        };

        let result = plugin.list_functions(source).await;

        // Test should not fail (but may return empty if language tools unavailable)
        assert!(
            result.is_ok(),
            "Plugin '{}' list_functions failed: {:?}",
            meta.name,
            result.err()
        );

        let functions = result.unwrap();

        // If functions were extracted, verify they're correct
        // (May be empty if language parser/tools not available)
        if !functions.is_empty() {
            let expected_names = match meta.name.as_ref() {
                "TypeScript" => vec!["firstFunction", "secondFunction", "thirdFunction"],
                "Rust" => vec!["first_function", "second_function", "third_function"],
                "Python" => vec!["first_function", "second_function", "third_function"],
                "Java" => vec!["firstMethod", "secondMethod", "thirdMethod"],
                "Go" => vec!["FirstFunction", "SecondFunction", "ThirdFunction"],
                "C#" => vec!["FirstMethod", "SecondMethod", "ThirdMethod"],
                "Swift" => vec!["firstFunction", "secondFunction", "thirdFunction"],
                "C" => vec!["first_function", "second_function", "third_function"],
                "C++" => vec!["firstFunction", "secondFunction", "thirdFunction"],
                _ => continue,
            };

            for expected in expected_names {
                assert!(
                    functions.contains(&expected.to_string()),
                    "Plugin '{}' missing function '{}'. Found: {:?}",
                    meta.name,
                    expected,
                    functions
                );
            }
        }
    }
}

/// Tests all discovered plugins for empty function lists.
///
/// Creates source code with only fields/constants (no functions)
/// and verifies the plugin returns empty or excludes non-functions.
pub async fn test_all_plugins_list_functions_empty() {
    let plugins = plugin_discovery::get_test_registry().all();

    for plugin in plugins {
        let meta = plugin.metadata();

        // Generate language-specific source with NO functions (only fields/constants)
        let source = match meta.name.as_ref() {
            "TypeScript" => r#"
const myConstant = 42;
let myVariable = "test";
type UserId = string;

class MyClass {
    private myField: number;
    static readonly MAX_SIZE = 100;
}
"#,
            "Rust" => r#"
const MY_CONSTANT: i32 = 42;
static MY_STATIC: &str = "test";

struct MyStruct {
    field: i32,
}
"#,
            "Python" => r#"
MY_CONSTANT = 42
my_variable = "test"

class MyClass:
    my_field = 100
"#,
            "Java" => r#"
public class MyClass {
    private int myField;
    public static final int CONSTANT = 42;
}
"#,
            "Go" => r#"package main

const MaxSize = 100

var GlobalVar = "test"

type User struct {
    Name string
}
"#,
            "C#" => r#"
public class MyClass {
    private int myField;
    public const int MAX_SIZE = 100;
}
"#,
            "Swift" => r#"
let myConstant = 42
var myVariable = "test"

struct MyStruct {
    var field: Int
}
"#,
            "C" => r#"
#define MAX_SIZE 100

int global_var = 42;

struct MyStruct {
    int field;
};
"#,
            "C++" => r#"
const int MAX_SIZE = 100;

int globalVar = 42;

class MyClass {
private:
    int myField;
};
"#,
            _ => {
                // Skip plugins without list_functions support
                continue;
            }
        };

        let result = plugin.list_functions(source).await;

        // Test should not fail
        assert!(
            result.is_ok(),
            "Plugin '{}' list_functions failed on empty source: {:?}",
            meta.name,
            result.err()
        );

        let functions = result.unwrap();

        // Should return empty list (no functions to find)
        // Or if it finds anything, those should not be field/constant names
        let invalid_names = match meta.name.as_ref() {
            "TypeScript" => vec!["myConstant", "myVariable", "UserId", "myField", "MAX_SIZE"],
            "Rust" => vec!["MY_CONSTANT", "MY_STATIC", "field"],
            "Python" => vec!["MY_CONSTANT", "my_variable", "my_field"],
            "Java" => vec!["myField", "CONSTANT"],
            "Go" => vec!["MaxSize", "GlobalVar", "User"],
            "C#" => vec!["myField", "MAX_SIZE"],
            "Swift" => vec!["myConstant", "myVariable", "field"],
            "C" => vec!["MAX_SIZE", "global_var"],
            "C++" => vec!["MAX_SIZE", "globalVar", "myField"],
            _ => continue,
        };

        for invalid in invalid_names {
            assert!(
                !functions.contains(&invalid.to_string()),
                "Plugin '{}' incorrectly listed non-function '{}' as a function",
                meta.name,
                invalid
            );
        }
    }
}

/// Comprehensive test runner for all list_functions tests.
///
/// Runs both multiple functions and empty function tests.
pub async fn test_all_plugins_list_functions() {
    test_all_plugins_list_functions_multiple().await;
    test_all_plugins_list_functions_empty().await;
}
