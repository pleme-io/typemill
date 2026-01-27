//! Unit tests for the dead code analysis crate.

use async_trait::async_trait;
use mill_analysis_common::{AnalysisError, LspProvider};
use mill_analysis_dead_code::{Config, DeadCodeAnalyzer, EntryPoints};
use serde_json::Value;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// A mock LSP provider for testing purposes.
struct MockLspProvider;

#[async_trait]
impl LspProvider for MockLspProvider {
    async fn workspace_symbols(&self, _query: &str) -> Result<Vec<Value>, AnalysisError> {
        Ok(vec![])
    }

    async fn find_references(
        &self,
        _uri: &str,
        _line: u32,
        _character: u32,
    ) -> Result<Vec<Value>, AnalysisError> {
        Ok(vec![])
    }

    async fn document_symbols(&self, _uri: &str) -> Result<Vec<Value>, AnalysisError> {
        Ok(vec![])
    }
}

#[tokio::test]
async fn test_analyzer_runs_without_error_empty_workspace() {
    // Use a temporary empty directory
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let mock_lsp = MockLspProvider;
    let config = Config::default();

    let result = DeadCodeAnalyzer::analyze(&mock_lsp, temp_dir.path(), config).await;

    assert!(result.is_ok(), "Analysis should not fail on empty directory");
    let report = result.unwrap();
    assert_eq!(
        report.dead_code.len(),
        0,
        "Should find no dead symbols in an empty workspace"
    );
}

#[tokio::test]
async fn test_analyzer_with_rust_file() {
    // Create a temporary directory with a Rust file
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let rust_file = temp_dir.path().join("lib.rs");
    std::fs::write(
        &rust_file,
        r#"
pub fn public_function() {}
fn private_function() {}
"#,
    )
    .expect("Failed to write test file");

    let mock_lsp = MockLspProvider;
    let config = Config::default();

    let result = DeadCodeAnalyzer::analyze(&mock_lsp, temp_dir.path(), config).await;

    assert!(result.is_ok(), "Analysis should not fail: {:?}", result.err());
    let report = result.unwrap();

    // With AST extraction, we should find the private function as potentially dead
    // (since public functions are entry points by default)
    assert!(
        report.stats.symbols_analyzed >= 2,
        "Should have analyzed at least 2 symbols, got {}",
        report.stats.symbols_analyzed
    );
}

#[tokio::test]
async fn test_config_defaults() {
    let config = Config::default();

    assert!(config.entry_points.include_main);
    assert!(config.entry_points.include_tests);
    assert!(config.entry_points.include_pub_exports);
    assert!(config.min_confidence > 0.0);
}

/// Integration test: analyze the circular-deps crate (a sibling crate).
/// This tests the analyzer on real Rust code with multiple files.
#[tokio::test]
async fn test_analyzer_on_real_crate_circular_deps() {
    // Point to the actual circular-deps crate
    let circular_deps_crate = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("mill-analysis-circular-deps")
        .join("src");

    if !circular_deps_crate.exists() {
        eprintln!("Skipping test: circular-deps crate not found at {:?}", circular_deps_crate);
        return;
    }

    let mock_lsp = MockLspProvider;
    let config = Config::default();

    let result = DeadCodeAnalyzer::analyze(&mock_lsp, &circular_deps_crate, config).await;

    assert!(result.is_ok(), "Analysis should succeed: {:?}", result.err());
    let report = result.unwrap();

    println!("\n=== Dead Code Analysis Report ===");
    println!("Files analyzed: {}", report.stats.files_analyzed);
    println!("Symbols analyzed: {}", report.stats.symbols_analyzed);
    println!("Dead code found: {}", report.stats.dead_found);
    println!("Duration: {}ms", report.stats.duration_ms);

    if !report.dead_code.is_empty() {
        println!("\nPotentially dead symbols:");
        for dead in &report.dead_code {
            println!(
                "  - {} {} at {}:{} (confidence: {:.2}) - {}",
                dead.kind,
                dead.name,
                dead.location.file.display(),
                dead.location.line,
                dead.confidence,
                dead.reason
            );
        }
    }

    // Should have analyzed at least a couple files (lib.rs, builder.rs)
    assert!(
        report.stats.files_analyzed >= 2,
        "Expected at least 2 files, got {}",
        report.stats.files_analyzed
    );

    // Should have found some symbols via AST
    assert!(
        report.stats.symbols_analyzed >= 5,
        "Expected at least 5 symbols, got {}",
        report.stats.symbols_analyzed
    );
}

/// Test with only pub exports as entry points (no tests, no main).
#[tokio::test]
async fn test_analyzer_with_restricted_entry_points() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let rust_file = temp_dir.path().join("lib.rs");
    std::fs::write(
        &rust_file,
        r#"
pub fn public_function() {
    private_helper();
}

fn private_helper() {}

fn completely_unused() {}

pub const PUBLIC_CONST: i32 = 42;

const PRIVATE_CONST: i32 = 100;
"#,
    )
    .expect("Failed to write test file");

    let mock_lsp = MockLspProvider;

    // Only consider public exports as entry points
    let config = Config {
        entry_points: EntryPoints {
            include_main: false,
            include_tests: false,
            include_pub_exports: true,
            custom: vec![],
        },
        min_confidence: 0.5, // Lower threshold to catch more
        file_extensions: None,
        max_symbols: None,
    };

    let result = DeadCodeAnalyzer::analyze(&mock_lsp, temp_dir.path(), config).await;

    assert!(result.is_ok(), "Analysis should not fail: {:?}", result.err());
    let report = result.unwrap();

    println!("\n=== Restricted Entry Points Test ===");
    println!("Symbols analyzed: {}", report.stats.symbols_analyzed);
    println!("Dead code found: {}", report.stats.dead_found);

    for dead in &report.dead_code {
        println!(
            "  - {} {} (confidence: {:.2})",
            dead.kind, dead.name, dead.confidence
        );
    }

    // We should find at least the completely_unused function as dead
    let dead_names: Vec<&str> = report.dead_code.iter().map(|d| d.name.as_str()).collect();

    // Note: With no LSP references, we can't determine that private_helper is used by public_function
    // So it may be marked as dead depending on confidence threshold
    // But completely_unused and PRIVATE_CONST should definitely be dead
    assert!(
        dead_names.contains(&"completely_unused") || dead_names.contains(&"PRIVATE_CONST"),
        "Expected to find completely_unused or PRIVATE_CONST as dead, got: {:?}",
        dead_names
    );
}

/// Test analyzing a single file (not a directory).
#[tokio::test]
async fn test_analyzer_single_file() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let rust_file = temp_dir.path().join("single.rs");
    std::fs::write(
        &rust_file,
        r#"
pub struct MyStruct {
    field: i32,
}

impl MyStruct {
    pub fn new() -> Self {
        Self { field: 0 }
    }
}

fn unused_in_single_file() {}
"#,
    )
    .expect("Failed to write test file");

    let mock_lsp = MockLspProvider;
    let config = Config::default();

    // Analyze just the single file
    let result = DeadCodeAnalyzer::analyze(&mock_lsp, &rust_file, config).await;

    assert!(result.is_ok(), "Analysis of single file should succeed: {:?}", result.err());
    let report = result.unwrap();

    println!("\n=== Single File Analysis ===");
    println!("Files analyzed: {}", report.stats.files_analyzed);
    println!("Symbols analyzed: {}", report.stats.symbols_analyzed);

    assert_eq!(report.stats.files_analyzed, 1, "Should analyze exactly 1 file");
    assert!(report.stats.symbols_analyzed >= 2, "Should find struct and function");
}
