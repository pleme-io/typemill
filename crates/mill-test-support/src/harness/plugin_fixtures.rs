//! Reusable Test Fixtures for Language Plugins
//!
//! Provides edge case test inputs and templates used across all language plugins.
//! These fixtures help ensure consistent test coverage and catch common edge cases.

/// Edge case test inputs used across all language plugins
///
/// These fixtures represent common edge cases that all plugins should handle:
/// - Unicode identifiers and comments
/// - Extremely long lines
/// - Files without newlines
/// - Mixed line endings (CRLF and LF)
/// - Empty files and whitespace-only files
/// - Special regex characters in code
/// - Null bytes and binary-like content
pub mod edge_cases {
    /// Unicode identifiers in code
    ///
    /// Tests that plugins correctly handle variable and function names
    /// containing non-ASCII Unicode characters.
    ///
    /// # Example
    /// ```text
    /// fn 测试_函数() { }
    /// let 変数 = 日本語;
    /// ```
    pub fn unicode_identifiers() -> &'static str {
        "// Test: 变量名 函数名\nfn test_函数() { let 變數 = 42; }"
    }

    /// Extremely long line
    ///
    /// Tests that plugins can handle very long lines (15,000+ characters)
    /// without performance degradation or errors.
    pub fn extremely_long_line() -> String {
        "let x = ".to_string() + &"a".repeat(15000) + ";"
    }

    /// Source code without newlines
    ///
    /// Single-line file with no trailing newline.
    /// Tests that plugins don't assume newline-based structure.
    pub fn no_newlines() -> &'static str {
        "fn main() { let x = 42; }"
    }

    /// Mixed line endings (CRLF and LF)
    ///
    /// File with inconsistent line endings - some CRLF (\r\n), some LF (\n).
    /// Tests that plugins handle cross-platform line ending variations.
    pub fn mixed_line_endings() -> String {
        "line1\r\nline2\nline3\r\nline4\n".to_string()
    }

    /// Empty file
    ///
    /// Zero-length file or file with only whitespace.
    pub fn empty_file() -> &'static str {
        ""
    }

    /// Whitespace-only file
    ///
    /// File containing only spaces, tabs, and newlines.
    pub fn whitespace_only() -> &'static str {
        "   \n\t\n   "
    }

    /// Special regex characters in string literals
    ///
    /// Tests that plugins don't treat string content as code structure.
    /// Includes characters that have special meaning in regex: . * + ? [ ] { } ( ) ^ $ | \
    pub fn special_regex_chars() -> &'static str {
        r#"let pattern = ".*+?[]{}()^$|\\";"#
    }

    /// Null bytes in content
    ///
    /// Binary data embedded in code (if supported by language).
    /// Tests robustness against unusual input.
    pub fn null_bytes() -> String {
        "fn test() { let s = \"null byte here\"; }".to_string()
    }

    /// Comments containing code-like syntax
    ///
    /// Tests that plugins correctly skip comments and don't parse them as code.
    pub fn comments_with_code_syntax() -> &'static str {
        "// import something\n/* fn fake() {} */\nfn real() {}"
    }

    /// Strings containing comment-like syntax
    ///
    /// Tests that plugins don't treat string content as comments.
    pub fn strings_with_comment_syntax() -> &'static str {
        r#"let s = "// this is not a comment";"#
    }

    /// Nested multiline comments/strings
    ///
    /// Tests handling of nested structures (if language supports them).
    pub fn nested_structures() -> &'static str {
        "/* comment /* nested */ still comment */ code"
    }

    /// Unmatched brackets and braces
    ///
    /// Tests robustness to malformed code.
    pub fn unmatched_delimiters() -> &'static str {
        "fn broken() { [ ] }"
    }
}

/// Language-specific large file generator
///
/// Generates test files with many symbols for performance testing.
/// Used to verify plugins can handle large codebases without performance issues.
pub fn large_file_template(language: &str, item_count: usize) -> String {
    match language {
        "rust" => generate_rust_large_file(item_count),
        "typescript" => generate_typescript_large_file(item_count),
        "python" => generate_python_large_file(item_count),
        "go" => generate_go_large_file(item_count),
        "java" => generate_java_large_file(item_count),
        "c" | "cpp" | "c++" => generate_c_large_file(item_count),
        "csharp" => generate_csharp_large_file(item_count),
        "swift" => generate_swift_large_file(item_count),
        _ => panic!("Unsupported language for large file template: {}", language),
    }
}

