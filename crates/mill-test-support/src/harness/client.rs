use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

// ============================================================================
// Test Client Timeout Constants
// ============================================================================
// These values are tuned for test reliability across different systems.
// Override with TYPEMILL_TEST_TOOL_TIMEOUT_SECS environment variable if needed.

/// Time to wait for server to become ready after spawn (health check passes)
const SERVER_READY_TIMEOUT_SECS: u64 = 5;

/// Default timeout for tool call requests (most operations).
/// Must be long enough to cover LSP initialization on first tool call (~60-120s).
/// Override via TYPEMILL_TEST_TOOL_TIMEOUT_SECS env var.
const DEFAULT_TOOL_CALL_TIMEOUT_SECS: u64 = 180;

/// Returns the effective tool call timeout, checking env override first.
fn tool_call_timeout() -> Duration {
    if let Ok(val) = std::env::var("TYPEMILL_TEST_TOOL_TIMEOUT_SECS") {
        if let Ok(secs) = val.parse::<u64>() {
            return Duration::from_secs(secs);
        }
    }
    Duration::from_secs(DEFAULT_TOOL_CALL_TIMEOUT_SECS)
}

/// Timeout for graceful shutdown before force kill
const SHUTDOWN_TIMEOUT_SECS: u64 = 2;

/// Poll interval when waiting for async operations
const POLL_INTERVAL_MS: u64 = 100;

/// Short poll interval for shutdown loop
const SHUTDOWN_POLL_INTERVAL_MS: u64 = 50;

/// Delay before checking stderr after startup
const STARTUP_STDERR_DELAY_MS: u64 = 500;

/// Test client for interacting with the mill server binary.
/// Manages process lifecycle and JSON-RPC communication.
pub struct TestClient {
    pub process: Child,
    pub stdin: ChildStdin,
    pub stdout_receiver: mpsc::Receiver<String>,
    pub stderr_receiver: mpsc::Receiver<String>,
}

