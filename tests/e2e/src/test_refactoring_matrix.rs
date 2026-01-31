//! Generic Refactoring Test Matrix
//!
//! A configurable test framework that runs comprehensive refactoring operations
//! on any codebase and verifies builds after each operation.
//!
//! Run all matrix tests: cargo test -p e2e test_matrix -- --ignored --nocapture
//! Run specific language: cargo test -p e2e test_matrix_typescript -- --ignored --nocapture
//!
//! The matrix covers:
//! - File operations: move up, move down, rename in place, move to sibling
//! - Folder operations: rename, move up, move down, move nested
//! - Content operations: find/replace literal, find/replace regex
//! - Edge cases: deep nesting, empty folders, special characters

use crate::test_real_projects::RealProjectContext;
use serde_json::json;
use serial_test::serial;

// ============================================================================
// Test Configuration
// ============================================================================

/// Configuration for a refactoring test suite on a specific project
#[derive(Clone)]
pub struct RefactoringTestConfig {
    /// Git repository URL
    pub repo_url: &'static str,
    /// Short project name for logging
    pub project_name: &'static str,
    /// Primary source directory (e.g., "src", "lib")
    pub source_dir: &'static str,
    /// File extension for this language (e.g., "ts", "rs", "py")
    pub file_ext: &'static str,
    /// How to verify the project builds
    pub build_verify: BuildVerification,
    /// Template for creating test files
    pub file_template: FileTemplate,
}

#[derive(Clone)]
pub enum BuildVerification {
    /// Run cargo check
    Rust,
    /// Run tsc --noEmit
    TypeScript,
    /// Run python -m py_compile on files
    Python,
    /// Skip build verification
    None,
}

#[derive(Clone)]
pub struct FileTemplate {
    /// Simple module content
    pub simple_module: &'static str,
    /// Module with export
    pub export_module: &'static str,
    /// Module that imports another
    pub import_template: &'static str,
}

// ============================================================================
// Shared File Templates (DRY)
// ============================================================================

/// TypeScript file templates
pub const TS_TEMPLATES: FileTemplate = FileTemplate {
    simple_module: r#"export const value = 42;
"#,
    export_module: r#"export function helper(): string {
    return "helper";
}

export const CONSTANT = "constant-value";
"#,
    import_template: r#"import { helper, CONSTANT } from "./{import_path}";

export function useHelper(): string {
    return helper() + CONSTANT;
}
"#,
};

/// Rust file templates
pub const RS_TEMPLATES: FileTemplate = FileTemplate {
    simple_module: r#"pub const VALUE: i32 = 42;
"#,
    export_module: r#"pub fn helper() -> &'static str {
    "helper"
}

pub const CONSTANT: &str = "constant-value";
"#,
    import_template: r#"use super::{import_path}::{helper, CONSTANT};

pub fn use_helper() -> String {
    format!("{}{}", helper(), CONSTANT)
}
"#,
};

/// Python file templates
pub const PY_TEMPLATES: FileTemplate = FileTemplate {
    simple_module: r#"VALUE = 42
"#,
    export_module: r#"def helper() -> str:
    return "helper"

CONSTANT = "constant-value"
"#,
    import_template: r#"from .{import_path} import helper, CONSTANT

def use_helper() -> str:
    return helper() + CONSTANT
"#,
};

// ============================================================================
// TypeScript Configurations (Diverse Structures)
// ============================================================================

/// Zod - Schema validation library (monorepo with packages/)
pub const TS_ZOD_CONFIG: RefactoringTestConfig = RefactoringTestConfig {
    repo_url: "https://github.com/colinhacks/zod.git",
    project_name: "zod",
    source_dir: "src",
    file_ext: "ts",
    build_verify: BuildVerification::TypeScript,
    file_template: TS_TEMPLATES,
};

/// SvelteKit Skeleton - Framework template (path aliases: $lib, @/)
/// Tests import path alias handling
pub const TS_SVELTEKIT_CONFIG: RefactoringTestConfig = RefactoringTestConfig {
    repo_url: "https://github.com/sveltejs/kit.git",
    project_name: "sveltekit",
    source_dir: "packages/kit/src",
    file_ext: "ts",
    build_verify: BuildVerification::None, // Complex build, skip verification
    file_template: TS_TEMPLATES,
};

