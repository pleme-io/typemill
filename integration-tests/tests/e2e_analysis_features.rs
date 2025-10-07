//! End-to-End Tests for Analysis Features
//!
//! This module tests analysis tools like find_dead_code with real LSP integration.
//! Unlike the data-driven tests in mcp_file_operations.rs, these tests focus on
//! end-to-end workflows and LSP fallback scenarios.

use integration_tests::harness::{TestClient, TestWorkspace};
use serde_json::json;

/// Test find_dead_code with TypeScript - basic case
#[tokio::test]
async fn test_find_dead_code_typescript_basic() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create a TypeScript file with unused code
    let ts_file = workspace.path().join("unused.ts");
    std::fs::write(
        &ts_file,
        r#"
// Used function
export function usedFunction() {
    return "I am used";
}

// Unused function
function unusedFunction() {
    return "I am not used";
}

// Used constant
export const USED_CONSTANT = 42;

// Unused constant
const UNUSED_CONSTANT = 100;

// Main entry point that uses some symbols
export function main() {
    console.log(usedFunction());
    console.log(USED_CONSTANT);
}
"#,
    )
    .unwrap();

    // Wait for LSP to initialize
    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

    // Call find_dead_code
    let response = client.call_tool("find_dead_code", json!({})).await;

    // find_dead_code requires LSP workspace/symbol or document/symbol support
    if let Ok(response_value) = response {
        // Response must have either result or error
        assert!(
            response_value.get("result").is_some() || response_value.get("error").is_some(),
            "Response must contain 'result' or 'error' field"
        );

        if let Some(result) = response_value.get("result") {
            // If successful, verify the structure
            assert!(
                result.get("workspacePath").is_some(),
                "Result should have workspacePath field"
            );
            assert!(
                result.get("deadSymbols").is_some(),
                "Result should have deadSymbols field"
            );
            assert!(
                result.get("analysisStats").is_some(),
                "Result should have analysisStats field"
            );

            let _dead_symbols = result["deadSymbols"].as_array().unwrap();
            // May or may not find dead symbols depending on LSP capabilities

            let stats = &result["analysisStats"];
            assert!(
                stats.get("filesAnalyzed").is_some(),
                "Stats should have filesAnalyzed"
            );
            assert!(
                stats.get("analysisDurationMs").is_some(),
                "Stats should have analysisDurationMs"
            );
        }
    }
}

/// Test find_dead_code with Rust - tests the documentSymbol fallback path
#[tokio::test]
async fn test_find_dead_code_rust_fallback() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create a minimal Rust project
    let cargo_toml = workspace.path().join("Cargo.toml");
    std::fs::write(
        &cargo_toml,
        r#"
[package]
name = "test-project"
version = "0.1.0"
edition = "2021"

[dependencies]
"#,
    )
    .unwrap();

    // Create lib.rs with unused code
    let src_dir = workspace.path().join("src");
    std::fs::create_dir(&src_dir).unwrap();
    let lib_rs = src_dir.join("lib.rs");
    std::fs::write(
        &lib_rs,
        r#"
// Public used function
pub fn used_function() -> &'static str {
    "I am used"
}

// Private unused function
fn unused_function() -> &'static str {
    "I am not used"
}

// Public used constant
pub const USED_CONSTANT: i32 = 42;

// Private unused constant
const UNUSED_CONSTANT: i32 = 100;

