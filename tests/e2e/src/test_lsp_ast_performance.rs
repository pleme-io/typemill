//! LSP vs AST Performance Comparison Tests
//!
//! This module benchmarks the performance difference between:
//! - LSP-based operations (using language servers for code intelligence)
//! - AST-based operations (using parser-based analysis without LSP)
//!
//! Run all tests: cargo test -p e2e test_lsp_ast -- --ignored --nocapture
//!
//! The tests measure:
//! - Time to complete each operation
//! - Whether the operation succeeded
//! - Build verification after changes (correctness check)
//!
//! ## Architecture Overview
//!
//! ### LSP-Based Operations:
//! - Code intelligence (inspect_code): definition, references, diagnostics
//! - Symbol search (search_code): workspace/symbol
//! - Import detection: workspace/willRenameFiles for O(1) lookup
//!
//! ### AST-Based Operations:
//! - Import graph building via language plugins
//! - Plugin-based reference scanning (O(N) file scan)
//! - Refactoring operations (extract, inline, rename, move)
//!
//! ### Hybrid Operations (can use either):
//! - File/directory rename: LSP for fast detection OR AST scanning
//! - Reference updates: LspImportFinder (O(1)) vs Plugin scanning (O(N))

use crate::test_real_projects::RealProjectContext;
use crate::test_refactoring_matrix::{
    RefactoringTestConfig, BuildVerification, FileTemplate,
    TS_TEMPLATES, RS_TEMPLATES, PY_TEMPLATES,
};
use serde_json::json;
use serial_test::serial;
use std::time::{Duration, Instant};
use std::collections::HashMap;

// ============================================================================
// Performance Test Configuration
// ============================================================================

/// Result of a performance test
#[derive(Debug, Clone)]
pub struct PerformanceResult {
    /// Test name
    pub test_name: String,
    /// Operation type: "lsp", "ast", or "hybrid"
    pub operation_type: String,
    /// Time taken for the operation
    pub duration: Duration,
    /// Whether the operation completed successfully
    pub success: bool,
    /// Error message if failed
    pub error: Option<String>,
    /// Whether the build passed after the operation
    pub build_passed: Option<bool>,
    /// Additional notes
    pub notes: String,
}

/// Performance test runner for comparing LSP vs AST
pub struct PerformanceTestRunner {
    pub config: RefactoringTestConfig,
    pub ctx: RealProjectContext,
    pub results: Vec<PerformanceResult>,
    pub baseline_errors: usize,
}

impl PerformanceTestRunner {
    pub fn new(config: RefactoringTestConfig) -> Self {
        let ctx = RealProjectContext::new(config.repo_url, config.project_name);
        Self {
            config,
            ctx,
            results: Vec::new(),
            baseline_errors: 0,
        }
    }

    /// Record baseline error count before tests
    pub fn record_baseline(&mut self) {
        match self.config.build_verify {
            BuildVerification::TypeScript => {
                let (count, _) = self.ctx.count_typescript_errors();
                self.baseline_errors = count;
                if count > 0 {
                    println!("📊 Baseline: {} pre-existing errors", count);
                }
            }
            _ => {
                self.baseline_errors = 0;
            }
        }
    }

    /// Verify the project builds (comparative)
    pub fn verify_build(&self) -> Result<(), String> {
        match self.config.build_verify {
            BuildVerification::Rust => self.ctx.verify_rust_compiles(),
            BuildVerification::TypeScript => {
                let (current_errors, error_output) = self.ctx.count_typescript_errors();
                if current_errors <= self.baseline_errors {
                    Ok(())
                } else {
                    let new_errors = current_errors - self.baseline_errors;
                    Err(format!(
                        "Introduced {} new errors (was: {}, now: {}): {}",
                        new_errors, self.baseline_errors, current_errors,
                        error_output.chars().take(1000).collect::<String>()
                    ))
                }
            }
            BuildVerification::Python => Ok(()),
            BuildVerification::None => Ok(()),
        }
    }