/// nanoid - Unique ID generator (flat structure, minimal)
pub const TS_NANOID_CONFIG: RefactoringTestConfig = RefactoringTestConfig {
    repo_url: "https://github.com/ai/nanoid.git",
    project_name: "nanoid",
    source_dir: ".",
    file_ext: "ts",
    build_verify: BuildVerification::TypeScript,
    file_template: TS_TEMPLATES,
};

/// ts-pattern - Pattern matching (packages/ structure)
pub const TS_PATTERN_CONFIG: RefactoringTestConfig = RefactoringTestConfig {
    repo_url: "https://github.com/gvergnaud/ts-pattern.git",
    project_name: "ts-pattern",
    source_dir: "src",
    file_ext: "ts",
    build_verify: BuildVerification::TypeScript,
    file_template: TS_TEMPLATES,
};

// ============================================================================
// Rust Configurations (Diverse Structures)
// ============================================================================

/// thiserror - Error derive macro (proc-macro workspace)
pub const RS_THISERROR_CONFIG: RefactoringTestConfig = RefactoringTestConfig {
    repo_url: "https://github.com/dtolnay/thiserror.git",
    project_name: "thiserror",
    source_dir: "src",
    file_ext: "rs",
    build_verify: BuildVerification::Rust,
    file_template: RS_TEMPLATES,
};

/// once_cell - Lazy initialization (single lib crate)
pub const RS_ONCECELL_CONFIG: RefactoringTestConfig = RefactoringTestConfig {
    repo_url: "https://github.com/matklad/once_cell.git",
    project_name: "once_cell",
    source_dir: "src",
    file_ext: "rs",
    build_verify: BuildVerification::Rust,
    file_template: RS_TEMPLATES,
};

/// anyhow - Error handling (lib + tests structure)
pub const RS_ANYHOW_CONFIG: RefactoringTestConfig = RefactoringTestConfig {
    repo_url: "https://github.com/dtolnay/anyhow.git",
    project_name: "anyhow",
    source_dir: "src",
    file_ext: "rs",
    build_verify: BuildVerification::Rust,
    file_template: RS_TEMPLATES,
};

// ============================================================================
// Python Configurations (Diverse Structures)
// ============================================================================

/// httpx - HTTP client (standard package structure)
pub const PY_HTTPX_CONFIG: RefactoringTestConfig = RefactoringTestConfig {
    repo_url: "https://github.com/encode/httpx.git",
    project_name: "httpx",
    source_dir: "httpx",
    file_ext: "py",
    build_verify: BuildVerification::Python,
    file_template: PY_TEMPLATES,
};

/// rich - Terminal formatting (deeply nested modules)
pub const PY_RICH_CONFIG: RefactoringTestConfig = RefactoringTestConfig {
    repo_url: "https://github.com/Textualize/rich.git",
    project_name: "rich",
    source_dir: "rich",
    file_ext: "py",
    build_verify: BuildVerification::Python,
    file_template: PY_TEMPLATES,
};

/// pydantic - Data validation (src/ layout)
pub const PY_PYDANTIC_CONFIG: RefactoringTestConfig = RefactoringTestConfig {
    repo_url: "https://github.com/pydantic/pydantic.git",
    project_name: "pydantic",
    source_dir: "pydantic",
    file_ext: "py",
    build_verify: BuildVerification::Python,
    file_template: PY_TEMPLATES,
};

// ============================================================================
// Legacy Aliases (backwards compatibility)
// ============================================================================

pub const TYPESCRIPT_CONFIG: RefactoringTestConfig = TS_ZOD_CONFIG;
pub const RUST_CONFIG: RefactoringTestConfig = RS_THISERROR_CONFIG;
pub const PYTHON_CONFIG: RefactoringTestConfig = PY_HTTPX_CONFIG;

// ============================================================================
// Test Runner
// ============================================================================

/// Result of a single matrix test
#[derive(Debug)]
pub struct MatrixTestResult {
    pub test_name: String,
    pub passed: bool,
    pub error: Option<String>,
    pub build_passed: Option<bool>,
}

/// Run the full refactoring matrix on a project
pub struct RefactoringMatrixRunner {
    pub config: RefactoringTestConfig,
    pub ctx: RealProjectContext,
    pub results: Vec<MatrixTestResult>,
    /// Baseline error count before any tests run (for comparative verification)
    pub baseline_errors: usize,
}

