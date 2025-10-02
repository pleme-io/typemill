//! CLI command handling for the codebuddy server

use cb_core::config::{AppConfig, LogFormat};
use clap::{Parser, Subcommand};
use fs2::FileExt;
use std::fs::{File, OpenOptions};
use std::path::PathBuf;
use std::process;
use tracing::{error, info};
use tracing_subscriber::fmt;
use tracing_subscriber::prelude::*;

/// Parse JSON argument from string
fn parse_json(s: &str) -> Result<serde_json::Value, String> {
    serde_json::from_str(s).map_err(|e| format!("Invalid JSON: {}", e))
}

#[derive(Parser)]
#[command(name = "codebuddy")]
#[command(about = "Pure Rust MCP server bridging Language Server Protocol functionality")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Start the MCP server in stdio mode for Claude Code
    Start {
        /// Run as daemon (write PID file)
        #[arg(long)]
        daemon: bool,
    },
    /// Start WebSocket server
    Serve {
        /// Run as daemon (write PID file)
        #[arg(long)]
        daemon: bool,
        /// Port to bind to
        #[arg(long, default_value = "3000")]
        port: u16,
    },
    /// Show status
    Status,
    /// Setup configuration
    Setup,
    /// Stop the running server
    Stop,
    /// Link to AI assistants
    Link,
    /// Remove AI from config
    Unlink,
    /// Check client configuration and diagnose potential problems
    Doctor,
    /// Call an MCP tool directly (without WebSocket server)
    Tool {
        /// Tool name (e.g., "rename_directory", "find_definition")
        tool_name: String,
        /// Tool arguments as JSON string
        args: String,
        /// Output format (pretty or compact)
        #[arg(long, default_value = "pretty", value_parser = ["pretty", "compact"])]
        format: String,
    },
    /// List all available MCP tools
    Tools {
        /// Output format (table, json, or names-only)
        #[arg(long, default_value = "table", value_parser = ["table", "json", "names"])]
        format: String,
    },
    /// Manage MCP server presets
    #[cfg(feature = "mcp-proxy")]
    #[command(subcommand)]
    Mcp(cb_client::McpCommands),
}

/// Main CLI entry point
pub async fn run() {
    // Parse CLI arguments first
    let cli = Cli::parse();

    // Only initialize tracing for server commands
    match &cli.command {
        Commands::Start { .. } | Commands::Serve { .. } => {
            // Load configuration to determine log format
            let config = AppConfig::load().unwrap_or_default();
            initialize_tracing(&config);
        }
        _ => {
            // For other commands, we want direct console output
        }
    }

    match cli.command {
        Commands::Start { daemon: _ } => {
            // Acquire exclusive lock on PID file (prevents multiple instances)
            let _lock_guard = match acquire_pid_lock() {
                Ok(guard) => guard,
                Err(e) => {
                    eprintln!("❌ Error: Codebuddy server is already running");
                    eprintln!("   Use 'codebuddy stop' to stop the running instance first");
                    eprintln!("   ({})", e);
                    process::exit(1);
                }
            };

            crate::run_stdio_mode().await;
            // Lock is automatically released when _lock_guard is dropped
        }
        Commands::Serve { daemon: _, port } => {
            // Acquire exclusive lock on PID file (prevents multiple instances)
            let _lock_guard = match acquire_pid_lock() {
                Ok(guard) => guard,
                Err(e) => {
                    eprintln!("❌ Error: Codebuddy server is already running");
                    eprintln!("   Use 'codebuddy stop' to stop the running instance first");
                    eprintln!("   ({})", e);
                    process::exit(1);
                }
            };

            crate::run_websocket_server_with_port(port).await;
            // Lock is automatically released when _lock_guard is dropped
        }
        Commands::Status => {
            handle_status().await;
        }
        Commands::Setup => {
            handle_setup().await;
        }
        Commands::Stop => {
            handle_stop().await;
        }
        Commands::Link => {
            handle_link().await;
        }
        Commands::Unlink => {
            handle_unlink().await;
        }
        Commands::Doctor => {
            handle_doctor().await;
        }
        Commands::Tool {
            tool_name,
            args,
            format,
        } => {
            handle_tool_command(&tool_name, &args, &format).await;
        }
        Commands::Tools { format } => {
            handle_tools_command(&format).await;
        }
        #[cfg(feature = "mcp-proxy")]
        Commands::Mcp(mcp_command) => {
            handle_mcp_command(mcp_command).await;
        }
    }
}