    /// Record a performance result
    fn record(&mut self, result: PerformanceResult) {
        println!(
            "  {} {} | {:?} | {} | {}",
            if result.success { "✅" } else { "❌" },
            result.test_name,
            result.duration,
            result.operation_type,
            result.notes
        );
        self.results.push(result);
    }

    /// Print comprehensive summary table
    pub fn print_summary(&self) {
        println!("\n{}", "=".repeat(100));
        println!("  LSP vs AST PERFORMANCE COMPARISON: {}", self.config.project_name);
        println!("{}\n", "=".repeat(100));

        // Group results by test category
        let mut lsp_results: Vec<&PerformanceResult> = Vec::new();
        let mut ast_results: Vec<&PerformanceResult> = Vec::new();
        let mut hybrid_results: Vec<&PerformanceResult> = Vec::new();

        for result in &self.results {
            match result.operation_type.as_str() {
                "lsp" => lsp_results.push(result),
                "ast" => ast_results.push(result),
                _ => hybrid_results.push(result),
            }
        }

        // Print table header
        println!("| {:<40} | {:<10} | {:<12} | {:<8} | {:<8} | {:<30} |",
            "Test Name", "Type", "Duration", "Success", "Build", "Notes");
        println!("|{}|{}|{}|{}|{}|{}|",
            "-".repeat(42), "-".repeat(12), "-".repeat(14), "-".repeat(10), "-".repeat(10), "-".repeat(32));

        for result in &self.results {
            let success_str = if result.success { "✅" } else { "❌" };
            let build_str = match result.build_passed {
                Some(true) => "✅",
                Some(false) => "❌",
                None => "N/A",
            };
            println!("| {:<40} | {:<10} | {:>12?} | {:<8} | {:<8} | {:<30} |",
                &result.test_name[..result.test_name.len().min(40)],
                result.operation_type,
                result.duration,
                success_str,
                build_str,
                &result.notes[..result.notes.len().min(30)]
            );
        }

        // Print statistics
        println!("\n{}", "-".repeat(100));
        println!("STATISTICS:");

        let total = self.results.len();
        let successful = self.results.iter().filter(|r| r.success).count();
        let builds_passed = self.results.iter().filter(|r| r.build_passed == Some(true)).count();

        println!("  Total tests: {}", total);
        println!("  Successful: {} ({:.1}%)", successful, (successful as f64 / total as f64) * 100.0);
        println!("  Builds passed: {} ({:.1}%)", builds_passed, (builds_passed as f64 / total as f64) * 100.0);

        // Compare LSP vs AST for similar operations
        if !lsp_results.is_empty() && !ast_results.is_empty() {
            let lsp_avg: Duration = lsp_results.iter().map(|r| r.duration).sum::<Duration>() / lsp_results.len() as u32;
            let ast_avg: Duration = ast_results.iter().map(|r| r.duration).sum::<Duration>() / ast_results.len() as u32;

            println!("\n  LSP average time: {:?}", lsp_avg);
            println!("  AST average time: {:?}", ast_avg);

            if lsp_avg < ast_avg {
                let speedup = ast_avg.as_millis() as f64 / lsp_avg.as_millis() as f64;
                println!("  LSP is {:.2}x faster than AST", speedup);
            } else {
                let speedup = lsp_avg.as_millis() as f64 / ast_avg.as_millis() as f64;
                println!("  AST is {:.2}x faster than LSP", speedup);
            }
        }

        println!("{}\n", "=".repeat(100));
    }

    // =========================================================================
    // LSP-Based Tests (Code Intelligence)
    // =========================================================================

