//! Real-world project integration tests framework
//!
//! Provides a reusable framework for testing mill operations against
//! real open-source projects across TypeScript, Rust, and Python.
//!
//! Each language module shares a single cloned project and TestClient
//! to avoid redundant setup time. Tests run serially within each module.
//!
//! LSP warmup happens once per project context, ensuring subsequent tests
//! run quickly with a warm LSP.

use crate::harness::{LspSetupHelper, TestClient, TestWorkspace};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

trait CommandPathExt {
    fn with_expanded_path(&mut self) -> &mut Self;
}

impl CommandPathExt for Command {
    fn with_expanded_path(&mut self) -> &mut Self {
        self.env("PATH", expanded_path_for_tools())
    }
}

/// Get the repo cache directory path
fn get_repo_cache_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("mill-test")
        .join("repos")
}

/// Check if a cached repo exists and is valid
fn is_cache_valid(cache_path: &PathBuf) -> bool {
    if !cache_path.exists() {
        return false;
    }
    // Check if it's a valid git repo
    Command::new("git")
        .with_expanded_path()
        .args(["rev-parse", "--git-dir"])
        .current_dir(cache_path)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Clone or update the cache for a repository
fn ensure_repo_cached(repo_url: &str, project_name: &str) -> PathBuf {
    let cache_dir = get_repo_cache_dir();
    std::fs::create_dir_all(&cache_dir).expect("Failed to create cache directory");

    let cache_path = cache_dir.join(project_name);

    if is_cache_valid(&cache_path) {
        println!("📦 Using cached repo for {} (fast path)", project_name);
        // Update the cache with fetch (non-blocking, best effort)
        let _ = Command::new("git")
            .with_expanded_path()
            .args(["fetch", "--depth", "1", "origin"])
            .current_dir(&cache_path)
            .output();
    } else {
        println!("📥 Cloning {} to cache (first time)...", project_name);
        // Remove any invalid cache
        let _ = std::fs::remove_dir_all(&cache_path);

        let status = Command::new("git")
            .with_expanded_path()
            .args([
                "clone",
                "--depth",
                "1",
                repo_url,
                cache_path.to_string_lossy().as_ref(),
            ])
            .status()
            .expect("Failed to clone repository to cache");

        assert!(
            status.success(),
            "Failed to clone {} to cache",
            project_name
        );
    }

    cache_path
}

/// Copy from local cache: copy .git + checkout (fastest approach tested)
fn copy_from_cache(cache_path: &PathBuf, workspace_path: &std::path::Path, project_name: &str) {
    println!("📋 Copying {} from cache...", project_name);

    // Copy just .git directory (small for shallow clones)
    let status = Command::new("cp")
        .with_expanded_path()
        .args(["-r", &format!("{}/.git", cache_path.display()), ".git"])
        .current_dir(workspace_path)
        .status()
        .expect("Failed to copy .git directory");

    assert!(status.success(), "Failed to copy .git from cache");

    // Checkout files from the copied .git
    let status = Command::new("git")
        .with_expanded_path()
        .args(["checkout", "."])
        .current_dir(workspace_path)
        .status()
        .expect("Failed to checkout files");

    assert!(status.success(), "Failed to checkout files from cache");

    println!("✅ Repo ready from cache");
}

fn expanded_path_for_tools() -> String {
    if let Ok(path) = std::env::var("PATH") {
        shellexpand::env_with_context_no_errors(&path, |var| std::env::var(var).ok()).to_string()
    } else {
        String::new()
    }
}

fn workspace_root_from_manifest_dir() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .map(|p| p.to_path_buf())
        .unwrap_or(manifest_dir)
}

fn resolve_mill_binary_path() -> PathBuf {
    workspace_root_from_manifest_dir().join("target/debug/mill")
}