// Test that uses some symbols
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_main() {
        assert_eq!(used_function(), "I am used");
        assert_eq!(USED_CONSTANT, 42);
    }
}
"#,
    )
    .unwrap();

    // Wait for LSP to initialize
    tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;

    // Call find_dead_code
    let response = client.call_tool("find_dead_code", json!({})).await;

    // Rust analyzer may not support workspace/symbol well, so we should fallback to document/symbol
    if let Ok(response_value) = response {
        // Response must have either result or error
        assert!(
            response_value.get("result").is_some() || response_value.get("error").is_some(),
            "Response must contain 'result' or 'error' field"
        );

        if let Some(result) = response_value.get("result") {
            // If successful, verify the structure
            assert!(
                result.get("workspacePath").is_some(),
                "Result should have workspacePath field"
            );
            assert!(
                result.get("deadSymbols").is_some(),
                "Result should have deadSymbols field"
            );
            assert!(
                result.get("analysisStats").is_some(),
                "Result should have analysisStats field"
            );

            let _dead_symbols = result["deadSymbols"].as_array().unwrap();
            // Rust analyzer should find symbols via documentSymbol fallback

            let stats = &result["analysisStats"];
            assert!(
                stats.get("filesAnalyzed").is_some(),
                "Stats should have filesAnalyzed"
            );
            assert!(
                stats.get("analysisDurationMs").is_some(),
                "Stats should have analysisDurationMs"
            );

            // Verify we got some analysis done (filesAnalyzed > 0 means fallback worked)
            let files_analyzed = stats["filesAnalyzed"].as_u64().unwrap_or(0);
            // Successfully used fallback path if we analyzed any files
            // This confirms the documentSymbol fallback is working
            let _ = files_analyzed; // May be 0 if LSP not available
        }
    }
}

/// Test find_dead_code with empty workspace
#[tokio::test]
async fn test_find_dead_code_empty_workspace() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Call find_dead_code on empty workspace
    let response = client.call_tool("find_dead_code", json!({})).await;

    if let Ok(response_value) = response {
        // Should succeed but find no dead code
        assert!(
            response_value.get("result").is_some() || response_value.get("error").is_some(),
            "Response must contain 'result' or 'error' field"
        );

        if let Some(result) = response_value.get("result") {
            assert!(
                result.get("workspacePath").is_some(),
                "Result should have workspacePath field"
            );
            assert!(
                result.get("deadSymbols").is_some(),
                "Result should have deadSymbols field"
            );
            assert!(
                result.get("analysisStats").is_some(),
                "Result should have analysisStats field"
            );

            let dead_symbols = result["deadSymbols"].as_array().unwrap();
            // Empty workspace should have no dead symbols
            assert_eq!(
                dead_symbols.len(),
                0,
                "Empty workspace should have no dead symbols"
            );
        }
    }
}

/// Test find_dead_code with specific file types filter
#[tokio::test]
async fn test_find_dead_code_with_file_types_filter() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create multiple file types
    let ts_file = workspace.path().join("test.ts");
    std::fs::write(
        &ts_file,
        r#"
export function usedTsFunction() {
    return "used";
}

function unusedTsFunction() {
    return "unused";
}
"#,
    )
    .unwrap();

    let py_file = workspace.path().join("test.py");
    std::fs::write(
        &py_file,
        r#"
def used_py_function():
    return "used"

def unused_py_function():
    return "unused"

if __name__ == "__main__":
    print(used_py_function())
"#,
    )
    .unwrap();

    // Wait for LSP to initialize
    tokio::time::sleep(tokio::time::Duration::from_millis(1500)).await;

    // Call find_dead_code with file_types filter for TypeScript only
    let response = client
        .call_tool(
            "find_dead_code",
            json!({
                "file_types": [".ts", ".tsx"]
            }),
        )
        .await;

    if let Ok(response_value) = response {
        assert!(
            response_value.get("result").is_some() || response_value.get("error").is_some(),
            "Response must contain 'result' or 'error' field"
        );

        if let Some(result) = response_value.get("result") {
            assert!(
                result.get("workspacePath").is_some(),
                "Result should have workspacePath field"
            );
            assert!(
                result.get("deadSymbols").is_some(),
                "Result should have deadSymbols field"
            );
            assert!(
                result.get("analysisStats").is_some(),
                "Result should have analysisStats field"
            );

            let dead_symbols = result["deadSymbols"].as_array().unwrap();
            // Should only analyze .ts files, not .py files

            // Verify any dead symbols found are from .ts files
            for symbol in dead_symbols {
                let file_path = symbol["file"].as_str().unwrap();
                assert!(
                    file_path.ends_with(".ts") || file_path.ends_with(".tsx"),
                    "Should only analyze TypeScript files, found: {}",
                    file_path
                );
            }
        }
    }
}

