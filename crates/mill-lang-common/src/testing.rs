//! Test utilities for language plugins
//!
//! Provides helpers for creating test fixtures, mocking AST tool output,
//! and asserting on plugin results.

use std::io::Write;
use std::path::Path;
use tempfile::{NamedTempFile, TempDir};

/// Create a temporary manifest file for testing
///
/// # Example
///
/// ```rust,ignore
/// use mill_lang_common::testing::create_temp_manifest;
///
/// let file = create_temp_manifest("[package]\nname = \"test\"", "Cargo.toml").await?;
/// // Use file.path() in tests
/// ```
pub async fn create_temp_manifest(content: &str, filename: &str) -> std::io::Result<NamedTempFile> {
    let mut file = tempfile::Builder::new().suffix(filename).tempfile()?;

    file.write_all(content.as_bytes())?;
    file.flush()?;

    Ok(file)
}

/// Create a temporary directory with source files
///
/// # Example
///
/// ```rust,ignore
/// use mill_lang_common::testing::create_temp_project;
///
/// let files = vec![
///     ("src/main.rs", "fn main() {}"),
///     ("Cargo.toml", "[package]\nname = \"test\""),
/// ];
///
/// let dir = create_temp_project(&files).await?;
/// // Use dir.path() to access the project root
/// ```
pub async fn create_temp_project(files: &[(&str, &str)]) -> std::io::Result<TempDir> {
    let dir = TempDir::new()?;

    for (path, content) in files {
        let file_path = dir.path().join(path);

        // Create parent directories if needed
        if let Some(parent) = file_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        tokio::fs::write(&file_path, content).await?;
    }

    Ok(dir)
}

/// Mock AST tool output as JSON
///
/// Creates a JSON string in the standard AstToolOutput format
///
/// # Example
///
/// ```rust
/// use mill_lang_common::testing::mock_ast_output;
///
/// let json = mock_ast_output(&[
///     ("foo", "function"),
///     ("Bar", "class"),
/// ]);
///
/// assert!(json.contains("\"name\": \"foo\""));
/// assert!(json.contains("\"kind\": \"function\""));
/// ```
pub fn mock_ast_output(symbols: &[(&str, &str)]) -> String {
    let symbols_json: Vec<String> = symbols
        .iter()
        .enumerate()
        .map(|(i, (name, kind))| {
            format!(
                r#"{{"name": "{}", "kind": "{}", "line": {}}}"#,
                name, kind, i
            )
        })
        .collect();

    format!(r#"{{"symbols": [{}]}}"#, symbols_json.join(","))
}

/// Mock a simple symbol array (for parsers that don't use AstToolOutput wrapper)
pub fn mock_symbol_array(symbols: &[(&str, &str)]) -> String {
    let symbols_json: Vec<String> = symbols
        .iter()
        .enumerate()
        .map(|(i, (name, kind))| {
            format!(
                r#"{{"name": "{}", "kind": "{}", "line": {}}}"#,
                name, kind, i
            )
        })
        .collect();

    format!("[{}]", symbols_json.join(","))
}

/// Assert that a PluginResult is Ok and return the value
///
/// # Example
///
/// ```rust,ignore
/// use mill_lang_common::assert_plugin_ok;
///
/// let result = plugin.parse(source).await;
/// let parsed = assert_plugin_ok!(result);
/// ```
#[macro_export]
macro_rules! assert_plugin_ok {
    ($result:expr) => {
        match $result {
            Ok(val) => val,
            Err(e) => panic!("Expected Ok, got Err: {:?}", e),
        }
    };
}

/// Assert that a PluginResult is Err
#[macro_export]
macro_rules! assert_plugin_err {
    ($result:expr) => {
        match $result {
            Ok(val) => panic!("Expected Err, got Ok: {:?}", val),
            Err(e) => e,
        }
    };
}

/// Create a simple test source file
pub fn create_test_source(language: &str) -> String {
    match language {
        "rust" => r#"
/// A test function
fn test_function() {
    println!("Hello");
}

struct TestStruct {
    field: i32,
}
"#
        .to_string(),
        "python" => r#"
def test_function():
    """A test function"""
    print("Hello")

class TestClass:
    pass
"#
        .to_string(),
        "typescript" | "javascript" => r#"
/**
 * A test function
 */
function testFunction() {
    console.log("Hello");
}

class TestClass {
}
"#
        .to_string(),
        "go" => r#"
package main

// testFunction is a test function
func testFunction() {
    fmt.Println("Hello")
}

type TestStruct struct {
    Field int
}
"#
        .to_string(),
        _ => "// Generic test source".to_string(),
    }
}

/// Assert that a path exists
pub async fn assert_path_exists(path: &Path) {
    assert!(
        tokio::fs::metadata(path).await.is_ok(),
        "Path does not exist: {}",
        path.display()
    );
}

/// Assert that a path does not exist
pub async fn assert_path_not_exists(path: &Path) {
    assert!(
        tokio::fs::metadata(path).await.is_err(),
        "Path exists but should not: {}",
        path.display()
    );
}

/// Read a file and assert it contains a substring
pub async fn assert_file_contains(path: &Path, substring: &str) {
    let content = tokio::fs::read_to_string(path)
        .await
        .unwrap_or_else(|_| panic!("Failed to read file: {}", path.display()));

    assert!(
        content.contains(substring),
        "File {} does not contain '{}'\nActual content:\n{}",
        path.display(),
        substring,
        content
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_ast_output() {
        let json = mock_ast_output(&[("foo", "function"), ("Bar", "class")]);

        assert!(json.contains("\"name\": \"foo\""));
        assert!(json.contains("\"kind\": \"function\""));
        assert!(json.contains("\"name\": \"Bar\""));
        assert!(json.contains("\"kind\": \"class\""));
        assert!(json.contains(r#""symbols":"#));
    }

    #[test]
    fn test_mock_symbol_array() {
        let json = mock_symbol_array(&[("test", "function")]);

        assert!(json.starts_with('['));
        assert!(json.ends_with(']'));
        assert!(json.contains("\"name\": \"test\""));
    }

    #[test]
    fn test_create_test_source() {
        let rust_src = create_test_source("rust");
        assert!(rust_src.contains("fn test_function"));

        let py_src = create_test_source("python");
        assert!(py_src.contains("def test_function"));

        let ts_src = create_test_source("typescript");
        assert!(ts_src.contains("function testFunction"));
    }

    #[tokio::test]
    async fn test_create_temp_manifest() {
        let file = create_temp_manifest("[package]\nname = \"test\"", "Cargo.toml")
            .await
            .unwrap();

        let content = tokio::fs::read_to_string(file.path()).await.unwrap();
        assert!(content.contains("[package]"));
        assert!(content.contains("name = \"test\""));
    }

    #[tokio::test]
    async fn test_create_temp_project() {
        let files = vec![("src/main.rs", "fn main() {}"), ("Cargo.toml", "[package]")];

        let dir = create_temp_project(&files).await.unwrap();

        assert_path_exists(&dir.path().join("src/main.rs")).await;
        assert_path_exists(&dir.path().join("Cargo.toml")).await;

        assert_file_contains(&dir.path().join("src/main.rs"), "fn main").await;
    }
}
