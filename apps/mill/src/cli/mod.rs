//! CLI command handling for the mill server

mod conventions;
mod docs;
mod flag_parser;
mod lsp_helpers;
mod user_input;

use clap::{Parser, Subcommand};
use fs2::FileExt;
use mill_client::format_plan;
use mill_config::config::AppConfig;
use mill_foundation::core::utils::system::command_exists;
use mill_foundation::errors::MillError;
use mill_foundation::planning::RefactorPlan;
use mill_transport::SessionInfo;
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
    Setup {
        /// Update existing config (don't fail if it exists)
        #[arg(long)]
        update: bool,

        /// Interactive mode - prompt for choices
        #[arg(long)]
        interactive: bool,
    },
    /// Stop the running server
    Stop,
    /// Link to AI assistants
    Link,
    /// Remove AI from config
    Unlink,
    /// Check client configuration and diagnose potential problems
    Doctor,
    /// Install LSP server for a specific language
    InstallLsp {
        /// Language name (e.g., "rust", "typescript", "python")
        language: String,
    },
    /// Manage the daemon (persistent LSP server for faster tool calls)
    ///
    /// The daemon keeps LSP servers running between CLI invocations,
    /// eliminating startup overhead for repeated tool calls.
    ///
    /// Examples:
    ///   mill daemon start      # Start daemon in background
    ///   mill daemon stop       # Stop running daemon
    ///   mill daemon status     # Check if daemon is running
    #[cfg(unix)]
    #[command(subcommand)]
    Daemon(DaemonCommands),
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
    /// View embedded documentation
    ///
    /// Examples:
    ///   mill docs                  # List all available docs
    ///   mill docs setup            # Show setup guide
    ///   mill docs tools            # Show tools documentation
    ///   mill docs architecture     # Show architecture overview
    Docs {
        /// Specific document to view (optional - shows list if omitted)
        topic: Option<String>,

        /// Show raw markdown instead of rendered output
        #[arg(long)]
        raw: bool,

        /// Search docs for a keyword
        #[arg(long)]
        search: Option<String>,
    },
    /// Manage MCP server presets
    #[cfg(feature = "mcp-proxy")]
    #[command(subcommand)]
    Mcp(mill_client::McpCommands),
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