#[cfg(feature = "mcp-proxy")]
async fn handle_mcp_command(command: cb_client::McpCommands) {
    let result = match command {
        cb_client::McpCommands::List => cb_client::commands::mcp::list_presets(),
        cb_client::McpCommands::Add { preset_id } => {
            cb_client::commands::mcp::add_preset(&preset_id)
        }
        cb_client::McpCommands::Remove { preset_id } => {
            cb_client::commands::mcp::remove_preset(&preset_id)
        }
        cb_client::McpCommands::Info { preset_id } => {
            cb_client::commands::mcp::info_preset(&preset_id)
        }
    };

    if let Err(e) = result {
        error!(error = %e, "MCP command failed");
        process::exit(1);
    }
}

/// Handle the setup command
async fn handle_setup() {
    println!("🚀 Setting up codebuddy configuration...");

    // Create default configuration
    let config = AppConfig::default();

    // Determine config file path
    let config_path = std::path::Path::new(".codebuddy/config.json");

    // Check if config already exists
    if config_path.exists() {
        println!(
            "⚠️  Configuration file already exists at: {}",
            config_path.display()
        );
        println!("   To recreate configuration, please delete the existing file first.");
        return;
    }

    // Save default configuration
    match config.save(config_path) {
        Ok(()) => {
            println!("✅ Configuration saved to: {}", config_path.display());
            println!();
            println!("📝 Default LSP servers configured:");
            println!("   • TypeScript/JavaScript: typescript-language-server");
            println!("   • Python: pylsp");
            println!("   • Go: gopls");
            println!("   • Rust: rust-analyzer");
            println!();
            println!(
                "💡 You can edit {} to customize LSP servers and other settings.",
                config_path.display()
            );
        }
        Err(e) => {
            error!(error = %e, "Failed to save configuration");
            process::exit(1);
        }
    }
}

/// Handle the status command
async fn handle_status() {
    println!("📊 Codebuddy Status\n");

    // 1. Check server status
    println!("🖥️  Server Status:");
    let pid_file = get_pid_file_path();
    let server_running = if pid_file.exists() {
        match std::fs::read_to_string(&pid_file) {
            Ok(pid_str) => {
                if let Ok(pid) = pid_str.trim().parse::<u32>() {
                    if is_process_running(pid) {
                        println!("   ✅ Running (PID: {})", pid);
                        println!("   📄 PID file: {}", pid_file.display());
                        true
                    } else {
                        println!("   ⚠️  Not running (stale PID file found)");
                        let _ = std::fs::remove_file(&pid_file);
                        false
                    }
                } else {
                    println!("   ❌ Invalid PID file format");
                    false
                }
            }
            Err(e) => {
                println!("   ❌ Failed to read PID file: {}", e);
                false
            }
        }
    } else {
        println!("   ⭕ Not running");
        println!("   💡 Start with: codebuddy start");
        false
    };

    println!();

    // 2. Check configuration
    println!("⚙️  Configuration:");
    match AppConfig::load() {
        Ok(config) => {
            let config_path = std::path::Path::new(".codebuddy/config.json");
            println!("   ✅ Found at: {}", config_path.display());
            println!("   📋 Log level: {}", config.logging.level);
            println!("   📝 Log format: {:?}", config.logging.format);

            if let Some(fuse_config) = &config.fuse {
                println!("   ⚠️  FUSE enabled: {}", fuse_config.mount_point.display());
            }

            println!();
            println!("🔧 Configured LSP Servers:");
            for (idx, server) in config.lsp.servers.iter().enumerate() {
                let cmd = &server.command[0];
                let extensions = server.extensions.join(", ");
                let status = if command_exists(cmd) { "✅" } else { "❌" };

                println!("   {}. {} {}", idx + 1, status, cmd);
                println!("      Extensions: {}", extensions);

                if let Some(restart) = server.restart_interval {
                    println!("      Restart interval: {} minutes", restart);
                }

                if !command_exists(cmd) {
                    println!("      ⚠️  Command not found in PATH");
                }
            }
        }
        Err(e) => {
            println!("   ❌ Configuration error: {}", e);
            println!("   💡 Run: codebuddy setup");
        }
    }

    println!();

    // 3. Show all running codebuddy processes (helpful for debugging)
    println!("🔍 Running Codebuddy Processes:");
    match find_all_codebuddy_processes() {
        Ok(pids) => {
            if pids.is_empty() {
                println!("   ⭕ No codebuddy processes found");
            } else {
                for pid in pids {
                    let marker = if server_running
                        && pid_file.exists()
                        && std::fs::read_to_string(&pid_file)
                            .ok()
                            .and_then(|s| s.trim().parse::<u32>().ok())
                            == Some(pid)
                    {
                        " (managed)"
                    } else {
                        ""
                    };
                    println!("   • PID: {}{}", pid, marker);
                }
            }
        }
        Err(e) => {
            println!("   ⚠️  Could not list processes: {}", e);
        }
    }

    println!();
    println!("✨ Status check complete");
}

