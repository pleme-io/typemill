//! cb-client: Command-line client implementation for Codebuddy
//!
//! This crate provides the client-side functionality for interacting with the
//! Codebuddy server, including WebSocket communication, interactive command
//! handling, configuration management, and user-friendly output formatting.
//! It enables developers to leverage all server capabilities through a clean CLI interface.

// Prevent technical debt accumulation
#![warn(clippy::unwrap_used)]
#![warn(clippy::expect_used)]

pub mod client_config;
pub mod commands;
pub mod error;
pub mod formatting;
pub mod interactive;
pub mod websocket;

pub use client_config::{ClientConfig, ConfigBuilder};
pub use error::{ClientError, ClientResult};

use serde::{Deserialize, Serialize};

/// Session report summarizing client operations
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SessionReport {
    /// Total number of operations attempted
    pub total_operations: u64,
    /// Number of successful operations
    pub successful_operations: u64,
    /// Number of failed operations
    pub failed_operations: u64,
    /// Session start time
    pub session_start: chrono::DateTime<chrono::Utc>,
    /// Session end time (if session has ended)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_end: Option<chrono::DateTime<chrono::Utc>>,
    /// Total session duration in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    /// Connection information
    pub connection_info: ConnectionInfo,
    /// Error summary
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<ErrorSummary>,
}

/// Connection information for the session
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionInfo {
    /// Server URL
    pub server_url: String,
    /// Whether authentication was used
    pub authenticated: bool,
    /// Number of reconnection attempts
    pub reconnection_attempts: u32,
    /// Whether the connection is currently active
    pub active: bool,
}

/// Error summary for reporting
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ErrorSummary {
    /// Error type/category
    pub error_type: String,
    /// Error message
    pub message: String,
    /// Number of times this error occurred
    pub count: u32,
    /// First occurrence timestamp
    pub first_seen: chrono::DateTime<chrono::Utc>,
    /// Last occurrence timestamp
    pub last_seen: chrono::DateTime<chrono::Utc>,
}

impl SessionReport {
    /// Create a new session report
    pub fn new(server_url: String, authenticated: bool) -> Self {
        Self {
            total_operations: 0,
            successful_operations: 0,
            failed_operations: 0,
            session_start: chrono::Utc::now(),
            session_end: None,
            duration_ms: None,
            connection_info: ConnectionInfo {
                server_url,
                authenticated,
                reconnection_attempts: 0,
                active: true,
            },
            errors: Vec::new(),
        }
    }

    /// Record a successful operation
    pub fn record_success(&mut self) {
        self.total_operations += 1;
        self.successful_operations += 1;
    }

    /// Record a failed operation
    pub fn record_failure(&mut self, error: ClientError) {
        self.total_operations += 1;
        self.failed_operations += 1;
        self.add_error(error);
    }

    /// Add an error to the error summary
    pub fn add_error(&mut self, error: ClientError) {
        let error_type = match error {
            ClientError::ConfigError(_) => "ConfigError",
            ClientError::ConnectionError(_) => "ConnectionError",
            ClientError::AuthError(_) => "AuthError",
            ClientError::TimeoutError(_) => "TimeoutError",
            ClientError::RequestError(_) => "RequestError",
            ClientError::SerializationError(_) => "SerializationError",
            ClientError::IoError(_) => "IoError",
            ClientError::TransportError(_) => "TransportError",
            ClientError::ProtocolError(_) => "ProtocolError",
            ClientError::Core(_) => "CoreError",
        }
        .to_string();

        let message = error.to_string();
        let now = chrono::Utc::now();

        // Try to find existing error of same type and message
        if let Some(existing) = self
            .errors
            .iter_mut()
            .find(|e| e.error_type == error_type && e.message == message)
        {
            existing.count += 1;
            existing.last_seen = now;
        } else {
            self.errors.push(ErrorSummary {
                error_type,
                message,
                count: 1,
                first_seen: now,
                last_seen: now,
            });
        }
    }

    /// Record a reconnection attempt
    pub fn record_reconnection(&mut self) {
        self.connection_info.reconnection_attempts += 1;
    }

    /// Mark the session as ended
    pub fn end_session(&mut self) {
        let now = chrono::Utc::now();
        self.session_end = Some(now);
        self.duration_ms = Some((now - self.session_start).num_milliseconds() as u64);
        self.connection_info.active = false;
    }

