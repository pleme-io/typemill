//! CLI command handling for the mill server

mod conventions;
mod flag_parser;

use mill_client::format_plan;
use mill_transport::SessionInfo;
use clap::{Parser, Subcommand};
use mill_config::config::AppConfig;
use mill_foundation::core::utils::system::command_exists;
use mill_foundation::protocol::analysis_result::AnalysisResult;
use mill_foundation::protocol::refactor_plan::RefactorPlan;
use fs2::FileExt;
use std::fs::{File, OpenOptions};
use std::path::PathBuf;
use std::process;
use tracing::{error, info};

/// Parse JSON argument from string
#[allow(dead_code)]
fn parse_json(s: &str) -> Result<serde_json::Value, String> {
    serde_json::from_str(s).map_err(|e| format!("Invalid JSON: {}", e))
}

/// The main CLI struct.
#[derive(Parser)]
#[command(name = "mill")]
#[command(about = "Pure Rust MCP server bridging Language Server Protocol functionality")]
#[command(version)]
pub struct Cli {
    /// The command to run.
    #[command(subcommand)]
    pub command: Commands,
}

/// The available commands.
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
        #[arg(long, default_value = "3040")]
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
    ///
    /// Common tools:
    ///   rename  - Move/rename files and directories
    ///   move    - Move code symbols (functions/classes) between files
    ///   extract - Extract code into functions/variables
    ///   inline  - Inline variables/functions
    ///
    /// Scope examples (for rename operations):
    ///   --scope code        : Code only (minimal)
    ///   --scope standard    : Code + docs + configs (default, recommended)
    ///   --scope comments    : Standard + code comments
    ///   --scope everything  : Comments + markdown prose (most comprehensive)
    ///
    /// Use 'mill tools' to list all available tools.
    Tool {
        /// Tool name (e.g., "rename", "move", "find_definition")
        ///
        /// Important: 'move' is for moving CODE SYMBOLS (functions, classes).
        ///           For moving/renaming FILES, use 'rename' instead.
        tool_name: String,

        /// Tool arguments as JSON string (use "-" for stdin, required if not using flags)
        #[arg(required_unless_present_any = ["target", "source", "input_file"])]
        args: Option<String>,

        /// Read arguments from file
        #[arg(long, conflicts_with_all = ["args", "target", "source", "destination", "new_name", "name", "kind", "scope", "update_comments", "update_markdown_prose", "update_all"])]
        input_file: Option<String>,

        /// Output format (pretty or compact)
        #[arg(long, default_value = "pretty", value_parser = ["pretty", "compact"])]
        format: String,

        // === Common flags across refactoring tools ===
        // NOTE: Not all flags work with all tools. Tool-specific validation
        // will provide helpful errors if you use the wrong flags.

        /// Target (format: kind:path or kind:path:line:char)
        /// Used by: rename, inline, reorder, transform, delete
        #[arg(long, conflicts_with_all = ["args", "input_file"])]
        target: Option<String>,

        /// Source (format: path:line:char for code positions)
        /// Used by: move, extract
        #[arg(long, conflicts_with_all = ["args", "input_file"])]
        source: Option<String>,

        /// Destination (format: path or path:line:char)
        #[arg(long, conflicts_with_all = ["args", "input_file"])]
        destination: Option<String>,

        /// New name
        #[arg(long, conflicts_with_all = ["args", "input_file"])]
        new_name: Option<String>,

        /// Name for extracted element
        #[arg(long, conflicts_with_all = ["args", "input_file"])]
        name: Option<String>,

        /// Kind (e.g., "function", "variable", "imports", "to_async")
        #[arg(long, conflicts_with_all = ["args", "input_file"])]
        kind: Option<String>,

        /// Scope preset: "standard" (default - code/docs/configs), "code" (imports/strings only), "comments" (+ comments), "everything" (+ prose), "custom" (via --input-file)
        /// Deprecated: "all" (use "standard"), "code-only" (use "code")
        #[arg(long, conflicts_with_all = ["args", "input_file"])]
        scope: Option<String>,

        /// Update code comments (opt-in - may modify historical/explanatory comments)
        #[arg(long, conflicts_with_all = ["args", "input_file"])]
        update_comments: Option<bool>,

        /// Update markdown prose and inline code (opt-in - may modify code examples)
        #[arg(long, conflicts_with_all = ["args", "input_file"])]
        update_markdown_prose: Option<bool>,

        /// Enable all opt-in flags (comments + markdown prose)
        #[arg(long, conflicts_with_all = ["args", "input_file"])]
        update_all: bool,
    },
    /// List all public MCP tools (excludes internal tools)
    Tools {
        /// Output format (table, json, or names-only)
        #[arg(long, default_value = "table", value_parser = ["table", "json", "names"])]
        format: String,
    },
    /// Manage MCP server presets
    #[cfg(feature = "mcp-proxy")]
    #[command(subcommand)]
    Mcp(mill_client::McpCommands),
    /// Perform static analysis on the codebase
    Analyze(Analyze),
    /// Convert naming conventions in bulk (e.g., kebab-case to camelCase)
    ///
    /// Scans files matching the pattern and renames them according to the
    /// specified naming convention conversion. Uses batch rename internally.
    ///
    /// Examples:
    ///   mill convert-naming --from kebab-case --to camelCase --glob "src/**/*.js"
    ///   mill convert-naming --from snake_case --to camelCase --glob "**/*.ts" --target files
    ConvertNaming {
        /// Source naming convention (kebab-case, snake_case, camelCase, PascalCase)
        #[arg(long)]
        from: String,

        /// Target naming convention (kebab-case, snake_case, camelCase, PascalCase)
        #[arg(long)]
        to: String,

        /// Glob pattern to match files (e.g., "src/**/*.js")
        #[arg(long)]
        glob: String,

        /// What to rename: files, directories, or symbols
        #[arg(long, default_value = "files", value_parser = ["files", "directories", "symbols"])]
        target: String,

        /// Dry run - show what would be renamed without executing
        #[arg(long)]
        dry_run: bool,

        /// Output format (pretty or compact)
        #[arg(long, default_value = "pretty", value_parser = ["pretty", "compact"])]
        format: String,
    },
}