impl TestClient {
    /// Spawns mill server in stdio mode with the given working directory.
    pub fn new(working_dir: &Path) -> Self {
        // Determine the path to the mill binary by finding the workspace root
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
            .expect("CARGO_MANIFEST_DIR not set. Please run tests with `cargo test`.");
        let workspace_root = find_workspace_root(&manifest_dir).expect(
            "Failed to find workspace root. Ensure tests are run within a Cargo workspace.",
        );
        let server_path = workspace_root.join("target/debug/mill");

        // Pre-check: Fail fast if binary doesn't exist with helpful message
        if !server_path.exists() {
            panic!(
                "\n\n\
                 ❌ \x1b[1;31mMill debug binary not found\x1b[0m\n\
                 \n\
                 Expected location: \x1b[1m{}\x1b[0m\n\
                 \n\
                 E2E tests require the debug build. Please run:\n\
                 \n\
                 \x1b[1;36m    cargo build --workspace\x1b[0m\n\
                 \n\
                 Or just the mill binary:\n\
                 \n\
                 \x1b[1;36m    cargo build -p mill\x1b[0m\n\
                 \n\
                 \x1b[33m💡 Note:\x1b[0m Building with \x1b[1m--release\x1b[0m only creates target/release/mill.\n\
                 Tests need target/debug/mill (the debug build).\n\
                 \n\
                 \x1b[33m💡 Low memory?\x1b[0m Use: cargo build -j 1\n\
                 \n",
                server_path.display()
            );
        }

        eprintln!(
            "DEBUG: TestClient using server path: {}",
            server_path.display()
        );

        // Expand ALL environment variables in PATH for LSP server spawning
        // This is needed because cargo config sets PATH with $HOME, $NVM_DIR, etc.
        // which don't get expanded when inherited by spawned processes
        let expanded_path = if let Ok(path) = std::env::var("PATH") {
            // Use shellexpand with custom context to handle missing variables gracefully
            // Missing variables expand to empty strings instead of causing errors
            let result =
                shellexpand::env_with_context_no_errors(&path, |var| std::env::var(var).ok())
                    .to_string();

            // Only log if RUST_LOG=debug to avoid exposing paths in CI by default
            if std::env::var("RUST_LOG")
                .unwrap_or_default()
                .to_lowercase()
                .contains("debug")
            {
                eprintln!("DEBUG: Expanded PATH for mill server (shellexpand)");
                eprintln!("DEBUG:   Original: {}", path);
                eprintln!("DEBUG:   Expanded: {}", result);
            }
            result
        } else {
            std::env::var("PATH").unwrap_or_default()
        };

        // Use a unique PID file for each test to avoid conflicts in parallel execution
        let pid_file = working_dir.join(".mill.pid");

        let mut command = Command::new(&server_path);
        command
            .arg("start")
            .current_dir(working_dir)
            .env("PATH", expanded_path)
            .env("MILL_PID_FILE", pid_file) // Unique PID file per test
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Propagate RUST_LOG to the server process for debugging
        if let Ok(rust_log) = std::env::var("RUST_LOG") {
            command.env("RUST_LOG", rust_log);
        }

        // Propagate NVM_DIR so LSP client can find the correct Node version
        if let Ok(nvm_dir) = std::env::var("NVM_DIR") {
            command.env("NVM_DIR", nvm_dir);
        }

        let mut process = command.spawn().unwrap_or_else(|e| {
            panic!(
                "Failed to start mill binary at {:?}: {}. \n\
                 Make sure to build the binary first with: cargo build",
                server_path, e
            )
        });

        let stdin = process.stdin.take().unwrap();
        let stdout = process.stdout.take().unwrap();
        let stderr = process.stderr.take().unwrap();

        // Spawn thread to read stdout with frame delimiter support
        let (stdout_sender, stdout_receiver) = mpsc::channel();
        thread::spawn(move || {
            const FRAME_DELIMITER: &[u8] = b"\n---FRAME---\n";
            let mut reader = BufReader::new(stdout);
            let mut buffer = Vec::new();

            loop {
                // Read until newline
                match reader.read_until(b'\n', &mut buffer) {
                    Ok(0) => break, // EOF
                    Ok(_) => {
                        // Check if we've reached the frame delimiter
                        if buffer.ends_with(FRAME_DELIMITER) {
                            // Remove the delimiter
                            buffer.truncate(buffer.len() - FRAME_DELIMITER.len());
                            let message = String::from_utf8_lossy(&buffer).trim().to_string();

                            // Send the complete framed message
                            if !message.is_empty() && stdout_sender.send(message).is_err() {
                                break;
                            }

                            // Clear buffer for next message
                            buffer.clear();
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        // Spawn thread to read stderr (for debugging crashes)
        let (stderr_sender, stderr_receiver) = mpsc::channel();
        thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines().map_while(Result::ok) {
                eprintln!("[SERVER STDERR] {}", &line); // Make server logs visible in test output
                if stderr_sender.send(line).is_err() {
                    break;
                }
            }
        });

        let mut client = TestClient {
            process,
            stdin,
            stdout_receiver,
            stderr_receiver,
        };

        // Wait for server to be ready via health check polling
        // This is much faster than fixed 5s sleep - typically ready in 200-500ms
        let start = Instant::now();
        client
            .wait_for_ready(Duration::from_secs(SERVER_READY_TIMEOUT_SECS))
            .unwrap_or_else(|e| panic!("Server failed to start: {}", e));
        eprintln!("✓ Server ready in {}ms", start.elapsed().as_millis());

        client
    }

    /// Send a JSON-RPC request and wait for response.
    pub fn send_request(&mut self, request: Value) -> Result<Value, Box<dyn std::error::Error>> {
        self.send_request_with_timeout(request, tool_call_timeout())
    }

    /// Send a JSON-RPC request with a custom timeout.
    pub fn send_request_with_timeout(
        &mut self,
        request: Value,
        timeout: Duration,
    ) -> Result<Value, Box<dyn std::error::Error>> {
        const FRAME_DELIMITER: &[u8] = b"\n---FRAME---\n";

        let request_str = serde_json::to_string(&request)?;
        // Send request followed by frame delimiter
        self.stdin.write_all(request_str.as_bytes())?;
        self.stdin.write_all(FRAME_DELIMITER)?;
        self.stdin.flush()?;

        // Wait for response with extended timeout for resilience tests
        let response_str = self.stdout_receiver.recv_timeout(timeout)?;
        let response: Value = serde_json::from_str(&response_str)?;
        Ok(response)
    }

    /// Send a tools/call request with the given tool name and arguments.
    /// Returns an error if the response contains an error field (JSON-RPC error).
    pub async fn call_tool(
        &mut self,
        tool_name: &str,
        arguments: Value,
    ) -> Result<Value, Box<dyn std::error::Error>> {
        static REQUEST_ID: AtomicI32 = AtomicI32::new(0);
        let id = REQUEST_ID.fetch_add(1, Ordering::SeqCst) + 1;

        let request = json!({
            "jsonrpc": "2.0",
            "id": format!("test-{}", id),
            "method": "tools/call",
            "params": {
                "name": tool_name,
                "arguments": arguments
            }
        });

        let response = self.send_request(request)?;

        // Check if the response contains an error field
        if let Some(error) = response.get("error") {
            let error_msg = error
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("Unknown error");
            return Err(format!("Tool call error: {}", error_msg).into());
        }

        Ok(response)
    }

    /// Check if the server process is still alive.
    pub fn is_alive(&mut self) -> bool {
        match self.process.try_wait() {
            Ok(Some(_)) => false, // Process has exited
            Ok(None) => true,     // Process is still running
            Err(_) => false,      // Error checking status
        }
    }

    /// Get stderr logs for debugging.
    pub fn get_stderr_logs(&self) -> Vec<String> {
        let mut logs = Vec::new();
        while let Ok(line) = self.stderr_receiver.try_recv() {
            logs.push(line);
        }
        logs
    }

    /// Get child processes (LSP servers spawned by cb-server).
    pub fn get_child_processes(&self) -> Vec<u32> {
        // Find child processes (LSP servers spawned by cb-server)
        let output = Command::new("pgrep")
            .arg("-P")
            .arg(self.process.id().to_string())
            .output();

        if let Ok(output) = output {
            String::from_utf8_lossy(&output.stdout)
                .lines()
                .filter_map(|line| line.trim().parse::<u32>().ok())
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Gracefully shutdown the server.
    pub fn shutdown(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Try to send a shutdown request first
        let shutdown_request = json!({
            "jsonrpc": "2.0",
            "id": "shutdown",
            "method": "shutdown",
            "params": {}
        });

        // Attempt graceful shutdown
        if self.send_request(shutdown_request).is_ok() {
            // Give the server time to shut down gracefully
            thread::sleep(Duration::from_millis(STARTUP_STDERR_DELAY_MS));
        }

        // Kill the process if it's still alive
        if self.is_alive() {
            self.process.kill()?;
            self.process.wait()?;
        }

        Ok(())
    }

    /// Call a tool with performance timing.
    pub async fn call_tool_with_timing(
        &mut self,
        tool_name: &str,
        arguments: Value,
    ) -> Result<(Value, Duration), Box<dyn std::error::Error>> {
        let start = Instant::now();
        let result = self.call_tool(tool_name, arguments).await?;
        let duration = start.elapsed();
        Ok((result, duration))
    }

    /// Call a tool with a custom timeout.
    pub async fn call_tool_with_timeout(
        &mut self,
        tool_name: &str,
        arguments: Value,
        timeout: Duration,
    ) -> Result<Value, Box<dyn std::error::Error>> {
        static REQUEST_ID: AtomicI32 = AtomicI32::new(0);
        let id = REQUEST_ID.fetch_add(1, Ordering::SeqCst) + 1;

        let request = json!({
            "jsonrpc": "2.0",
            "id": format!("test-timeout-{}", id),
            "method": "tools/call",
            "params": {
                "name": tool_name,
                "arguments": arguments
            }
        });

        self.send_request_with_timeout(request, timeout)
    }

    /// Call multiple tools sequentially and return results with timings.
    pub async fn call_multiple_tools(
        &mut self,
        calls: Vec<(&str, Value)>,
    ) -> Vec<Result<(Value, Duration), Box<dyn std::error::Error>>> {
        let mut results = Vec::new();

        for (tool_name, arguments) in calls {
            let result = self.call_tool_with_timing(tool_name, arguments).await;
            results.push(result);
        }

        results
    }

    /// Simulate a connection error by terminating stdin.
    pub fn force_connection_error(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Close stdin to simulate connection loss
        let _ = &mut self.stdin;
        Ok(())
    }

    /// Check server responsiveness with a health ping.
    pub fn ping_server(&mut self) -> Result<Duration, Box<dyn std::error::Error>> {
        let start = Instant::now();

        let ping_request = json!({
            "jsonrpc": "2.0",
            "id": "ping",
            "method": "tools/call",
            "params": {
                "name": "health_check",
                "arguments": {}
            }
        });

        self.send_request(ping_request)?;
        Ok(start.elapsed())
    }

    /// Batch execute multiple tool calls concurrently (simulated).
    pub async fn batch_execute_tools(
        &mut self,
        calls: Vec<(&str, Value)>,
    ) -> Vec<Result<Value, Box<dyn std::error::Error>>> {
        // Since we can't truly parallelize with a single stdin/stdout,
        // we'll execute them rapidly in sequence to simulate batch execution
        let mut results = Vec::new();

        for (tool_name, arguments) in calls {
            let result = self.call_tool(tool_name, arguments).await;
            results.push(result);

            // Small delay to prevent overwhelming the server
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        results
    }

    /// Get server memory usage and performance stats.
    pub fn get_server_stats(&self) -> Result<ServerStats, Box<dyn std::error::Error>> {
        let pid = self.process.id();

        #[cfg(target_os = "linux")]
        let mut stats = ServerStats {
            pid,
            memory_kb: 0,
            cpu_percent: 0.0,
            uptime_seconds: 0,
            child_processes: self.get_child_processes().len() as u32,
        };

        #[cfg(not(target_os = "linux"))]
        let stats = ServerStats {
            pid,
            memory_kb: 0,
            cpu_percent: 0.0,
            uptime_seconds: 0,
            child_processes: self.get_child_processes().len() as u32,
        };

        // Get process information using /proc filesystem (Linux-only)
        #[cfg(target_os = "linux")]
        {
            let status_path = format!("/proc/{}/status", pid);
            let stat_path = format!("/proc/{}/stat", pid);

            // Read memory usage from /proc/PID/status
            if let Ok(status_content) = std::fs::read_to_string(&status_path) {
                for line in status_content.lines() {
                    if line.starts_with("VmRSS:") {
                        if let Some(value_str) = line.split_whitespace().nth(1) {
                            stats.memory_kb = value_str.parse().unwrap_or(0);
                        }
                        break;
                    }
                }
            }

            // Basic uptime calculation (simplified)
            if let Ok(stat_content) = std::fs::read_to_string(&stat_path) {
                let fields: Vec<&str> = stat_content.split_whitespace().collect();
                if fields.len() > 21 {
                    // Field 22 is starttime in clock ticks
                    if let Ok(starttime) = fields[21].parse::<u64>() {
                        let clock_ticks_per_sec = 100; // Typical value, could be more precise
                        let _boot_time = 0; // Would need to read from /proc/stat for accuracy
                        stats.uptime_seconds = starttime / clock_ticks_per_sec;
                    }
                }
            }
        }

        Ok(stats)
    }

    /// Wait for server to be ready with a timeout.
    pub fn wait_for_ready(&mut self, timeout: Duration) -> Result<(), Box<dyn std::error::Error>> {
        let start = Instant::now();

        while start.elapsed() < timeout {
            if self.ping_server().is_ok() {
                return Ok(());
            }

            thread::sleep(Duration::from_millis(100));
        }

        Err("Server failed to become ready within timeout".into())
    }

    /// Stress test the server with rapid requests.
    pub async fn stress_test(&mut self, request_count: usize, delay_ms: u64) -> StressTestResults {
        let mut results = StressTestResults {
            total_requests: request_count,
            successful_requests: 0,
            failed_requests: 0,
            total_duration: Duration::ZERO,
            min_response_time: Duration::MAX,
            max_response_time: Duration::ZERO,
            avg_response_time: Duration::ZERO,
        };

        let start = Instant::now();
        let mut response_times = Vec::new();

        for _i in 0..request_count {
            let request_start = Instant::now();

            let result = self.call_tool("health_check", json!({})).await;
            let request_duration = request_start.elapsed();

            match result {
                Ok(_) => {
                    results.successful_requests += 1;
                    response_times.push(request_duration);

                    if request_duration < results.min_response_time {
                        results.min_response_time = request_duration;
                    }
                    if request_duration > results.max_response_time {
                        results.max_response_time = request_duration;
                    }
                }
                Err(_) => {
                    results.failed_requests += 1;
                }
            }

            if delay_ms > 0 {
                tokio::time::sleep(Duration::from_millis(delay_ms)).await;
            }
        }

        results.total_duration = start.elapsed();

        if !response_times.is_empty() {
            let total_response_time: Duration = response_times.iter().sum();
            results.avg_response_time = total_response_time / response_times.len() as u32;
        }

        if results.min_response_time == Duration::MAX {
            results.min_response_time = Duration::ZERO;
        }

        results
    }

    /// Wait for LSP to finish indexing a file by polling a symbol lookup.
    /// This is more reliable than diagnostics for LSPs that don't support pull-model diagnostics.
    ///
    /// # Arguments
    /// * `file_path` - Path to the file to wait for
    /// * `max_wait_ms` - Maximum time to wait in milliseconds
    ///
    /// # Returns
    /// Ok(()) when LSP has indexed the file (symbols appear)
    /// Err(String) if timeout is reached
    pub async fn wait_for_lsp_ready(
        &mut self,
        file_path: &std::path::Path,
        max_wait_ms: u64,
    ) -> Result<(), String> {
        use std::time::Instant;

        let symbol_name = extract_symbol_name(file_path);

        let start = Instant::now();
        let poll_interval = Duration::from_millis(POLL_INTERVAL_MS);
        let max_duration = Duration::from_millis(max_wait_ms);

        loop {
            // Prefer symbol-based readiness since some LSPs don't support pull-model diagnostics.
            if let Some(ref symbol) = symbol_name {
                if let Ok(response) = self
                    .call_tool(
                        "inspect_code",
                        serde_json::json!({
                            "filePath": file_path.to_string_lossy(),
                            "symbolName": symbol,
                            "include": ["definition"]
                        }),
                    )
                    .await
                {
                    if response
                        .get("result")
                        .and_then(|r| r.get("content"))
                        .and_then(|c| c.get("definition"))
                        .is_some()
                    {
                        return Ok(());
                    }
                }

                if let Ok(response) = self
                    .call_tool(
                        "search_code",
                        serde_json::json!({
                            "query": symbol
                        }),
                    )
                    .await
                {
                    let has_results = response
                        .get("result")
                        .and_then(|r| r.as_array().or_else(|| r.get("content").and_then(|c| c.as_array())))
                        .map(|arr| !arr.is_empty())
                        .unwrap_or(false);
                    if has_results {
                        return Ok(());
                    }
                }
            }

            // Fallback: diagnostics (some LSPs still support this)
            if let Ok(response) = self
                .call_tool(
                    "inspect_code",
                    serde_json::json!({
                        "filePath": file_path.to_string_lossy(),
                        "line": 0,
                        "character": 0,
                        "include": ["diagnostics"]
                    }),
                )
                .await
            {
                if response
                    .get("result")
                    .and_then(|r| r.get("content"))
                    .and_then(|c| c.get("diagnostics"))
                    .is_some()
                {
                    return Ok(());
                }
            }

            if start.elapsed() > max_duration {
                return Err(format!(
                    "LSP did not index file {} within {}ms",
                    file_path.display(),
                    max_wait_ms
                ));
            }

            tokio::time::sleep(poll_interval).await;
        }
    }
}

fn extract_symbol_name(file_path: &std::path::Path) -> Option<String> {
    use regex::Regex;

    let content = std::fs::read_to_string(file_path).ok()?;
    let re = Regex::new(
        r"(?m)^(?:\s*export\s+)?(?:class|interface|function|type|enum|const|let|var|struct|trait|fn)\s+([A-Za-z_][A-Za-z0-9_]*)",
    )
    .ok()?;
    re.captures(&content)
        .and_then(|caps| caps.get(1).map(|m| m.as_str().to_string()))
}

/// Performance statistics for the server process.
#[derive(Debug, Clone)]
pub struct ServerStats {
    pub pid: u32,
    pub memory_kb: u64,
    pub cpu_percent: f64,
    pub uptime_seconds: u64,
    pub child_processes: u32,
}

/// Results from a stress test.
#[derive(Debug, Clone)]
pub struct StressTestResults {
    pub total_requests: usize,
    pub successful_requests: usize,
    pub failed_requests: usize,
    pub total_duration: Duration,
    pub min_response_time: Duration,
    pub max_response_time: Duration,
    pub avg_response_time: Duration,
}

impl StressTestResults {
    pub fn success_rate(&self) -> f64 {
        if self.total_requests == 0 {
            0.0
        } else {
            (self.successful_requests as f64) / (self.total_requests as f64) * 100.0
        }
    }

    pub fn requests_per_second(&self) -> f64 {
        if self.total_duration.is_zero() {
            0.0
        } else {
            (self.successful_requests as f64) / self.total_duration.as_secs_f64()
        }
    }
}

impl Drop for TestClient {
    fn drop(&mut self) {
        // Try graceful shutdown first by sending SIGTERM (instead of SIGKILL)
        // This gives the server's Drop handlers time to cleanup LSP clients

        // Send SIGTERM for graceful shutdown
        unsafe {
            libc::kill(self.process.id() as i32, libc::SIGTERM);
        }

        // Wait for graceful shutdown
        let start = Instant::now();
        while start.elapsed() < Duration::from_secs(SHUTDOWN_TIMEOUT_SECS) {
            match self.process.try_wait() {
                Ok(Some(_)) => {
                    // Process exited gracefully
                    return;
                }
                Ok(None) => {
                    // Still running, wait a bit
                    thread::sleep(Duration::from_millis(SHUTDOWN_POLL_INTERVAL_MS));
                }
                Err(_) => break,
            }
        }

        // If still running after timeout, force kill
        let _ = self.process.kill();
        let _ = self.process.wait();
    }
}

/// Finds the workspace root by traversing up from a starting directory.
fn find_workspace_root(start_dir: &str) -> Option<std::path::PathBuf> {
    let mut current_dir = std::path::PathBuf::from(start_dir);
    loop {
        let cargo_toml_path = current_dir.join("Cargo.toml");
        if cargo_toml_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&cargo_toml_path) {
                if content.contains("[workspace]") {
                    return Some(current_dir);
                }
            }
        }
        if !current_dir.pop() {
            return None;
        }
    }
}
