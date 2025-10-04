//! cb-server main binary

use cb_core::{config::LogFormat, AppConfig};
use clap::{Parser, Subcommand};
use std::sync::Arc;

#[derive(Parser)]
#[command(name = "cb-server")]
#[command(about = "Codeflow Buddy Server")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start MCP server on stdio
    Start,
    /// Start WebSocket server (default)
    Serve,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse command line arguments first
    let cli = Cli::parse();

    // Load configuration
    let config = Arc::new(AppConfig::load()?);

    // Initialize tracing based on configuration
    initialize_tracing(&config);

    tracing::info!("Starting Codeflow Buddy Server");

    // Create workspace manager for tracking connected containers
    let workspace_manager = Arc::new(cb_core::workspaces::WorkspaceManager::new());

    // Create dispatcher using shared library function (reduces duplication)
    let dispatcher = cb_server::create_dispatcher_with_workspace(config.clone(), workspace_manager)
        .await
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

    // Execute based on command
    match cli.command {
        Some(Commands::Start) => {
            // Start stdio MCP server
            tracing::info!("Starting stdio MCP server");
            if let Err(e) = cb_transport::start_stdio_server(dispatcher).await {
                tracing::error!(
                    error_category = "transport_error",
                    error = %e,
                    "Failed to start stdio server"
                );
                return Err(e);
            }
        }
        Some(Commands::Serve) | None => {
            // Start admin server on a separate port
            let admin_port = config.server.port + 1000; // Admin on port+1000
            let admin_config = config.clone();
            let admin_workspace_manager = Arc::new(cb_server::workspaces::WorkspaceManager::new());
            tokio::spawn(async move {
                if let Err(e) = cb_transport::start_admin_server(
                    admin_port,
                    admin_config,
                    admin_workspace_manager,
                )
                .await
                {
                    tracing::error!(
                        error_category = "admin_server_error",
                        error = %e,
                        admin_port = admin_port,
                        "Failed to start admin server"
                    );
                }
            });

            // Start WebSocket server (default)
            tracing::info!(
                "Starting WebSocket server on {}:{}",
                config.server.host,
                config.server.port
            );
            tracing::info!("Admin endpoints available on 127.0.0.1:{}", admin_port);

            if let Err(e) = cb_transport::start_ws_server(config, dispatcher).await {
                tracing::error!(
                    error_category = "transport_error",
                    error = %e,
                    "Failed to start WebSocket server"
                );
                return Err(e.into());
            }
        }
    }

    tracing::info!("Server stopped");
    Ok(())
}

/// Initialize tracing based on configuration
fn initialize_tracing(config: &AppConfig) {
    use tracing_subscriber::{fmt, prelude::*};

    // Parse log level from config
    let log_level = match config.logging.level.to_lowercase().as_str() {
        "trace" => tracing::Level::TRACE,
        "debug" => tracing::Level::DEBUG,
        "info" => tracing::Level::INFO,
        "warn" => tracing::Level::WARN,
        "error" => tracing::Level::ERROR,
        _ => {
            eprintln!(
                "Invalid log level '{}', falling back to INFO",
                config.logging.level
            );
            tracing::Level::INFO
        }
    };

    // Create env filter with configured level and allow env overrides (RUST_LOG takes precedence)
    let env_filter =
        tracing_subscriber::EnvFilter::from_default_env().add_directive(log_level.into());

    // Use configured format
    // IMPORTANT: Always write logs to stderr to keep stdout clean for JSON-RPC messages
    match config.logging.format {
        LogFormat::Json => {
            // Use JSON formatter for structured logging
            tracing_subscriber::registry()
                .with(env_filter)
                .with(
                    fmt::layer()
                        .with_ansi(false)
                        .compact()
                        .with_writer(std::io::stderr),
                )
                .init();
        }
        LogFormat::Pretty => {
            // Use pretty (human-readable) formatter
            tracing_subscriber::registry()
                .with(env_filter)
                .with(fmt::layer().pretty().with_writer(std::io::stderr))
                .init();
        }
    }
}
