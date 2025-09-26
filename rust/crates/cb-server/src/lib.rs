//! cb-server: Codeflow Buddy server implementation

pub mod error;
pub mod handlers;
pub mod interfaces;

pub use error::{ServerError, ServerResult};
pub use interfaces::{AstService, LspService};

use cb_core::AppConfig;
use tokio::sync::oneshot;

/// Server configuration options
#[derive(Debug, Clone)]
pub struct ServerOptions {
    /// Application configuration
    pub config: AppConfig,
    /// Enable debug mode
    pub debug: bool,
}

/// Handle to a running server
#[derive(Debug)]
pub struct ServerHandle {
    shutdown_tx: oneshot::Sender<()>,
}

/// Bootstrap the server with given options
pub async fn bootstrap(options: ServerOptions) -> ServerResult<ServerHandle> {
    tracing::info!("Bootstrapping Codeflow Buddy server");

    // Validate configuration
    if options.config.server.port == 0 {
        return Err(ServerError::config("Invalid server port"));
    }

    // Create shutdown channel
    let (shutdown_tx, _shutdown_rx) = oneshot::channel();

    // In a real implementation, this would:
    // 1. Initialize LSP clients
    // 2. Set up MCP handlers
    // 3. Start transport layers (HTTP, WebSocket)
    // 4. Initialize FUSE filesystem (if configured)
    // 5. Set up monitoring and health checks

    tracing::info!("Server bootstrapped successfully");

    Ok(ServerHandle { shutdown_tx })
}

impl ServerOptions {
    /// Create server options from app config
    pub fn from_config(config: AppConfig) -> Self {
        Self {
            config,
            debug: false,
        }
    }

    /// Enable debug mode
    pub fn with_debug(mut self, debug: bool) -> Self {
        self.debug = debug;
        self
    }
}

impl ServerHandle {
    /// Start the server (async)
    pub async fn start(&self) -> ServerResult<()> {
        tracing::info!("Starting server...");

        // In a real implementation, this would start all services
        // For now, just log that we're "running"

        tracing::info!("Server started successfully");
        Ok(())
    }

    /// Shutdown the server gracefully
    pub async fn shutdown(self) -> ServerResult<()> {
        tracing::info!("Shutting down server...");

        // Send shutdown signal
        if let Err(_) = self.shutdown_tx.send(()) {
            tracing::warn!("Server already shut down");
        }

        // In a real implementation, this would:
        // 1. Stop accepting new connections
        // 2. Finish processing existing requests
        // 3. Clean up resources
        // 4. Unmount FUSE filesystem

        tracing::info!("Server shut down successfully");
        Ok(())
    }
}