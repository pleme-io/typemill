use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
    routing::get,
    Router,
};
use cb_server::handlers::plugin_dispatcher::{AppState, PluginDispatcher};
use cb_server::systems::LspManager;
use cb_core::config::LspConfig;
use clap::{Parser, Subcommand};
use std::sync::Arc;
use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader};
use tracing::{debug, error, info};

#[derive(Parser)]
#[command(name = "codeflow-buddy")]
#[command(about = "Pure Rust MCP server bridging Language Server Protocol functionality")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the MCP server in stdio mode for Claude Code
    Start,
    /// Start WebSocket server
    Serve,
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

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Start => {
            debug!("Starting MCP server in stdio mode");
            run_stdio_mode().await;
        }
        Commands::Serve => {
            debug!("Starting WebSocket server");
            run_websocket_server().await;
        }
        Commands::Status => {
            info!("Status: Running");
        }
        Commands::Setup => {
            info!("Setup: Not implemented");
        }
        Commands::Stop => {
            info!("Stop: Not implemented");
        }
        Commands::Link => {
            info!("Link: Not implemented");
        }
        Commands::Unlink => {
            info!("Unlink: Not implemented");
        }
    }
}

async fn run_stdio_mode() {
    debug!("Initializing stdio mode MCP server");
    debug!("Current working directory in run_stdio_mode: {:?}", std::env::current_dir());

    // Create AppState similar to the test implementation
    let app_state = match create_app_state().await {
        Ok(state) => state,
        Err(e) => {
            error!("Failed to create app state: {}", e);
            return;
        }
    };

    let dispatcher = Arc::new(PluginDispatcher::new(app_state));
    debug!("About to call dispatcher.initialize()");
    if let Err(e) = dispatcher.initialize().await {
        error!("Failed to initialize dispatcher: {}", e);
        return;
    }
    debug!("Plugin dispatcher initialized successfully");

    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut reader = BufReader::new(stdin);

    debug!("Starting stdio message loop");
    loop {
        let mut line = String::new();
        match reader.read_line(&mut line).await {
            Ok(0) => {
                debug!("EOF received, exiting");
                break; // EOF
            }
            Ok(_) => {
                debug!("Received message: {}", line.trim());
                match serde_json::from_str(&line) {
                    Ok(mcp_message) => {
                        debug!("Parsed MCP message, dispatching");
                        match dispatcher.dispatch(mcp_message).await {
                            Ok(response) => {
                                let response_json = match serde_json::to_string(&response) {
                                    Ok(json) => json,
                                    Err(e) => {
                                        error!("Failed to serialize response: {}", e);
                                        continue;
                                    }
                                };
                                debug!("Sending response: {}", response_json);
                                if let Err(e) = stdout.write_all(response_json.as_bytes()).await {
                                    error!("Error writing to stdout: {}", e);
                                    break;
                                }
                                if let Err(e) = stdout.write_all(b"\n").await {
                                    error!("Error writing newline: {}", e);
                                    break;
                                }
                                if let Err(e) = stdout.flush().await {
                                    error!("Error flushing stdout: {}", e);
                                    break;
                                }
                            }
                            Err(e) => {
                                error!("Error dispatching message: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to parse JSON: {}", e);
                    }
                }
            }
            Err(e) => {
                error!("Error reading from stdin: {}", e);
                break;
            }
        }
    }
    debug!("Stdio mode exiting");
}

async fn run_websocket_server() {
    // Create AppState similar to the test implementation
    let app_state = match create_app_state().await {
        Ok(state) => state,
        Err(e) => {
            error!("Failed to create app state: {}", e);
            return;
        }
    };

    let dispatcher = Arc::new(PluginDispatcher::new(app_state));
    if let Err(e) = dispatcher.initialize().await {
        error!("Failed to initialize dispatcher: {}", e);
        return;
    }

    let app = Router::new().route("/ws", get(ws_handler)).with_state(dispatcher);

    let listener = match tokio::net::TcpListener::bind("127.0.0.1:3000").await {
        Ok(listener) => listener,
        Err(e) => {
            error!("Failed to bind to 127.0.0.1:3000: {}", e);
            return;
        }
    };

    let addr = match listener.local_addr() {
        Ok(addr) => addr,
        Err(e) => {
            error!("Failed to get local address: {}", e);
            return;
        }
    };
    info!("Listening on {}", addr);

    if let Err(e) = axum::serve(listener, app).await {
        error!("Server error: {}", e);
    }
}

async fn create_app_state() -> Result<Arc<AppState>, std::io::Error> {
    let lsp_config = LspConfig::default();
    let lsp_manager = Arc::new(LspManager::new(lsp_config));

    // Use current working directory as project root for production
    let project_root = std::env::current_dir()?;
    debug!("Server project_root set to: {}", project_root.display());

    let file_service = Arc::new(cb_server::services::FileService::new(project_root.clone()));
    let lock_manager = Arc::new(cb_server::services::LockManager::new());
    let operation_queue = Arc::new(cb_server::services::OperationQueue::new(lock_manager.clone()));

    Ok(Arc::new(AppState {
        lsp: lsp_manager,
        file_service,
        project_root,
        lock_manager,
        operation_queue,
    }))
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(dispatcher): State<Arc<PluginDispatcher>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, dispatcher))
}

async fn handle_socket(mut socket: WebSocket, dispatcher: Arc<PluginDispatcher>) {
    loop {
        match socket.recv().await {
            Some(Ok(Message::Text(text))) => {
                let response = match serde_json::from_str(&text) {
                    Ok(mcp_message) => dispatcher.dispatch(mcp_message).await,
                    Err(e) => {
                        // Handle deserialization error
                        tracing::error!("Failed to deserialize message: {}", e);
                        continue;
                    }
                };

                match response {
                    Ok(response_message) => {
                        let response_text = match serde_json::to_string(&response_message) {
                            Ok(text) => text,
                            Err(e) => {
                                error!("Failed to serialize response: {}", e);
                                continue;
                            }
                        };
                        if socket.send(Message::Text(response_text.into())).await.is_err() {
                            break; // client disconnected
                        }
                    }
                    Err(e) => {
                        // Handle dispatch error
                        tracing::error!("Error dispatching message: {}", e);
                    }
                }
            }
            Some(Ok(Message::Close(_))) | None => {
                break; // client disconnected
            }
            Some(Ok(_)) => {
                // Ignore other message types (binary, ping, pong)
            }
            Some(Err(e)) => {
                tracing::error!("WebSocket error: {}", e);
                break;
            }
        }
    }
}