    /// Test: LSP-based symbol search (workspace/symbol)
    pub async fn test_lsp_symbol_search(&mut self) {
        let test_name = "lsp_symbol_search";
        println!("\n🔬 Running: {}", test_name);

        let start = Instant::now();
        let result = self.ctx.call_tool(
            "search_code",
            json!({
                "query": "function"
            })
        ).await;
        let duration = start.elapsed();

        let (success, error, notes) = match result {
            Ok(resp) => {
                let count = resp.get("result")
                    .and_then(|r| r.as_array())
                    .map(|a| a.len())
                    .or_else(|| resp.get("result")
                        .and_then(|r| r.get("results"))
                        .and_then(|s| s.as_array())
                        .map(|a| a.len()))
                    .unwrap_or(0);
                (true, None, format!("Found {} symbols", count))
            }
            Err(e) => (false, Some(e.to_string()), "Search failed".to_string()),
        };

        self.record(PerformanceResult {
            test_name: test_name.to_string(),
            operation_type: "lsp".to_string(),
            duration,
            success,
            error,
            build_passed: None, // No build needed for read-only operations
            notes,
        });
    }

    /// Test: LSP-based definition lookup (textDocument/definition)
    pub async fn test_lsp_definition(&mut self) {
        let test_name = "lsp_definition";
        println!("\n🔬 Running: {}", test_name);

        // Find a source file to test with
        let warmup_file = self.find_source_file();
        if warmup_file.is_none() {
            self.record(PerformanceResult {
                test_name: test_name.to_string(),
                operation_type: "lsp".to_string(),
                duration: Duration::ZERO,
                success: false,
                error: Some("No source file found".to_string()),
                build_passed: None,
                notes: "Skipped".to_string(),
            });
            return;
        }
        let file_path = warmup_file.unwrap();

        let start = Instant::now();
        let result = self.ctx.call_tool(
            "inspect_code",
            json!({
                "filePath": file_path.to_string_lossy(),
                "line": 5,
                "character": 10,
                "include": ["definition"]
            })
        ).await;
        let duration = start.elapsed();

        let (success, error, notes) = match result {
            Ok(resp) => {
                let has_def = resp.get("result")
                    .and_then(|r| r.get("content"))
                    .and_then(|c| c.get("definition"))
                    .is_some();
                (true, None, if has_def { "Definition found" } else { "No definition" }.to_string())
            }
            Err(e) => (false, Some(e.to_string()), "Lookup failed".to_string()),
        };

        self.record(PerformanceResult {
            test_name: test_name.to_string(),
            operation_type: "lsp".to_string(),
            duration,
            success,
            error,
            build_passed: None,
            notes,
        });
    }

    /// Test: LSP-based references lookup
    pub async fn test_lsp_references(&mut self) {
        let test_name = "lsp_references";
        println!("\n🔬 Running: {}", test_name);

        let warmup_file = self.find_source_file();
        if warmup_file.is_none() {
            self.record(PerformanceResult {
                test_name: test_name.to_string(),
                operation_type: "lsp".to_string(),
                duration: Duration::ZERO,
                success: false,
                error: Some("No source file found".to_string()),
                build_passed: None,
                notes: "Skipped".to_string(),
            });
            return;
        }
        let file_path = warmup_file.unwrap();

        let start = Instant::now();
        let result = self.ctx.call_tool(
            "inspect_code",
            json!({
                "filePath": file_path.to_string_lossy(),
                "line": 5,
                "character": 10,
                "include": ["references"]
            })
        ).await;
        let duration = start.elapsed();

        let (success, error, notes) = match result {
            Ok(resp) => {
                let ref_count = resp.get("result")
                    .and_then(|r| r.get("content"))
                    .and_then(|c| c.get("references"))
                    .and_then(|refs| refs.get("locations"))
                    .and_then(|l| l.as_array())
                    .map(|a| a.len())
                    .unwrap_or(0);
                (true, None, format!("Found {} references", ref_count))
            }
            Err(e) => (false, Some(e.to_string()), "Lookup failed".to_string()),
        };

        self.record(PerformanceResult {
            test_name: test_name.to_string(),
            operation_type: "lsp".to_string(),
            duration,
            success,
            error,
            build_passed: None,
            notes,
        });
    }