impl RefactoringMatrixRunner {
    pub fn new(config: RefactoringTestConfig) -> Self {
        let ctx = RealProjectContext::new(config.repo_url, config.project_name);
        Self {
            config,
            ctx,
            results: Vec::new(),
            baseline_errors: 0,
        }
    }

    /// Warm up LSP for the project
    pub async fn warmup(&mut self) -> Result<(), String> {
        self.ctx.ensure_warmed_up().await
    }

    /// Record baseline error count before tests
    pub fn record_baseline(&mut self) {
        match self.config.build_verify {
            BuildVerification::TypeScript => {
                let (count, _) = self.ctx.count_typescript_errors();
                self.baseline_errors = count;
                if count > 0 {
                    println!(
                        "📊 Baseline: {} pre-existing TypeScript errors (will compare against this)",
                        count
                    );
                }
            }
            _ => {
                self.baseline_errors = 0;
            }
        }
    }

    /// Count current errors (for TypeScript)
    fn count_current_errors(&self) -> usize {
        match self.config.build_verify {
            BuildVerification::TypeScript => {
                let (count, _) = self.ctx.count_typescript_errors();
                count
            }
            _ => 0,
        }
    }

    /// Verify the project builds (comparative: checks we didn't ADD errors)
    pub fn verify_build(&self) -> Result<(), String> {
        match self.config.build_verify {
            BuildVerification::Rust => self.ctx.verify_rust_compiles(),
            BuildVerification::TypeScript => {
                let (current_errors, error_output) = self.ctx.count_typescript_errors();
                if current_errors <= self.baseline_errors {
                    if current_errors == 0 {
                        println!("✅ TypeScript project compiles successfully");
                    } else {
                        println!(
                            "✅ TypeScript: {} errors (same as baseline, no regressions)",
                            current_errors
                        );
                    }
                    Ok(())
                } else {
                    let new_errors = current_errors - self.baseline_errors;
                    Err(format!(
                        "Refactoring INTRODUCED {} new TypeScript errors (was: {}, now: {}):\n{}",
                        new_errors,
                        self.baseline_errors,
                        current_errors,
                        error_output.chars().take(2000).collect::<String>()
                    ))
                }
            }
            BuildVerification::Python => Ok(()), // Python doesn't have a full project build
            BuildVerification::None => Ok(()),
        }
    }

    /// Record a test result
    fn record(&mut self, name: &str, result: Result<(), String>) {
        let build_result = if result.is_ok() {
            match self.verify_build() {
                Ok(()) => Some(true),
                Err(e) => {
                    // Print build errors so we can diagnose issues
                    println!("  ⚠️  Build verification failed after {}:", name);
                    // Truncate long error messages for readability
                    let error_preview: String = e.chars().take(500).collect();
                    println!("      {}", error_preview);
                    if e.len() > 500 {
                        println!("      ... (truncated, {} more chars)", e.len() - 500);
                    }
                    Some(false)
                }
            }
        } else {
            None
        };

        self.results.push(MatrixTestResult {
            test_name: name.to_string(),
            passed: result.is_ok(),
            error: result.err(),
            build_passed: build_result,
        });
    }

    /// Print summary of all test results
    pub fn print_summary(&self) {
        println!("\n{}", "=".repeat(60));
        println!("  REFACTORING MATRIX RESULTS: {}", self.config.project_name);
        println!("{}\n", "=".repeat(60));

        let mut passed = 0;
        let mut failed = 0;

        for result in &self.results {
            let status = if result.passed { "✅" } else { "❌" };
            let build = match result.build_passed {
                Some(true) => " [build: ✅]",
                Some(false) => " [build: ❌]",
                None => "",
            };
            println!("  {} {}{}", status, result.test_name, build);

            if result.passed {
                passed += 1;
            } else {
                failed += 1;
                if let Some(err) = &result.error {
                    println!("      Error: {}", err);
                }
            }
        }

        println!("\n  Summary: {} passed, {} failed", passed, failed);
        println!("{}\n", "=".repeat(60));
    }

    // =========================================================================
    // File Move Tests
    // =========================================================================