/// Handle the doctor command
async fn handle_doctor() {
    println!("🩺 Running Codebuddy Doctor...");
    println!();

    // 1. Check for and validate the configuration file
    print!("Checking for configuration file... ");
    match AppConfig::load() {
        Ok(config) => {
            println!("[✓] Found and parsed successfully.");
            println!();

            // 2. Check language servers
            println!("Checking language servers:");
            for server in &config.lsp.servers {
                let cmd = &server.command[0];
                print!(
                    "  - Checking for '{}' (for {})... ",
                    cmd,
                    server.extensions.join(", ")
                );
                if command_exists(cmd) {
                    println!("[✓] Found in PATH.");
                } else {
                    println!("[✗] Not found in PATH.");
                    println!(
                        "    > Please install '{}' and ensure it is available in your system's PATH.",
                        cmd
                    );
                }
            }
        }
        Err(e) => {
            println!("[✗] Error: {}", e);
            println!("  > Run `codebuddy setup` to create a new configuration file.");
        }
    }

    println!();
    println!("✨ Doctor's checkup complete.");
}

/// Helper to check if a command exists on the system's PATH
fn command_exists(cmd: &str) -> bool {
    std::process::Command::new(if cfg!(target_os = "windows") {
        "where"
    } else {
        "command"
    })
    .arg("-v")
    .arg(cmd)
    .stdout(std::process::Stdio::null())
    .stderr(std::process::Stdio::null())
    .status()
    .map_or(false, |status| status.success())
}

/// Handle the stop command
async fn handle_stop() {
    let pid_file = get_pid_file_path();

    if !pid_file.exists() {
        println!("ℹ️  No PID file found - server may not be running");
        return;
    }

    match std::fs::read_to_string(&pid_file) {
        Ok(pid_str) => {
            if let Ok(pid) = pid_str.trim().parse::<u32>() {
                if is_process_running(pid) {
                    println!("🛑 Stopping codebuddy server (PID: {})...", pid);
                    if terminate_process(pid) {
                        println!("✅ Successfully stopped codebuddy server");
                        let _ = std::fs::remove_file(&pid_file);
                    } else {
                        error!("Failed to stop server process");
                        process::exit(1);
                    }
                } else {
                    println!("ℹ️  Server process is not running (cleaning up PID file)");
                    let _ = std::fs::remove_file(&pid_file);
                }
            } else {
                error!("Invalid PID file format");
                process::exit(1);
            }
        }
        Err(e) => {
            error!(error = %e, "Failed to read PID file");
            process::exit(1);
        }
    }
}

/// Handle the link command (placeholder)
async fn handle_link() {
    println!("🔗 Link command not yet implemented");
    println!("   This will configure codebuddy to work with AI assistants");
}

/// Handle the unlink command (placeholder)
async fn handle_unlink() {
    println!("🔓 Unlink command not yet implemented");
    println!("   This will remove AI assistant configurations");
}

/// Acquire exclusive lock on PID file to prevent multiple instances
/// Returns a File handle that holds the lock for the lifetime of the process
fn acquire_pid_lock() -> Result<File, std::io::Error> {
    let pid_file_path = get_pid_file_path();

    // Ensure parent directory exists
    if let Some(parent) = pid_file_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Open or create the PID file
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&pid_file_path)?;

    // Try to acquire exclusive lock (non-blocking)
    match file.try_lock_exclusive() {
        Ok(()) => {
            // Successfully acquired lock - write our PID
            let pid = process::id();
            // Truncate and write PID
            file.set_len(0)?;
            use std::io::Write;
            let mut file_write = &file;
            write!(file_write, "{}", pid)?;
            file_write.flush()?;

            info!(path = %pid_file_path.display(), pid, "PID file locked");
            Ok(file)
        }
        Err(e) => {
            // Lock failed - another instance is running
            Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                format!("Failed to acquire lock: {}", e),
            ))
        }
    }
}

/// Get the path to the PID file
fn get_pid_file_path() -> PathBuf {
    #[cfg(unix)]
    {
        PathBuf::from("/tmp/codebuddy.pid")
    }
    #[cfg(windows)]
    {
        std::env::temp_dir().join("codebuddy.pid")
    }
}