/// Rust: Generate file with many functions and modules
fn generate_rust_large_file(count: usize) -> String {
    let mut content = String::from("// Generated large Rust file for performance testing\n\n");

    for i in 0..count {
        content.push_str(&format!(
            "/// Function {}\npub fn function_{}() -> i32 {{\n    {}\n}}\n\n",
            i, i, i
        ));
    }

    content
}

/// TypeScript: Generate file with many functions
fn generate_typescript_large_file(count: usize) -> String {
    let mut content = String::from("// Generated large TypeScript file for performance testing\n\n");

    for i in 0..count {
        content.push_str(&format!(
            "/**\n * Function {}\n */\nexport function function{}(): number {{\n    return {};\n}}\n\n",
            i, i, i
        ));
    }

    content
}

/// Python: Generate file with many functions
fn generate_python_large_file(count: usize) -> String {
    let mut content = String::from("# Generated large Python file for performance testing\n\n");

    for i in 0..count {
        content.push_str(&format!(
            "def function_{}():\n    \"\"\"Function {}.\"\"\"\n    return {}\n\n",
            i, i, i
        ));
    }

    content
}

/// Go: Generate file with many functions
fn generate_go_large_file(count: usize) -> String {
    let mut content = String::from("// Generated large Go file for performance testing\npackage main\n\n");

    for i in 0..count {
        content.push_str(&format!(
            "// Function{}\nfunc Function{}() int {{\n    return {}\n}}\n\n",
            i, i, i
        ));
    }

    content
}

/// Java: Generate file with many methods in a class
fn generate_java_large_file(count: usize) -> String {
    let mut content = String::from("// Generated large Java file for performance testing\npublic class LargeClass {\n\n");

    for i in 0..count {
        content.push_str(&format!(
            "    /**\n     * Method {}\n     */\n    public int method{}() {{\n        return {};\n    }}\n\n",
            i, i, i
        ));
    }

    content.push_str("}\n");
    content
}

/// C/C++: Generate file with many functions
fn generate_c_large_file(count: usize) -> String {
    let mut content = String::from("// Generated large C file for performance testing\n\n");

    for i in 0..count {
        content.push_str(&format!(
            "int function_{}() {{\n    return {};\n}}\n\n",
            i, i
        ));
    }

    content
}

/// C#: Generate file with many methods
fn generate_csharp_large_file(count: usize) -> String {
    let mut content = String::from("// Generated large C# file for performance testing\npublic class LargeClass\n{{\n\n");

    for i in 0..count {
        content.push_str(&format!(
            "    /// <summary>\n    /// Method {}\n    /// </summary>\n    public int Method{}()\n    {{\n        return {};\n    }}\n\n",
            i, i, i
        ));
    }

    content.push_str("}\n");
    content
}

/// Swift: Generate file with many functions
fn generate_swift_large_file(count: usize) -> String {
    let mut content = String::from("// Generated large Swift file for performance testing\n\n");

    for i in 0..count {
        content.push_str(&format!(
            "/// Function {}\nfunc function{}() -> Int {{\n    return {}\n}}\n\n",
            i, i, i
        ));
    }

    content
}

/// Test fixtures for common import patterns
pub mod import_fixtures {
    /// Single import statement
    pub fn single_import(language: &str) -> &'static str {
        match language {
            "rust" => "use std::collections::HashMap;",
            "typescript" => "import { Component } from 'react';",
            "python" => "import os",
            "go" => r#"import "fmt""#,
            "java" => "import java.util.List;",
            "c" | "cpp" => "#include <stdio.h>",
            "csharp" => "using System.Collections;",
            "swift" => "import Foundation",
            _ => panic!("Unsupported language: {}", language),
        }
    }

    /// Multiple imports
    pub fn multiple_imports(language: &str) -> &'static str {
        match language {
            "rust" => "use std::io;\nuse std::fs::File;",
            "typescript" => "import React from 'react';\nimport { useState } from 'react';",
            "python" => "import os\nimport sys\nimport json",
            "go" => r#"import (\n    "fmt"\n    "io"\n)"#,
            "java" => "import java.util.List;\nimport java.util.Map;",
            "c" => "#include <stdio.h>\n#include <stdlib.h>",
            "csharp" => "using System;\nusing System.Collections;",
            "swift" => "import Foundation\nimport UIKit",
            _ => panic!("Unsupported language: {}", language),
        }
    }

    /// Qualified/namespaced imports
    pub fn qualified_imports(language: &str) -> &'static str {
        match language {
            "rust" => "use std::collections::HashMap as Map;",
            "typescript" => "import * as React from 'react';",
            "python" => "import numpy as np",
            "go" => r#"import fmt "fmt""#,
            "java" => "import static java.lang.Math.max;",
            "csharp" => "using System.Collections.Generic;",
            "swift" => "import class Foundation.NSObject",
            _ => panic!("Unsupported language: {}", language),
        }
    }
}