fn run_mill_setup(workspace_path: &std::path::Path) -> Result<(), String> {
    let mill_path = resolve_mill_binary_path();

    if mill_path.is_file() {
        let status = Command::new(&mill_path)
            .with_expanded_path()
            .args(["setup", "--update"])
            .current_dir(workspace_path)
            .status()
            .map_err(|e| {
                format!(
                    "Failed to run mill setup via binary {}: {}",
                    mill_path.display(),
                    e
                )
            })?;

        if status.success() {
            return Ok(());
        }

        return Err(format!(
            "mill setup failed via binary {} with status {}",
            mill_path.display(),
            status
        ));
    }

    let workspace_root = workspace_root_from_manifest_dir();
    println!(
        "⚠️ mill binary not found at {}, falling back to cargo run --bin mill",
        mill_path.display()
    );

    let manifest_path = workspace_root.join("Cargo.toml");
    let status = Command::new("cargo")
        .with_expanded_path()
        .args([
            "run",
            "--quiet",
            "--manifest-path",
            manifest_path.to_string_lossy().as_ref(),
            "--bin",
            "mill",
            "--",
            "setup",
            "--update",
        ])
        .current_dir(workspace_path)
        .status()
        .map_err(|e| format!("Failed to run cargo fallback for mill setup: {}", e))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "mill setup failed via cargo fallback in {} with status {}",
            workspace_root.display(),
            status
        ))
    }
}

/// Extended timeout for LSP warmup on large projects (3 minutes)
pub const LSP_WARMUP_TIMEOUT: Duration = Duration::from_secs(180);

/// Standard timeout for operations after warmup (3 minutes to allow for slow LSP + cargo check)
pub const LARGE_PROJECT_TIMEOUT: Duration = Duration::from_secs(180);

/// Test context that manages a cloned project and mill client
pub struct RealProjectContext {
    pub workspace: TestWorkspace,
    pub client: TestClient,
    pub project_name: String,
    warmed_up: AtomicBool,
    lsp_enabled: bool,
}

impl RealProjectContext {
    /// Clone a git repository and set up mill
    /// Uses repo caching for faster subsequent runs
    pub fn new(repo_url: &str, project_name: &str) -> Self {
        let workspace = TestWorkspace::new();

        // Use cached repo if available (saves ~20-30s per test)
        let cache_path = ensure_repo_cached(repo_url, project_name);
        copy_from_cache(&cache_path, workspace.path(), project_name);

        // Run mill setup (binary fast-path with cargo fallback for fresh test envs)
        run_mill_setup(workspace.path()).expect("Failed to run mill setup");
        let lsp_enabled = LspSetupHelper::prune_unavailable_lsp_servers(&workspace)
            .expect("Failed to prune unavailable LSP servers");

        let previous_lsp_mode = std::env::var("TYPEMILL_LSP_MODE").ok();
        if !lsp_enabled {
            std::env::set_var("TYPEMILL_LSP_MODE", "off");
        }
        let client = TestClient::new(workspace.path());
        if !lsp_enabled {
            if let Some(mode) = previous_lsp_mode {
                std::env::set_var("TYPEMILL_LSP_MODE", mode);
            } else {
                std::env::remove_var("TYPEMILL_LSP_MODE");
            }
        }

        Self {
            workspace,
            client,
            project_name: project_name.to_string(),
            warmed_up: AtomicBool::new(false),
            lsp_enabled,
        }
    }

    /// Ensure LSP is warmed up and connected. Call this before any LSP-dependent operations.
    /// First call does full warmup (3 min timeout). Subsequent calls are no-ops (fast path).
    /// The LSP server persists across tests via the shared context, so re-verification is unnecessary.
    pub async fn ensure_warmed_up(&mut self) -> Result<(), String> {
        // Fast path: already warmed up, skip LSP call entirely
        // The LSP server is kept alive by the TestClient, so no need to verify
        if self.warmed_up.load(Ordering::SeqCst) {
            return Ok(());
        }

        if !self.lsp_enabled {
            self.warmed_up.store(true, Ordering::SeqCst);
            println!(
                "⚠️ Skipping LSP warmup for {} (no runnable LSP servers available)",
                self.project_name
            );
            return Ok(());
        }

        println!(
            "🔥 Warming up LSP for {} (this may take up to 3 minutes)...",
            self.project_name
        );

        // Find a representative file to trigger LSP initialization
        let warmup_file = self.find_warmup_file();

        if let Some(file_path) = warmup_file {
            // Use inspect_code to trigger LSP initialization
            let result = self
                .client
                .call_tool_with_timeout(
                    "inspect_code",
                    json!({
                        "filePath": file_path.to_string_lossy(),
                        "line": 1,
                        "character": 0,
                        "include": ["diagnostics"]
                    }),
                    LSP_WARMUP_TIMEOUT,
                )
                .await;

            match result {
                Ok(_) => {
                    self.warmed_up.store(true, Ordering::SeqCst);
                    println!("✅ LSP warmed up for {}", self.project_name);
                    Ok(())
                }
                Err(e) => {
                    let msg = format!("LSP warmup failed for {}: {}", self.project_name, e);
                    println!("❌ {}", msg);
                    Err(msg)
                }
            }
        } else {
            // No source files found, but that's okay for file-only operations
            self.warmed_up.store(true, Ordering::SeqCst);
            println!(
                "⚠️ No source files found for LSP warmup in {}",
                self.project_name
            );
            Ok(())
        }
    }