    /// Test: LSP-based diagnostics
    pub async fn test_lsp_diagnostics(&mut self) {
        let test_name = "lsp_diagnostics";
        println!("\n🔬 Running: {}", test_name);

        let warmup_file = self.find_source_file();
        if warmup_file.is_none() {
            self.record(PerformanceResult {
                test_name: test_name.to_string(),
                operation_type: "lsp".to_string(),
                duration: Duration::ZERO,
                success: false,
                error: Some("No source file found".to_string()),
                build_passed: None,
                notes: "Skipped".to_string(),
            });
            return;
        }
        let file_path = warmup_file.unwrap();

        let start = Instant::now();
        let result = self.ctx.call_tool(
            "inspect_code",
            json!({
                "filePath": file_path.to_string_lossy(),
                "line": 1,
                "character": 0,
                "include": ["diagnostics"]
            })
        ).await;
        let duration = start.elapsed();

        let (success, error, notes) = match result {
            Ok(resp) => {
                let diag_count = resp.get("result")
                    .and_then(|r| r.get("content"))
                    .and_then(|c| c.get("diagnostics"))
                    .and_then(|d| d.as_array())
                    .map(|a| a.len())
                    .unwrap_or(0);
                (true, None, format!("Found {} diagnostics", diag_count))
            }
            Err(e) => (false, Some(e.to_string()), "Diagnostics failed".to_string()),
        };

        self.record(PerformanceResult {
            test_name: test_name.to_string(),
            operation_type: "lsp".to_string(),
            duration,
            success,
            error,
            build_passed: None,
            notes,
        });
    }

    // =========================================================================
    // AST-Based Tests (Refactoring Operations)
    // =========================================================================

    /// Test: AST-based file rename with reference updates
    pub async fn test_ast_file_rename(&mut self) {
        let test_name = "ast_file_rename";
        println!("\n🔬 Running: {}", test_name);

        let src_dir = self.config.source_dir;
        let ext = self.config.file_ext;

        // Create test file
        self.ctx.create_test_file(
            &format!("{}/perf_ast_test.{}", src_dir, ext),
            self.config.file_template.simple_module,
        );

        let old_path = self.ctx.absolute_path(&format!("{}/perf_ast_test.{}", src_dir, ext));
        let new_path = self.ctx.absolute_path(&format!("{}/perf_ast_renamed.{}", src_dir, ext));

        let start = Instant::now();
        let result = self.ctx.call_tool(
            "rename_all",
            json!({
                "target": { "kind": "file", "filePath": old_path.to_string_lossy() },
                "newName": new_path.to_string_lossy(),
                "options": { "dryRun": false }
            })
        ).await;
        let duration = start.elapsed();

        let (success, error) = match result {
            Ok(_) => (true, None),
            Err(e) => (false, Some(e.to_string())),
        };

        let build_passed = if success {
            Some(self.verify_build().is_ok())
        } else {
            None
        };

        // Cleanup
        let _ = std::fs::remove_file(&new_path);

        self.record(PerformanceResult {
            test_name: test_name.to_string(),
            operation_type: "ast".to_string(),
            duration,
            success,
            error,
            build_passed,
            notes: "File rename via AST".to_string(),
        });
    }