/// Test fixtures for symbol definitions
pub mod symbol_fixtures {
    /// Function definition
    pub fn function_definition(language: &str) -> &'static str {
        match language {
            "rust" => "pub fn my_function(x: i32) -> String { x.to_string() }",
            "typescript" => "export function myFunction(x: number): string { return x.toString(); }",
            "python" => "def my_function(x):\n    return str(x)",
            "go" => "func MyFunction(x int) string { return fmt.Sprint(x) }",
            "java" => "public static String myFunction(int x) { return String.valueOf(x); }",
            "c" => "char* my_function(int x) { /* impl */ }",
            "csharp" => "public static string MyFunction(int x) { return x.ToString(); }",
            "swift" => "func myFunction(x: Int) -> String { return String(x) }",
            _ => panic!("Unsupported language: {}", language),
        }
    }

    /// Class/Type definition
    pub fn class_definition(language: &str) -> &'static str {
        match language {
            "rust" => "pub struct MyStruct { field: i32 }",
            "typescript" => "export class MyClass { field: number = 0; }",
            "python" => "class MyClass:\n    field = 0",
            "go" => "type MyStruct struct { Field int }",
            "java" => "public class MyClass { private int field; }",
            "c" => "struct MyStruct { int field; };",
            "csharp" => "public class MyClass { private int field; }",
            "swift" => "class MyClass { var field: Int = 0 }",
            _ => panic!("Unsupported language: {}", language),
        }
    }

    /// Variable declaration
    pub fn variable_declaration(language: &str) -> &'static str {
        match language {
            "rust" => "let my_var: i32 = 42;",
            "typescript" => "const myVar: number = 42;",
            "python" => "my_var = 42",
            "go" => "var myVar int = 42",
            "java" => "int myVar = 42;",
            "c" => "int my_var = 42;",
            "csharp" => "int myVar = 42;",
            "swift" => "var myVar: Int = 42",
            _ => panic!("Unsupported language: {}", language),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unicode_identifiers() {
        let content = edge_cases::unicode_identifiers();
        assert!(content.contains("变量名"));
        assert!(content.contains("函数"));
    }

    #[test]
    fn test_extremely_long_line() {
        let content = edge_cases::extremely_long_line();
        assert!(content.len() > 15000);
        assert!(content.starts_with("let x = "));
    }

    #[test]
    fn test_mixed_line_endings() {
        let content = edge_cases::mixed_line_endings();
        assert!(content.contains("\r\n"));
        assert!(content.contains("\n"));
    }

    #[test]
    fn test_special_regex_chars() {
        let content = edge_cases::special_regex_chars();
        assert!(content.contains(".*+?"));
        assert!(content.contains("{}"));
    }

    #[test]
    fn test_large_file_rust() {
        let content = large_file_template("rust", 100);
        assert!(content.contains("fn function_0"));
        assert!(content.contains("fn function_99"));
    }

    #[test]
    fn test_large_file_typescript() {
        let content = large_file_template("typescript", 50);
        assert!(content.contains("function function0"));
        assert!(content.contains("function function49"));
    }

    #[test]
    fn test_large_file_python() {
        let content = large_file_template("python", 50);
        assert!(content.contains("def function_0"));
        assert!(content.contains("def function_49"));
    }

    #[test]
    fn test_import_fixture_rust() {
        let single = import_fixtures::single_import("rust");
        assert!(single.contains("use std"));
    }

    #[test]
    fn test_symbol_fixture_function() {
        let func = symbol_fixtures::function_definition("rust");
        assert!(func.contains("fn my_function"));
    }
}