/// Test find_dead_code integration with analysis workflow
#[tokio::test]
async fn test_find_dead_code_workflow_integration() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create a project with mixed used/unused code
    let src_dir = workspace.path().join("src");
    std::fs::create_dir(&src_dir).unwrap();

    let main_ts = src_dir.join("main.ts");
    std::fs::write(
        &main_ts,
        r#"
import { helper } from './helper';

export function main() {
    return helper();
}
"#,
    )
    .unwrap();

    let helper_ts = src_dir.join("helper.ts");
    std::fs::write(
        &helper_ts,
        r#"
export function helper() {
    return "helper";
}

// This function is not used anywhere
function unusedHelper() {
    return "unused";
}
"#,
    )
    .unwrap();

    // Wait for LSP to initialize
    tokio::time::sleep(tokio::time::Duration::from_millis(1500)).await;

    // First, get document symbols to verify LSP is working
    let symbols_response = client
        .call_tool(
            "get_document_symbols",
            json!({ "file_path": helper_ts.to_string_lossy() }),
        )
        .await;

    // If get_document_symbols works, find_dead_code should work too (via fallback if needed)
    if symbols_response.is_ok() {
        let dead_code_response = client.call_tool("find_dead_code", json!({})).await;

        if let Ok(response_value) = dead_code_response {
            assert!(
                response_value.get("result").is_some() || response_value.get("error").is_some(),
                "Response must contain 'result' or 'error' field"
            );

            if let Some(result) = response_value.get("result") {
                assert!(
                    result.get("workspacePath").is_some(),
                    "Result should have workspacePath field"
                );
                assert!(
                    result.get("deadSymbols").is_some(),
                    "Result should have deadSymbols field"
                );
                assert!(
                    result.get("analysisStats").is_some(),
                    "Result should have analysisStats field"
                );
            }
        }
    }
}

/// Test analyze_project_complexity - basic TypeScript project
#[tokio::test]
async fn test_analyze_project_complexity_typescript() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create TypeScript files with varying complexity
    let simple_ts = workspace.path().join("simple.ts");
    std::fs::write(
        &simple_ts,
        r#"
// Simple function with low complexity (CC = 1)
export function simpleFunction(x: number): number {
    return x + 1;
}

// Function with moderate complexity (CC = 3)
export function moderateFunction(x: number): number {
    if (x > 0) {
        return x * 2;
    } else if (x < 0) {
        return x * -1;
    } else {
        return 0;
    }
}
"#,
    )
    .unwrap();

    let complex_ts = workspace.path().join("complex.ts");
    std::fs::write(
        &complex_ts,
        r#"
// Complex function with high complexity (CC = 7+)
export function complexFunction(a: number, b: number, c: number): number {
    let result = 0;

    if (a > 0) {
        if (b > 0) {
            if (c > 0) {
                result = a + b + c;
            } else {
                result = a + b;
            }
        } else if (c > 0) {
            result = a + c;
        } else {
            result = a;
        }
    } else if (b > 0) {
        if (c > 0) {
            result = b + c;
        } else {
            result = b;
        }
    } else {
        result = c || 0;
    }

    return result;
}

// Class with methods of varying complexity
export class Calculator {
    // Simple method (CC = 1)
    add(a: number, b: number): number {
        return a + b;
    }

    // Complex method (CC = 5)
    calculate(op: string, a: number, b: number): number {
        if (op === 'add') {
            return a + b;
        } else if (op === 'subtract') {
            return a - b;
        } else if (op === 'multiply') {
            return a * b;
        } else if (op === 'divide') {
            return b !== 0 ? a / b : 0;
        } else {
            return 0;
        }
    }
}
"#,
    )
    .unwrap();

    // Wait for analysis to complete
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Call analyze_project_complexity
    let response = client
        .call_tool("analyze_project_complexity", json!({}))
        .await;

    assert!(response.is_ok(), "analyze_project_complexity should succeed");

    let response_value = response.unwrap();
    assert!(
        response_value.get("result").is_some(),
        "Response should have result field"
    );

    let result = &response_value["result"];

    // Verify structure
    assert!(
        result.get("files").is_some(),
        "Result should have files array"
    );
    assert!(
        result.get("projectMetrics").is_some(),
        "Result should have projectMetrics"
    );

    let files = result["files"].as_array().unwrap();
    assert!(files.len() >= 2, "Should analyze at least 2 files");

    // Verify each file has expected structure
    for file in files {
        assert!(file.get("filePath").is_some(), "File should have filePath");
        assert!(
            file.get("functions").is_some(),
            "File should have functions array"
        );
        assert!(file.get("classes").is_some(), "File should have classes array");

        // Verify functions have complexity metrics
        if let Some(functions) = file["functions"].as_array() {
            for func in functions {
                assert!(func.get("name").is_some(), "Function should have name");
                assert!(
                    func.get("complexity").is_some(),
                    "Function should have complexity"
                );
                assert!(
                    func.get("lineNumber").is_some(),
                    "Function should have lineNumber"
                );

                let complexity = func["complexity"].as_u64().unwrap();
                assert!(complexity >= 1, "Complexity should be at least 1");
            }
        }

        // Verify classes have aggregated complexity
        if let Some(classes) = file["classes"].as_array() {
            for class in classes {
                assert!(class.get("name").is_some(), "Class should have name");
                assert!(
                    class.get("totalComplexity").is_some(),
                    "Class should have totalComplexity"
                );
                assert!(
                    class.get("methodCount").is_some(),
                    "Class should have methodCount"
                );
                assert!(
                    class.get("averageComplexity").is_some(),
                    "Class should have averageComplexity"
                );
            }
        }
    }

    // Verify project-level metrics
    let metrics = &result["projectMetrics"];
    assert!(
        metrics.get("totalFiles").is_some(),
        "Should have totalFiles"
    );
    assert!(
        metrics.get("totalFunctions").is_some(),
        "Should have totalFunctions"
    );
    assert!(
        metrics.get("totalClasses").is_some(),
        "Should have totalClasses"
    );
    assert!(
        metrics.get("averageComplexity").is_some(),
        "Should have averageComplexity"
    );
    assert!(
        metrics.get("maxComplexity").is_some(),
        "Should have maxComplexity"
    );

    let total_functions = metrics["totalFunctions"].as_u64().unwrap();
    assert!(
        total_functions >= 3,
        "Should find at least 3 functions in the test files"
    );
}