/// Daemon management subcommands
#[cfg(unix)]
#[derive(Subcommand)]
pub enum DaemonCommands {
    /// Start the daemon in the background
    Start {
        /// Keep daemon in foreground (don't daemonize)
        #[arg(long)]
        foreground: bool,
    },
    /// Stop the running daemon
    Stop,
    /// Check daemon status
    Status,
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
        Commands::Setup {
            update,
            interactive,
        } => {
            handle_setup(update, interactive).await;
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
        Commands::InstallLsp { language } => {
            handle_install_lsp(&language).await;
        }
        #[cfg(unix)]
        Commands::Daemon(daemon_command) => {
            handle_daemon_command(daemon_command).await;
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
        Commands::Docs { topic, raw, search } => {
            docs::show_docs(topic, raw, search);
        }
        #[cfg(feature = "mcp-proxy")]
        Commands::Mcp(mcp_command) => {
            handle_mcp_command(mcp_command).await;
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

/// Handle the setup command
async fn handle_setup(update: bool, interactive: bool) {
    println!("🚀 Setting up mill configuration...");

    let config_path = std::path::Path::new(".typemill/config.json");

    // Load existing config or create new one
    let mut config = if config_path.exists() {
        if !update {
            println!(
                "⚠️  Configuration file already exists at: {}",
                config_path.display()
            );
            println!("   Use `mill setup --update` to update it, or delete it to start fresh.");
            return;
        }

        println!("📝 Updating existing configuration...");
        match AppConfig::load() {
            Ok(cfg) => cfg,
            Err(e) => {
                error!(error = %e, "Failed to load config");
                eprintln!("❌ Failed to load config: {}", e);
                process::exit(1);
            }
        }
    } else {
        println!("📝 Creating new configuration...");
        AppConfig::default()
    };

    // Auto-detect and offer to install LSPs
    println!();
    println!("🔍 Detecting project languages...");

    let needed_lsps = match lsp_helpers::detect_needed_lsps(std::path::Path::new(".")) {
        Ok(lsps) => lsps,
        Err(e) => {
            error!(error = %e, "Failed to detect needed LSPs");
            eprintln!("⚠️  Warning: Could not detect languages ({})", e);
            vec![]
        }
    };

    if needed_lsps.is_empty() {
        println!("   No project languages detected in current directory");
    } else {
        println!("   Detected: {}", needed_lsps.join(", "));
    }

    // Detect TypeScript root directory if TypeScript is needed
    if needed_lsps.contains(&"typescript".to_string()) {
        println!();
        println!("🔍 Detecting TypeScript project root...");
        if let Some(ts_root) = lsp_helpers::detect_typescript_root(std::path::Path::new(".")) {
            println!("   Found: {}/", ts_root.display());

            // Update the TypeScript server config with rootDir
            if let Some(server) = config
                .lsp
                .servers
                .iter_mut()
                .find(|s| s.extensions.contains(&"ts".to_string()))
            {
                server.root_dir = Some(ts_root);
                println!("   ✅ Set rootDir for TypeScript LSP");
            }
        } else {
            println!("   ⚠️  No TypeScript project found (no tsconfig.json or package.json)");
        }
    }

    // Save configuration
    match config.save(config_path) {
        Ok(()) => {
            println!();
            println!("✅ Configuration saved to: {}", config_path.display());
        }
        Err(e) => {
            error!(error = %e, "Failed to save configuration");
            eprintln!("❌ Failed to save configuration: {}", e);
            process::exit(1);
        }
    }

    // Check which LSPs are already installed
    if !needed_lsps.is_empty() {
        println!();
        println!("🔍 Checking installed LSP servers...");

        let mut missing_lsps = Vec::new();
        for lang_name in &needed_lsps {
            match lsp_helpers::check_lsp_installed(lang_name).await {
                Ok(Some(_path)) => {
                    println!("   ✅ {} - already installed", lang_name);
                }
                Ok(None) => {
                    println!("   📥 {} - not installed", lang_name);
                    missing_lsps.push(lang_name.clone());
                }
                Err(e) => {
                    error!(error = %e, lang_name, "Failed to check LSP status");
                    println!("   ⚠️  {} - status unknown", lang_name);
                }
            }
        }

        // Offer to install missing LSPs if interactive
        if !missing_lsps.is_empty() {
            if interactive && !mill_foundation::core::utils::system::is_ci() {
                println!();
                match user_input::read_yes_no(
                    &format!(
                        "📥 Install {} missing LSP server(s)? [Y/n]",
                        missing_lsps.len()
                    ),
                    true,
                ) {
                    Ok(true) => {
                        println!();
                        println!("📦 Installing LSP servers...");

                        for lang_name in &missing_lsps {
                            print!("   Installing {}... ", lang_name);
                            match lsp_helpers::install_lsp(lang_name).await {
                                Ok(path) => {
                                    println!("✅ {}", path.display());

                                    // Update config after install
                                    if let Err(e) = lsp_helpers::update_config_after_install(
                                        lang_name,
                                        &path,
                                        interactive,
                                    )
                                    .await
                                    {
                                        eprintln!("      ⚠️  Config update failed: {}", e);
                                    }
                                }
                                Err(e) => {
                                    println!("❌");
                                    error!(error = %e, lang_name, "Failed to install LSP");
                                    eprintln!("      Error: {}", e);
                                }
                            }
                        }

                        println!();
                        println!("✅ LSP installation complete!");
                    }
                    Ok(false) => {
                        println!();
                        println!("⏭️  Skipped LSP installation");
                        print_install_commands(&missing_lsps);
                    }
                    Err(e) => {
                        error!(error = %e, "Failed to read input");
                        print_install_commands(&missing_lsps);
                    }
                }
            } else {
                println!();
                if mill_foundation::core::utils::system::is_ci() {
                    println!("ℹ️  CI detected - skipping interactive installation");
                }
                print_install_commands(&missing_lsps);
            }
        }
    }

    println!();
    println!("✨ Setup complete!");
    println!("   Run `mill doctor` to verify configuration");
    println!("   Run `mill start` to start the server");
}

/// Print install commands for missing LSPs
fn print_install_commands(missing_lsps: &[String]) {
    println!("   You can install them later with:");
    for lang_name in missing_lsps {
        println!("   mill install-lsp {}", lang_name);
    }
}

/// Handle the status command
async fn handle_status() {
    use mill_client::formatting::Formatter;
    let fmt = Formatter::new();

    println!("{}\n", fmt.title("TypeMill Status"));

    // 1. Check server status
    println!("{}", fmt.header("Server Status"));
    let pid_file = get_pid_file_path();
    let server_running = if pid_file.exists() {
        match std::fs::read_to_string(&pid_file) {
            Ok(pid_str) => {
                if let Ok(pid) = pid_str.trim().parse::<u32>() {
                    if is_process_running(pid) {
                        println!("  {}", fmt.success(&format!("Running (PID: {})", pid)));
                        println!(
                            "  {}",
                            fmt.key_value("PID file", &pid_file.display().to_string())
                        );
                        true
                    } else {
                        println!("  {}", fmt.warning("Not running (stale PID file found)"));
                        let _ = std::fs::remove_file(&pid_file);
                        false
                    }
                } else {
                    println!("  {}", fmt.error("Invalid PID file format"));
                    false
                }
            }
            Err(e) => {
                println!(
                    "  {}",
                    fmt.error(&format!("Failed to read PID file: {}", e))
                );
                false
            }
        }
    } else {
        println!("  {}", fmt.info("Not running"));
        println!("  {}", fmt.subtitle("Start with: mill start"));
        false
    };

    println!();

    // 2. Check configuration
    println!("{}", fmt.header("Configuration"));
    match AppConfig::load() {
        Ok(config) => {
            let config_path = std::path::Path::new(".typemill/config.json");
            println!("  {}", fmt.success("Configuration loaded"));
            println!(
                "  {}",
                fmt.key_value("Path", &config_path.display().to_string())
            );
            println!("  {}", fmt.key_value("Log level", &config.logging.level));
            println!(
                "  {}",
                fmt.key_value("Log format", &format!("{:?}", config.logging.format))
            );

            if let Some(fuse_config) = &config.fuse {
                println!(
                    "  {}",
                    fmt.warning(&format!(
                        "FUSE enabled: {}",
                        fuse_config.mount_point.display()
                    ))
                );
            }

            println!();
            println!("{}", fmt.header("LSP Servers"));

            // Build status summary data
            let mut status_items = Vec::new();
            for server in &config.lsp.servers {
                let cmd = &server.command[0];
                let extensions = server.extensions.join(", ");
                let is_ok = command_exists(cmd);

                status_items.push((cmd.clone(), format!("Extensions: {}", extensions), is_ok));
            }

            println!("{}", fmt.status_summary(&status_items));

            // Show warnings for missing LSPs
            for server in &config.lsp.servers {
                let cmd = &server.command[0];
                if !command_exists(cmd) {
                    println!("  {}", fmt.warning(&format!("'{}' not found in PATH", cmd)));
                }
            }
        }
        Err(e) => {
            println!("  {}", fmt.error(&format!("Configuration error: {}", e)));
            println!("  {}", fmt.subtitle("Run: mill setup"));
        }
    }

    println!();

    // 3. Show all running mill processes (helpful for debugging)
    println!("{}", fmt.header("Running Processes"));
    match find_all_mill_processes() {
        Ok(pids) => {
            if pids.is_empty() {
                println!("  {}", fmt.info("No mill processes found"));
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
                    println!("  • PID: {}{}", pid, marker);
                }
            }
        }
        Err(e) => {
            println!(
                "  {}",
                fmt.warning(&format!("Could not list processes: {}", e))
            );
        }
    }

    println!();
    println!("{}", fmt.success("Status check complete"));
}

/// Handle the doctor command
async fn handle_doctor() {
    use mill_client::formatting::Formatter;
    use mill_foundation::core::utils::system;

    let fmt = Formatter::new();

    println!("{}\n", fmt.title("TypeMill Doctor"));

    // 1. Check for and validate the configuration file
    println!("{}", fmt.header("Configuration File"));
    match AppConfig::load() {
        Ok(config) => {
            println!("  {}\n", fmt.success("Found and parsed successfully"));

            // 2. Check language servers with actual testing
            println!("{}", fmt.header("Language Servers"));
            for server in &config.lsp.servers {
                let cmd = &server.command[0];
                let exts = server.extensions.join(", ");
                println!("  {}", fmt.subtitle(&format!("{} (for {})", cmd, exts)));

                // Test if command actually works
                let (works, info) = system::test_command_with_version(
                    cmd,
                    &server.command[1..]
                        .iter()
                        .map(|s| s.as_str())
                        .collect::<Vec<_>>(),
                );

                if works {
                    // Show version info
                    let version = if info.is_empty() {
                        "unknown version"
                    } else {
                        &info
                    };
                    println!("    {}", fmt.success(&format!("Installed: {}", version)));

                    // Additional checks for TypeScript
                    if server.extensions.contains(&"ts".to_string()) && server.root_dir.is_none() {
                        println!("    {}", fmt.warning("rootDir not set"));
                        println!(
                            "      {}",
                            fmt.subtitle("TypeScript LSP may not find dependencies")
                        );

                        // Suggest rootDir
                        if let Some(detected) =
                            lsp_helpers::detect_typescript_root(std::path::Path::new("."))
                        {
                            println!(
                                "      {}",
                                fmt.info(&format!(
                                    "Suggestion: Set rootDir to '{}'",
                                    detected.display()
                                ))
                            );
                            println!("      {}", fmt.subtitle("Run: mill setup --update"));
                        }
                    }
                } else {
                    // Command doesn't work
                    if std::path::Path::new(cmd).is_absolute() {
                        println!("    {}", fmt.error("Absolute path not found"));
                        println!(
                            "      {}",
                            fmt.subtitle(&format!("File doesn't exist: {}", cmd))
                        );
                    } else {
                        println!("    {}", fmt.error("Not found in PATH"));
                        println!(
                            "      {}",
                            fmt.subtitle(&format!(
                                "Install via: mill install-lsp {}",
                                server.extensions[0]
                            ))
                        );
                    }
                }
                println!();
            }
        }
        Err(e) => {
            println!("  {}", fmt.error(&format!("Error: {}", e)));
            println!(
                "  {}",
                fmt.subtitle("Run `mill setup` to create a new configuration file.")
            );
        }
    }

    println!("{}", fmt.success("Doctor's checkup complete"));
}

/// Handle the install-lsp command
async fn handle_install_lsp(language: &str) {
    println!("📥 Installing LSP server for {}...", language);

    // Check if plugin exists for this language
    let plugin = match lsp_helpers::find_plugin_by_language(language) {
        Some(p) => p,
        None => {
            eprintln!("❌ No plugin found for language: {}", language);
            eprintln!("   Supported languages:");
            let supported = lsp_helpers::list_supported_languages();
            for (lang_name, lsp_name) in &supported {
                eprintln!("   • {} ({})", lang_name, lsp_name);
            }
            process::exit(1);
        }
    };

    // Check if plugin supports LSP installation
    let installer = match lsp_helpers::get_lsp_installer(&*plugin) {
        Some(i) => i,
        None => {
            eprintln!(
                "❌ The {} plugin does not support automatic LSP installation",
                plugin.metadata().name
            );
            process::exit(1);
        }
    };

    let lsp_name = installer.lsp_name();
    println!("   Found: {} ({})", plugin.metadata().name, lsp_name);

    // Check if already installed
    match lsp_helpers::check_lsp_installed(language).await {
        Ok(Some(path)) => {
            println!(
                "✅ {} is already installed at: {}",
                lsp_name,
                path.display()
            );
            return;
        }
        Ok(None) => {
            // Not installed, continue
        }
        Err(e) => {
            error!(error = %e, "Failed to check LSP status");
            eprintln!("⚠️  Warning: Could not check LSP status ({})", e);
            eprintln!("   Proceeding with installation...");
        }
    }

    // Install LSP
    match lsp_helpers::install_lsp(language).await {
        Ok(path) => {
            println!("✅ Successfully installed {} to:", lsp_name);
            println!("   {}", path.display());

            // Update config after install
            let interactive = user_input::is_interactive();
            match lsp_helpers::update_config_after_install(language, &path, interactive).await {
                Ok(()) => {
                    // Config updated successfully (messages already printed by the function)
                }
                Err(e) => {
                    error!(error = %e, "Failed to update config");
                    eprintln!();
                    eprintln!("⚠️  Config update failed: {}", e);
                    eprintln!("   You may need to manually update .typemill/config.json");
                }
            }
        }
        Err(e) => {
            error!(error = %e, lsp_name, "Failed to install LSP");
            eprintln!("❌ Installation failed: {}", e);
            process::exit(1);
        }
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
    use mill_client::formatting::Formatter;
    let fmt = Formatter::new();

    let dispatcher = match crate::dispatcher_factory::create_initialized_dispatcher().await {
        Ok(d) => d,
        Err(e) => {
            eprintln!("{}", fmt.error(&format!("Error initializing: {}", e)));
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
            println!("{}\n", fmt.title("Available MCP Tools"));

            // Build table data
            let headers = vec!["Tool Name", "Handler"];
            let rows: Vec<Vec<String>> = tools_with_handlers
                .iter()
                .map(|(name, handler)| vec![name.clone(), handler.clone()])
                .collect();

            println!("{}", fmt.table(&headers, &rows));

            // Summary
            let handler_count = tools_with_handlers
                .iter()
                .map(|(_, h)| h)
                .collect::<std::collections::HashSet<_>>()
                .len();

            println!(
                "{}",
                fmt.info(&format!(
                    "Public tools: {} across {} handlers",
                    tools_with_handlers.len(),
                    handler_count
                ))
            );
            println!(
                "{}",
                fmt.subtitle("(Internal tools hidden - 20 backend-only tools not shown)")
            );
        }
    }
}

/// Handle the tool command - call MCP tool directly
#[allow(clippy::too_many_arguments)]
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
    let arguments: serde_json::Value =
        if let Some(file_path) = input_file {
            // Read JSON from file
            let json = match std::fs::read_to_string(file_path) {
                Ok(content) => content,
                Err(e) => {
                    let error = mill_foundation::core::model::mcp::McpError::invalid_request(
                        format!("Failed to read input file '{}': {}", file_path, e),
                    );
                    let api_error = MillError::from(error);
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
                    let api_error = MillError::from(error);
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
                    let api_error = MillError::from(error);
                    output_error(&api_error, format);
                    process::exit(1);
                }
                match serde_json::from_str(&stdin_content) {
                    Ok(val) => val,
                    Err(e) => {
                        let error = mill_foundation::core::model::mcp::McpError::invalid_request(
                            format!("Invalid JSON from stdin: {}", e),
                        );
                        let api_error = MillError::from(error);
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
                        let api_error = MillError::from(error);
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
                    let error = MillError::InvalidRequest {
                        message: format!("Invalid flag arguments: {}", e),
                        parameter: Some("arguments".to_string()),
                    };
                    output_error(&error, format);
                    process::exit(1);
                }
            }
        };

    // Construct MCP request message
    use mill_foundation::core::model::mcp::{McpMessage, McpRequest};
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

    // Try daemon first (Unix only), then fall back to in-process dispatcher
    #[cfg(unix)]
    let response = {
        use mill_transport::{default_socket_path, is_daemon_running, UnixSocketClient};

        let socket_path = default_socket_path();
        if is_daemon_running(&socket_path).await {
            // Use daemon for faster execution (LSP servers already running)
            match UnixSocketClient::connect(&socket_path).await {
                Ok(mut client) => match client.call(message.clone()).await {
                    Ok(resp) => Some(resp),
                    Err(e) => {
                        eprintln!(
                            "⚠️  Daemon connection error, falling back to in-process: {}",
                            e
                        );
                        None
                    }
                },
                Err(e) => {
                    eprintln!(
                        "⚠️  Could not connect to daemon, falling back to in-process: {}",
                        e
                    );
                    None
                }
            }
        } else {
            None
        }
    };

    #[cfg(not(unix))]
    let response: Option<McpMessage> = None;

    // If daemon didn't handle it, use in-process dispatcher
    let response = if let Some(resp) = response {
        resp
    } else {
        // Initialize dispatcher via factory
        let dispatcher = match crate::dispatcher_factory::create_initialized_dispatcher().await {
            Ok(d) => d,
            Err(e) => {
                let error = MillError::internal(format!("Failed to initialize: {}", e));
                output_error(&error, format);
                process::exit(1);
            }
        };

        let session_info = SessionInfo::default();

        // Execute tool call via dispatcher
        let result = dispatcher.dispatch(message, &session_info).await;

        // Shutdown dispatcher before continuing
        if let Err(e) = dispatcher.shutdown().await {
            tracing::warn!(error = %e, "Failed to shutdown dispatcher cleanly");
        }

        match result {
            Ok(resp) => resp,
            Err(server_error) => {
                let api_error = MillError::internal(server_error.to_string());
                output_error(&api_error, format);
                process::exit(1);
            }
        }
    };

    // Process the response
    match response {
        McpMessage::Response(resp) => {
            if let Some(result) = resp.result {
                output_result(&result, format);
            } else if let Some(error) = resp.error {
                let api_error = MillError::from(error);
                output_error(&api_error, format);
                process::exit(1);
            }
        }
        _ => {
            eprintln!("Unexpected response type");
            process::exit(1);
        }
    }
}

/// Handle daemon commands (Unix only)
#[cfg(unix)]
async fn handle_daemon_command(command: DaemonCommands) {
    use mill_transport::{default_socket_path, is_daemon_running};
    use nix::sys::signal::{self, Signal};
    use nix::unistd::Pid;

    let socket_path = default_socket_path();
    let pid_path = socket_path.with_extension("pid");

    match command {
        DaemonCommands::Start { foreground } => {
            // Check if daemon is already running
            if is_daemon_running(&socket_path).await {
                eprintln!("❌ Daemon is already running");
                eprintln!("   Socket: {}", socket_path.display());
                eprintln!("   Use 'mill daemon stop' to stop it first");
                process::exit(1);
            }

            if foreground {
                // Run in foreground (useful for debugging)
                println!("🚀 Starting daemon in foreground mode...");
                println!("   Socket: {}", socket_path.display());
                println!("   Press Ctrl+C to stop");

                run_daemon_server(&socket_path, &pid_path).await;
            } else {
                // Daemonize: fork and run in background
                println!("🚀 Starting daemon in background...");
                println!("   Socket: {}", socket_path.display());

                // For now, just run in foreground with a message
                // True daemonization requires fork() which is complex
                // Users can use `mill daemon start --foreground &` for now
                eprintln!("   Note: Use 'mill daemon start --foreground &' for background mode");
                eprintln!("   or run with 'nohup mill daemon start --foreground &'");

                run_daemon_server(&socket_path, &pid_path).await;
            }
        }
        DaemonCommands::Stop => {
            if !is_daemon_running(&socket_path).await {
                eprintln!("❌ Daemon is not running");
                process::exit(1);
            }

            // Try to read PID file and send SIGTERM
            if let Ok(pid_str) = std::fs::read_to_string(&pid_path) {
                if let Ok(pid) = pid_str.trim().parse::<i32>() {
                    println!("🛑 Stopping daemon (PID: {})...", pid);
                    match signal::kill(Pid::from_raw(pid), Signal::SIGTERM) {
                        Ok(_) => {
                            // Wait a moment for graceful shutdown
                            tokio::time::sleep(std::time::Duration::from_millis(500)).await;

                            if is_daemon_running(&socket_path).await {
                                // Force kill if still running
                                let _ = signal::kill(Pid::from_raw(pid), Signal::SIGKILL);
                                tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                            }

                            // Clean up socket and pid files
                            let _ = std::fs::remove_file(&socket_path);
                            let _ = std::fs::remove_file(&pid_path);

                            println!("✅ Daemon stopped");
                        }
                        Err(e) => {
                            eprintln!("❌ Failed to stop daemon: {}", e);
                            // Try to clean up anyway
                            let _ = std::fs::remove_file(&socket_path);
                            let _ = std::fs::remove_file(&pid_path);
                            process::exit(1);
                        }
                    }
                } else {
                    eprintln!("❌ Invalid PID file");
                    // Clean up stale files
                    let _ = std::fs::remove_file(&socket_path);
                    let _ = std::fs::remove_file(&pid_path);
                    process::exit(1);
                }
            } else {
                // No PID file, but socket exists - clean up
                eprintln!("⚠️  No PID file found, cleaning up stale socket");
                let _ = std::fs::remove_file(&socket_path);
                println!("✅ Cleaned up");
            }
        }
        DaemonCommands::Status => {
            if is_daemon_running(&socket_path).await {
                println!("✅ Daemon is running");
                println!("   Socket: {}", socket_path.display());
                if let Ok(pid_str) = std::fs::read_to_string(&pid_path) {
                    println!("   PID: {}", pid_str.trim());
                }
            } else {
                println!("❌ Daemon is not running");
                if socket_path.exists() {
                    println!("   (stale socket file exists - will be cleaned on next start)");
                }
            }
        }
    }
}

/// Run the daemon server (used by handle_daemon_command)
#[cfg(unix)]
async fn run_daemon_server(socket_path: &std::path::Path, pid_path: &std::path::Path) {
    use mill_transport::UnixSocketServer;
    use std::io::Write;

    // Initialize dispatcher
    let dispatcher = match crate::dispatcher_factory::create_initialized_dispatcher().await {
        Ok(d) => d,
        Err(e) => {
            eprintln!("❌ Failed to initialize dispatcher: {}", e);
            process::exit(1);
        }
    };

    // Write PID file
    let pid = std::process::id();
    if let Some(parent) = pid_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(mut file) = std::fs::File::create(pid_path) {
        let _ = writeln!(file, "{}", pid);
    }

    // Create and run server
    let server = match UnixSocketServer::bind(socket_path).await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("❌ Failed to bind socket: {}", e);
            let _ = std::fs::remove_file(pid_path);
            process::exit(1);
        }
    };

    println!("✅ Daemon started (PID: {})", pid);

    // Handle shutdown signal
    let socket_path_clone = socket_path.to_path_buf();
    let pid_path_clone = pid_path.to_path_buf();
    let dispatcher_clone = dispatcher.clone();

    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        println!("\n🛑 Shutting down daemon...");

        // Shutdown dispatcher
        if let Err(e) = dispatcher_clone.shutdown().await {
            eprintln!("Warning: Failed to shutdown dispatcher: {}", e);
        }

        // Clean up files
        let _ = std::fs::remove_file(&socket_path_clone);
        let _ = std::fs::remove_file(&pid_path_clone);

        println!("✅ Daemon stopped");
        process::exit(0);
    });

