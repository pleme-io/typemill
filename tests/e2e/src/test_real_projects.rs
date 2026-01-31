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

use crate::harness::{TestClient, TestWorkspace};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

/// Extended timeout for LSP warmup on large projects (3 minutes)
pub const LSP_WARMUP_TIMEOUT: Duration = Duration::from_secs(180);

/// Standard timeout for operations after warmup (2 minutes to allow reconnection)
pub const LARGE_PROJECT_TIMEOUT: Duration = Duration::from_secs(120);

/// Test context that manages a cloned project and mill client
pub struct RealProjectContext {
    pub workspace: TestWorkspace,
    pub client: TestClient,
    pub project_name: String,
    warmed_up: AtomicBool,
}

impl RealProjectContext {
    /// Clone a git repository and set up mill
    pub fn new(repo_url: &str, project_name: &str) -> Self {
        let workspace = TestWorkspace::new();

        // Clone the repository
        let status = Command::new("git")
            .args(["clone", "--depth", "1", repo_url, "."])
            .current_dir(workspace.path())
            .status()
            .expect("Failed to clone repository");

        assert!(
            status.success(),
            "Failed to clone {} repository",
            project_name
        );

        // Run mill setup
        let mill_path = std::env::var("CARGO_MANIFEST_DIR")
            .map(|dir| {
                let mut path = PathBuf::from(dir);
                path.pop(); // e2e
                path.pop(); // tests
                path.push("target/debug/mill");
                path
            })
            .expect("CARGO_MANIFEST_DIR not set");

        let setup_status = Command::new(&mill_path)
            .args(["setup", "--update"])
            .current_dir(workspace.path())
            .status()
            .expect("Failed to run mill setup");

        assert!(setup_status.success(), "Failed to run mill setup");

        let client = TestClient::new(workspace.path());

        Self {
            workspace,
            client,
            project_name: project_name.to_string(),
            warmed_up: AtomicBool::new(false),
        }
    }

    /// Ensure LSP is warmed up and connected. Call this before any LSP-dependent operations.
    /// First call does full warmup (3 min timeout). Subsequent calls verify connection (1 min timeout).
    /// Note: Each tokio::test has its own runtime, so connection verification is needed per test.
    pub async fn ensure_warmed_up(&mut self) -> Result<(), String> {
        let is_first_warmup = !self.warmed_up.load(Ordering::SeqCst);
        let timeout = if is_first_warmup {
            println!(
                "🔥 Warming up LSP for {} (this may take up to 3 minutes)...",
                self.project_name
            );
            LSP_WARMUP_TIMEOUT
        } else {
            println!("🔄 Verifying LSP connection for {}...", self.project_name);
            Duration::from_secs(60)
        };

        // Find a representative file to trigger LSP initialization
        let warmup_file = self.find_warmup_file();

        if let Some(file_path) = warmup_file {
            // Use inspect_code to trigger LSP initialization/reconnection
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
                    timeout,
                )
                .await;

            match result {
                Ok(_) => {
                    if is_first_warmup {
                        self.warmed_up.store(true, Ordering::SeqCst);
                        println!("✅ LSP warmed up for {}", self.project_name);
                    } else {
                        println!("✅ LSP connection verified for {}", self.project_name);
                    }
                    Ok(())
                }
                Err(e) => {
                    let msg = format!("LSP warmup/verification failed for {}: {}", self.project_name, e);
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
                println!("✅ Search for '{}' completed with {} results", query, arr.len());
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