    /// Find a representative source file for LSP warmup
    fn find_warmup_file(&self) -> Option<PathBuf> {
        let extensions = ["ts", "rs", "py", "js", "tsx"];
        let search_dirs = ["src", "lib", "packages", "."];

        for dir in search_dirs {
            let search_path = if dir == "." {
                self.workspace.path().to_path_buf()
            } else {
                self.workspace.path().join(dir)
            };

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

    /// Helper to call a tool with the standard timeout (use after warmup)
    pub async fn call_tool(
        &mut self,
        name: &str,
        args: Value,
    ) -> Result<Value, Box<dyn std::error::Error>> {
        self.client
            .call_tool_with_timeout(name, args, LARGE_PROJECT_TIMEOUT)
            .await
    }

    /// Create a test file in the workspace
    pub fn create_test_file(&self, path: &str, content: &str) {
        self.workspace.create_file(path, content);
    }

    /// Get absolute path for a relative path
    pub fn absolute_path(&self, path: &str) -> PathBuf {
        self.workspace.absolute_path(path)
    }

    /// Read a file from the workspace
    pub fn read_file(&self, path: &str) -> String {
        self.workspace.read_file(path)
    }

    /// Wait for LSP to be ready for a specific file (legacy, prefer ensure_warmed_up)
    pub async fn wait_for_lsp(&mut self, file_path: &PathBuf) {
        let _ = self.client.wait_for_lsp_ready(file_path, 15000).await;
    }

    // =========================================================================
    // Verification Methods - Ensure operations actually worked
    // =========================================================================

    /// Verify Rust project compiles after refactoring
    pub fn verify_rust_compiles(&self) -> Result<(), String> {
        println!("🔍 Verifying Rust project compiles...");
        let output = Command::new("cargo")
            .with_expanded_path()
            .args(["check", "--message-format=short"])
            .current_dir(self.workspace.path())
            .output()
            .map_err(|e| format!("Failed to run cargo check: {}", e))?;

        if output.status.success() {
            println!("✅ Rust project compiles successfully");
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(format!("Rust compilation failed:\n{}", stderr))
        }
    }

    /// Count TypeScript errors in the project
    /// Returns (error_count, error_output) - useful for comparing before/after refactoring
    pub fn count_typescript_errors(&self) -> (usize, String) {
        let output = Command::new("npx")
            .with_expanded_path()
            .args(["tsc", "--noEmit", "--skipLibCheck"])
            .current_dir(self.workspace.path())
            .output()
            .or_else(|_| {
                Command::new("tsc")
                    .with_expanded_path()
                    .args(["--noEmit", "--skipLibCheck"])
                    .current_dir(self.workspace.path())
                    .output()
            });

        match output {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout).to_string();
                // Count "error TS" occurrences - each represents a TypeScript error
                let error_count = stdout.matches("error TS").count();
                (error_count, stdout)
            }
            Err(e) => (usize::MAX, format!("Failed to run tsc: {}", e)),
        }
    }