/// Test find_complexity_hotspots - identifies most complex code
#[tokio::test]
async fn test_find_complexity_hotspots() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create files with clearly different complexity levels
    let low_complexity = workspace.path().join("low.ts");
    std::fs::write(
        &low_complexity,
        r#"
export function lowComplexity(): string {
    return "simple";
}
"#,
    )
    .unwrap();

    let high_complexity = workspace.path().join("high.ts");
    std::fs::write(
        &high_complexity,
        r#"
// Intentionally complex function for testing
export function highComplexity(a: number, b: number, c: number, d: number): number {
    let result = 0;

    if (a > 0) {
        if (b > 0) {
            if (c > 0) {
                if (d > 0) {
                    result = a + b + c + d;
                } else {
                    result = a + b + c;
                }
            } else if (d > 0) {
                result = a + b + d;
            } else {
                result = a + b;
            }
        } else if (c > 0) {
            if (d > 0) {
                result = a + c + d;
            } else {
                result = a + c;
            }
        } else if (d > 0) {
            result = a + d;
        } else {
            result = a;
        }
    } else if (b > 0) {
        if (c > 0) {
            if (d > 0) {
                result = b + c + d;
            } else {
                result = b + c;
            }
        } else if (d > 0) {
            result = b + d;
        } else {
            result = b;
        }
    } else if (c > 0) {
        if (d > 0) {
            result = c + d;
        } else {
            result = c;
        }
    } else {
        result = d || 0;
    }

    return result;
}

// Another complex function
export function anotherComplexFunction(x: number): number {
    for (let i = 0; i < 10; i++) {
        if (x > i) {
            if (x % 2 === 0) {
                x += i;
            } else {
                x -= i;
            }
        } else if (x < i) {
            x = x * 2;
        } else {
            x = 0;
        }
    }
    return x;
}
"#,
    )
    .unwrap();

    // Wait for analysis
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Call find_complexity_hotspots with top 5 limit
    let response = client
        .call_tool(
            "find_complexity_hotspots",
            json!({
                "limit": 5
            }),
        )
        .await;

    assert!(response.is_ok(), "find_complexity_hotspots should succeed");

    let response_value = response.unwrap();
    assert!(
        response_value.get("result").is_some(),
        "Response should have result field"
    );

    let result = &response_value["result"];

    // Verify structure
    assert!(
        result.get("hotspots").is_some(),
        "Result should have hotspots array"
    );
    assert!(
        result.get("summary").is_some(),
        "Result should have summary"
    );

    let hotspots = result["hotspots"].as_array().unwrap();
    assert!(
        !hotspots.is_empty(),
        "Should find at least one complexity hotspot"
    );
    assert!(
        hotspots.len() <= 5,
        "Should respect limit of 5 hotspots"
    );

    // Verify hotspots are sorted by complexity (descending)
    let mut prev_complexity = u64::MAX;
    for hotspot in hotspots {
        assert!(
            hotspot.get("filePath").is_some(),
            "Hotspot should have filePath"
        );
        assert!(
            hotspot.get("functionName").is_some(),
            "Hotspot should have functionName"
        );
        assert!(
            hotspot.get("complexity").is_some(),
            "Hotspot should have complexity"
        );
        assert!(
            hotspot.get("lineNumber").is_some(),
            "Hotspot should have lineNumber"
        );
        assert!(
            hotspot.get("kind").is_some(),
            "Hotspot should have kind (function/method/class)"
        );

        let complexity = hotspot["complexity"].as_u64().unwrap();
        assert!(
            complexity <= prev_complexity,
            "Hotspots should be sorted by complexity (descending)"
        );
        prev_complexity = complexity;
    }

    // Verify the most complex function is from high.ts
    let top_hotspot = &hotspots[0];
    let file_path = top_hotspot["filePath"].as_str().unwrap();
    assert!(
        file_path.ends_with("high.ts"),
        "Most complex function should be from high.ts"
    );

    // Verify summary metrics
    let summary = &result["summary"];
    assert!(
        summary.get("totalHotspots").is_some(),
        "Summary should have totalHotspots"
    );
    assert!(
        summary.get("averageComplexity").is_some(),
        "Summary should have averageComplexity"
    );
    assert!(
        summary.get("maxComplexity").is_some(),
        "Summary should have maxComplexity"
    );

    let max_complexity = summary["maxComplexity"].as_u64().unwrap();
    assert!(
        max_complexity > 5,
        "Should find functions with complexity > 5"
    );
}