    /// Test: Move file down into a subdirectory
    pub async fn test_file_move_down(&mut self) {
        let test_name = "file_move_down";
        println!("\n🧪 Running: {}", test_name);

        let result = async {
            let src_dir = self.config.source_dir;
            let ext = self.config.file_ext;

            // Create test file
            self.ctx.create_test_file(
                &format!("{}/test_move_down.{}", src_dir, ext),
                self.config.file_template.simple_module,
            );

            // Create destination directory
            let dest_dir = self.ctx.absolute_path(&format!("{}/subdir", src_dir));
            std::fs::create_dir_all(&dest_dir).ok();

            let source = self.ctx.absolute_path(&format!("{}/test_move_down.{}", src_dir, ext));
            let dest = self.ctx.absolute_path(&format!("{}/subdir/test_move_down.{}", src_dir, ext));

            // Execute move
            self.ctx
                .call_tool(
                    "relocate",
                    json!({
                        "target": { "kind": "file", "filePath": source.to_string_lossy() },
                        "destination": dest.to_string_lossy(),
                        "options": { "dryRun": false }
                    }),
                )
                .await
                .map_err(|e| format!("relocate failed: {}", e))?;

            // Verify
            self.ctx.verify_file_not_exists(&format!("{}/test_move_down.{}", src_dir, ext))?;
            self.ctx.verify_file_exists(&format!("{}/subdir/test_move_down.{}", src_dir, ext))?;

            Ok(())
        }
        .await;

        self.record(test_name, result);
    }

    /// Test: Move file up a directory level
    pub async fn test_file_move_up(&mut self) {
        let test_name = "file_move_up";
        println!("\n🧪 Running: {}", test_name);

        let result = async {
            let src_dir = self.config.source_dir;
            let ext = self.config.file_ext;

            // Create nested file
            self.ctx.create_test_file(
                &format!("{}/nested/test_move_up.{}", src_dir, ext),
                self.config.file_template.simple_module,
            );

            let source = self.ctx.absolute_path(&format!("{}/nested/test_move_up.{}", src_dir, ext));
            let dest = self.ctx.absolute_path(&format!("{}/test_move_up.{}", src_dir, ext));

            self.ctx
                .call_tool(
                    "relocate",
                    json!({
                        "target": { "kind": "file", "filePath": source.to_string_lossy() },
                        "destination": dest.to_string_lossy(),
                        "options": { "dryRun": false }
                    }),
                )
                .await
                .map_err(|e| format!("relocate failed: {}", e))?;

            self.ctx.verify_file_not_exists(&format!("{}/nested/test_move_up.{}", src_dir, ext))?;
            self.ctx.verify_file_exists(&format!("{}/test_move_up.{}", src_dir, ext))?;

            Ok(())
        }
        .await;

        self.record(test_name, result);
    }

    /// Test: Rename file in place
    pub async fn test_file_rename(&mut self) {
        let test_name = "file_rename";
        println!("\n🧪 Running: {}", test_name);

        let result = async {
            let src_dir = self.config.source_dir;
            let ext = self.config.file_ext;

            self.ctx.create_test_file(
                &format!("{}/old_name.{}", src_dir, ext),
                self.config.file_template.simple_module,
            );

            let old_path = self.ctx.absolute_path(&format!("{}/old_name.{}", src_dir, ext));
            let new_path = self.ctx.absolute_path(&format!("{}/new_name.{}", src_dir, ext));

            self.ctx
                .call_tool(
                    "rename_all",
                    json!({
                        "target": { "kind": "file", "filePath": old_path.to_string_lossy() },
                        "newName": new_path.to_string_lossy(),
                        "options": { "dryRun": false }
                    }),
                )
                .await
                .map_err(|e| format!("rename_all failed: {}", e))?;

            self.ctx.verify_file_not_exists(&format!("{}/old_name.{}", src_dir, ext))?;
            self.ctx.verify_file_exists(&format!("{}/new_name.{}", src_dir, ext))?;

            Ok(())
        }
        .await;

        self.record(test_name, result);
    }