    /// Get success rate as a percentage
    pub fn success_rate(&self) -> f64 {
        if self.total_operations == 0 {
            0.0
        } else {
            (self.successful_operations as f64 / self.total_operations as f64) * 100.0
        }
    }

    /// Check if the session is currently active
    pub fn is_active(&self) -> bool {
        self.session_end.is_none() && self.connection_info.active
    }
}

impl ConnectionInfo {
    /// Create new connection info
    pub fn new(server_url: String, authenticated: bool) -> Self {
        Self {
            server_url,
            authenticated,
            reconnection_attempts: 0,
            active: true,
        }
    }
}

impl ErrorSummary {
    /// Create a new error summary
    pub fn new(error_type: String, message: String) -> Self {
        let now = chrono::Utc::now();
        Self {
            error_type,
            message,
            count: 1,
            first_seen: now,
            last_seen: now,
        }
    }
}

use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use commands::call::{CallCommand, OutputFormat};
use commands::connect::ConnectCommand;
use commands::doctor::DoctorCommand;
use commands::setup::SetupCommand;
use commands::status::StatusCommand;
use commands::{Command, GlobalArgs};
use std::time::Duration;

/// A powerful, interactive client for the Codebuddy server.
#[derive(Parser, Debug)]
#[command(name = "codebuddy")]
#[command(about = "Codebuddy Client - Connect to and interact with codebuddy servers", long_about = None)]
#[command(version)]
#[command(propagate_version = true)]
pub struct CliArgs {
    #[command(subcommand)]
    pub command: Commands,

    /// Enable debug logging for detailed diagnostic output.
    #[arg(short, long, global = true)]
    pub debug: bool,

    /// Path to a custom configuration file.
    #[arg(short, long, global = true, help_heading = "Connection")]
    pub config: Option<String>,

    /// Request timeout in milliseconds. Overrides config file and environment variables.
    #[arg(short, long, global = true, help_heading = "Connection")]
    pub timeout: Option<u64>,

    /// Disable colored output.
    #[arg(long, global = true, help_heading = "Display")]
    pub no_color: bool,

    /// Disable emoji icons in output.
    #[arg(long, global = true, help_heading = "Display")]
    pub no_emoji: bool,
}

/// Defines the available subcommands for the CLI.
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Run an interactive setup wizard to create a configuration file.
    #[command(
        long_about = "Guides you through setting up a connection to a Codebuddy server and saves the settings to a configuration file."
    )]
    Setup,

    /// Connect to the server and start an interactive session.
    #[command(
        long_about = "Establishes a persistent WebSocket connection to the server for interactive operations."
    )]
    Connect {
        /// The WebSocket URL of the server (e.g., ws://localhost:3000). Overrides config and environment variables.
        #[arg(short, long, help_heading = "Connection")]
        url: Option<String>,

        /// The authentication token for the server. Overrides config and environment variables.
        #[arg(short, long, help_heading = "Connection")]
        token: Option<String>,

        /// Disable automatic reconnection if the connection is lost.
        #[arg(long)]
        no_auto_reconnect: bool,

        /// Auto-disconnect after a specified number of seconds of inactivity.
        #[arg(long)]
        session_timeout: Option<u64>,
    },

    /// Execute a raw MCP tool on the server.
    #[command(
        long_about = "Execute a raw MCP tool on the server. This is useful for scripting and advanced operations."
    )]
    #[command(
        after_help = "Example: codebuddy tool read_file '{\"file_path\":\"/path/to/file.txt\"}'"
    )]
    Call {
        /// The name of the MCP tool to execute (e.g., `read_file`, `get_hover`).
        tool: String,

        /// The parameters for the tool, provided as a single JSON string.
        params: Option<String>,

        /// The WebSocket URL of the server. Overrides all other settings.
        #[arg(short, long, help_heading = "Connection")]
        url: Option<String>,

        /// The authentication token for the server. Overrides all other settings.
        #[arg(short, long, help_heading = "Connection")]
        token: Option<String>,

        /// The output format for the command's result.
        #[arg(short, long, default_value = "pretty", value_enum)]
        format: OutputFormatArg,

        /// Read the JSON parameters from a specified file instead of the command line.
        #[arg(long, conflicts_with = "params_stdin")]
        params_file: Option<String>,

        /// Read the JSON parameters from standard input (stdin).
        #[arg(long, conflicts_with = "params_file")]
        params_stdin: bool,
    },

    /// Check client status and verify connectivity to the server.
    #[command(
        long_about = "Performs a health check on the client and attempts to connect to the server to verify configuration and connectivity."
    )]
    Status {
        /// The WebSocket URL of the server to check.
        #[arg(short, long, help_heading = "Connection")]
        url: Option<String>,

        /// The authentication token to use for the check.
        #[arg(short, long, help_heading = "Connection")]
        token: Option<String>,

        /// Show detailed information, including configuration sources and values.
        #[arg(short, long)]
        verbose: bool,
    },

    /// Manage MCP server presets.
    #[cfg(feature = "mcp-proxy")]
    #[command(
        long_about = "Add, list, and manage MCP server presets for easy integration with external tools."
    )]
    Mcp {
        #[command(subcommand)]
        command: McpCommands,
    },

    /// Check client configuration and diagnose potential problems.
    #[command(
        long_about = "Performs a series of checks to validate the configuration, find required executables, and ensure the client is ready to connect to the server."
    )]
    Doctor,

    /// Generate shell completion scripts.
    #[command(long_about = "Generate shell completion scripts for your shell.