    /// Test: AST-based directory rename
    pub async fn test_ast_directory_rename(&mut self) {
        let test_name = "ast_directory_rename";
        println!("\n🔬 Running: {}", test_name);

        let src_dir = self.config.source_dir;
        let ext = self.config.file_ext;

        // Create test directory with files
        self.ctx.create_test_file(
            &format!("{}/perf_dir/file1.{}", src_dir, ext),
            self.config.file_template.simple_module,
        );
        self.ctx.create_test_file(
            &format!("{}/perf_dir/file2.{}", src_dir, ext),
            self.config.file_template.simple_module,
        );

        let old_path = self.ctx.absolute_path(&format!("{}/perf_dir", src_dir));
        let new_path = self.ctx.absolute_path(&format!("{}/perf_dir_renamed", src_dir));

        let start = Instant::now();
        let result = self.ctx.call_tool(
            "rename_all",
            json!({
                "target": { "kind": "directory", "filePath": old_path.to_string_lossy() },
                "newName": new_path.to_string_lossy(),
                "options": { "dryRun": false }
            })
        ).await;
        let duration = start.elapsed();

        let (success, error) = match result {
            Ok(_) => (true, None),
            Err(e) => (false, Some(e.to_string())),
        };

        let build_passed = if success {
            Some(self.verify_build().is_ok())
        } else {
            None
        };

        // Cleanup
        let _ = std::fs::remove_dir_all(&new_path);

        self.record(PerformanceResult {
            test_name: test_name.to_string(),
            operation_type: "ast".to_string(),
            duration,
            success,
            error,
            build_passed,
            notes: "Directory rename via AST".to_string(),
        });
    }

    /// Test: AST-based file move (relocate)
    pub async fn test_ast_file_move(&mut self) {
        let test_name = "ast_file_move";
        println!("\n🔬 Running: {}", test_name);

        let src_dir = self.config.source_dir;
        let ext = self.config.file_ext;

        // Create test file
        self.ctx.create_test_file(
            &format!("{}/perf_move_test.{}", src_dir, ext),
            self.config.file_template.simple_module,
        );

        // Create destination directory
        let dest_dir = self.ctx.absolute_path(&format!("{}/perf_subdir", src_dir));
        std::fs::create_dir_all(&dest_dir).ok();

        let source = self.ctx.absolute_path(&format!("{}/perf_move_test.{}", src_dir, ext));
        let dest = self.ctx.absolute_path(&format!("{}/perf_subdir/perf_move_test.{}", src_dir, ext));

        let start = Instant::now();
        let result = self.ctx.call_tool(
            "relocate",
            json!({
                "target": { "kind": "file", "filePath": source.to_string_lossy() },
                "destination": dest.to_string_lossy(),
                "options": { "dryRun": false }
            })
        ).await;
        let duration = start.elapsed();

        let (success, error) = match result {
            Ok(_) => (true, None),
            Err(e) => (false, Some(e.to_string())),
        };

        let build_passed = if success {
            Some(self.verify_build().is_ok())
        } else {
            None
        };

        // Cleanup
        let _ = std::fs::remove_dir_all(&dest_dir);

        self.record(PerformanceResult {
            test_name: test_name.to_string(),
            operation_type: "ast".to_string(),
            duration,
            success,
            error,
            build_passed,
            notes: "File move via AST".to_string(),
        });
    }

    /// Test: AST-based find/replace
    pub async fn test_ast_find_replace(&mut self) {
        let test_name = "ast_find_replace";
        println!("\n🔬 Running: {}", test_name);

        let src_dir = self.config.source_dir;
        let ext = self.config.file_ext;

        // Create test file with content to replace
        self.ctx.create_test_file(
            &format!("{}/perf_replace.{}", src_dir, ext),
            "OLD_PERF_VALUE = 1;\nuse_OLD_PERF_VALUE();\n",
        );

        let start = Instant::now();
        let result = self.ctx.call_tool(
            "workspace",
            json!({
                "action": "find_replace",
                "params": {
                    "pattern": "OLD_PERF_VALUE",
                    "replacement": "NEW_PERF_VALUE",
                    "mode": "literal"
                },
                "options": { "dryRun": false }
            })
        ).await;
        let duration = start.elapsed();

        let (success, error) = match result {
            Ok(_) => (true, None),
            Err(e) => (false, Some(e.to_string())),
        };

        // Cleanup
        let _ = std::fs::remove_file(self.ctx.absolute_path(&format!("{}/perf_replace.{}", src_dir, ext)));

        self.record(PerformanceResult {
            test_name: test_name.to_string(),
            operation_type: "ast".to_string(),
            duration,
            success,
            error,
            build_passed: None, // Skip build for find/replace test
            notes: "Find/replace via AST".to_string(),
        });
    }