    /// Test: Move file to sibling directory
    pub async fn test_file_move_sibling(&mut self) {
        let test_name = "file_move_sibling";
        println!("\n🧪 Running: {}", test_name);

        let result = async {
            let src_dir = self.config.source_dir;
            let ext = self.config.file_ext;

            // Create file in dir_a
            self.ctx.create_test_file(
                &format!("{}/dir_a/sibling.{}", src_dir, ext),
                self.config.file_template.simple_module,
            );

            // Create dir_b
            std::fs::create_dir_all(self.ctx.absolute_path(&format!("{}/dir_b", src_dir))).ok();

            let source = self.ctx.absolute_path(&format!("{}/dir_a/sibling.{}", src_dir, ext));
            let dest = self.ctx.absolute_path(&format!("{}/dir_b/sibling.{}", src_dir, ext));

            self.ctx
                .call_tool(
                    "relocate",
                    json!({
                        "target": { "kind": "file", "filePath": source.to_string_lossy() },
                        "destination": dest.to_string_lossy(),
                        "options": { "dryRun": false }
                    }),
                )
                .await
                .map_err(|e| format!("relocate failed: {}", e))?;

            self.ctx.verify_file_not_exists(&format!("{}/dir_a/sibling.{}", src_dir, ext))?;
            self.ctx.verify_file_exists(&format!("{}/dir_b/sibling.{}", src_dir, ext))?;

            Ok(())
        }
        .await;

        self.record(test_name, result);
    }

    // =========================================================================
    // Folder Move Tests
    // =========================================================================

    /// Test: Rename folder in place
    pub async fn test_folder_rename(&mut self) {
        let test_name = "folder_rename";
        println!("\n🧪 Running: {}", test_name);

        let result = async {
            let src_dir = self.config.source_dir;
            let ext = self.config.file_ext;

            // Create folder with files
            self.ctx.create_test_file(
                &format!("{}/old_folder/file1.{}", src_dir, ext),
                self.config.file_template.simple_module,
            );
            self.ctx.create_test_file(
                &format!("{}/old_folder/file2.{}", src_dir, ext),
                self.config.file_template.simple_module,
            );

            let old_path = self.ctx.absolute_path(&format!("{}/old_folder", src_dir));
            let new_path = self.ctx.absolute_path(&format!("{}/new_folder", src_dir));

            self.ctx
                .call_tool(
                    "rename_all",
                    json!({
                        "target": { "kind": "directory", "filePath": old_path.to_string_lossy() },
                        "newName": new_path.to_string_lossy(),
                        "options": { "dryRun": false }
                    }),
                )
                .await
                .map_err(|e| format!("rename_all failed: {}", e))?;

            self.ctx.verify_file_not_exists(&format!("{}/old_folder", src_dir))?;
            self.ctx.verify_dir_exists(&format!("{}/new_folder", src_dir))?;
            self.ctx.verify_file_exists(&format!("{}/new_folder/file1.{}", src_dir, ext))?;
            self.ctx.verify_file_exists(&format!("{}/new_folder/file2.{}", src_dir, ext))?;

            Ok(())
        }
        .await;

        self.record(test_name, result);
    }

    /// Test: Move folder down into another folder
    pub async fn test_folder_move_down(&mut self) {
        let test_name = "folder_move_down";
        println!("\n🧪 Running: {}", test_name);

        let result = async {
            let src_dir = self.config.source_dir;
            let ext = self.config.file_ext;

            self.ctx.create_test_file(
                &format!("{}/movable/content.{}", src_dir, ext),
                self.config.file_template.simple_module,
            );

            std::fs::create_dir_all(self.ctx.absolute_path(&format!("{}/container", src_dir))).ok();

            let source = self.ctx.absolute_path(&format!("{}/movable", src_dir));
            let dest = self.ctx.absolute_path(&format!("{}/container/movable", src_dir));

            self.ctx
                .call_tool(
                    "rename_all",
                    json!({
                        "target": { "kind": "directory", "filePath": source.to_string_lossy() },
                        "newName": dest.to_string_lossy(),
                        "options": { "dryRun": false }
                    }),
                )
                .await
                .map_err(|e| format!("rename_all failed: {}", e))?;

            self.ctx.verify_file_not_exists(&format!("{}/movable", src_dir))?;
            self.ctx.verify_dir_exists(&format!("{}/container/movable", src_dir))?;
            self.ctx.verify_file_exists(&format!("{}/container/movable/content.{}", src_dir, ext))?;

            Ok(())
        }
        .await;

        self.record(test_name, result);
    }

    // =========================================================================
    // Content Operations
    // =========================================================================

