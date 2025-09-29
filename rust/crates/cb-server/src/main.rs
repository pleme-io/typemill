//! cb-server main binary

use cb_ast::AstCache;
use cb_core::{AppConfig, config::LogFormat};
use cb_server::handlers::{AppState, PluginDispatcher};
use cb_api::AstService;
use cb_server::services::{DefaultAstService, FileService, LockManager, OperationQueue};
use cb_vfs::start_fuse_mount;
use cb_transport;
use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};
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

    // Get project root
    let project_root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    // Create shared AST cache for performance optimization
    let ast_cache = Arc::new(AstCache::new());

    // Create services
    let ast_service: Arc<dyn AstService> = Arc::new(DefaultAstService::new(ast_cache.clone()));
    let lock_manager = Arc::new(LockManager::new());
    let file_service = Arc::new(FileService::new(
        &project_root,
        ast_cache.clone(),
        lock_manager.clone(),
    ));
    let operation_queue = Arc::new(OperationQueue::new(lock_manager.clone()));

    // Create application state
    let app_state = Arc::new(AppState {
        ast_service,
        file_service,
        project_root,
        lock_manager,
        operation_queue,
    });

    // Create Plugin dispatcher with app state
    let dispatcher = PluginDispatcher::new(app_state);

    let dispatcher = Arc::new(dispatcher);

    // Start FUSE filesystem if enabled (only for WebSocket server)
    if matches!(cli.command, Some(Commands::Serve) | None) {
        if let Some(fuse_config) = &config.fuse {
            let workspace_path = Path::new(".");
            tracing::info!(
                "FUSE enabled, mounting filesystem at {:?}",
                fuse_config.mount_point
            );

            if let Err(e) = start_fuse_mount(fuse_config, workspace_path) {
                tracing::error!("Failed to start FUSE mount: {}", e);
                // Continue without FUSE - it's not critical for core functionality
            } else {
                tracing::info!("FUSE filesystem mounted successfully");
            }
        }
    }

    // Execute based on command
    match cli.command {
        Some(Commands::Start) => {
            // Start stdio MCP server
            tracing::info!("Starting stdio MCP server");
            if let Err(e) = cb_transport::start_stdio_server(dispatcher).await {
                tracing::error!("Failed to start stdio server: {}", e);
                return Err(e);
            }
        }
        Some(Commands::Serve) | None => {
            // Start WebSocket server (default)
            tracing::info!(
                "Starting WebSocket server on {}:{}",
                config.server.host,
                config.server.port
            );

            if let Err(e) = cb_transport::start_ws_server(config, dispatcher).await {
                tracing::error!("Failed to start WebSocket server: {}", e);
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

    // Parse log level from config, with fallback to INFO
    let log_level = config.logging.level.parse()
        .unwrap_or(tracing::Level::INFO);

    // Create env filter with configured level and allow env overrides
    let env_filter = tracing_subscriber::EnvFilter::from_default_env()
        .add_directive(log_level.into());

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