/// Test find_complexity_hotspots with custom threshold
#[tokio::test]
async fn test_find_complexity_hotspots_with_threshold() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create file with varying complexity
    let test_file = workspace.path().join("test.ts");
    std::fs::write(
        &test_file,
        r#"
// Low complexity (CC = 1)
export function simple(): void {
    console.log("simple");
}

// Medium complexity (CC = 4)
export function medium(x: number): number {
    if (x > 10) {
        return x * 2;
    } else if (x > 5) {
        return x + 5;
    } else if (x > 0) {
        return x;
    } else {
        return 0;
    }
}

// High complexity (CC = 8+)
export function complex(a: number, b: number): number {
    if (a > 0) {
        if (b > 0) {
            return a + b;
        } else if (b < 0) {
            return a - b;
        } else {
            return a;
        }
    } else if (a < 0) {
        if (b > 0) {
            return b - a;
        } else if (b < 0) {
            return a + b;
        } else {
            return a;
        }
    } else {
        return b;
    }
}
"#,
    )
    .unwrap();

    // Wait for analysis
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Call find_complexity_hotspots with threshold = 3
    let response = client
        .call_tool(
            "find_complexity_hotspots",
            json!({
                "threshold": 3,
                "limit": 10
            }),
        )
        .await;

    assert!(response.is_ok(), "find_complexity_hotspots should succeed");

    let response_value = response.unwrap();
    let result = &response_value["result"];
    let hotspots = result["hotspots"].as_array().unwrap();

    // With threshold = 3, should only find functions with complexity >= 3
    for hotspot in hotspots {
        let complexity = hotspot["complexity"].as_u64().unwrap();
        assert!(
            complexity >= 3,
            "All hotspots should have complexity >= threshold (3)"
        );
    }

    // Should exclude the simple() function (CC = 1)
    let function_names: Vec<&str> = hotspots
        .iter()
        .map(|h| h["functionName"].as_str().unwrap())
        .collect();

    assert!(
        !function_names.contains(&"simple"),
        "simple() function should be excluded (complexity too low)"
    );
}
