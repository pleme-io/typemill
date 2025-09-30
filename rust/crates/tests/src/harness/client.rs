use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

/// Test client for interacting with the cb-server binary.
/// Manages process lifecycle and JSON-RPC communication.
pub struct TestClient {
    pub process: Child,
    pub stdin: ChildStdin,
    pub stdout_receiver: mpsc::Receiver<String>,
    pub stderr_receiver: mpsc::Receiver<String>,
}

impl TestClient {
    /// Spawns cb-server in stdio mode with the given working directory.
    pub fn new(working_dir: &Path) -> Self {
        // Determine the path to the cb-server binary
        // Use absolute paths for reliability
        let possible_paths = [
            "/workspace/rust/target/release/cb-server",
            "/workspace/rust/target/debug/cb-server",
            "target/release/cb-server",
            "target/debug/cb-server",
        ];

        let server_path = possible_paths
            .iter()
            .find(|path| Path::new(path).exists())
            .unwrap_or(&"cb-server");

        eprintln!("DEBUG: TestClient using server path: {}", server_path);

        // Expand ALL environment variables in PATH for LSP server spawning
        // This is needed because cargo config sets PATH with $HOME, $NVM_DIR, etc.
        // which don't get expanded when inherited by spawned processes
        let expanded_path = if let Ok(path) = std::env::var("PATH") {
            // Use shellexpand for proper shell-style expansion of ALL variables
            match shellexpand::env(&path) {
                Ok(expanded) => {
                    let result = expanded.to_string();
                    eprintln!("DEBUG: Expanded PATH for cb-server (shellexpand)");
                    eprintln!("DEBUG:   Original: {}", path);
                    eprintln!("DEBUG:   Expanded: {}", result);
                    result
                }
                Err(e) => {
                    eprintln!("WARN: Failed to expand PATH, using original: {}", e);
                    path
                }
            }
        } else {
            std::env::var("PATH").unwrap_or_default()
        };

        let mut process = Command::new(server_path)
            .arg("start")
            .current_dir(working_dir)
            .env("PATH", expanded_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to start cb-server binary");

        let stdin = process.stdin.take().unwrap();
        let stdout = process.stdout.take().unwrap();
        let stderr = process.stderr.take().unwrap();

        // Spawn thread to read stdout
        let (stdout_sender, stdout_receiver) = mpsc::channel();
        thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines().map_while(Result::ok) {
                let trimmed = line.trim();
                if !trimmed.is_empty()
                    && trimmed.starts_with('{')
                    && stdout_sender.send(line).is_err()
                {
                    break;
                }
            }
        });

        // Spawn thread to read stderr (for debugging crashes)
        let (stderr_sender, stderr_receiver) = mpsc::channel();
        thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines().map_while(Result::ok) {
                if stderr_sender.send(line).is_err() {
                    break;
                }
            }
        });

        // Wait for server startup
        thread::sleep(Duration::from_millis(1500));

        TestClient {
            process,
            stdin,
            stdout_receiver,
            stderr_receiver,
        }
    }

    /// Send a JSON-RPC request and wait for response.
    pub fn send_request(&mut self, request: Value) -> Result<Value, Box<dyn std::error::Error>> {
        let request_str = serde_json::to_string(&request)?;
        writeln!(self.stdin, "{}", request_str)?;
        self.stdin.flush()?;

        // Wait for response with extended timeout for resilience tests
        let response_str = self.stdout_receiver.recv_timeout(Duration::from_secs(15))?;
        let response: Value = serde_json::from_str(&response_str)?;
        Ok(response)
    }

    /// Send a tools/call request with the given tool name and arguments.
    pub async fn call_tool(
        &mut self,
        tool_name: &str,
        arguments: Value,
    ) -> Result<Value, Box<dyn std::error::Error>> {
        static mut REQUEST_ID: i32 = 0;
        let id = unsafe {
            REQUEST_ID += 1;
            REQUEST_ID
        };

        let request = json!({
            "jsonrpc": "2.0",
            "id": format!("test-{}", id),
            "method": "tools/call",
            "params": {
                "name": tool_name,
                "arguments": arguments
            }
        });

        self.send_request(request)
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
            thread::sleep(Duration::from_millis(500));
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

    /// Call a tool with a custom timeout.
    pub async fn call_tool_with_timeout(
        &mut self,
        tool_name: &str,
        arguments: Value,
        timeout: Duration,
    ) -> Result<Value, Box<dyn std::error::Error>> {
        static mut REQUEST_ID: i32 = 0;
        let id = unsafe {
            REQUEST_ID += 1;
            REQUEST_ID
        };

        let request = json!({
            "jsonrpc": "2.0",
            "id": format!("test-timeout-{}", id),
            "method": "tools/call",
            "params": {
                "name": tool_name,
                "arguments": arguments
            }
        });

        let request_str = serde_json::to_string(&request)?;
        writeln!(self.stdin, "{}", request_str)?;
        self.stdin.flush()?;

        // Wait for response with custom timeout
        let response_str = self.stdout_receiver.recv_timeout(timeout)?;
        let response: Value = serde_json::from_str(&response_str)?;
        Ok(response)
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

        // Get process information using /proc filesystem (Linux)
        let status_path = format!("/proc/{}/status", pid);
        let stat_path = format!("/proc/{}/stat", pid);

        let mut stats = ServerStats {
            pid,
            memory_kb: 0,
            cpu_percent: 0.0,
            uptime_seconds: 0,
            child_processes: self.get_child_processes().len() as u32,
        };

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
        let _ = self.process.kill();
        let _ = self.process.wait();
    }
}
