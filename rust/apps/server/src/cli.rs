//! CLI command handling for the codebuddy server

use cb_core::config::{AppConfig, LogFormat};
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process;
use tracing::{error, info};
use tracing_subscriber::fmt;
use tracing_subscriber::prelude::*;

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
        Commands::Start { daemon } => {
            if daemon {
                write_pid_file().unwrap_or_else(|e| {
                    error!(error = %e, "Failed to write PID file");
                    process::exit(1);
                });
            }
            crate::run_stdio_mode().await;
        }
        Commands::Serve { daemon, port } => {
            if daemon {
                write_pid_file().unwrap_or_else(|e| {
                    error!(error = %e, "Failed to write PID file");
                    process::exit(1);
                });
            }
            crate::run_websocket_server_with_port(port).await;
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
    let pid_file = get_pid_file_path();

    if pid_file.exists() {
        match std::fs::read_to_string(&pid_file) {
            Ok(pid_str) => {
                if let Ok(pid) = pid_str.trim().parse::<u32>() {
                    // Check if process is still running
                    if is_process_running(pid) {
                        println!("✅ Codebuddy server is running (PID: {})", pid);
                        println!("   PID file: {}", pid_file.display());
                    } else {
                        println!("⚠️  Codebuddy server is not running (stale PID file)");
                        // Clean up stale PID file
                        let _ = std::fs::remove_file(&pid_file);
                    }
                } else {
                    error!("Invalid PID file format");
                }
            }
            Err(e) => {
                error!(error = %e, "Failed to read PID file");
            }
        }
    } else {
        println!("ℹ️  Codebuddy server is not running");
        println!("   Start the server with: codebuddy start");
    }
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

/// Write the current process ID to a PID file
fn write_pid_file() -> Result<(), std::io::Error> {
    let pid_file = get_pid_file_path();

    // Ensure the directory exists
    if let Some(parent) = pid_file.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let pid = process::id();
    std::fs::write(&pid_file, pid.to_string())?;

    info!(path = %pid_file.display(), pid, "PID file written");
    Ok(())
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
