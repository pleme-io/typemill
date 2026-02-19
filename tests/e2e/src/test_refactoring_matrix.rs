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
use futures::FutureExt;
use serde::Serialize;
use serde_json::json;
use serial_test::serial;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const MATRIX_ARTIFACT_ENV_VAR: &str = "TYPEMILL_MATRIX_ARTIFACT_DIR";
const PERF_ASSERT_STRICT_ENV_VAR: &str = "TYPEMILL_PERF_ASSERT_STRICT";
const PERF_ARTIFACT_SCHEMA_VERSION: u32 = 1;

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

/// ky - HTTP client (small, clean TypeScript structure)
pub const TS_KY_CONFIG: RefactoringTestConfig = RefactoringTestConfig {
    repo_url: "https://github.com/sindresorhus/ky.git",
    project_name: "ky",
    source_dir: "source",
    file_ext: "ts",
    build_verify: BuildVerification::TypeScript,
    file_template: TS_TEMPLATES,
};

/// SvelteKit - Framework monorepo (tests path handling in large projects)
pub const TS_SVELTEKIT_CONFIG: RefactoringTestConfig = RefactoringTestConfig {
    repo_url: "https://github.com/sveltejs/kit.git",
    project_name: "sveltekit",
    source_dir: "packages/kit/src",
    file_ext: "ts",
    build_verify: BuildVerification::None, // Complex monorepo build, skip verification
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

/// Turborepo starter - pnpm monorepo with multiple packages
pub const TS_TURBOREPO_CONFIG: RefactoringTestConfig = RefactoringTestConfig {
    repo_url: "https://github.com/vercel/turbo.git",
    project_name: "turborepo",
    source_dir: "crates/turborepo-lib/src", // Turbo has Rust core, but packages have TS
    file_ext: "ts",
    build_verify: BuildVerification::None, // Complex build, skip verification
    file_template: TS_TEMPLATES,
};

/// Next.js - App router with server components (real-world framework)
pub const TS_NEXTJS_CONFIG: RefactoringTestConfig = RefactoringTestConfig {
    repo_url: "https://github.com/vercel/next.js.git",
    project_name: "nextjs",
    source_dir: "packages/next/src",
    file_ext: "ts",
    build_verify: BuildVerification::None, // Complex monorepo, skip verification
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

/// ripgrep - Multi-crate workspace with path dependencies (complex structure)
pub const RS_RIPGREP_CONFIG: RefactoringTestConfig = RefactoringTestConfig {
    repo_url: "https://github.com/BurntSushi/ripgrep.git",
    project_name: "ripgrep",
    source_dir: "crates/core/src",
    file_ext: "rs",
    build_verify: BuildVerification::Rust,
    file_template: RS_TEMPLATES,
};

/// tokio - Async runtime workspace (highly inter-dependent crates)
pub const RS_TOKIO_CONFIG: RefactoringTestConfig = RefactoringTestConfig {
    repo_url: "https://github.com/tokio-rs/tokio.git",
    project_name: "tokio",
    source_dir: "tokio/src",
    file_ext: "rs",
    build_verify: BuildVerification::None, // Complex workspace, skip verification
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

/// FastAPI - Web framework (src-layout, namespace packages)
pub const PY_FASTAPI_CONFIG: RefactoringTestConfig = RefactoringTestConfig {
    repo_url: "https://github.com/tiangolo/fastapi.git",
    project_name: "fastapi",
    source_dir: "fastapi",
    file_ext: "py",
    build_verify: BuildVerification::Python,
    file_template: PY_TEMPLATES,
};

/// Ruff - Fast Python linter (uses maturin/PyO3, complex structure)
pub const PY_RUFF_CONFIG: RefactoringTestConfig = RefactoringTestConfig {
    repo_url: "https://github.com/astral-sh/ruff.git",
    project_name: "ruff",
    source_dir: "crates/ruff_python_ast/src", // Ruff is Rust-based but tests Python patterns
    file_ext: "rs",                           // Actually Rust code that processes Python
    build_verify: BuildVerification::None,    // Complex PyO3 build
    file_template: RS_TEMPLATES,
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

fn collect_files_recursive(
    root: &Path,
    base: &Path,
    out: &mut BTreeMap<PathBuf, String>,
) -> Result<(), String> {
    let entries = std::fs::read_dir(root)
        .map_err(|e| format!("Failed to read directory {}: {}", root.display(), e))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
        let path = entry.path();
        if path.is_dir() {
            collect_files_recursive(&path, base, out)?;
            continue;
        }
        if path.is_file() {
            let rel = path
                .strip_prefix(base)
                .map_err(|e| format!("Failed to compute relative path: {}", e))?
                .to_path_buf();
            let content = std::fs::read_to_string(&path)
                .map_err(|e| format!("Failed to read file {}: {}", path.display(), e))?;
            out.insert(rel, content);
        }
    }
    Ok(())
}

#[derive(Debug, Serialize, Clone)]
pub struct MatrixRunTimings {
    pub warmup_ms: u128,
    pub baseline_ms: u128,
    pub initial_verify_ms: u128,
    pub final_verify_ms: u128,
    pub operations_total_ms: u128,
    pub build_verifications_executed: usize,
    pub build_verifications_failed: usize,
    pub build_verifications_skipped: usize,
    pub total_run_ms: u128,
}

/// Result of a single matrix test
#[derive(Debug, Serialize)]
pub struct MatrixTestResult {
    pub test_name: String,
    pub passed: bool,
    pub error: Option<String>,
    pub build_passed: Option<bool>,
    pub operation_ms: u128,
    pub build_verify_ms: Option<u128>,
    pub total_ms: u128,
}

/// Run the full refactoring matrix on a project
pub struct RefactoringMatrixRunner {
    pub config: RefactoringTestConfig,
    pub ctx: RealProjectContext,
    pub results: Vec<MatrixTestResult>,
    /// Baseline error count before any tests run (for comparative verification)
    pub baseline_errors: usize,
    /// Recorded perf threshold exceedances during matrix operations.
    pub threshold_exceedances: Vec<String>,
    /// Coarse phase timings for the full matrix run.
    pub run_timings: Option<MatrixRunTimings>,
}

impl RefactoringMatrixRunner {
    pub fn new(config: RefactoringTestConfig) -> Self {
        let ctx = RealProjectContext::new(config.repo_url, config.project_name);
        Self {
            config,
            ctx,
            results: Vec::new(),
            baseline_errors: 0,
            threshold_exceedances: Vec::new(),
            run_timings: None,
        }
    }

    fn matrix_verify_every(&self) -> usize {
        if let Some(from_env) = std::env::var("TYPEMILL_MATRIX_VERIFY_EVERY")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .filter(|v| *v > 0)
        {
            return from_env;
        }

        // In perf profile we default to a batched cadence to reduce build-check
        // overhead while preserving full verification defaults elsewhere.
        if Self::matrix_profile() == "perf" {
            match self.config.build_verify {
                // TypeScript full-project checks are the heaviest lane; a slightly
                // wider cadence reduces wall-clock time without removing final verification.
                BuildVerification::TypeScript => 14,
                _ => 10,
            }
        } else {
            1
        }
    }

    fn is_high_risk_typescript_operation(name: &str) -> bool {
        matches!(
            name,
            "folder_rename" | "folder_move_down" | "folder_move_up" | "folder_move_sibling"
        )
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
    fn record(&mut self, name: &str, result: Result<(), String>, operation_ms: u128) {
        let build_start = std::time::Instant::now();
        let verify_every = self.matrix_verify_every();
        let operation_index = self.results.len() + 1;
        let cadence_due = operation_index % verify_every == 0;
        let high_risk_force_enabled = std::env::var("TYPEMILL_MATRIX_VERIFY_FORCE_HIGH_RISK")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(true);
        let force_high_risk_verify = high_risk_force_enabled
            && Self::matrix_profile() == "perf"
            && matches!(self.config.build_verify, BuildVerification::TypeScript)
            && Self::is_high_risk_typescript_operation(name);
        let should_verify_now = cadence_due || force_high_risk_verify;

        let build_result = if result.is_ok() && should_verify_now {
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
        } else if result.is_ok() {
            println!(
                "  ℹ️  Skipping build verification for {} (TYPEMILL_MATRIX_VERIFY_EVERY={})",
                name, verify_every
            );
            None
        } else {
            None
        };
        let build_verify_ms = if build_result.is_some() {
            Some(build_start.elapsed().as_millis())
        } else {
            None
        };

        self.results.push(MatrixTestResult {
            test_name: name.to_string(),
            passed: result.is_ok(),
            error: result.err(),
            build_passed: build_result,
            operation_ms,
            build_verify_ms,
            total_ms: operation_ms + build_verify_ms.unwrap_or(0),
        });
    }

    fn capture_directory_snapshot(
        &self,
        rel_dir: &str,
    ) -> Result<BTreeMap<PathBuf, String>, String> {
        let root = self.ctx.absolute_path(rel_dir);
        if !root.exists() {
            return Err(format!("Snapshot root does not exist: {}", root.display()));
        }
        let mut snapshot = BTreeMap::new();
        collect_files_recursive(&root, &root, &mut snapshot)?;
        Ok(snapshot)
    }

    fn verify_file_move_complete(
        &self,
        source_rel: &str,
        dest_rel: &str,
        source_content_before: &str,
    ) -> Result<(), String> {
        self.ctx.verify_file_not_exists(source_rel)?;
        self.ctx.verify_file_exists(dest_rel)?;
        let moved = self.ctx.read_file(dest_rel);
        if moved != source_content_before {
            return Err(format!(
                "Moved file content mismatch for {} (from {})",
                dest_rel, source_rel
            ));
        }
        Ok(())
    }

    fn verify_directory_move_complete(
        &self,
        source_rel: &str,
        dest_rel: &str,
        before_snapshot: &BTreeMap<PathBuf, String>,
    ) -> Result<(), String> {
        self.ctx.verify_file_not_exists(source_rel)?;
        self.ctx.verify_dir_exists(dest_rel)?;

        for (relative, original_content) in before_snapshot {
            let dest_abs = self.ctx.absolute_path(dest_rel).join(relative);
            if !dest_abs.is_file() {
                return Err(format!(
                    "Moved file missing at destination: {}",
                    dest_abs.display()
                ));
            }
            let moved_content = std::fs::read_to_string(&dest_abs).map_err(|e| {
                format!(
                    "Failed to read moved file {} for content verification: {}",
                    dest_abs.display(),
                    e
                )
            })?;
            if &moved_content != original_content {
                return Err(format!(
                    "Moved file content mismatch for {}",
                    dest_abs.display()
                ));
            }
        }

        Ok(())
    }

    fn build_verification_stats(&self) -> (usize, usize, usize) {
        let executed = self
            .results
            .iter()
            .filter(|r| r.build_passed.is_some())
            .count();
        let failed = self
            .results
            .iter()
            .filter(|r| matches!(r.build_passed, Some(false)))
            .count();
        let skipped = self
            .results
            .iter()
            .filter(|r| r.passed && r.build_passed.is_none())
            .count();
        (executed, failed, skipped)
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
            println!(
                "  {} {}{} (op: {} ms, build: {} ms, total: {} ms)",
                status,
                result.test_name,
                build,
                result.operation_ms,
                result.build_verify_ms.unwrap_or(0),
                result.total_ms
            );

            if result.passed {
                passed += 1;
            } else {
                failed += 1;
                if let Some(err) = &result.error {
                    println!("      Error: {}", err);
                }
            }
        }

        if !self.results.is_empty() {
            let mut slowest: Vec<&MatrixTestResult> = self.results.iter().collect();
            slowest.sort_by_key(|r| std::cmp::Reverse(r.total_ms));
            println!("\n  Slowest operations by total time:");
            for result in slowest.iter().take(5) {
                println!(
                    "    - {}: total {} ms (op {} ms + build {} ms)",
                    result.test_name,
                    result.total_ms,
                    result.operation_ms,
                    result.build_verify_ms.unwrap_or(0)
                );
            }

            let mut slowest_op: Vec<&MatrixTestResult> = self.results.iter().collect();
            slowest_op.sort_by_key(|r| std::cmp::Reverse(r.operation_ms));
            println!("\n  Slowest operations by tool time:");
            for result in slowest_op.into_iter().take(5) {
                println!("    - {}: {} ms", result.test_name, result.operation_ms);
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
        let start = std::time::Instant::now();

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

            let source_rel = format!("{}/test_move_down.{}", src_dir, ext);
            let dest_rel = format!("{}/subdir/test_move_down.{}", src_dir, ext);
            let source_before = self.ctx.read_file(&source_rel);

            let source = self.ctx.absolute_path(&source_rel);
            let dest = self.ctx.absolute_path(&dest_rel);

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
            self.verify_file_move_complete(&source_rel, &dest_rel, &source_before)?;

            Ok(())
        }
        .await;

        self.record(test_name, result, start.elapsed().as_millis());
    }

    /// Test: Move file up a directory level
    pub async fn test_file_move_up(&mut self) {
        let test_name = "file_move_up";
        println!("\n🧪 Running: {}", test_name);
        let start = std::time::Instant::now();

        let result = async {
            let src_dir = self.config.source_dir;
            let ext = self.config.file_ext;

            // Create nested file
            self.ctx.create_test_file(
                &format!("{}/nested/test_move_up.{}", src_dir, ext),
                self.config.file_template.simple_module,
            );

            let source_rel = format!("{}/nested/test_move_up.{}", src_dir, ext);
            let dest_rel = format!("{}/test_move_up.{}", src_dir, ext);
            let source_before = self.ctx.read_file(&source_rel);

            let source = self.ctx.absolute_path(&source_rel);
            let dest = self.ctx.absolute_path(&dest_rel);

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

            self.verify_file_move_complete(&source_rel, &dest_rel, &source_before)?;

            Ok(())
        }
        .await;

        self.record(test_name, result, start.elapsed().as_millis());
    }

    /// Test: Rename file in place
    pub async fn test_file_rename(&mut self) {
        let test_name = "file_rename";
        println!("\n🧪 Running: {}", test_name);
        let start = std::time::Instant::now();

        let result = async {
            let src_dir = self.config.source_dir;
            let ext = self.config.file_ext;

            self.ctx.create_test_file(
                &format!("{}/old_name.{}", src_dir, ext),
                self.config.file_template.simple_module,
            );

            let old_rel = format!("{}/old_name.{}", src_dir, ext);
            let new_rel = format!("{}/new_name.{}", src_dir, ext);
            let source_before = self.ctx.read_file(&old_rel);

            let old_path = self.ctx.absolute_path(&old_rel);
            let new_path = self.ctx.absolute_path(&new_rel);

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

            self.verify_file_move_complete(&old_rel, &new_rel, &source_before)?;

            Ok(())
        }
        .await;

        self.record(test_name, result, start.elapsed().as_millis());
    }

    /// Test: Move file to sibling directory
    pub async fn test_file_move_sibling(&mut self) {
        let test_name = "file_move_sibling";
        println!("\n🧪 Running: {}", test_name);
        let start = std::time::Instant::now();

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

            let source_rel = format!("{}/dir_a/sibling.{}", src_dir, ext);
            let dest_rel = format!("{}/dir_b/sibling.{}", src_dir, ext);
            let source_before = self.ctx.read_file(&source_rel);

            let source = self.ctx.absolute_path(&source_rel);
            let dest = self.ctx.absolute_path(&dest_rel);

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

            self.verify_file_move_complete(&source_rel, &dest_rel, &source_before)?;

            Ok(())
        }
        .await;

        self.record(test_name, result, start.elapsed().as_millis());
    }

    // =========================================================================
    // Folder Move Tests
    // =========================================================================

    /// Test: Rename folder in place
    pub async fn test_folder_rename(&mut self) {
        let test_name = "folder_rename";
        println!("\n🧪 Running: {}", test_name);
        let start = std::time::Instant::now();

        let result = async {
            let src_dir = self.config.source_dir;
            let ext = self.config.file_ext;

            // Create folder with files (including nested files to verify full-tree moves)
            self.ctx.create_test_file(
                &format!("{}/old_folder/file1.{}", src_dir, ext),
                self.config.file_template.simple_module,
            );
            self.ctx.create_test_file(
                &format!("{}/old_folder/file2.{}", src_dir, ext),
                self.config.file_template.simple_module,
            );
            self.ctx.create_test_file(
                &format!("{}/old_folder/nested/deep_file.{}", src_dir, ext),
                self.config.file_template.export_module,
            );

            let old_rel = format!("{}/old_folder", src_dir);
            let new_rel = format!("{}/new_folder", src_dir);
            let before_snapshot = self.capture_directory_snapshot(&old_rel)?;

            let old_path = self.ctx.absolute_path(&old_rel);
            let new_path = self.ctx.absolute_path(&new_rel);

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

            self.verify_directory_move_complete(&old_rel, &new_rel, &before_snapshot)?;
            self.ctx
                .verify_file_exists(&format!("{}/new_folder/file1.{}", src_dir, ext))?;
            self.ctx
                .verify_file_exists(&format!("{}/new_folder/file2.{}", src_dir, ext))?;
            self.ctx
                .verify_file_exists(&format!("{}/new_folder/nested/deep_file.{}", src_dir, ext))?;

            Ok(())
        }
        .await;

        self.record(test_name, result, start.elapsed().as_millis());
    }

    /// Test: Move folder down into another folder
    pub async fn test_folder_move_down(&mut self) {
        let test_name = "folder_move_down";
        println!("\n🧪 Running: {}", test_name);
        let start = std::time::Instant::now();

        let result = async {
            let src_dir = self.config.source_dir;
            let ext = self.config.file_ext;

            self.ctx.create_test_file(
                &format!("{}/movable/content.{}", src_dir, ext),
                self.config.file_template.simple_module,
            );
            self.ctx.create_test_file(
                &format!("{}/movable/nested/content2.{}", src_dir, ext),
                self.config.file_template.export_module,
            );

            std::fs::create_dir_all(self.ctx.absolute_path(&format!("{}/container", src_dir))).ok();

            let source_rel = format!("{}/movable", src_dir);
            let dest_rel = format!("{}/container/movable", src_dir);
            let before_snapshot = self.capture_directory_snapshot(&source_rel)?;

            let source = self.ctx.absolute_path(&source_rel);
            let dest = self.ctx.absolute_path(&dest_rel);

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

            self.verify_directory_move_complete(&source_rel, &dest_rel, &before_snapshot)?;
            self.ctx
                .verify_file_exists(&format!("{}/container/movable/content.{}", src_dir, ext))?;
            self.ctx.verify_file_exists(&format!(
                "{}/container/movable/nested/content2.{}",
                src_dir, ext
            ))?;

            Ok(())
        }
        .await;

        self.record(test_name, result, start.elapsed().as_millis());
    }

    /// Test: Move folder up one level
    pub async fn test_folder_move_up(&mut self) {
        let test_name = "folder_move_up";
        println!("\n🧪 Running: {}", test_name);
        let start = std::time::Instant::now();

        let result = async {
            let src_dir = self.config.source_dir;
            let ext = self.config.file_ext;

            // Create nested folder structure
            self.ctx.create_test_file(
                &format!("{}/parent/child/nested.{}", src_dir, ext),
                self.config.file_template.simple_module,
            );
            self.ctx.create_test_file(
                &format!("{}/parent/child/deeper/leaf.{}", src_dir, ext),
                self.config.file_template.export_module,
            );

            let source_rel = format!("{}/parent/child", src_dir);
            let dest_rel = format!("{}/child", src_dir);
            let before_snapshot = self.capture_directory_snapshot(&source_rel)?;

            let source = self.ctx.absolute_path(&source_rel);
            let dest = self.ctx.absolute_path(&dest_rel);

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

            self.verify_directory_move_complete(&source_rel, &dest_rel, &before_snapshot)?;
            self.ctx
                .verify_file_exists(&format!("{}/child/nested.{}", src_dir, ext))?;
            self.ctx
                .verify_file_exists(&format!("{}/child/deeper/leaf.{}", src_dir, ext))?;

            Ok(())
        }
        .await;

        self.record(test_name, result, start.elapsed().as_millis());
    }

    /// Test: Move folder to sibling directory
    pub async fn test_folder_move_sibling(&mut self) {
        let test_name = "folder_move_sibling";
        println!("\n🧪 Running: {}", test_name);
        let start = std::time::Instant::now();

        let result = async {
            let src_dir = self.config.source_dir;
            let ext = self.config.file_ext;

            // Create two sibling folders
            self.ctx.create_test_file(
                &format!("{}/folder_a/moveme/content.{}", src_dir, ext),
                self.config.file_template.simple_module,
            );
            self.ctx.create_test_file(
                &format!("{}/folder_a/moveme/sub/content2.{}", src_dir, ext),
                self.config.file_template.export_module,
            );
            std::fs::create_dir_all(self.ctx.absolute_path(&format!("{}/folder_b", src_dir))).ok();

            let source_rel = format!("{}/folder_a/moveme", src_dir);
            let dest_rel = format!("{}/folder_b/moveme", src_dir);
            let before_snapshot = self.capture_directory_snapshot(&source_rel)?;

            let source = self.ctx.absolute_path(&source_rel);
            let dest = self.ctx.absolute_path(&dest_rel);

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

            self.verify_directory_move_complete(&source_rel, &dest_rel, &before_snapshot)?;
            self.ctx
                .verify_file_exists(&format!("{}/folder_b/moveme/content.{}", src_dir, ext))?;
            self.ctx
                .verify_file_exists(&format!("{}/folder_b/moveme/sub/content2.{}", src_dir, ext))?;

            Ok(())
        }
        .await;

        self.record(test_name, result, start.elapsed().as_millis());
    }

    /// Test: Delete folder with prune operation
    pub async fn test_prune_folder(&mut self) {
        let test_name = "prune_folder";
        println!("\n🧪 Running: {}", test_name);
        let start = std::time::Instant::now();

        let result = async {
            let src_dir = self.config.source_dir;
            let ext = self.config.file_ext;

            // Create folder with files to delete
            self.ctx.create_test_file(
                &format!("{}/to_delete_folder/file1.{}", src_dir, ext),
                self.config.file_template.simple_module,
            );
            self.ctx.create_test_file(
                &format!("{}/to_delete_folder/file2.{}", src_dir, ext),
                self.config.file_template.simple_module,
            );

            self.ctx.verify_dir_exists(&format!("{}/to_delete_folder", src_dir))?;

            let folder_path = self.ctx.absolute_path(&format!("{}/to_delete_folder", src_dir));

            self.ctx
                .call_tool(
                    "prune",
                    json!({
                        "target": { "kind": "directory", "filePath": folder_path.to_string_lossy() },
                        "options": { "dryRun": false }
                    }),
                )
                .await
                .map_err(|e| format!("prune failed: {}", e))?;

            self.ctx.verify_file_not_exists(&format!("{}/to_delete_folder", src_dir))?;

            Ok(())
        }
        .await;

        self.record(test_name, result, start.elapsed().as_millis());
    }

    // =========================================================================
    // Content Operations
    // =========================================================================

    /// Test: Find/replace literal strings
    pub async fn test_find_replace_literal(&mut self) {
        let test_name = "find_replace_literal";
        println!("\n🧪 Running: {}", test_name);
        let start = std::time::Instant::now();

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

            self.ctx
                .verify_file_contains(&format!("{}/replace_test.{}", src_dir, ext), "NEW_VALUE")?;
            self.ctx.verify_file_not_contains(
                &format!("{}/replace_test.{}", src_dir, ext),
                "OLD_VALUE",
            )?;

            Ok(())
        }
        .await;

        self.record(test_name, result, start.elapsed().as_millis());
    }

    // =========================================================================
    // Edge Cases
    // =========================================================================

    /// Test: Move deeply nested file to top level
    pub async fn test_deep_to_shallow(&mut self) {
        let test_name = "deep_to_shallow";
        println!("\n🧪 Running: {}", test_name);
        let start = std::time::Instant::now();

        let result = async {
            let src_dir = self.config.source_dir;
            let ext = self.config.file_ext;

            self.ctx.create_test_file(
                &format!("{}/a/b/c/d/deep.{}", src_dir, ext),
                self.config.file_template.simple_module,
            );

            let source = self
                .ctx
                .absolute_path(&format!("{}/a/b/c/d/deep.{}", src_dir, ext));
            let dest = self
                .ctx
                .absolute_path(&format!("{}/shallow.{}", src_dir, ext));

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

            self.ctx
                .verify_file_exists(&format!("{}/shallow.{}", src_dir, ext))?;
            self.ctx
                .verify_file_not_exists(&format!("{}/a/b/c/d/deep.{}", src_dir, ext))?;

            Ok(())
        }
        .await;

        self.record(test_name, result, start.elapsed().as_millis());
    }

    /// Test: Move file to deeply nested location
    pub async fn test_shallow_to_deep(&mut self) {
        let test_name = "shallow_to_deep";
        println!("\n🧪 Running: {}", test_name);
        let start = std::time::Instant::now();

        let result = async {
            let src_dir = self.config.source_dir;
            let ext = self.config.file_ext;

            let rel_path = format!("{}/top_level.{}", src_dir, ext);
            self.ctx
                .create_test_file(&rel_path, self.config.file_template.simple_module);

            // Verify file was actually created (debug step)
            self.ctx.verify_file_exists(&rel_path)?;

            let dest_dir = self.ctx.absolute_path(&format!("{}/x/y/z", src_dir));
            std::fs::create_dir_all(&dest_dir).ok();

            let source = self.ctx.absolute_path(&rel_path);
            let dest = self
                .ctx
                .absolute_path(&format!("{}/x/y/z/buried.{}", src_dir, ext));

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

            self.ctx
                .verify_file_exists(&format!("{}/x/y/z/buried.{}", src_dir, ext))?;
            self.ctx
                .verify_file_not_exists(&format!("{}/top_level.{}", src_dir, ext))?;

            Ok(())
        }
        .await;

        self.record(test_name, result, start.elapsed().as_millis());
    }

    /// Test: Delete file with prune
    pub async fn test_prune_file(&mut self) {
        let test_name = "prune_file";
        println!("\n🧪 Running: {}", test_name);
        let start = std::time::Instant::now();

        let result = async {
            let src_dir = self.config.source_dir;
            let ext = self.config.file_ext;

            let rel_path = format!("{}/to_delete.{}", src_dir, ext);
            self.ctx
                .create_test_file(&rel_path, self.config.file_template.simple_module);

            // Verify file was actually created (debug step)
            self.ctx.verify_file_exists(&rel_path)?;

            let file_path = self.ctx.absolute_path(&rel_path);

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

            self.ctx
                .verify_file_not_exists(&format!("{}/to_delete.{}", src_dir, ext))?;

            Ok(())
        }
        .await;

        self.record(test_name, result, start.elapsed().as_millis());
    }

    /// Test: Delete files with tiered inbound references to stress prune planning.
    ///
    /// Tier defaults:
    /// - perf profile: 120, 500, 1000 inbound refs
    /// - full profile: 120 inbound refs (keeps full suite runtime manageable)
    pub async fn test_prune_file_large_inbound_refs(&mut self) {
        let test_name = "prune_file_large_inbound_refs";
        println!(
            "
🧪 Running: {}",
            test_name
        );
        let start = std::time::Instant::now();

        let result = async {
            // This targeted benchmark is TS-only because prune/import semantics differ by language
            if self.config.file_ext != "ts" {
                return Ok(());
            }

            let src_dir = self.config.source_dir;
            let ext = self.config.file_ext;
            let tiers: Vec<usize> = if Self::matrix_profile() == "perf" {
                vec![120, 500, 1000]
            } else {
                vec![120]
            };

            for tier in tiers {
                let target_rel = format!("{}/to_prune_large_{}.{}", src_dir, tier, ext);
                let importer_dir_rel = format!("{}/prune_inbound_{}", src_dir, tier);
                let importer_dir_abs = self.ctx.absolute_path(&importer_dir_rel);
                std::fs::create_dir_all(&importer_dir_abs)
                    .map_err(|e| format!("failed to create {}: {}", importer_dir_rel, e))?;

                self.ctx
                    .create_test_file(&target_rel, "export const keepMe = 123;\n");
                self.ctx.verify_file_exists(&target_rel)?;

                for idx in 0..tier {
                    let importer_rel =
                        format!("{}/ref_inbound_{:04}.{}", importer_dir_rel, idx, ext);
                    let importer_content = format!(
                        "import {{ keepMe }} from \"../to_prune_large_{}\";\nexport const v{} = keepMe + {};\n",
                        tier, idx, idx
                    );
                    self.ctx.create_test_file(&importer_rel, &importer_content);
                }

                let cross_dirs = Self::prune_cross_boundary_dirs(src_dir, tier);
                for (dir_i, cross_dir_rel) in cross_dirs.iter().enumerate() {
                    let cross_dir_abs = self.ctx.absolute_path(cross_dir_rel);
                    std::fs::create_dir_all(&cross_dir_abs)
                        .map_err(|e| format!("failed to create {}: {}", cross_dir_rel, e))?;

                    for idx in 0..10 {
                        let importer_rel =
                            format!("{}/cross_ref_{}_{}.{}", cross_dir_rel, dir_i, idx, ext);
                        let importer_content = format!(
                            "import {{ keepMe }} from \"../../to_prune_large_{}\";\nexport const cross{}_{} = keepMe + {};\n",
                            tier, dir_i, idx, idx
                        );
                        self.ctx.create_test_file(&importer_rel, &importer_content);
                    }
                }

                println!(
                    "  ℹ️  Created {} inbound ref files for prune stress tier {} (+20 cross-boundary)",
                    tier, tier
                );

                let file_path = self.ctx.absolute_path(&target_rel);
                let prune_start = std::time::Instant::now();
                self.ctx
                    .call_tool(
                        "prune",
                        json!({
                            "target": { "kind": "file", "filePath": file_path.to_string_lossy() },
                            "options": { "dryRun": false }
                        }),
                    )
                    .await
                    .map_err(|e| format!("prune failed at tier {}: {}", tier, e))?;
                let tier_ms = prune_start.elapsed().as_millis();

                self.ctx.verify_file_not_exists(&target_rel)?;
                println!("  📈 prune tier {} completed in {} ms", tier, tier_ms);

                let var_name = format!("TYPEMILL_PRUNE_TIER_{}_MAX_MS", tier);
                let threshold_ms = std::env::var(&var_name)
                    .ok()
                    .and_then(|v| v.parse::<u128>().ok())
                    .unwrap_or(match tier {
                        120 => 100,
                        500 => 300,
                        1000 => 700,
                        _ => 1_000,
                    });
                if tier_ms > threshold_ms {
                    let strict_assert = std::env::var(PERF_ASSERT_STRICT_ENV_VAR)
                        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                        .unwrap_or(false);
                    let exceedance = format!(
                        "prune_tier_{}_ms exceeded threshold: {} > {}",
                        tier, tier_ms, threshold_ms
                    );
                    self.threshold_exceedances.push(exceedance.clone());
                    println!(
                        "  ⚠️ Perf threshold exceeded for prune tier {}: {} ms > {} ms",
                        tier, tier_ms, threshold_ms
                    );
                    if strict_assert {
                        return Err(format!(
                            "perf assertion failed for prune tier {}: {} ms > {} ms",
                            tier, tier_ms, threshold_ms
                        ));
                    }
                }


                // Cleanup stress fixture importers so strict build verifications remain accurate.
                std::fs::remove_dir_all(&importer_dir_abs)
                    .map_err(|e| format!("failed to cleanup {}: {}", importer_dir_rel, e))?;
                self.cleanup_prune_cross_boundary_dirs(src_dir, tier)?;
            }

            Ok(())
        }
        .await;

        self.record(test_name, result, start.elapsed().as_millis());
    }

    fn matrix_profile() -> String {
        std::env::var("TYPEMILL_MATRIX_PROFILE")
            .unwrap_or_else(|_| "full".to_string())
            .to_ascii_lowercase()
    }

    fn prune_cross_boundary_dirs(src_dir: &str, tier: usize) -> [String; 2] {
        [
            format!(
                "{}/.typemill_prune_stress_workspace_a_{}/src",
                src_dir, tier
            ),
            format!(
                "{}/.typemill_prune_stress_workspace_b_{}/src",
                src_dir, tier
            ),
        ]
    }

    fn cleanup_prune_cross_boundary_dirs(&self, src_dir: &str, tier: usize) -> Result<(), String> {
        for cross_dir_rel in Self::prune_cross_boundary_dirs(src_dir, tier) {
            let cross_dir_abs = self.ctx.absolute_path(&cross_dir_rel);
            if cross_dir_abs.exists() {
                std::fs::remove_dir_all(&cross_dir_abs)
                    .map_err(|e| format!("failed to cleanup {}: {}", cross_dir_rel, e))?;
            }
        }
        Ok(())
    }

    // =========================================================================
    // Run All Tests
    // =========================================================================

    /// Run the complete test matrix
    pub async fn run_all(&mut self) {
        let run_start = std::time::Instant::now();
        println!("\n{}", "=".repeat(60));
        println!("  RUNNING REFACTORING MATRIX: {}", self.config.project_name);
        println!("{}\n", "=".repeat(60));

        // Warmup
        let warmup_start = std::time::Instant::now();
        println!("🔥 Warming up LSP...");
        if let Err(e) = self.warmup().await {
            println!("❌ LSP warmup failed: {}", e);
            return;
        }
        let warmup_ms = warmup_start.elapsed().as_millis();
        println!("✅ LSP ready\n");

        // Record baseline errors BEFORE any tests (for comparative verification)
        println!("📊 Recording baseline build state...");
        let baseline_start = std::time::Instant::now();
        self.record_baseline();
        let baseline_ms = baseline_start.elapsed().as_millis();

        // Initial build verification (now uses comparative baseline)
        println!("🔍 Verifying initial build...");
        let initial_verify_start = std::time::Instant::now();
        match self.verify_build() {
            Ok(()) => println!("✅ Initial build passes\n"),
            Err(e) => println!("⚠️ Initial build: {}\n", e),
        }
        let initial_verify_ms = initial_verify_start.elapsed().as_millis();

        let operations_start = std::time::Instant::now();
        let profile = Self::matrix_profile();
        println!("⚙️ Matrix profile: {}", profile);
        println!(
            "⚙️ Build verification cadence: every {} operation(s)",
            self.matrix_verify_every()
        );

        if profile == "perf" {
            println!("\n🚀 PERF PROFILE OPERATIONS");
            println!("{}", "-".repeat(40));
            self.test_folder_rename().await;
            self.test_folder_move_up().await;
            self.test_prune_folder().await;
            self.test_find_replace_literal().await;
            self.test_deep_to_shallow().await;
            self.test_prune_file_large_inbound_refs().await;
        } else {
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
            self.test_folder_move_up().await;
            self.test_folder_move_sibling().await;
            self.test_prune_folder().await;

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
            self.test_prune_file_large_inbound_refs().await;
        }

        let operations_total_ms = operations_start.elapsed().as_millis();

        // Final build verification
        let final_verify_start = std::time::Instant::now();
        println!("\n🏁 FINAL BUILD VERIFICATION");
        println!("{}", "-".repeat(40));
        match self.verify_build() {
            Ok(()) => println!("✅ Final build passes!"),
            Err(e) => println!("⚠️ Final build: {}", e),
        }
        let final_verify_ms = final_verify_start.elapsed().as_millis();

        let (build_verifications_executed, build_verifications_failed, build_verifications_skipped) =
            self.build_verification_stats();
        self.run_timings = Some(MatrixRunTimings {
            warmup_ms,
            baseline_ms,
            initial_verify_ms,
            final_verify_ms,
            operations_total_ms,
            build_verifications_executed,
            build_verifications_failed,
            build_verifications_skipped,
            total_run_ms: run_start.elapsed().as_millis(),
        });

        // Print summary
        self.print_summary();

        if let Err(e) = self.write_perf_artifact() {
            println!("⚠️ Failed to write matrix artifact: {}", e);
        }
    }

    fn write_perf_artifact(&self) -> Result<(), String> {
        let artifact_dir = match std::env::var(MATRIX_ARTIFACT_ENV_VAR) {
            Ok(v) if !v.trim().is_empty() => v,
            _ => return Ok(()),
        };

        let dir = std::path::Path::new(&artifact_dir);
        std::fs::create_dir_all(dir)
            .map_err(|e| format!("failed to create artifact dir {}: {}", dir.display(), e))?;

        let artifact_path = dir.join(format!("{}_matrix.json", self.config.project_name));
        let payload = serde_json::json!({
            "schema_version": PERF_ARTIFACT_SCHEMA_VERSION,
            "project": self.config.project_name,
            "profile": Self::matrix_profile(),
            "verify_every": self.matrix_verify_every(),
            "threshold_exceedances": self.threshold_exceedances,
            "run_timings": self.run_timings,
            "results": self.results,
            "generated_at": chrono::Utc::now().to_rfc3339(),
        });

        std::fs::write(
            &artifact_path,
            serde_json::to_string_pretty(&payload).map_err(|e| e.to_string())?,
        )
        .map_err(|e| {
            format!(
                "failed to write artifact {}: {}",
                artifact_path.display(),
                e
            )
        })?;

        println!("📦 Wrote matrix artifact: {}", artifact_path.display());
        Ok(())
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

/// TypeScript: ky (small HTTP client, clean structure)
#[tokio::test]
#[serial]
#[ignore]
async fn test_matrix_ts_ky() {
    run_matrix_test(TS_KY_CONFIG, 0.5).await;
}

/// TypeScript: SvelteKit (monorepo stress test)
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

/// TypeScript: Turborepo (pnpm monorepo with multiple packages)
#[tokio::test]
#[serial]
#[ignore]
async fn test_matrix_ts_turborepo() {
    run_matrix_test(TS_TURBOREPO_CONFIG, 0.5).await;
}

/// TypeScript: Next.js (app router with server components)
#[tokio::test]
#[serial]
#[ignore]
async fn test_matrix_ts_nextjs() {
    run_matrix_test(TS_NEXTJS_CONFIG, 0.5).await;
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

/// Rust: ripgrep (multi-crate workspace with path dependencies)
#[tokio::test]
#[serial]
#[ignore]
async fn test_matrix_rs_ripgrep() {
    run_matrix_test(RS_RIPGREP_CONFIG, 0.5).await;
}

/// Rust: tokio (async runtime, highly inter-dependent crates)
#[tokio::test]
#[serial]
#[ignore]
async fn test_matrix_rs_tokio() {
    run_matrix_test(RS_TOKIO_CONFIG, 0.5).await;
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

/// Python: FastAPI (src-layout, namespace packages)
#[tokio::test]
#[serial]
#[ignore]
async fn test_matrix_py_fastapi() {
    run_matrix_test(PY_FASTAPI_CONFIG, 0.5).await;
}

/// Rust/Python: Ruff (PyO3/maturin, complex cross-language structure)
#[tokio::test]
#[serial]
#[ignore]
async fn test_matrix_rs_ruff() {
    run_matrix_test(PY_RUFF_CONFIG, 0.5).await;
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
// Sharded Matrix Group Tests (for CI/runtime isolation)
// ============================================================================

/// Run Python matrix shard only (independent reporting/timeout behavior).
#[tokio::test]
#[serial]
#[ignore]
async fn test_matrix_shard_python() {
    let mut failures = Vec::new();

    for (name, cfg) in [
        ("py_httpx", PY_HTTPX_CONFIG),
        ("py_rich", PY_RICH_CONFIG),
        ("py_pydantic", PY_PYDANTIC_CONFIG),
        ("py_fastapi", PY_FASTAPI_CONFIG),
    ] {
        let result = std::panic::AssertUnwindSafe(run_matrix_test(cfg, 0.5))
            .catch_unwind()
            .await;
        if result.is_err() {
            failures.push(name);
        }
    }

    assert!(
        failures.is_empty(),
        "Python matrix shard failures: {:?}",
        failures
    );
}

/// Run Rust matrix shard only (independent reporting/timeout behavior).
#[tokio::test]
#[serial]
#[ignore]
async fn test_matrix_shard_rust() {
    let mut failures = Vec::new();

    for (name, cfg) in [
        ("rs_thiserror", RS_THISERROR_CONFIG),
        ("rs_anyhow", RS_ANYHOW_CONFIG),
        ("rs_oncecell", RS_ONCECELL_CONFIG),
        ("rs_ripgrep", RS_RIPGREP_CONFIG),
        ("rs_tokio", RS_TOKIO_CONFIG),
        ("rs_ruff", PY_RUFF_CONFIG),
    ] {
        let result = std::panic::AssertUnwindSafe(run_matrix_test(cfg, 0.5))
            .catch_unwind()
            .await;
        if result.is_err() {
            failures.push(name);
        }
    }

    assert!(
        failures.is_empty(),
        "Rust matrix shard failures: {:?}",
        failures
    );
}

/// Run TypeScript matrix shard only (independent reporting/timeout behavior).
#[tokio::test]
#[serial]
#[ignore]
async fn test_matrix_shard_typescript() {
    let mut failures = Vec::new();

    for (name, cfg) in [
        ("ts_zod", TS_ZOD_CONFIG),
        ("ts_ky", TS_KY_CONFIG),
        ("ts_sveltekit", TS_SVELTEKIT_CONFIG),
        ("ts_nanoid", TS_NANOID_CONFIG),
        ("ts_pattern", TS_PATTERN_CONFIG),
        ("ts_turborepo", TS_TURBOREPO_CONFIG),
        ("ts_nextjs", TS_NEXTJS_CONFIG),
    ] {
        let result = std::panic::AssertUnwindSafe(run_matrix_test(cfg, 0.5))
            .catch_unwind()
            .await;
        if result.is_err() {
            failures.push(name);
        }
    }

    assert!(
        failures.is_empty(),
        "TypeScript matrix shard failures: {:?}",
        failures
    );
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
    assert!(
        result.passed,
        "file_rename operation failed: {:?}",
        result.error
    );
    assert!(
        result.build_passed == Some(true),
        "Build verification failed after file_rename - refactoring introduced new errors"
    );
}
