//! cb-server main binary

use cb_core::AppConfig;
use cb_server::handlers::{PluginDispatcher, AppState};
use cb_server::systems::{LspManager, fuse::start_fuse_mount};
use cb_server::services::{FileService, LockManager, OperationQueue};
use cb_server::transport;
use clap::{Parser, Subcommand};
use std::sync::Arc;
use std::path::{Path, PathBuf};
use tracing_subscriber;

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
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Parse command line arguments
    let cli = Cli::parse();

    tracing::info!("Starting Codeflow Buddy Server");

    // Load configuration
    let config = Arc::new(AppConfig::load()?);

    // Create LSP manager
    let lsp_manager = Arc::new(LspManager::new(config.lsp.clone()));

    // Get project root
    let project_root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    // Create file service
    let file_service = Arc::new(FileService::new(&project_root));

    // Create lock manager and operation queue
    let lock_manager = Arc::new(LockManager::new());
    let operation_queue = Arc::new(OperationQueue::new(lock_manager.clone()));

    // Create application state
    let app_state = Arc::new(AppState {
        lsp: lsp_manager,
        file_service,
        project_root,
        lock_manager,
        operation_queue,
    });

    // Create Plugin dispatcher
    let dispatcher = PluginDispatcher::new();

    let dispatcher = Arc::new(dispatcher);

    // Start FUSE filesystem if enabled (only for WebSocket server)
    if matches!(cli.command, Some(Commands::Serve) | None) {
        if let Some(fuse_config) = &config.fuse {
            let workspace_path = Path::new(".");
            tracing::info!("FUSE enabled, mounting filesystem at {:?}", fuse_config.mount_point);

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
            if let Err(e) = transport::start_stdio_server(dispatcher).await {
                tracing::error!("Failed to start stdio server: {}", e);
                return Err(e);
            }
        }
        Some(Commands::Serve) | None => {
            // Start WebSocket server (default)
            tracing::info!("Starting WebSocket server on {}:{}", config.server.host, config.server.port);

            if let Err(e) = transport::start_ws_server(config, dispatcher).await {
                tracing::error!("Failed to start WebSocket server: {}", e);
                return Err(e.into());
            }
        }
    }

    tracing::info!("Server stopped");
    Ok(())
}