To use, add `source <(codebuddy completions <shell>)` to your shell's startup file.")]
    Completions {
        /// The shell to generate completions for.
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
}

/// MCP preset subcommands
#[cfg(feature = "mcp-proxy")]
#[derive(Debug, Subcommand)]
pub enum McpCommands {
    /// List available MCP presets
    List,
    /// Add an MCP preset to the configuration
    Add {
        /// The preset ID to add (e.g., context7, git, filesystem)
        preset_id: String,
    },
    /// Remove an MCP preset from the configuration
    Remove {
        /// The preset ID to remove (e.g., context7, git, filesystem)
        preset_id: String,
    },
    /// Show detailed information about an MCP preset
    Info {
        /// The preset ID to show info for (e.g., context7, git, filesystem)
        preset_id: String,
    },
}

/// Output format for the `call` command.
#[derive(Debug, Clone, ValueEnum)]
pub enum OutputFormatArg {
    /// Pretty-printed with colors and formatting.
    Pretty,
    /// Raw JSON output.
    Json,
    /// Minimal output, showing only the raw result data.
    Raw,
}

impl From<OutputFormatArg> for OutputFormat {
    fn from(arg: OutputFormatArg) -> Self {
        match arg {
            OutputFormatArg::Pretty => OutputFormat::Pretty,
            OutputFormatArg::Json => OutputFormat::Json,
            OutputFormatArg::Raw => OutputFormat::Raw,
        }
    }
}