/// The analyze command.
#[derive(Parser)]
pub struct Analyze {
    /// The analyze subcommand to run.
    #[command(subcommand)]
    pub command: AnalyzeCommands,
}

/// The available analyze subcommands.
#[derive(Subcommand)]
pub enum AnalyzeCommands {
    /// Find dead code
    DeadCode(DeadCode),
    /// Find circular dependencies
    Cycles(Cycles),
}

/// The dead code command.
#[derive(Parser)]
pub struct DeadCode {
    /// Types of symbols to check
    #[arg(long, value_delimiter = ',', default_value = "all")]
    pub symbol_types: Vec<String>,
    /// Check public exports (aggressive mode)
    #[arg(long)]
    pub include_public: bool,
    /// The path to analyze
    #[arg(long, default_value = ".")]
    pub path: String,
}

/// The cycles command.
#[derive(Parser)]
pub struct Cycles {
    /// The path to analyze
    #[arg(long, default_value = ".")]
    pub path: String,
    /// Return error if cycles found (for CI/CD)
    #[arg(long)]
    pub fail_on_cycles: bool,
    /// Only report cycles with N or more modules
    #[arg(long, name = "min-size")]
    pub min_size: Option<usize>,
    /// Output format (pretty, json)
    #[arg(long, default_value = "pretty", value_parser = ["pretty", "json"])]
    pub format: String,
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
            mill_config::logging::initialize(&config);
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
                    eprintln!("❌ Error: TypeMill server is already running");
                    eprintln!("   Use 'mill stop' to stop the running instance first");
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
                    eprintln!("❌ Error: TypeMill server is already running");
                    eprintln!("   Use 'mill stop' to stop the running instance first");
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
            input_file,
            format,
            target,
            source,
            destination,
            new_name,
            name,
            kind,
            scope,
            update_comments,
            update_markdown_prose,
            update_all,
        } => {
            handle_tool_command(
                &tool_name,
                args.as_deref(),
                input_file.as_deref(),
                target.as_deref(),
                source.as_deref(),
                destination.as_deref(),
                new_name.as_deref(),
                name.as_deref(),
                kind.as_deref(),
                scope.as_deref(),
                update_comments,
                update_markdown_prose,
                update_all,
                &format,
            )
            .await;
        }
        Commands::Tools { format } => {
            handle_tools_command(&format).await;
        }
        #[cfg(feature = "mcp-proxy")]
        Commands::Mcp(mcp_command) => {
            handle_mcp_command(mcp_command).await;
        }
        Commands::Analyze(analyze_command) => {
            handle_analyze_command(analyze_command).await;
        }
        Commands::ConvertNaming {
            from,
            to,
            glob,
            target,
            dry_run,
            format,
        } => {
            handle_convert_naming(&from, &to, &glob, &target, dry_run, &format).await;
        }
    }
}