    /// Test: AST-based file deletion (prune)
    pub async fn test_ast_file_delete(&mut self) {
        let test_name = "ast_file_delete";
        println!("\n🔬 Running: {}", test_name);

        let src_dir = self.config.source_dir;
        let ext = self.config.file_ext;

        // Create test file to delete
        self.ctx.create_test_file(
            &format!("{}/perf_delete.{}", src_dir, ext),
            self.config.file_template.simple_module,
        );

        let file_path = self.ctx.absolute_path(&format!("{}/perf_delete.{}", src_dir, ext));

        let start = Instant::now();
        let result = self.ctx.call_tool(
            "prune",
            json!({
                "target": { "kind": "file", "filePath": file_path.to_string_lossy() },
                "options": { "dryRun": false }
            })
        ).await;
        let duration = start.elapsed();

        let (success, error) = match result {
            Ok(_) => (true, None),
            Err(e) => (false, Some(e.to_string())),
        };

        let build_passed = if success {
            Some(self.verify_build().is_ok())
        } else {
            None
        };

        self.record(PerformanceResult {
            test_name: test_name.to_string(),
            operation_type: "ast".to_string(),
            duration,
            success,
            error,
            build_passed,
            notes: "File delete via AST".to_string(),
        });
    }

    // =========================================================================
    // Hybrid Tests (Compare LSP vs AST for same operation)
    // =========================================================================

    /// Test: File rename with import updates - comparing LSP detection vs AST scanning
    pub async fn test_hybrid_rename_with_imports(&mut self) {
        // This test creates files with actual imports and measures:
        // 1. Time for LSP to detect importing files (via workspace/willRenameFiles)
        // 2. Time for AST to scan and detect importing files (via plugin scanning)

        let src_dir = self.config.source_dir;
        let ext = self.config.file_ext;

        // Create a module with exports
        let export_content = self.config.file_template.export_module;
        self.ctx.create_test_file(
            &format!("{}/perf_export_module.{}", src_dir, ext),
            export_content,
        );

        // Create multiple files that import from it (to simulate real-world scenario)
        for i in 0..5 {
            let import_content = self.config.file_template.import_template
                .replace("{import_path}", "perf_export_module");
            self.ctx.create_test_file(
                &format!("{}/perf_importer_{}.{}", src_dir, i, ext),
                &import_content,
            );
        }

        // Test 1: Dry-run rename (measures detection time including LSP if available)
        let test_name_preview = "hybrid_rename_preview";
        println!("\n🔬 Running: {}", test_name_preview);

        let old_path = self.ctx.absolute_path(&format!("{}/perf_export_module.{}", src_dir, ext));
        let new_path = self.ctx.absolute_path(&format!("{}/perf_export_module_renamed.{}", src_dir, ext));

        let start = Instant::now();
        let result = self.ctx.call_tool(
            "rename_all",
            json!({
                "target": { "kind": "file", "filePath": old_path.to_string_lossy() },
                "newName": new_path.to_string_lossy(),
                "options": { "dryRun": true }
            })
        ).await;
        let duration = start.elapsed();

        let (success, error, notes) = match &result {
            Ok(resp) => {
                let affected = resp.get("result")
                    .and_then(|r| r.get("content"))
                    .and_then(|c| c.get("filesChanged"))
                    .and_then(|f| f.as_array())
                    .map(|a| a.len())
                    .unwrap_or(0);
                (true, None, format!("Preview: {} files affected", affected))
            }
            Err(e) => (false, Some(e.to_string()), "Preview failed".to_string()),
        };

        self.record(PerformanceResult {
            test_name: test_name_preview.to_string(),
            operation_type: "hybrid".to_string(),
            duration,
            success,
            error,
            build_passed: None,
            notes,
        });

        // Test 2: Execute rename (measures full operation time)
        let test_name_execute = "hybrid_rename_execute";
        println!("\n🔬 Running: {}", test_name_execute);

        let start = Instant::now();
        let result = self.ctx.call_tool(
            "rename_all",
            json!({
                "target": { "kind": "file", "filePath": old_path.to_string_lossy() },
                "newName": new_path.to_string_lossy(),
                "options": { "dryRun": false }
            })
        ).await;
        let duration = start.elapsed();

        let (success, error) = match result {
            Ok(_) => (true, None),
            Err(e) => (false, Some(e.to_string())),
        };

        let build_passed = if success {
            Some(self.verify_build().is_ok())
        } else {
            None
        };

        // Cleanup all test files
        let _ = std::fs::remove_file(&new_path);
        for i in 0..5 {
            let _ = std::fs::remove_file(self.ctx.absolute_path(&format!("{}/perf_importer_{}.{}", src_dir, i, ext)));
        }

        self.record(PerformanceResult {
            test_name: test_name_execute.to_string(),
            operation_type: "hybrid".to_string(),
            duration,
            success,
            error,
            build_passed,
            notes: "Rename with import updates".to_string(),
        });
    }

