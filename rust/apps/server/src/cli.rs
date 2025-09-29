//! CLI command handling for the codebuddy server

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process;
use cb_core::config::AppConfig;

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
}

/// Main CLI entry point
pub async fn run() {
    // Parse CLI arguments first
    let cli = Cli::parse();

    // Only initialize tracing for server commands
    match &cli.command {
        Commands::Start { .. } | Commands::Serve { .. } => {
            tracing_subscriber::fmt()
                .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
                .init();
        }
        _ => {
            // For other commands, we want direct console output
        }
    }

    match cli.command {
        Commands::Start { daemon } => {
            if daemon {
                write_pid_file().unwrap_or_else(|e| {
                    eprintln!("❌ Failed to write PID file: {}", e);
                    process::exit(1);
                });
            }
            crate::run_stdio_mode().await;
        }
        Commands::Serve { daemon, port } => {
            if daemon {
                write_pid_file().unwrap_or_else(|e| {
                    eprintln!("❌ Failed to write PID file: {}", e);
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
        println!("⚠️  Configuration file already exists at: {}", config_path.display());
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
            println!("💡 You can edit {} to customize LSP servers and other settings.", config_path.display());
        }
        Err(e) => {
            eprintln!("❌ Failed to save configuration: {}", e);
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
                    println!("❌ Invalid PID file format");
                }
            }
            Err(e) => {
                eprintln!("❌ Failed to read PID file: {}", e);
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
                        eprintln!("❌ Failed to stop server process");
                        process::exit(1);
                    }
                } else {
                    println!("ℹ️  Server process is not running (cleaning up PID file)");
                    let _ = std::fs::remove_file(&pid_file);
                }
            } else {
                eprintln!("❌ Invalid PID file format");
                process::exit(1);
            }
        }
        Err(e) => {
            eprintln!("❌ Failed to read PID file: {}", e);
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

    println!("📝 PID file written: {} (PID: {})", pid_file.display(), pid);
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
        unsafe {
            libc::kill(pid as i32, 0) == 0
        }
    }
    #[cfg(windows)]
    {
        // On Windows, try to open the process handle
        use std::os::windows::process::ExitStatusExt;
        use std::process::Command;

        Command::new("tasklist")
            .args(&["/FI", &format!("PID eq {}", pid)])
            .output()
            .map(|output| {
                String::from_utf8_lossy(&output.stdout)
                    .contains(&pid.to_string())
            })
            .unwrap_or(false)
    }
}

/// Terminate a process with the given PID
fn terminate_process(pid: u32) -> bool {
    #[cfg(unix)]
    {
        unsafe {
            libc::kill(pid as i32, libc::SIGTERM) == 0
        }
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