/// Run the CLI application.
pub async fn run_cli() -> ClientResult<()> {
    let args = CliArgs::parse();

    // Initialize logging if debug is enabled
    if args.debug {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::DEBUG)
            .init();
        tracing::debug!("Debug mode enabled");
    }

    // Create global arguments
    let global_args = GlobalArgs {
        debug: args.debug,
        config_path: args.config,
        timeout: args.timeout,
        no_color: args.no_color,
        no_emoji: args.no_emoji,
    };

    // Dispatch to appropriate command
    let result = match args.command {
        Commands::Setup => {
            let cmd = SetupCommand::new();
            cmd.execute(&global_args).await
        }
        Commands::Connect {
            url,
            token,
            no_auto_reconnect,
            session_timeout,
        } => {
            let mut cmd = ConnectCommand::new(url, token).with_auto_reconnect(!no_auto_reconnect);

            if let Some(timeout_secs) = session_timeout {
                cmd = cmd.with_session_timeout(Duration::from_secs(timeout_secs));
            }

            cmd.execute(&global_args).await
        }
        Commands::Call {
            tool,
            params,
            url,
            token,
            format,
            params_file,
            params_stdin,
        } => {
            let mut cmd = CallCommand::new(tool, params).with_format(format.into());

            if let Some(url) = url {
                cmd = cmd.with_url(url);
            }
            if let Some(token) = token {
                cmd = cmd.with_token(token);
            }
            if let Some(file) = params_file {
                cmd = cmd.with_params_file(file);
            }
            if params_stdin {
                cmd = cmd.with_params_stdin();
            }

            cmd.execute(&global_args).await
        }
        Commands::Status {
            url,
            token,
            verbose,
        } => {
            let cmd = StatusCommand::new(url, token).with_verbose(verbose);
            cmd.execute(&global_args).await
        }
        #[cfg(feature = "mcp-proxy")]
        Commands::Mcp { command } => match command {
            McpCommands::List => commands::mcp::list_presets()
                .map_err(|e| ClientError::RequestError(format!("Failed to list presets: {}", e))),
            McpCommands::Add { preset_id } => commands::mcp::add_preset(&preset_id)
                .map_err(|e| ClientError::RequestError(format!("Failed to add preset: {}", e))),
            McpCommands::Remove { preset_id } => commands::mcp::remove_preset(&preset_id)
                .map_err(|e| ClientError::RequestError(format!("Failed to remove preset: {}", e))),
            McpCommands::Info { preset_id } => {
                commands::mcp::info_preset(&preset_id).map_err(|e| {
                    ClientError::RequestError(format!("Failed to show preset info: {}", e))
                })
            }
        },
        Commands::Doctor => {
            let cmd = DoctorCommand::new();
            cmd.execute(&global_args).await
        }
        Commands::Completions { shell } => {
            let mut cmd = CliArgs::command();
            let name = cmd.get_name().to_string();
            clap_complete::generate(shell, &mut cmd, name, &mut std::io::stdout());
            Ok(())
        }
    };

    // Handle results and exit codes
    match result {
        Ok(()) => {
            if global_args.debug {
                tracing::debug!("Command completed successfully");
            }
            Ok(())
        }
        Err(e) => {
            // Error handling - always display error to stderr
            let formatter =
                formatting::Formatter::with_settings(!global_args.no_color, !global_args.no_emoji);
            eprintln!("{}", formatter.client_error(&e));

            if global_args.debug {
                tracing::error!("Command failed: {:?}", e);
            }

            // Set appropriate exit codes
            match e {
                ClientError::ConfigError(_) => std::process::exit(2),
                ClientError::ConnectionError(_) => std::process::exit(3),
                ClientError::AuthError(_) => std::process::exit(4),
                ClientError::TimeoutError(_) => std::process::exit(5),
                ClientError::RequestError(_) => std::process::exit(6),
                ClientError::SerializationError(_) => std::process::exit(7),
                ClientError::IoError(_) => std::process::exit(8),
                ClientError::TransportError(_) => std::process::exit(9),
                ClientError::ProtocolError(_) => std::process::exit(10),
                ClientError::Core(_) => std::process::exit(11),
            }
        }
    }
}

/// Convenience function to create a client config from args
pub fn create_client_config_from_args(
    url: Option<String>,
    token: Option<String>,
    timeout: Option<u64>,
) -> ClientResult<ClientConfig> {
    let mut config = ClientConfig::new();

    if let Some(url) = url {
        config.set_url(url);
    }
    if let Some(token) = token {
        config.set_token(token);
    }
    if let Some(timeout) = timeout {
        config.set_timeout_ms(timeout);
    }

    config.validate()?;
    Ok(config)
}

/// Get version information
pub fn version_info() -> String {
    format!(
        "{} {} ({})",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
        env!("CARGO_PKG_DESCRIPTION")
    )
}

/// Check if running in a terminal (for color/emoji detection)
pub fn is_terminal() -> bool {
    use std::io::IsTerminal;
    std::io::stdout().is_terminal()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_global_args_creation() {
        let global_args = GlobalArgs {
            debug: true,
            config_path: Some("/path/to/config".to_string()),
            timeout: Some(30000),
            no_color: false,
            no_emoji: false,
        };

        assert!(global_args.debug);
        assert_eq!(global_args.config_path, Some("/path/to/config".to_string()));
        assert_eq!(global_args.timeout, Some(30000));
        assert!(!global_args.no_color);
        assert!(!global_args.no_emoji);
    }

    #[test]
    fn test_output_format_conversion() {
        assert!(matches!(
            OutputFormat::from(OutputFormatArg::Pretty),
            OutputFormat::Pretty
        ));
        assert!(matches!(
            OutputFormat::from(OutputFormatArg::Json),
            OutputFormat::Json
        ));
        assert!(matches!(
            OutputFormat::from(OutputFormatArg::Raw),
            OutputFormat::Raw
        ));
    }

    #[test]
    fn test_client_config_creation() {
        let config = create_client_config_from_args(
            Some("ws://localhost:3000".to_string()),
            Some("test-token".to_string()),
            Some(60000),
        )
        .unwrap();

        assert_eq!(config.get_url().unwrap(), "ws://localhost:3000");
        assert_eq!(config.get_token(), Some("test-token"));
        assert_eq!(config.get_timeout_ms(), 60000);
    }

    #[test]
    fn test_version_info() {
        let version = version_info();
        assert!(version.contains("cb-client"));
        assert!(version.contains("1.0.0-beta"));
    }
}