    // =========================================================================
    // Helper Methods
    // =========================================================================

    /// Find a source file in the project for testing
    fn find_source_file(&self) -> Option<std::path::PathBuf> {
        let extensions = ["ts", "rs", "py", "js", "tsx"];
        let search_dirs = [self.config.source_dir, "src", "lib", "."];

        for dir in search_dirs {
            let search_path = self.ctx.absolute_path(dir);
            if search_path.is_dir() {
                if let Ok(entries) = std::fs::read_dir(&search_path) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.is_file() {
                            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                                if extensions.contains(&ext) {
                                    return Some(path);
                                }
                            }
                        }
                    }
                }
            }
        }
        None
    }

    /// Run all performance tests
    pub async fn run_all(&mut self) {
        println!("\n{}", "=".repeat(100));
        println!("  LSP vs AST PERFORMANCE TESTS: {}", self.config.project_name);
        println!("{}\n", "=".repeat(100));

        // Warmup
        println!("🔥 Warming up LSP...");
        if let Err(e) = self.ctx.ensure_warmed_up().await {
            println!("⚠️ LSP warmup failed: {} (some tests may fail)", e);
        } else {
            println!("✅ LSP ready\n");
        }

        // Record baseline
        println!("📊 Recording baseline build state...");
        self.record_baseline();

        // LSP-based tests
        println!("\n📡 LSP-BASED OPERATIONS");
        println!("{}", "-".repeat(60));
        self.test_lsp_symbol_search().await;
        self.test_lsp_definition().await;
        self.test_lsp_references().await;
        self.test_lsp_diagnostics().await;

        // AST-based tests
        println!("\n🌳 AST-BASED OPERATIONS");
        println!("{}", "-".repeat(60));
        self.test_ast_file_rename().await;
        self.test_ast_directory_rename().await;
        self.test_ast_file_move().await;
        self.test_ast_find_replace().await;
        self.test_ast_file_delete().await;

        // Hybrid tests
        println!("\n🔄 HYBRID OPERATIONS (LSP+AST)");
        println!("{}", "-".repeat(60));
        self.test_hybrid_rename_with_imports().await;

        // Print summary
        self.print_summary();
    }
}

// ============================================================================
// Test Configurations for Large Repos
// ============================================================================

/// SvelteKit - Large TypeScript monorepo (stress test)
pub const TS_SVELTEKIT_CONFIG: RefactoringTestConfig = RefactoringTestConfig {
    repo_url: "https://github.com/sveltejs/kit.git",
    project_name: "sveltekit",
    source_dir: "packages/kit/src",
    file_ext: "ts",
    build_verify: BuildVerification::None,
    file_template: TS_TEMPLATES,
};