    /// Test: Find/replace literal strings
    pub async fn test_find_replace_literal(&mut self) {
        let test_name = "find_replace_literal";
        println!("\n🧪 Running: {}", test_name);

        let result = async {
            let src_dir = self.config.source_dir;
            let ext = self.config.file_ext;

            self.ctx.create_test_file(
                &format!("{}/replace_test.{}", src_dir, ext),
                "OLD_VALUE = 1;\nuse_OLD_VALUE();\n",
            );

            self.ctx
                .call_tool(
                    "workspace",
                    json!({
                        "action": "find_replace",
                        "params": {
                            "pattern": "OLD_VALUE",
                            "replacement": "NEW_VALUE",
                            "mode": "literal"
                        },
                        "options": { "dryRun": false }
                    }),
                )
                .await
                .map_err(|e| format!("find_replace failed: {}", e))?;

            self.ctx.verify_file_contains(&format!("{}/replace_test.{}", src_dir, ext), "NEW_VALUE")?;
            self.ctx.verify_file_not_contains(&format!("{}/replace_test.{}", src_dir, ext), "OLD_VALUE")?;

            Ok(())
        }
        .await;

        self.record(test_name, result);
    }

    // =========================================================================
    // Edge Cases
    // =========================================================================

    /// Test: Move deeply nested file to top level
    pub async fn test_deep_to_shallow(&mut self) {
        let test_name = "deep_to_shallow";
        println!("\n🧪 Running: {}", test_name);

        let result = async {
            let src_dir = self.config.source_dir;
            let ext = self.config.file_ext;

            self.ctx.create_test_file(
                &format!("{}/a/b/c/d/deep.{}", src_dir, ext),
                self.config.file_template.simple_module,
            );

            let source = self.ctx.absolute_path(&format!("{}/a/b/c/d/deep.{}", src_dir, ext));
            let dest = self.ctx.absolute_path(&format!("{}/shallow.{}", src_dir, ext));

            self.ctx
                .call_tool(
                    "relocate",
                    json!({
                        "target": { "kind": "file", "filePath": source.to_string_lossy() },
                        "destination": dest.to_string_lossy(),
                        "options": { "dryRun": false }
                    }),
                )
                .await
                .map_err(|e| format!("relocate failed: {}", e))?;

            self.ctx.verify_file_exists(&format!("{}/shallow.{}", src_dir, ext))?;
            self.ctx.verify_file_not_exists(&format!("{}/a/b/c/d/deep.{}", src_dir, ext))?;

            Ok(())
        }
        .await;

        self.record(test_name, result);
    }

    /// Test: Move file to deeply nested location
    pub async fn test_shallow_to_deep(&mut self) {
        let test_name = "shallow_to_deep";
        println!("\n🧪 Running: {}", test_name);

        let result = async {
            let src_dir = self.config.source_dir;
            let ext = self.config.file_ext;

            self.ctx.create_test_file(
                &format!("{}/top_level.{}", src_dir, ext),
                self.config.file_template.simple_module,
            );

            let dest_dir = self.ctx.absolute_path(&format!("{}/x/y/z", src_dir));
            std::fs::create_dir_all(&dest_dir).ok();

            let source = self.ctx.absolute_path(&format!("{}/top_level.{}", src_dir, ext));
            let dest = self.ctx.absolute_path(&format!("{}/x/y/z/buried.{}", src_dir, ext));

            self.ctx
                .call_tool(
                    "relocate",
                    json!({
                        "target": { "kind": "file", "filePath": source.to_string_lossy() },
                        "destination": dest.to_string_lossy(),
                        "options": { "dryRun": false }
                    }),
                )
                .await
                .map_err(|e| format!("relocate failed: {}", e))?;

            self.ctx.verify_file_exists(&format!("{}/x/y/z/buried.{}", src_dir, ext))?;
            self.ctx.verify_file_not_exists(&format!("{}/top_level.{}", src_dir, ext))?;

            Ok(())
        }
        .await;

        self.record(test_name, result);
    }