    /// Verify TypeScript project compiles after refactoring
    pub fn verify_typescript_compiles(&self) -> Result<(), String> {
        println!("🔍 Verifying TypeScript project compiles...");

        let (error_count, error_output) = self.count_typescript_errors();

        if error_count == 0 {
            println!("✅ TypeScript project compiles successfully");
            Ok(())
        } else {
            Err(format!(
                "TypeScript compilation failed ({} errors):\n{}",
                error_count, error_output
            ))
        }
    }

    /// Verify Python file is syntactically valid
    pub fn verify_python_syntax(&self, file_path: &str) -> Result<(), String> {
        println!("🔍 Verifying Python syntax for {}...", file_path);
        let abs_path = self.absolute_path(file_path);

        let output = Command::new("python3")
            .with_expanded_path()
            .args(["-m", "py_compile", abs_path.to_string_lossy().as_ref()])
            .current_dir(self.workspace.path())
            .output()
            .or_else(|_| {
                Command::new("python")
                    .with_expanded_path()
                    .args(["-m", "py_compile", abs_path.to_string_lossy().as_ref()])
                    .current_dir(self.workspace.path())
                    .output()
            })
            .map_err(|e| format!("Failed to run python: {}", e))?;

        if output.status.success() {
            println!("✅ Python file {} is syntactically valid", file_path);
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(format!("Python syntax error in {}:\n{}", file_path, stderr))
        }
    }

    /// Verify a Python module can be imported
    pub fn verify_python_import(&self, module_path: &str) -> Result<(), String> {
        println!("🔍 Verifying Python import {}...", module_path);

        let output = Command::new("python3")
            .with_expanded_path()
            .args(["-c", &format!("import {}", module_path)])
            .current_dir(self.workspace.path())
            .env("PYTHONPATH", self.workspace.path())
            .output()
            .or_else(|_| {
                Command::new("python")
                    .with_expanded_path()
                    .args(["-c", &format!("import {}", module_path)])
                    .current_dir(self.workspace.path())
                    .env("PYTHONPATH", self.workspace.path())
                    .output()
            })
            .map_err(|e| format!("Failed to run python: {}", e))?;

        if output.status.success() {
            println!("✅ Python module {} imports successfully", module_path);
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(format!(
                "Python import failed for {}:\n{}",
                module_path, stderr
            ))
        }
    }

    /// Verify file contains expected content
    pub fn verify_file_contains(&self, file_path: &str, expected: &str) -> Result<(), String> {
        let content = self.read_file(file_path);
        if content.contains(expected) {
            println!("✅ File {} contains expected content", file_path);
            Ok(())
        } else {
            Err(format!(
                "File {} does not contain expected content.\nExpected to find: {}\nActual content:\n{}",
                file_path, expected, content
            ))
        }
    }

    /// Verify file does NOT contain specific content (useful for checking old imports removed)
    pub fn verify_file_not_contains(
        &self,
        file_path: &str,
        unexpected: &str,
    ) -> Result<(), String> {
        let content = self.read_file(file_path);
        if !content.contains(unexpected) {
            println!(
                "✅ File {} correctly does not contain: {}",
                file_path, unexpected
            );
            Ok(())
        } else {
            Err(format!(
                "File {} unexpectedly contains: {}\nActual content:\n{}",
                file_path, unexpected, content
            ))
        }
    }

    /// Verify import was updated in a file (old import gone, new import present)
    pub fn verify_import_updated(
        &self,
        file_path: &str,
        old_import: &str,
        new_import: &str,
    ) -> Result<(), String> {
        println!("🔍 Verifying import update in {}...", file_path);
        let content = self.read_file(file_path);

        let has_old = content.contains(old_import);
        let has_new = content.contains(new_import);

        match (has_old, has_new) {
            (false, true) => {
                println!(
                    "✅ Import correctly updated: '{}' → '{}'",
                    old_import, new_import
                );
                Ok(())
            }
            (true, false) => Err(format!(
                "Import NOT updated in {}.\nStill contains old: {}\nMissing new: {}\nContent:\n{}",
                file_path, old_import, new_import, content
            )),
            (true, true) => Err(format!(
                "Both old and new imports present in {}.\nOld: {}\nNew: {}\nContent:\n{}",
                file_path, old_import, new_import, content
            )),
            (false, false) => Err(format!(
                "Neither old nor new import found in {}.\nExpected new: {}\nContent:\n{}",
                file_path, new_import, content
            )),
        }
    }