/// Check if a process with the given PID is running
fn is_process_running(pid: u32) -> bool {
    #[cfg(unix)]
    {
        // On Unix systems, we can send signal 0 to check if process exists
        // SAFETY: Sending signal 0 to check process existence is safe:
        // - Signal 0 doesn't deliver a signal, only checks permissions and existence
        // - pid is validated as positive u32 before this call
        // - Return value indicates: true = process exists & accessible, false = doesn't exist or no permission
        // - No memory is accessed, only a kernel syscall that cannot cause undefined behavior
        // - Worst case: Returns false if pid invalid, no side effects
        unsafe { libc::kill(pid as i32, 0) == 0 }
    }
    #[cfg(windows)]
    {
        // On Windows, try to open the process handle
        use std::os::windows::process::ExitStatusExt;
        use std::process::Command;

        Command::new("tasklist")
            .args(&["/FI", &format!("PID eq {}", pid)])
            .output()
            .map(|output| String::from_utf8_lossy(&output.stdout).contains(&pid.to_string()))
            .unwrap_or(false)
    }
}

/// Find all running codebuddy processes
fn find_all_codebuddy_processes() -> Result<Vec<u32>, String> {
    #[cfg(unix)]
    {
        use std::process::Command;

        let output = Command::new("pgrep")
            .arg("codebuddy")
            .output()
            .map_err(|e| format!("Failed to run pgrep: {}", e))?;

        if !output.status.success() {
            return Ok(Vec::new());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let pids: Vec<u32> = stdout
            .lines()
            .filter_map(|line| line.trim().parse::<u32>().ok())
            .collect();

        Ok(pids)
    }
    #[cfg(windows)]
    {
        use std::process::Command;

        let output = Command::new("tasklist")
            .args(&["/FI", "IMAGENAME eq codebuddy.exe", "/FO", "CSV", "/NH"])
            .output()
            .map_err(|e| format!("Failed to run tasklist: {}", e))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let pids: Vec<u32> = stdout
            .lines()
            .filter_map(|line| {
                // CSV format: "codebuddy.exe","1234","Console","1","12,345 K"
                line.split(',')
                    .nth(1)
                    .and_then(|pid_str| pid_str.trim_matches('"').parse::<u32>().ok())
            })
            .collect();

        Ok(pids)
    }
}

/// Terminate a process with the given PID
fn terminate_process(pid: u32) -> bool {
    #[cfg(unix)]
    {
        // SAFETY: Sending SIGTERM is safe under these conditions:
        // - pid is validated as positive u32 from PID file that we created
        // - SIGTERM is a standard POSIX graceful shutdown signal
        // - Caller has verified process ownership via PID file location (temp dir)
        // - Kernel handles all permission checks; we only check return value
        // - No memory access, pure syscall with well-defined behavior
        // - Worst case: Signal delivery fails, we return false, no undefined behavior
        unsafe { libc::kill(pid as i32, libc::SIGTERM) == 0 }
    }
    #[cfg(windows)]
    {
        use std::process::Command;

        Command::new("taskkill")
            .args(&["/PID", &pid.to_string(), "/F"])
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }
}

/// Handle tools list command - list all available MCP tools
async fn handle_tools_command(format: &str) {
    let dispatcher = match crate::dispatcher_factory::create_initialized_dispatcher().await {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Error initializing: {}", e);
            process::exit(1);
        }
    };

    // Create MCP tools/list request
    use cb_core::model::mcp::{McpMessage, McpRequest};
    let request = McpRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(serde_json::json!(1)),
        method: "tools/list".to_string(),
        params: None,
    };

    match dispatcher.dispatch(McpMessage::Request(request)).await {
        Ok(McpMessage::Response(response)) => {
            if let Some(result) = response.result {
                match format {
                    "json" => println!("{}", serde_json::to_string_pretty(&result).unwrap()),
                    "names" => {
                        if let Some(tools) = result.get("tools").and_then(|t| t.as_array()) {
                            for tool in tools {
                                if let Some(name) = tool.get("name").and_then(|n| n.as_str()) {
                                    println!("{}", name);
                                }
                            }
                        }
                    }
                    _ => {
                        // Table format
                        println!("{:<30} {}", "TOOL NAME", "DESCRIPTION");
                        println!("{}", "=".repeat(80));
                        if let Some(tools) = result.get("tools").and_then(|t| t.as_array()) {
                            for tool in tools {
                                let name = tool
                                    .get("name")
                                    .and_then(|n| n.as_str())
                                    .unwrap_or("unknown");
                                let desc = tool
                                    .get("description")
                                    .and_then(|d| d.as_str())
                                    .unwrap_or("");
                                let desc_short = if desc.len() > 48 {
                                    format!("{}...", &desc[..45])
                                } else {
                                    desc.to_string()
                                };
                                println!("{:<30} {}", name, desc_short);
                            }
                        }
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("Error listing tools: {}", e);
            process::exit(1);
        }
        _ => {
            eprintln!("Unexpected response type");
            process::exit(1);
        }
    }
}

/// Handle the tool command - call MCP tool directly
async fn handle_tool_command(tool_name: &str, args_json: &str, format: &str) {
    // Parse JSON arguments
    let arguments: serde_json::Value = match serde_json::from_str(args_json) {
        Ok(val) => val,
        Err(e) => {
            let error = cb_core::model::mcp::McpError::invalid_request(format!(
                "Invalid JSON arguments: {}",
                e
            ));
            let api_error = cb_api::ApiError::from(error);
            output_error(&api_error, format);
            process::exit(1);
        }
    };

    // Initialize dispatcher via factory
    let dispatcher = match crate::dispatcher_factory::create_initialized_dispatcher().await {
        Ok(d) => d,
        Err(e) => {
            let error = cb_api::ApiError::internal(format!("Failed to initialize: {}", e));
            output_error(&error, format);
            process::exit(1);
        }
    };

    // Construct MCP request message
    use cb_core::model::mcp::{McpMessage, McpRequest};
    let params = serde_json::json!({
        "name": tool_name,
        "arguments": arguments,
    });
    let request = McpRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(serde_json::json!(1)),
        method: "tools/call".to_string(),
        params: Some(params),
    };
    let message = McpMessage::Request(request);

    // Execute tool call via dispatcher
    match dispatcher.dispatch(message).await {
        Ok(McpMessage::Response(response)) => {
            // Wait for async operations (like batch_execute) to complete
            let operation_queue = dispatcher.operation_queue();
            let start_time = std::time::Instant::now();
            let timeout = std::time::Duration::from_secs(30);

            loop {
                if operation_queue.is_idle().await {
                    break;
                }
                if start_time.elapsed() > timeout {
                    eprintln!("⚠️  Warning: Timed out waiting for operations to complete");
                    eprintln!("   Some operations may still be running in the background");
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            }

            if let Some(result) = response.result {
                output_result(&result, format);
            } else if let Some(error) = response.error {
                let api_error = cb_api::ApiError::from(error);
                output_error(&api_error, format);
                process::exit(1);
            }
        }
        Ok(_) => {
            eprintln!("Unexpected response type");
            process::exit(1);
        }
        Err(server_error) => {
            // Convert ServerError to ApiError and output to stderr
            let api_error = cb_api::ApiError::internal(server_error.to_string());
            output_error(&api_error, format);
            process::exit(1);
        }
    }
}

/// Output result to stdout based on format
fn output_result(result: &serde_json::Value, format: &str) {
    let output = match format {
        "compact" => serde_json::to_string(result).unwrap_or_else(|_| "{}".to_string()),
        _ => serde_json::to_string_pretty(result).unwrap_or_else(|_| "{}".to_string()),
    };
    println!("{}", output);
}

/// Output error to stderr based on format
fn output_error(error: &cb_api::ApiError, format: &str) {
    let error_json = serde_json::to_value(error).unwrap_or(serde_json::json!({
        "error": error.to_string()
    }));

    let output = match format {
        "compact" => serde_json::to_string(&error_json).unwrap_or_else(|_| "{}".to_string()),
        _ => serde_json::to_string_pretty(&error_json).unwrap_or_else(|_| "{}".to_string()),
    };
    eprintln!("{}", output);
}

/// Initialize tracing based on configuration
fn initialize_tracing(config: &AppConfig) {
    // Parse log level from config, with fallback to INFO
    let log_level = config.logging.level.parse().unwrap_or(tracing::Level::INFO);

    // Create env filter with configured level and allow env overrides
    let env_filter =
        tracing_subscriber::EnvFilter::from_default_env().add_directive(log_level.into());

    match config.logging.format {
        LogFormat::Json => {
            // Use JSON formatter for structured logging
            tracing_subscriber::registry()
                .with(env_filter)
                .with(fmt::layer().json())
                .init();
        }
        LogFormat::Pretty => {
            // Use pretty (human-readable) formatter
            tracing_subscriber::registry()
                .with(env_filter)
                .with(fmt::layer())
                .init();
        }
    }
}