#[cfg(feature = "mcp-proxy")]
async fn handle_mcp_command(command: mill_client::McpCommands) {
    let result = match command {
        mill_client::McpCommands::List => mill_client::commands::mcp::list_presets(),
        mill_client::McpCommands::Add { preset_id } => {
            mill_client::commands::mcp::add_preset(&preset_id)
        }
        mill_client::McpCommands::Remove { preset_id } => {
            mill_client::commands::mcp::remove_preset(&preset_id)
        }
        mill_client::McpCommands::Info { preset_id } => {
            mill_client::commands::mcp::info_preset(&preset_id)
        }
    };

    if let Err(e) = result {
        error!(error = %e, "MCP command failed");
        process::exit(1);
    }
}

async fn handle_analyze_command(command: Analyze) {
    match command.command {
        AnalyzeCommands::DeadCode(dead_code_command) => {
            handle_dead_code_command(dead_code_command).await;
        }
        AnalyzeCommands::Cycles(cycles_command) => {
            handle_cycles_command(cycles_command).await;
        }
    }
}

fn output_pretty_cycles(analysis_result: &AnalysisResult) {
    if analysis_result.summary.total_findings == 0 {
        println!("✅ No circular dependencies found.");
        return;
    }

    println!(
        "Found {} circular dependencies:",
        analysis_result.summary.total_findings
    );
    println!();

    for (i, finding) in analysis_result.findings.iter().enumerate() {
        if let Some(metrics) = &finding.metrics {
            if let Some(cycle_path_val) = metrics.get("cycle_path") {
                if let Some(cycle_path) = cycle_path_val.as_array() {
                    println!("Cycle {} ({} modules):", i + 1, cycle_path.len());
                    for module in cycle_path {
                        println!("  → {}", module.as_str().unwrap_or(""));
                    }
                    if let Some(first_module) = cycle_path.get(0) {
                        println!("  → {}", first_module.as_str().unwrap_or(""));
                    }
                    println!();
                }
            }
        }
    }
}

async fn handle_cycles_command(command: Cycles) {
    let mut args = serde_json::json!({
        "scope": {
            "scope_type": "workspace",
            "path": command.path,
        },
    });

    if let Some(min_size) = command.min_size {
        if let Some(obj) = args.as_object_mut() {
            obj.insert("min_size".to_string(), serde_json::json!(min_size));
        }
    }

    let dispatcher = match crate::dispatcher_factory::create_initialized_dispatcher().await {
        Ok(d) => d,
        Err(e) => {
            let error = mill_foundation::protocol::ApiError::internal(format!(
                "Failed to initialize: {}",
                e
            ));
            output_error(&error, &command.format);
            process::exit(1);
        }
    };

    use mill_foundation::core::model::mcp::{ McpMessage , McpRequest };
    let params = serde_json::json!({
        "name": "analyze.circular_dependencies",
        "arguments": args,
    });
    let request = McpRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(serde_json::json!(1)),
        method: "tools/call".to_string(),
        params: Some(params),
    };
    let message = McpMessage::Request(request);
    let session_info = SessionInfo::default();

    match dispatcher.dispatch(message, &session_info).await {
        Ok(McpMessage::Response(response)) => {
            if let Some(result) = response.result {
                if let Ok(analysis_result) =
                    serde_json::from_value::<AnalysisResult>(result.clone())
                {
                    if command.format == "pretty" {
                        output_pretty_cycles(&analysis_result);
                    } else {
                        let output = serde_json::to_string_pretty(&analysis_result).unwrap();
                        println!("{}", output);
                    }

                    if command.fail_on_cycles && analysis_result.summary.total_findings > 0 {
                        eprintln!("\n❌ Error: Circular dependencies detected.");
                        process::exit(1);
                    }
                } else {
                    output_result(&result, &command.format);
                }
            } else if let Some(error) = response.error {
                let api_error = mill_foundation::protocol::ApiError::from(error);
                output_error(&api_error, &command.format);
                process::exit(1);
            }
        }
        Ok(_) => {
            eprintln!("Unexpected response type");
            process::exit(1);
        }
        Err(server_error) => {
            let api_error =
                mill_foundation::protocol::ApiError::internal(server_error.to_string());
            output_error(&api_error, &command.format);
            process::exit(1);
        }
    }
}