    /// Verify a file exists at the expected path
    pub fn verify_file_exists(&self, path: &str) -> Result<(), String> {
        let abs_path = self.absolute_path(path);
        if abs_path.exists() {
            println!("✅ File exists: {}", path);
            Ok(())
        } else {
            Err(format!("File does not exist: {}", path))
        }
    }

    /// Verify a file does NOT exist (was moved/deleted)
    pub fn verify_file_not_exists(&self, path: &str) -> Result<(), String> {
        let abs_path = self.absolute_path(path);
        if !abs_path.exists() {
            println!("✅ File correctly removed: {}", path);
            Ok(())
        } else {
            Err(format!("File still exists but should be gone: {}", path))
        }
    }

    /// Verify directory exists
    pub fn verify_dir_exists(&self, path: &str) -> Result<(), String> {
        let abs_path = self.absolute_path(path);
        if abs_path.is_dir() {
            println!("✅ Directory exists: {}", path);
            Ok(())
        } else {
            Err(format!("Directory does not exist: {}", path))
        }
    }

    /// Run a custom verification command
    pub fn verify_command(
        &self,
        cmd: &str,
        args: &[&str],
        description: &str,
    ) -> Result<(), String> {
        println!("🔍 Running verification: {}...", description);
        let output = Command::new(cmd)
            .args(args)
            .current_dir(self.workspace.path())
            .output()
            .map_err(|e| format!("Failed to run {}: {}", cmd, e))?;

        if output.status.success() {
            println!("✅ {}", description);
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            Err(format!(
                "Verification failed: {}\nstdout: {}\nstderr: {}",
                description, stdout, stderr
            ))
        }
    }
}

/// Common test assertions
pub mod assertions {
    use serde_json::Value;

    /// Assert a tool response has success status
    pub fn assert_success(result: &Value, operation: &str) {
        let status = result
            .get("result")
            .and_then(|r| r.get("content"))
            .and_then(|c| c.get("status"))
            .and_then(|s| s.as_str());

        assert_eq!(
            status,
            Some("success"),
            "{} should succeed, got: {:?}",
            operation,
            result
        );
    }

    /// Assert a tool response has preview status (dry-run)
    pub fn assert_preview(result: &Value, operation: &str) {
        let status = result
            .get("result")
            .and_then(|r| r.get("content"))
            .and_then(|c| c.get("status"))
            .and_then(|s| s.as_str());

        assert!(
            status == Some("preview") || status == Some("success"),
            "{} should return preview or success, got: {:?}",
            operation,
            result
        );
    }

    /// Assert search returned results (fails if empty after warmup)
    pub fn assert_search_results(result: &Value, query: &str) {
        let inner_result = result.get("result").expect("Should have result field");
        let results = inner_result.get("results").and_then(|s| s.as_array());

        match results {
            Some(arr) if !arr.is_empty() => {
                println!("✅ Found {} results for '{}'", arr.len(), query);
            }
            Some(_) => {
                panic!(
                    "Search for '{}' returned empty results. LSP may have failed to index. Response: {:?}",
                    query, result
                );
            }
            None => {
                if let Some(error) = inner_result.get("error") {
                    panic!("Search returned error: {:?}", error);
                } else {
                    panic!(
                        "Search for '{}' returned no results array. Response: {:?}",
                        query, result
                    );
                }
            }
        }
    }

    /// Assert search returned results, but allow empty (for cases where LSP may not have symbols)
    pub fn assert_search_completed(result: &Value, query: &str) {
        let inner_result = result.get("result").expect("Should have result field");
        let results = inner_result.get("results").and_then(|s| s.as_array());

        match results {
            Some(arr) => {
                println!(
                    "✅ Search for '{}' completed with {} results",
                    query,
                    arr.len()
                );
            }
            None => {
                if let Some(error) = inner_result.get("error") {
                    panic!("Search returned error: {:?}", error);
                } else {
                    panic!(
                        "Search for '{}' returned no results array. Response: {:?}",
                        query, result
                    );
                }
            }
        }
    }
}