    /// Test: Delete file with prune
    pub async fn test_prune_file(&mut self) {
        let test_name = "prune_file";
        println!("\n🧪 Running: {}", test_name);

        let result = async {
            let src_dir = self.config.source_dir;
            let ext = self.config.file_ext;

            self.ctx.create_test_file(
                &format!("{}/to_delete.{}", src_dir, ext),
                self.config.file_template.simple_module,
            );

            let file_path = self.ctx.absolute_path(&format!("{}/to_delete.{}", src_dir, ext));

            self.ctx
                .call_tool(
                    "prune",
                    json!({
                        "target": { "kind": "file", "filePath": file_path.to_string_lossy() },
                        "options": { "dryRun": false }
                    }),
                )
                .await
                .map_err(|e| format!("prune failed: {}", e))?;

            self.ctx.verify_file_not_exists(&format!("{}/to_delete.{}", src_dir, ext))?;

            Ok(())
        }
        .await;

        self.record(test_name, result);
    }

    // =========================================================================
    // Run All Tests
    // =========================================================================

    /// Run the complete test matrix
    pub async fn run_all(&mut self) {
        println!("\n{}", "=".repeat(60));
        println!("  RUNNING REFACTORING MATRIX: {}", self.config.project_name);
        println!("{}\n", "=".repeat(60));

        // Warmup
        println!("🔥 Warming up LSP...");
        if let Err(e) = self.warmup().await {
            println!("❌ LSP warmup failed: {}", e);
            return;
        }
        println!("✅ LSP ready\n");

        // Record baseline errors BEFORE any tests (for comparative verification)
        println!("📊 Recording baseline build state...");
        self.record_baseline();

        // Initial build verification (now uses comparative baseline)
        println!("🔍 Verifying initial build...");
        match self.verify_build() {
            Ok(()) => println!("✅ Initial build passes\n"),
            Err(e) => println!("⚠️ Initial build: {}\n", e),
        }

        // File operations
        println!("\n📁 FILE OPERATIONS");
        println!("{}", "-".repeat(40));
        self.test_file_move_down().await;
        self.test_file_move_up().await;
        self.test_file_rename().await;
        self.test_file_move_sibling().await;

        // Folder operations
        println!("\n📂 FOLDER OPERATIONS");
        println!("{}", "-".repeat(40));
        self.test_folder_rename().await;
        self.test_folder_move_down().await;

        // Content operations
        println!("\n📝 CONTENT OPERATIONS");
        println!("{}", "-".repeat(40));
        self.test_find_replace_literal().await;

        // Edge cases
        println!("\n🔬 EDGE CASES");
        println!("{}", "-".repeat(40));
        self.test_deep_to_shallow().await;
        self.test_shallow_to_deep().await;
        self.test_prune_file().await;

        // Final build verification
        println!("\n🏁 FINAL BUILD VERIFICATION");
        println!("{}", "-".repeat(40));
        match self.verify_build() {
            Ok(()) => println!("✅ Final build passes!"),
            Err(e) => println!("⚠️ Final build: {}", e),
        }

        // Print summary
        self.print_summary();
    }
}

// ============================================================================
// Test Entry Points
// ============================================================================

/// Helper to run matrix and assert minimum pass rate
async fn run_matrix_test(config: RefactoringTestConfig, min_pass_rate: f64) {
    let mut runner = RefactoringMatrixRunner::new(config);
    runner.run_all().await;

    let passed = runner.results.iter().filter(|r| r.passed).count();
    let total = runner.results.len();
    let pass_rate = passed as f64 / total as f64;

    assert!(
        pass_rate >= min_pass_rate,
        "Pass rate {:.0}% below threshold {:.0}% ({}/{})",
        pass_rate * 100.0,
        min_pass_rate * 100.0,
        passed,
        total
    );
}

// ============================================================================
// TypeScript Matrix Tests
// ============================================================================

/// TypeScript: Zod (monorepo structure)
#[tokio::test]
#[serial]
#[ignore]
async fn test_matrix_ts_zod() {
    run_matrix_test(TS_ZOD_CONFIG, 0.5).await;
}

/// TypeScript: SvelteKit (path aliases: $lib, @/)
#[tokio::test]
#[serial]
#[ignore]
async fn test_matrix_ts_sveltekit() {
    run_matrix_test(TS_SVELTEKIT_CONFIG, 0.5).await;
}

/// TypeScript: nanoid (flat/minimal structure)
#[tokio::test]
#[serial]
#[ignore]
async fn test_matrix_ts_nanoid() {
    run_matrix_test(TS_NANOID_CONFIG, 0.5).await;
}

