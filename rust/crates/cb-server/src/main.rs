//! cb-server main binary

use cb_api::AstService;
use cb_ast::AstCache;
use cb_core::{config::LogFormat, AppConfig};
use cb_plugins::PluginManager;
use cb_server::handlers::{AppState, PluginDispatcher};
use cb_server::services::{
    planner::{DefaultPlanner, Planner},
    workflow_executor::{DefaultWorkflowExecutor, WorkflowExecutor},
    DefaultAstService, FileService, LockManager, OperationQueue,
};
use cb_transport;
use cb_vfs::start_fuse_mount;
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
    let planner: Arc<dyn Planner> = DefaultPlanner::new();
    let file_service = Arc::new(FileService::new(
        &project_root,
        ast_cache.clone(),
        lock_manager.clone(),
    ));
    let operation_queue = Arc::new(OperationQueue::new(lock_manager.clone()));

    // Create plugin manager (needed by both dispatcher and workflow executor)
    let plugin_manager = Arc::new(PluginManager::new());

    // Create workflow executor with plugin manager
    let workflow_executor: Arc<dyn WorkflowExecutor> =
        DefaultWorkflowExecutor::new(plugin_manager.clone());

    // Create application state
    let app_state = Arc::new(AppState {
        ast_service,
        file_service,
        planner,
        workflow_executor,
        project_root,
        lock_manager,
        operation_queue,
        start_time: std::time::Instant::now(),
    });

    // Create Plugin dispatcher with app state and plugin manager
    let dispatcher = PluginDispatcher::new(app_state, plugin_manager);

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
                tracing::error!(
                    error_category = "fuse_error",
                    error = %e,
                    "Failed to start FUSE mount"
                );
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
            tokio::spawn(async move {
                if let Err(e) = cb_transport::start_admin_server(admin_port).await {
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
                .with(fmt::layer().pretty())
                .init();
        }
    }
}