/// Ripgrep - Large Rust multi-crate workspace
pub const RS_RIPGREP_CONFIG: RefactoringTestConfig = RefactoringTestConfig {
    repo_url: "https://github.com/BurntSushi/ripgrep.git",
    project_name: "ripgrep",
    source_dir: "crates/core/src",
    file_ext: "rs",
    build_verify: BuildVerification::Rust,
    file_template: RS_TEMPLATES,
};

/// Zod - Medium TypeScript library
pub const TS_ZOD_CONFIG: RefactoringTestConfig = RefactoringTestConfig {
    repo_url: "https://github.com/colinhacks/zod.git",
    project_name: "zod",
    source_dir: "src",
    file_ext: "ts",
    build_verify: BuildVerification::TypeScript,
    file_template: TS_TEMPLATES,
};

/// Httpx - Medium Python HTTP client
pub const PY_HTTPX_CONFIG: RefactoringTestConfig = RefactoringTestConfig {
    repo_url: "https://github.com/encode/httpx.git",
    project_name: "httpx",
    source_dir: "httpx",
    file_ext: "py",
    build_verify: BuildVerification::Python,
    file_template: PY_TEMPLATES,
};

// ============================================================================
// Test Entry Points
// ============================================================================

/// TypeScript (SvelteKit) - Large monorepo performance test
#[tokio::test]
#[serial]
#[ignore]
async fn test_lsp_ast_performance_typescript_sveltekit() {
    let mut runner = PerformanceTestRunner::new(TS_SVELTEKIT_CONFIG);
    runner.run_all().await;

    // Assert minimum success rate
    let successful = runner.results.iter().filter(|r| r.success).count();
    let total = runner.results.len();
    assert!(
        successful as f64 / total as f64 >= 0.7,
        "At least 70% of tests should pass"
    );
}

/// Rust (Ripgrep) - Multi-crate workspace performance test
#[tokio::test]
#[serial]
#[ignore]
async fn test_lsp_ast_performance_rust_ripgrep() {
    let mut runner = PerformanceTestRunner::new(RS_RIPGREP_CONFIG);
    runner.run_all().await;

    let successful = runner.results.iter().filter(|r| r.success).count();
    let total = runner.results.len();
    assert!(
        successful as f64 / total as f64 >= 0.7,
        "At least 70% of tests should pass"
    );
}

/// TypeScript (Zod) - Medium library performance test
#[tokio::test]
#[serial]
#[ignore]
async fn test_lsp_ast_performance_typescript_zod() {
    let mut runner = PerformanceTestRunner::new(TS_ZOD_CONFIG);
    runner.run_all().await;

    let successful = runner.results.iter().filter(|r| r.success).count();
    let total = runner.results.len();
    assert!(
        successful as f64 / total as f64 >= 0.7,
        "At least 70% of tests should pass"
    );
}

/// Python (Httpx) - Medium library performance test
#[tokio::test]
#[serial]
#[ignore]
async fn test_lsp_ast_performance_python_httpx() {
    let mut runner = PerformanceTestRunner::new(PY_HTTPX_CONFIG);
    runner.run_all().await;

    let successful = runner.results.iter().filter(|r| r.success).count();
    let total = runner.results.len();
    assert!(
        successful as f64 / total as f64 >= 0.7,
        "At least 70% of tests should pass"
    );
}

/// Quick performance test on Zod (for CI)
#[tokio::test]
#[serial]
#[ignore]
async fn test_lsp_ast_quick_benchmark() {
    let mut runner = PerformanceTestRunner::new(TS_ZOD_CONFIG);

    println!("\n{}", "=".repeat(80));
    println!("  QUICK LSP vs AST BENCHMARK");
    println!("{}\n", "=".repeat(80));

    // Warmup
    if let Err(e) = runner.ctx.ensure_warmed_up().await {
        println!("⚠️ LSP warmup failed: {}", e);
    }

    // Run subset of tests
    runner.test_lsp_symbol_search().await;
    runner.test_ast_file_rename().await;
    runner.test_hybrid_rename_with_imports().await;

    runner.print_summary();
}