    // Run the server (blocks until shutdown)
    if let Err(e) = server.run(dispatcher).await {
        eprintln!("❌ Server error: {}", e);
        let _ = std::fs::remove_file(pid_path);
        process::exit(1);
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
    use glob::glob;
    use mill_foundation::core::model::mcp::{McpMessage, McpRequest};
    use serde_json::json;

    // Scan files matching glob pattern
    let matches: Vec<String> = match glob(glob_pattern) {
        Ok(paths) => paths
            .filter_map(|p| p.ok())
            .filter_map(|p| p.to_str().map(String::from))
            .collect(),
        Err(e) => {
            let error = MillError::InvalidRequest {
                message: format!("Invalid glob pattern '{}': {}", glob_pattern, e),
                parameter: Some("glob_pattern".to_string()),
            };
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
            if let Some(new_filename) =
                conventions::convert_filename(fname, from_convention, to_convention)
            {
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
    println!(
        "🔄 Converting {} {} from {} to {}",
        targets.len(),
        target_type,
        from_convention,
        to_convention
    );
    println!();
    for target in &targets {
        println!(
            "  {} → {}",
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
            let error = MillError::internal(format!("Failed to initialize: {}", e));
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
                    match dispatcher
                        .dispatch(McpMessage::Request(apply_request), &session_info)
                        .await
                    {
                        Ok(McpMessage::Response(apply_response)) => {
                            if apply_response.error.is_some() {
                                eprintln!("❌ Failed to apply renames");
                                output_error(
                                    &MillError::internal(format!("{:?}", apply_response.error)),
                                    format,
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
fn output_error(error: &MillError, format: &str) {
    let error_json = serde_json::json!({
        "error": error.to_string()
    });

    let output = match format {
        "compact" => serde_json::to_string(&error_json).unwrap_or_else(|_| "{}".to_string()),
        _ => serde_json::to_string_pretty(&error_json).unwrap_or_else(|_| "{}".to_string()),
    };
    eprintln!("{}", output);
}