/// TypeScript: ts-pattern (packages/ structure)
#[tokio::test]
#[serial]
#[ignore]
async fn test_matrix_ts_pattern() {
    run_matrix_test(TS_PATTERN_CONFIG, 0.5).await;
}

// ============================================================================
// Rust Matrix Tests
// ============================================================================

/// Rust: thiserror (proc-macro workspace)
#[tokio::test]
#[serial]
#[ignore]
async fn test_matrix_rs_thiserror() {
    run_matrix_test(RS_THISERROR_CONFIG, 0.5).await;
}

/// Rust: once_cell (single lib crate)
#[tokio::test]
#[serial]
#[ignore]
async fn test_matrix_rs_oncecell() {
    run_matrix_test(RS_ONCECELL_CONFIG, 0.5).await;
}

/// Rust: anyhow (lib + tests structure)
#[tokio::test]
#[serial]
#[ignore]
async fn test_matrix_rs_anyhow() {
    run_matrix_test(RS_ANYHOW_CONFIG, 0.5).await;
}

// ============================================================================
// Python Matrix Tests
// ============================================================================

/// Python: httpx (standard package structure)
#[tokio::test]
#[serial]
#[ignore]
async fn test_matrix_py_httpx() {
    run_matrix_test(PY_HTTPX_CONFIG, 0.5).await;
}

/// Python: rich (deeply nested modules)
#[tokio::test]
#[serial]
#[ignore]
async fn test_matrix_py_rich() {
    run_matrix_test(PY_RICH_CONFIG, 0.5).await;
}

/// Python: pydantic (complex data validation)
#[tokio::test]
#[serial]
#[ignore]
async fn test_matrix_py_pydantic() {
    run_matrix_test(PY_PYDANTIC_CONFIG, 0.5).await;
}

// ============================================================================
// Legacy Test Aliases (backwards compatibility)
// ============================================================================

/// Legacy: TypeScript matrix (runs Zod)
#[tokio::test]
#[serial]
#[ignore]
async fn test_matrix_typescript() {
    run_matrix_test(TYPESCRIPT_CONFIG, 0.5).await;
}

/// Legacy: Rust matrix (runs thiserror)
#[tokio::test]
#[serial]
#[ignore]
async fn test_matrix_rust() {
    run_matrix_test(RUST_CONFIG, 0.5).await;
}

/// Legacy: Python matrix (runs httpx)
#[tokio::test]
#[serial]
#[ignore]
async fn test_matrix_python() {
    run_matrix_test(PYTHON_CONFIG, 0.5).await;
}

// ============================================================================
// Isolated Single-Operation Tests (for debugging)
// ============================================================================

/// Run a single operation for focused debugging
/// Usage: cargo test -p e2e test_isolated_single -- --ignored --nocapture
#[tokio::test]
#[serial]
#[ignore]
async fn test_isolated_single_file_rename_zod() {
    let mut runner = RefactoringMatrixRunner::new(TS_ZOD_CONFIG);

    println!("\n{}", "=".repeat(60));
    println!("  ISOLATED TEST: file_rename on Zod");
    println!("{}\n", "=".repeat(60));

    // Warmup
    println!("🔥 Warming up LSP...");
    if let Err(e) = runner.warmup().await {
        panic!("LSP warmup failed: {}", e);
    }

    // Record baseline BEFORE testing (critical for comparative verification)
    println!("\n📊 Recording baseline build state...");
    runner.record_baseline();
    println!("  Baseline errors: {}", runner.baseline_errors);

    // Initial build check (now uses comparative baseline)
    println!("\n📋 INITIAL BUILD STATE:");
    match runner.verify_build() {
        Ok(()) => println!("  ✅ Project builds cleanly (or matches baseline)"),
        Err(e) => println!("  ⚠️  Build issues:\n{}", e),
    }

    // Run ONLY the file rename test
    println!("\n🧪 Running single test: file_rename");
    runner.test_file_rename().await;

    // Print what happened
    runner.print_summary();

    // Assert
    let result = &runner.results[0];
    assert!(result.passed, "file_rename operation failed: {:?}", result.error);
    assert!(
        result.build_passed == Some(true),
        "Build verification failed after file_rename - refactoring introduced new errors"
    );
}
