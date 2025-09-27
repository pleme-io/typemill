//! cb-client: Codeflow Buddy client implementation

pub mod error;
pub mod client_config;
pub mod websocket;
pub mod formatting;
pub mod interactive;
pub mod commands;

pub use error::{ClientError, ClientResult};
pub use client_config::{ClientConfig, ConfigBuilder};

use commands::{Command, GlobalArgs};
use commands::setup::SetupCommand;
use commands::connect::ConnectCommand;
use commands::call::{CallCommand, OutputFormat};
use commands::status::StatusCommand;
use clap::{Parser, Subcommand, ValueEnum};
use std::time::Duration;

/// CLI arguments
#[derive(Parser, Debug)]
#[command(name = "codeflow-buddy")]
#[command(about = "Codeflow Buddy Client - Connect to and interact with codeflow-buddy servers", long_about = None)]
#[command(version)]
pub struct CliArgs {
    #[command(subcommand)]
    pub command: Commands,

    /// Enable debug output
    #[arg(short, long, global = true)]
    pub debug: bool,

    /// Custom configuration file path
    #[arg(short, long, global = true)]
    pub config: Option<String>,

    /// Request timeout in milliseconds
    #[arg(short, long, global = true)]
    pub timeout: Option<u64>,

    /// Disable colored output
    #[arg(long, global = true)]
    pub no_color: bool,

    /// Disable emoji in output
    #[arg(long, global = true)]
    pub no_emoji: bool,
}

/// CLI subcommands
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Interactive setup wizard for configuration
    Setup,

    /// Connect to server and run interactive session
    Connect {
        /// Server URL override
        #[arg(short, long)]
        url: Option<String>,

        /// Authentication token override
        #[arg(short, long)]
        token: Option<String>,

        /// Disable auto-reconnect on connection loss
        #[arg(long)]
        no_auto_reconnect: bool,

        /// Session timeout in seconds (auto-disconnect after inactivity)
        #[arg(long)]
        session_timeout: Option<u64>,
    },

    /// Call a specific MCP tool
    Call {
        /// Tool name to call
        tool: String,

        /// JSON parameters (optional)
        params: Option<String>,

        /// Server URL override
        #[arg(short, long)]
        url: Option<String>,

        /// Authentication token override
        #[arg(short, long)]
        token: Option<String>,

        /// Output format
        #[arg(short, long, default_value = "pretty")]
        format: OutputFormatArg,

        /// Read parameters from file
        #[arg(long)]
        params_file: Option<String>,

        /// Read parameters from stdin
        #[arg(long)]
        params_stdin: bool,
    },

    /// Show client status and verify connectivity
    Status {
        /// Server URL override
        #[arg(short, long)]
        url: Option<String>,

        /// Authentication token override
        #[arg(short, long)]
        token: Option<String>,

        /// Show detailed information
        #[arg(short, long)]
        verbose: bool,
    },
}

/// Output format for call command
#[derive(Debug, Clone, ValueEnum)]
pub enum OutputFormatArg {
    /// Pretty-printed with colors and formatting
    Pretty,
    /// Raw JSON output
    Json,
    /// Minimal output (result only)
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

/// Run the CLI application
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
        Commands::Connect { url, token, no_auto_reconnect, session_timeout } => {
            let mut cmd = ConnectCommand::new(url, token)
                .with_auto_reconnect(!no_auto_reconnect);

            if let Some(timeout_secs) = session_timeout {
                cmd = cmd.with_session_timeout(Duration::from_secs(timeout_secs));
            }

            cmd.execute(&global_args).await
        }
        Commands::Call { tool, params, url, token, format, params_file, params_stdin } => {
            let mut cmd = CallCommand::new(tool, params)
                .with_format(format.into());

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
        Commands::Status { url, token, verbose } => {
            let cmd = StatusCommand::new(url, token)
                .with_verbose(verbose);
            cmd.execute(&global_args).await
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
            let formatter = formatting::Formatter::with_settings(!global_args.no_color, !global_args.no_emoji);
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
pub fn create_client_config_from_args(url: Option<String>, token: Option<String>, timeout: Option<u64>) -> ClientResult<ClientConfig> {
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
    atty::is(atty::Stream::Stdout)
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
        assert!(matches!(OutputFormat::from(OutputFormatArg::Pretty), OutputFormat::Pretty));
        assert!(matches!(OutputFormat::from(OutputFormatArg::Json), OutputFormat::Json));
        assert!(matches!(OutputFormat::from(OutputFormatArg::Raw), OutputFormat::Raw));
    }

    #[test]
    fn test_client_config_creation() {
        let config = create_client_config_from_args(
            Some("ws://localhost:3000".to_string()),
            Some("test-token".to_string()),
            Some(60000)
        ).unwrap();

        assert_eq!(config.get_url().unwrap(), "ws://localhost:3000");
        assert_eq!(config.get_token(), Some("test-token"));
        assert_eq!(config.get_timeout_ms(), 60000);
    }

    #[test]
    fn test_version_info() {
        let version = version_info();
        assert!(version.contains("cb-client"));
        assert!(version.contains("0.1.0"));
    }
}