async fn handle_dead_code_command(command: DeadCode) {
    let args = serde_json::json!({
        "kind": "deep",
        "scope": {
            "scope_type": "workspace",
            "path": command.path,
        },
        "check_public_exports": command.include_public,
        "symbol_types": command.symbol_types,
    });
    let args_json = serde_json::to_string(&args).unwrap();
    handle_tool_command(
        "analyze.dead_code",
        Some(&args_json),
        None, // input_file
        None, // target
        None, // source
        None, // destination
        None, // new_name
        None, // name
        None, // kind
        None, // scope
        None, // update_comments
        None, // update_markdown_prose
        false, // update_all
        "pretty",
    )
    .await;
}

/// Handle the setup command
async fn handle_setup() {
    println!("🚀 Setting up mill configuration...");

    // Create default configuration
    let config = AppConfig::default();

    // Determine config file path
    let config_path = std::path::Path::new(".typemill/config.json");

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
            println!("   • Rust: rust-analyzer");
            println!();
            println!("ℹ️  Note: Language support temporarily reduced to TypeScript + Rust");
            println!("   during unified API refactoring. Python/Go/Java support available");
            println!("   in git tag 'pre-language-reduction'");
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
    println!("📊 TypeMill Status\n");

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
        println!("   💡 Start with: mill start");
        false
    };

    println!();

    // 2. Check configuration
    println!("⚙️  Configuration:");
    match AppConfig::load() {
        Ok(config) => {
            let config_path = std::path::Path::new(".typemill/config.json");
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
            println!("   💡 Run: mill setup");
        }
    }

    println!();

    // 3. Show all running mill processes (helpful for debugging)
    println!("🔍 Running TypeMill Processes:");
    match find_all_mill_processes() {
        Ok(pids) => {
            if pids.is_empty() {
                println!("   ⭕ No mill processes found");
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
    println!("🩺 Running TypeMill Doctor...");
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
            println!("  > Run `mill setup` to create a new configuration file.");
        }
    }

    println!();
    println!("✨ Doctor's checkup complete.");
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
                    println!("🛑 Stopping mill server (PID: {})...", pid);
                    if terminate_process(pid) {
                        println!("✅ Successfully stopped mill server");
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
    println!("   This will configure mill to work with AI assistants");
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
    // Allow tests to override PID file location to avoid conflicts
    if let Ok(pid_file) = std::env::var("MILL_PID_FILE") {
        return PathBuf::from(pid_file);
    }

    #[cfg(unix)]
    {
        PathBuf::from("/tmp/mill.pid")
    }
    #[cfg(windows)]
    {
        std::env::temp_dir().join("mill.pid")
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

/// Find all running mill processes
fn find_all_mill_processes() -> Result<Vec<u32>, String> {
    #[cfg(unix)]
    {
        use std::process::Command;

        let output = Command::new("pgrep")
            .arg("mill")
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
            .args(&["/FI", "IMAGENAME eq mill.exe", "/FO", "CSV", "/NH"])
            .output()
            .map_err(|e| format!("Failed to run tasklist: {}", e))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let pids: Vec<u32> = stdout
            .lines()
            .filter_map(|line| {
                // CSV format: "mill.exe","1234","Console","1","12,345 K"
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

    // Get public tool list with handlers (excludes internal tools)
    let registry = dispatcher.tool_registry.lock().await;
    let tools_with_handlers = registry.list_public_tools_with_handlers();
    drop(registry); // Release lock

    match format {
        "json" => {
            let json_output: Vec<serde_json::Value> = tools_with_handlers
                .iter()
                .map(|(name, handler)| {
                    serde_json::json!({
                        "name": name,
                        "handler": handler
                    })
                })
                .collect();
            println!("{}", serde_json::to_string_pretty(&json_output).unwrap());
        }
        "names" => {
            for (name, _) in &tools_with_handlers {
                println!("{}", name);
            }
        }
        _ => {
            // Table format with handler information
            println!("┌{0:─<32}┬{0:─<20}┐", "");
            println!("│ {:<30} │ {:<18} │", "TOOL NAME", "HANDLER");
            println!("├{0:─<32}┼{0:─<20}┤", "");

            for (tool_name, handler_name) in &tools_with_handlers {
                println!("│ {:<30} │ {:<18} │", tool_name, handler_name);
            }

            println!("└{0:─<32}┴{0:─<20}┘", "");
            println!();
            println!(
                "Public tools: {} across {} handlers",
                tools_with_handlers.len(),
                tools_with_handlers
                    .iter()
                    .map(|(_, h)| h)
                    .collect::<std::collections::HashSet<_>>()
                    .len()
            );
            println!("(Internal tools hidden - 20 backend-only tools not shown)");
        }
    }
}

/// Handle the tool command - call MCP tool directly
async fn handle_tool_command(
    tool_name: &str,
    args_json: Option<&str>,
    input_file: Option<&str>,
    target: Option<&str>,
    source: Option<&str>,
    destination: Option<&str>,
    new_name: Option<&str>,
    name: Option<&str>,
    kind: Option<&str>,
    scope: Option<&str>,
    update_comments: Option<bool>,
    update_markdown_prose: Option<bool>,
    update_all: bool,
    format: &str,
) {
    use std::collections::HashMap;
    use std::io::{self, Read};

    // Build arguments from either JSON, file, stdin, or flags
    let arguments: serde_json::Value = if let Some(file_path) = input_file {
        // Read JSON from file
        let json = match std::fs::read_to_string(file_path) {
            Ok(content) => content,
            Err(e) => {
                let error = mill_foundation::core::model::mcp::McpError::invalid_request(
                    format!("Failed to read input file '{}': {}", file_path, e),
                );
                let api_error = mill_foundation::protocol::ApiError::from(error);
                output_error(&api_error, format);
                process::exit(1);
            }
        };
        match serde_json::from_str(&json) {
            Ok(val) => val,
            Err(e) => {
                let error = mill_foundation::core::model::mcp::McpError::invalid_request(
                    format!("Invalid JSON in file '{}': {}", file_path, e),
                );
                let api_error = mill_foundation::protocol::ApiError::from(error);
                output_error(&api_error, format);
                process::exit(1);
            }
        }
    } else if let Some(json) = args_json {
        // Check if args is "-" for stdin
        if json == "-" {
            // Read JSON from stdin
            let mut stdin_content = String::new();
            if let Err(e) = io::stdin().read_to_string(&mut stdin_content) {
                let error = mill_foundation::core::model::mcp::McpError::invalid_request(
                    format!("Failed to read from stdin: {}", e),
                );
                let api_error = mill_foundation::protocol::ApiError::from(error);
                output_error(&api_error, format);
                process::exit(1);
            }
            match serde_json::from_str(&stdin_content) {
                Ok(val) => val,
                Err(e) => {
                    let error = mill_foundation::core::model::mcp::McpError::invalid_request(
                        format!("Invalid JSON from stdin: {}", e),
                    );
                    let api_error = mill_foundation::protocol::ApiError::from(error);
                    output_error(&api_error, format);
                    process::exit(1);
                }
            }
        } else {
            // Use JSON string directly
            match serde_json::from_str(json) {
                Ok(val) => val,
                Err(e) => {
                    let error = mill_foundation::core::model::mcp::McpError::invalid_request(
                        format!("Invalid JSON arguments: {}", e),
                    );
                    let api_error = mill_foundation::protocol::ApiError::from(error);
                    output_error(&api_error, format);
                    process::exit(1);
                }
            }
        }
    } else {
        // Build from flags using flag_parser
        let mut flags = HashMap::new();
        if let Some(v) = target {
            flags.insert("target".to_string(), v.to_string());
        }
        if let Some(v) = source {
            flags.insert("source".to_string(), v.to_string());
        }
        if let Some(v) = destination {
            flags.insert("destination".to_string(), v.to_string());
        }
        if let Some(v) = new_name {
            flags.insert("new_name".to_string(), v.to_string());
        }
        if let Some(v) = name {
            flags.insert("name".to_string(), v.to_string());
        }
        if let Some(v) = kind {
            flags.insert("kind".to_string(), v.to_string());
        }
        if let Some(v) = scope {
            flags.insert("scope".to_string(), v.to_string());
        }

        // Handle update flags (opt-in flags only - scope presets handle defaults)
        if update_all {
            flags.insert("update_all".to_string(), "true".to_string());
        }
        if let Some(v) = update_comments {
            flags.insert("update_comments".to_string(), v.to_string());
        }
        if let Some(v) = update_markdown_prose {
            flags.insert("update_markdown_prose".to_string(), v.to_string());
        }

        match flag_parser::parse_flags_to_json(tool_name, flags) {
            Ok(json) => json,
            Err(e) => {
                let error = mill_foundation::protocol::ApiError::InvalidRequest(format!(
                    "Invalid flag arguments: {}",
                    e
                ));
                output_error(&error, format);
                process::exit(1);
            }
        }
    };

    // Initialize dispatcher via factory
    let dispatcher = match crate::dispatcher_factory::create_initialized_dispatcher().await {
        Ok(d) => d,
        Err(e) => {
            let error = mill_foundation::protocol::ApiError::internal(format!(
                "Failed to initialize: {}",
                e
            ));
            output_error(&error, format);
            process::exit(1);
        }
    };

    // Construct MCP request message
    use mill_foundation::core::model::mcp::{ McpMessage , McpRequest };
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
    let session_info = SessionInfo::default();

    // Execute tool call via dispatcher
    match dispatcher.dispatch(message, &session_info).await {
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
                let api_error = mill_foundation::protocol::ApiError::from(error);
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
            let api_error =
                mill_foundation::protocol::ApiError::internal(server_error.to_string());
            output_error(&api_error, format);
            process::exit(1);
        }
    }
}

/// Handle convert-naming command - bulk rename files based on naming convention
async fn handle_convert_naming(
    from_convention: &str,
    to_convention: &str,
    glob_pattern: &str,
    target_type: &str,
    dry_run: bool,
    format: &str,
) {
    use mill_foundation::core::model::mcp::{ McpMessage , McpRequest };
    use glob::glob;
    use serde_json::json;

    // Scan files matching glob pattern
    let matches: Vec<String> = match glob(glob_pattern) {
        Ok(paths) => paths
            .filter_map(|p| p.ok())
            .filter_map(|p| p.to_str().map(String::from))
            .collect(),
        Err(e) => {
            let error = mill_foundation::protocol::ApiError::InvalidRequest(format!(
                "Invalid glob pattern '{}': {}",
                glob_pattern, e
            ));
            output_error(&error, format);
            process::exit(1);
        }
    };

    if matches.is_empty() {
        eprintln!("No files matched pattern: {}", glob_pattern);
        eprintln!("Tip: Use quotes around glob patterns: \"src/**/*.js\"");
        process::exit(1);
    }

    // Generate targets array by converting each filename
    let mut targets = Vec::new();
    let mut skipped = Vec::new();

    for file_path in &matches {
        // Extract just the filename (not the full path)
        let path = std::path::Path::new(file_path);
        let filename = path.file_name().and_then(|n| n.to_str());

        if let Some(fname) = filename {
            // Try to convert the filename
            if let Some(new_filename) = conventions::convert_filename(fname, from_convention, to_convention) {
                // Skip if no change
                if fname == new_filename {
                    skipped.push(file_path.clone());
                    continue;
                }

                // Build new path with converted filename
                let new_path = if let Some(parent) = path.parent() {
                    parent.join(&new_filename)
                } else {
                    std::path::PathBuf::from(&new_filename)
                };

                targets.push(json!({
                    "kind": target_type,
                    "path": file_path,
                    "new_name": new_path.to_str().unwrap(),
                }));
            } else {
                skipped.push(file_path.clone());
            }
        }
    }

    if targets.is_empty() {
        eprintln!("✅ No files need renaming (all already match target convention)");
        eprintln!("   Files checked: {}", matches.len());
        process::exit(0);
    }

    // Show preview
    println!("🔄 Converting {} {} from {} to {}", targets.len(), target_type, from_convention, to_convention);
    println!();
    for target in &targets {
        println!("  {} → {}",
            target["path"].as_str().unwrap(),
            target["new_name"].as_str().unwrap()
        );
    }
    if !skipped.is_empty() {
        println!();
        println!("⏭️  Skipped {} files (no change needed)", skipped.len());
    }
    println!();

    if dry_run {
        println!("🔍 Dry run complete (use without --dry-run to execute)");
        return;
    }

    // Call batch rename via dispatcher
    let dispatcher = match crate::dispatcher_factory::create_initialized_dispatcher().await {
        Ok(d) => d,
        Err(e) => {
            let error = mill_foundation::protocol::ApiError::internal(format!(
                "Failed to initialize: {}",
                e
            ));
            output_error(&error, format);
            process::exit(1);
        }
    };

    // Build rename request with batch targets
    let arguments = json!({
        "targets": targets,
        "options": {
            "scope": "all"  // Update imports, string literals, docs, configs
        }
    });

    let params = json!({
        "name": "rename",
        "arguments": arguments,
    });

    let request = McpRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(json!(1)),
        method: "tools/call".to_string(),
        params: Some(params),
    };

    let message = McpMessage::Request(request);
    let session_info = SessionInfo::default();

    // Execute rename plan
    println!("📝 Generating rename plan...");
    match dispatcher.dispatch(message, &session_info).await {
        Ok(McpMessage::Response(response)) => {
            if let Some(result) = response.result {
                if let Some(content) = result.get("content") {
                    // Got the plan, now apply it
                    println!("✅ Plan generated");
                    println!();

                    // Apply the plan
                    let apply_params = json!({
                        "name": "DELETED_TOOL",
                        "arguments": {
                            "plan": content,
                            "options": {
                                "dry_run": false
                            }
                        }
                    });

                    let apply_request = McpRequest {
                        jsonrpc: "2.0".to_string(),
                        id: Some(json!(2)),
                        method: "tools/call".to_string(),
                        params: Some(apply_params),
                    };

                    println!("🚀 Applying renames...");
                    match dispatcher.dispatch(McpMessage::Request(apply_request), &session_info).await {
                        Ok(McpMessage::Response(apply_response)) => {
                            if apply_response.error.is_some() {
                                eprintln!("❌ Failed to apply renames");
                                output_error(
                                    &mill_foundation::protocol::ApiError::internal(
                                        format!("{:?}", apply_response.error)
                                    ),
                                    format
                                );
                                process::exit(1);
                            } else {
                                println!("✅ Successfully renamed {} files!", targets.len());
                            }
                        }
                        Ok(_) => {
                            eprintln!("❌ Unexpected response type from apply_edit");
                            process::exit(1);
                        }
                        Err(e) => {
                            eprintln!("❌ Error applying renames: {}", e);
                            process::exit(1);
                        }
                    }
                } else {
                    eprintln!("❌ Plan response missing content");
                    process::exit(1);
                }
            } else if let Some(error) = response.error {
                eprintln!("❌ Failed to generate plan: {:?}", error);
                process::exit(1);
            }
        }
        Ok(_) => {
            eprintln!("❌ Unexpected response type");
            process::exit(1);
        }
        Err(e) => {
            eprintln!("❌ Error: {}", e);
            process::exit(1);
        }
    }
}

/// Output result to stdout based on format
fn output_result(result: &serde_json::Value, format: &str) {
    // Check if this is a refactor plan by looking for plan_type field
    let is_plan = result.get("plan_type").is_some();

    // For pretty format, show human-readable summary for plans
    if format != "compact" && is_plan {
        // Try to deserialize as RefactorPlan
        if let Ok(plan) = serde_json::from_value::<RefactorPlan>(result.clone()) {
            let description = format_plan(&plan);

            // Print human-readable summary with visual separator
            println!("📋 {}", description);
            println!();
        }
    }

    // Always output full JSON (for programmatic use and non-plans)
    let output = match format {
        "compact" => serde_json::to_string(result).unwrap_or_else(|_| "{}".to_string()),
        _ => serde_json::to_string_pretty(result).unwrap_or_else(|_| "{}".to_string()),
    };
    println!("{}", output);
}

/// Output error to stderr based on format
fn output_error(error: &mill_foundation::protocol::ApiError, format: &str) {
    let error_json = serde_json::to_value(error).unwrap_or(serde_json::json!({
        "error": error.to_string()
    }));

    let output = match format {
        "compact" => serde_json::to_string(&error_json).unwrap_or_else(|_| "{}".to_string()),
        _ => serde_json::to_string_pretty(&error_json).unwrap_or_else(|_| "{}".to_string()),
    };
    eprintln!("{}", output);
}